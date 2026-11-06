"""Ensure explicit local Ely patches do not hide unreviewed snapshot changes."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def digest(value):
    return hashlib.sha256(value.encode()).hexdigest()


class ElySnapshotTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.snapshot = self.root / "vendor/ely"
        (self.snapshot / "src").mkdir(parents=True)
        (self.snapshot / "assets").mkdir()
        (self.root / ".github/scripts").mkdir(parents=True)
        self.script = self.root / ".github/scripts/check-ely-snapshot.py"
        shutil.copyfile(ROOT / ".github/scripts/check-ely-snapshot.py", self.script)
        shutil.copyfile(ROOT / "Cargo.toml", self.root / "Cargo.toml")
        shutil.copyfile(ROOT / "vendor/ely/Cargo.upstream.toml", self.snapshot / "Cargo.upstream.toml")
        provenance = json.loads((ROOT / "vendor/ely/UPSTREAM.json").read_text())
        provenance["files"] = {"LICENSE": digest("original license"), "src/existing.rs": digest("upstream")}
        (self.snapshot / "UPSTREAM.json").write_text(json.dumps(provenance))
        self.patches = {"base_revision": provenance["revision"], "description": "Inspector integration",
                        "files": {"src/existing.rs": digest("patched"), "src/added.rs": digest("added")}}
        self.write_patches()
        for name, content in {"LICENSE": "original license", "src/existing.rs": "patched",
                              "src/added.rs": "added"}.items():
            (self.snapshot / name).write_text(content)

    def write_patches(self):
        (self.snapshot / "LOCAL_PATCHES.json").write_text(json.dumps(self.patches))

    def check(self):
        return subprocess.run([sys.executable, str(self.script)], capture_output=True, text=True)

    def test_declared_changes_and_original_baseline_can_coexist(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_unlisted_files_and_missing_declared_files_are_rejected(self):
        extra = self.snapshot / "src/unreviewed.rs"
        extra.write_text("unreviewed")
        self.assertIn("Snapshot file inventory changed", self.check().stderr)
        extra.unlink()
        (self.snapshot / "src/added.rs").unlink()
        self.assertIn("Snapshot file inventory changed", self.check().stderr)

    def test_changes_to_reviewed_patch_or_original_file_are_rejected(self):
        for name in ["src/existing.rs", "src/added.rs", "LICENSE"]:
            path = self.snapshot / name
            original = path.read_text()
            with self.subTest(name=name):
                path.write_text("unreviewed")
                result = self.check()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"Changed bundled file: {name}", result.stderr)
                path.write_text(original)

    def test_patches_cannot_override_license_or_escape_the_snapshot(self):
        for name in ["LICENSE", "src/../../outside.rs"]:
            with self.subTest(name=name):
                self.patches["files"][name] = digest("unreviewed")
                self.write_patches()
                self.assertIn("Local patches must name Ely source files", self.check().stderr)
                del self.patches["files"][name]

    def test_patch_base_must_match_the_pinned_revision(self):
        self.patches["base_revision"] = "different revision"
        self.write_patches()
        self.assertIn("Local patches target a different Ely revision", self.check().stderr)


if __name__ == "__main__":
    unittest.main()
