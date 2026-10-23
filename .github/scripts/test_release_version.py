"""Exercise release preparation and promotion gates in isolated repositories."""
import contextlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest

import release_version as release


class ReleaseVersionTests(unittest.TestCase):
    def setUp(self):
        directory = self.enterContext(tempfile.TemporaryDirectory())
        self.root = Path(directory)
        self.enterContext(contextlib.redirect_stdout(io.StringIO()))
        for name in release.FILES + ["Cargo.toml", "Cargo.lock", "server.json", "src/overlay/settings.rs"]:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(release.ROOT / name, target)
        self.original = release.current_version(self.root)

    def git(self, *args):
        return release.git(self.root, *args)

    def init_git(self, tag=None):
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Release tests")
        self.git("config", "user.email", "release-tests@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        self.commit("chore: baseline")
        if tag:
            self.git("tag", tag)

    def commit(self, subject, body=None):
        args = ["commit", "--allow-empty", "-m", subject]
        if body:
            args.extend(["-m", body])
        self.git(*args)

    def snapshot(self):
        return {str(path.relative_to(self.root)): path.read_bytes()
                for path in self.root.rglob("*") if path.is_file() and ".git" not in path.parts}

    def test_prepare_syncs_current_references_and_preserves_unrelated_content(self):
        before = self.snapshot()
        manifest = tomllib.loads(before["Cargo.toml"].decode())
        version = f'{int(self.original.split(".")[0]) + 1}.7.3'
        release.prepare(self.root, version)
        release.check_references(self.root)
        after = self.snapshot()
        self.assertEqual(set(before), set(after))
        self.assertEqual(release.current_version(self.root), version)
        updated_manifest = tomllib.loads(after["Cargo.toml"].decode())
        manifest["package"]["version"] = version
        self.assertEqual(manifest, updated_manifest)
        old_lock = tomllib.loads(before["Cargo.lock"].decode())
        new_lock = tomllib.loads(after["Cargo.lock"].decode())
        for package in old_lock["package"]:
            if package["name"] == "gpui-agentation":
                package["version"] = version
        self.assertEqual(old_lock, new_lock)
        registry = json.loads(after["server.json"])
        self.assertEqual(registry["version"], version)
        self.assertEqual(registry["packages"][0]["version"], version)
        for name in release.FILES:
            self.assertNotEqual(before[name], after[name], name)
        self.assertIn("Version 0.3.0 bundles", after["docs/package-layout.md"].decode())
        self.assertEqual(before["src/overlay/settings.rs"], after["src/overlay/settings.rs"])
        release.prepare(self.root, version)
        self.assertEqual(after, self.snapshot(), "Repeated preparation must be idempotent")

    def test_preparation_rejects_invalid_versions_without_writes(self):
        before = self.snapshot()
        for version in ("01.2.3", "1.2", "1.2.3-dev.1", "0.0.0"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                release.prepare(self.root, version)
            self.assertEqual(before, self.snapshot())

    def test_unclassified_reference_stops_preparation_before_any_writes(self):
        readme = self.root / "README.md"
        readme.write_text(readme.read_text(encoding="utf-8") + f"\nUnknown current version {self.original}\n")
        before = self.snapshot()
        with self.assertRaisesRegex(ValueError, "unclassified mention"):
            release.prepare(self.root, "9.8.7")
        self.assertEqual(before, self.snapshot())

    def test_settings_drift_stops_preparation_before_any_writes(self):
        settings = self.root / "src/overlay/settings.rs"
        settings.write_text(settings.read_text(encoding="utf-8").replace('env!("CARGO_PKG_VERSION")', '"0.0.0"'))
        before = self.snapshot()
        with self.assertRaisesRegex(ValueError, "Settings About version"):
            release.prepare(self.root, "9.8.7")
        self.assertEqual(before, self.snapshot())

    def test_bridge_checker_reads_utf8_independently_of_platform_defaults(self):
        script = self.root / ".github/scripts/check-bridge-release.py"
        script.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(release.ROOT / ".github/scripts/check-bridge-release.py", script)
        subprocess.run([sys.executable, "-X", "warn_default_encoding", "-W",
                        "error::EncodingWarning", str(script)], check=True,
                       capture_output=True, encoding="utf-8")

    def test_package_size_guard_rejects_missing_and_oversized_archives(self):
        script = self.root / ".github/scripts/check-bridge-release.py"
        script.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(release.ROOT / ".github/scripts/check-bridge-release.py", script)
        command = [sys.executable, str(script), "--check-package-size"]
        missing = subprocess.run(command, capture_output=True, encoding="utf-8")
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("Package archive missing", missing.stderr)
        archive = self.root / "target/package" / f"gpui-agentation-{self.original}.crate"
        archive.parent.mkdir(parents=True)
        with archive.open("wb") as file:
            file.truncate(10 * 1024 * 1024)
        subprocess.run(command, check=True, capture_output=True, encoding="utf-8")
        with archive.open("ab") as file:
            file.write(b"x")
        oversized = subprocess.run(command, capture_output=True, encoding="utf-8")
        self.assertNotEqual(oversized.returncode, 0)
        self.assertIn("crates.io allows at most", oversized.stderr)

    def test_gate_detects_drift_in_docs_lock_registry_and_settings(self):
        for name in ("README.md", "site/src/pages/index.astro", "Cargo.lock", "server.json", "src/overlay/settings.rs"):
            path = self.root / name
            original = path.read_text(encoding="utf-8")
            if name.endswith("settings.rs"):
                broken = original.replace('env!("CARGO_PKG_VERSION")', '"0.0.0"')
            else:
                broken = original.replace(self.original, "0.0.0")
            with self.subTest(name=name):
                path.write_text(broken)
                with self.assertRaises(ValueError):
                    release.check_references(self.root)
                path.write_text(original)

    def test_no_stable_tags_bootstraps_from_reviewed_cargo_version(self):
        self.init_git()
        self.git("tag", "v99.0.0-dev.1")
        self.assertEqual(release.next_version(self.root), self.original)

    def test_conventional_commits_select_patch_minor_and_major(self):
        self.init_git("v1.2.3")
        self.commit("fix(settings): repair label")
        self.git("tag", "v99.0.0-dev.2")
        self.assertEqual(release.next_version(self.root), "1.2.4")
        self.commit("feat: add annotations")
        self.assertEqual(release.next_version(self.root), "1.3.0")
        self.commit("refactor(api)!: change protocol")
        self.assertEqual(release.next_version(self.root), "2.0.0")

    def test_breaking_footer_selects_major(self):
        self.init_git("v1.2.3")
        self.commit("fix: update protocol", "BREAKING CHANGE: consumers must migrate")
        self.assertEqual(release.next_version(self.root), "2.0.0")

    def test_unreachable_stable_tag_does_not_set_the_baseline(self):
        self.init_git("v1.2.3")
        self.git("switch", "-c", "unreleased")
        self.commit("feat: future work")
        self.git("tag", "v99.0.0")
        self.git("switch", "main")
        self.commit("docs: clarify usage")
        self.assertEqual(release.next_version(self.root), "1.2.4")

    def test_stable_tags_are_sorted_numerically(self):
        self.init_git("v1.2.9")
        self.git("tag", "v1.2.10")
        self.commit("fix: repair bug")
        self.assertEqual(release.next_version(self.root), "1.2.11")

    def test_promotion_requires_the_inferred_version_then_accepts_preparation(self):
        self.init_git(f"v{self.original}")
        self.commit("feat: new capability")
        expected = release.next_version(self.root)
        with self.assertRaisesRegex(ValueError, "Promotion requires"):
            release.check_release(self.root)
        release.prepare(self.root)
        self.assertEqual(release.current_version(self.root), expected)
        release.check_release(self.root)
        self.commit(f"chore(release): prepare {expected}")
        release.check_release(self.root)


if __name__ == "__main__":
    unittest.main()
