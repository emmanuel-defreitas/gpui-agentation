"""Prepare and validate release versions without committing or pushing changes."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
STABLE = r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
V = rf"({STABLE})"
PATTERNS = [
    rf'gpui-agentation = \{{ version = "{V}"',
    rf"crates\.io/crates/gpui-agentation/{V}",
    rf"cargo install gpui-agentation --version {V}",
    rf"/versions/{V}",
    rf'class="version">v{V}<',
    rf"for gpui-agentation {V}\.",
    rf"using version {V} or",
    rf"The {V} candidate",
    rf"target the {V} Agentation API",
    rf"Version {V} does not",
    rf"{V} is on \[crates\.io\]",
    rf"> \*\*Version: {V}\.\*\*",
]
FILES = [
    "README.md", "docs/consumer-setup.md", "docs/package-layout.md",
    "docs/publishing-bridge.md", "docs/reference.md",
    "skills/gpui-agentation-setup/SKILL.md", "site/src/pages/index.astro",
]


def current_version(root):
    return tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]


def stable_version(value):
    value = value.removeprefix("v")
    if not re.fullmatch(STABLE, value):
        raise ValueError("Expected a stable version such as 0.5.0 or v0.5.0")
    return value


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def next_version(root, revision="HEAD"):
    """Ignore candidate/unreachable tags; share the production bump rules."""
    tags = [tag for tag in git(root, "tag", "--merged", revision).splitlines()
            if re.fullmatch(rf"v{STABLE}", tag)]
    if not tags:
        return stable_version(current_version(root))
    latest = max(tags, key=lambda tag: tuple(map(int, tag[1:].split("."))))
    major, minor, patch = map(int, latest[1:].split("."))
    subjects = git(root, "log", "--format=%s", f"{latest}..{revision}")
    bodies = git(root, "log", "--format=%b", f"{latest}..{revision}")
    if re.search(r"BREAKING[\s-]+CHANGE", subjects + "\n" + bodies, re.I) or re.search(
        r"^[a-z]+(?:\([^)]*\))?!:", subjects, re.M
    ):
        return f"{major + 1}.0.0"
    if re.search(r"^(feat|feature)(\(|:|!)", subjects, re.M | re.I):
        return f"{major}.{minor + 1}.0"
    return f"{major}.{minor}.{patch + 1}"


def reference_updates(root, version):
    """Only update classified current references, preserving historical versions."""
    old = current_version(root)
    updates, errors = {}, []
    for name in FILES:
        text = (root / name).read_text()
        spans = sorted({match.span(1) for pattern in PATTERNS
                        for match in re.finditer(pattern, text)})
        if not spans:
            errors.append(f"{name}: no current-version references matched")
        for match in re.finditer(rf"(?<![\d.]){re.escape(old)}(?![\d.])", text):
            if match.span() not in spans:
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{name}:{line}: unclassified mention of {old}; classify or reword it")
        for start, end in reversed(spans):
            text = text[:start] + version + text[end:]
        updates[name] = text
    if errors:
        raise ValueError("Version references out of sync:\n" + "\n".join(errors))
    return updates


def version_updates(root, version):
    updates = reference_updates(root, version)
    manifest = (root / "Cargo.toml").read_text()
    section = re.search(r"(?ms)^\[package\]\n.*?(?=^\[|\Z)", manifest)
    if not section:
        raise ValueError("Cargo.toml: missing package table")
    package, count = re.subn(r'^version\s*=\s*"[^"]+"', f'version = "{version}"',
                             section[0], count=1, flags=re.M)
    if count != 1:
        raise ValueError("Cargo.toml: missing package version")
    updates["Cargo.toml"] = manifest[:section.start()] + package + manifest[section.end():]
    lock = (root / "Cargo.lock").read_text()
    lock, count = re.subn(r'(\[\[package\]\]\nname = "gpui-agentation"\nversion = ")[^"]+("\n)',
                          rf"\g<1>{version}\2", lock)
    if count != 1:
        raise ValueError("Cargo.lock: expected exactly one gpui-agentation package")
    updates["Cargo.lock"] = lock
    registry = json.loads((root / "server.json").read_text())
    registry["version"] = version
    entries = [entry for entry in registry["packages"]
               if entry.get("registryType") == "cargo" and entry.get("identifier") == "gpui-agentation"]
    if not entries:
        raise ValueError("server.json: missing gpui-agentation Cargo package")
    for entry in entries:
        entry["version"] = version
    updates["server.json"] = json.dumps(registry, indent=2, ensure_ascii=False) + "\n"
    return updates


def check_references(root):
    version = stable_version(current_version(root))
    errors = []
    for name, expected in version_updates(root, version).items():
        if (root / name).read_text() != expected:
            errors.append(f"{name}: release version must match Cargo.toml {version}")
    settings = (root / "src/overlay/settings.rs").read_text()
    if not re.search(r'"GPUI Agentation \{\}"\s*,\s*env!\("CARGO_PKG_VERSION"\)', settings):
        errors.append("Settings About version must derive from CARGO_PKG_VERSION")
    if errors:
        raise ValueError("Version references out of sync:\n" + "\n".join(errors))
    print(f"Release manifests, docs, site and Settings match Cargo.toml {version}")


def check_release(root, revision="HEAD"):
    check_references(root)
    expected = next_version(root, revision)
    if current_version(root) != expected:
        raise ValueError(f"Promotion requires {expected}; run make release-prepare, review and commit the changes before promotion")
    print(f"Release {expected} matches the conventional-commit promotion version")


def prepare(root, version=None):
    version = stable_version(version) if version else next_version(root)
    if tuple(map(int, version.split("."))) < tuple(map(int, current_version(root).split("."))):
        raise ValueError("Refusing to decrease the package version")
    updates = version_updates(root, version)
    # Compute and validate every edit before writing any file.
    for name, text in updates.items():
        if (root / name).read_text() != text:
            (root / name).write_text(text)
            print(f"Updated {name}")
    check_references(root)
    print("Review the diff and CHANGELOG.md, then commit through dev → next → main.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check")
    commands.add_parser("prepare").add_argument("version", nargs="?")
    for name in ("next", "check-release"):
        commands.add_parser(name).add_argument("--revision", default="HEAD")
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            prepare(ROOT, args.version)
        elif args.command == "check":
            check_references(ROOT)
        elif args.command == "check-release":
            check_release(ROOT, args.revision)
        else:
            print(next_version(ROOT, args.revision))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
