"""Validate package/Registry release identity; no credentials are read here."""
import argparse
import json
import os
from pathlib import Path
import re
import tomllib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--require-publishable", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
manifest = tomllib.loads((root / "Cargo.toml").read_text())
package = manifest["package"]
registry = json.loads((root / "server.json").read_text())
candidate = os.environ.get("BRIDGE_RELEASE_VERSION", package["version"])
version = candidate.removeprefix("v")
if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
    raise SystemExit("Expected a stable version such as 0.1.0 or v0.1.0")
if not version == package["version"] == registry["version"] == registry["packages"][0]["version"]:
    raise SystemExit("Release version, package version, and Registry versions must match")
if package["name"] != "gpui-agentation":
    raise SystemExit("Expected the combined gpui-agentation package")
if manifest["features"]["default"] != ["native"] or not any(
    target["name"] == "gpui-agentation" and target.get("required-features") == ["mcp"]
    for target in manifest["bin"]
):
    raise SystemExit("Expected a native library default and an opt-in MCP binary")
entry = registry["packages"][0]
if entry["registryType"] != "cargo" or entry["identifier"] != package["name"] or entry["transport"]["type"] != "stdio":
    raise SystemExit("Registry package must identify the Cargo/stdio bridge")
marker = f'mcp-name: {registry["name"]}'
if not any(line.startswith("- MCP Registry name:") and marker in line for line in (root / "README.md").read_text().splitlines()):
    raise SystemExit("Cargo README must contain the visible MCP ownership marker")
if args.require_publishable and package.get("publish") is False:
    raise SystemExit("Publication blocked: resolve the pinned Ely Git dependency before enabling publication")
for specification in manifest.get("dependencies", {}).values():
    if args.require_publishable and isinstance(specification, dict) and ("git" in specification or "path" in specification) and not specification.get("version"):
        raise SystemExit("Publication requires registry-compatible dependencies, including optional native dependencies")
print(f'Validated {package["name"]} {version} and Registry metadata')
