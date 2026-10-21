"""Fail when a "tracks the current version" reference disagrees with Cargo.toml.

Historical mentions (for example CHANGELOG.md headings) are intentionally not
scanned. Every reference to the current version in a scanned file must match one
of the patterns below, so a new unclassified mention is reported as well.
"""
from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parents[2]
version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
V = r"(\d+\.\d+\.\d+)"
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
]
FILES = [
    "README.md",
    "docs/consumer-setup.md",
    "docs/package-layout.md",
    "docs/publishing-bridge.md",
    "docs/reference.md",
    "skills/gpui-agentation-setup/SKILL.md",
    "site/src/pages/index.astro",
]
errors = []
for name in FILES:
    text = (root / name).read_text()
    covered = []
    for pattern in PATTERNS:
        for match in re.finditer(pattern, text):
            covered.append(match.span(1))
            if match.group(1) != version:
                line = text.count("\n", 0, match.start(1)) + 1
                errors.append(f"{name}:{line}: {match.group(1)} != Cargo.toml {version}")
    if not covered:
        errors.append(f"{name}: no version references matched; update {Path(__file__).name}")
    for match in re.finditer(re.escape(version), text):
        if not any(start <= match.start() and match.end() <= end for start, end in covered):
            line = text.count("\n", 0, match.start()) + 1
            errors.append(f"{name}:{line}: unclassified mention of {version}; add a pattern or reword")
if errors:
    raise SystemExit("Version references out of sync:\n" + "\n".join(errors))
print(f"Version references match Cargo.toml {version}")
