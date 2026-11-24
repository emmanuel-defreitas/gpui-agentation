"""Check resolved packages, not inactive optional packages in Cargo.lock."""
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parent
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--manifest-path", str(root / "Cargo.toml"),
    "--locked", "--format-version", "1", "--no-deps",
]))
# Cargo metadata's resolve may contain inactive optional nodes. Cargo tree is
# feature-aware and includes only enabled normal/build/dev edges for the
# evaluated arm64 macOS consumer (Intel native acceptance remains separate).
tree = subprocess.check_output([
    "cargo", "tree", "--manifest-path", str(root / "Cargo.toml"), "--locked",
    "--target", "aarch64-apple-darwin", "--prefix", "none", "--format", "{p}",
    "--edges", "normal,build,dev",
], text=True)
packages = set(re.findall(r"^([\w-]+) v(\S+)", tree, re.MULTILINE))
for name, version in {
    "gpui-kit": "0.11.0", "gpui-base": "0.11.0", "gpui-component": "0.11.0",
    "gpui-kit-assets": "0.11.0", "gpui-pre": "0.3.14",
    "gpui-pre-platform": "0.3.14", "gpui-pre-macos": "0.3.14",
    "gpui-pre-macros": "0.3.14",
}.items():
    found = [v for n, v in packages if n == name]
    assert found == [version], f"Unexpected {name} graph: {found}"
for name, _ in packages:
    assert not (name == "ely" or name.startswith(("ely-", "gpui-fast"))
                or name in {"gpui-agentation", "gpui-agentation-ely"}), name
for name, version in packages:
    if name.startswith("gpui-pre"):
        expected = "0.12.15" if name == "gpui-pre-reqwest" else "0.3.14"
        assert version == expected, f"Mixed GPUI snapshot: {name} {version}"
assert len(metadata["workspace_members"]) == 1
assert metadata["packages"][0]["name"] == "agentation-kit-probe"
print("Kit 0.11.0 / GPUI 0.3.14 only; no Ely, Fast, or production Agentation dependency")
