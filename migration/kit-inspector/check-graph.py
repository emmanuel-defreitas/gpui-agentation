"""Validate the active graph of the patched cutover consumer."""
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parent
tree = subprocess.check_output([
    sys.executable, str(root / "cargo.py"), "tree", "--locked",
    "--target", "aarch64-apple-darwin", "--prefix", "none", "--format", "{p}",
    "--edges", "normal,build,dev",
], text=True)
packages = set(re.findall(r"^([\w-]+) v(\S+)", tree, re.MULTILINE))
for name, version in {
    "gpui-kit": "0.11.0", "gpui-base": "0.11.0", "gpui-component": "0.11.0",
    "gpui-kit-assets": "0.11.0", "gpui-pre": "0.3.14",
    "gpui-pre-platform": "0.3.14", "gpui-pre-macos": "0.3.14",
}.items():
    assert {v for n, v in packages if n == name} == {version}, name
for name, version in packages:
    assert name != "gpui-agentation" and not name.startswith(("ely", "gpui-fast")), name
    if name.startswith("gpui-pre"):
        assert version == ("0.12.15" if name == "gpui-pre-reqwest" else "0.3.14"), (name, version)
print("Inspector consumer: Kit 0.11.0 / GPUI 0.3.14 only; no Ely or old native workspace")
