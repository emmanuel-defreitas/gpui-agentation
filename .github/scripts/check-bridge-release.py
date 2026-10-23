"""Validate package/Registry release identity; no credentials are read here."""
import argparse
import json
import os
from pathlib import Path
import re
import tomllib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--require-publishable", action="store_true")
parser.add_argument("--check-package-size", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
package = manifest["package"]
registry = json.loads((root / "server.json").read_text(encoding="utf-8"))
candidate = os.environ.get("BRIDGE_RELEASE_VERSION", package["version"])
version = candidate.removeprefix("v")
if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
    raise SystemExit("Expected a stable version such as 0.1.0 or v0.1.0")
if not version == package["version"] == registry["version"] == registry["packages"][0]["version"]:
    raise SystemExit("Release version, package version, and Registry versions must match")
if package["name"] != "gpui-agentation":
    raise SystemExit("Expected the combined gpui-agentation package")
if manifest["features"]["default"] != ["mcp"] or not any(
    target["name"] == "gpui-agentation" and target.get("required-features") == ["mcp"]
    for target in manifest["bin"]
):
    raise SystemExit("Expected a portable default MCP binary and opt-in native library")
entry = registry["packages"][0]
if entry["registryType"] != "cargo" or entry["identifier"] != package["name"] or entry["transport"]["type"] != "stdio":
    raise SystemExit("Registry package must identify the Cargo/stdio bridge")
marker = f'mcp-name: {registry["name"]}'
if not any(line.startswith("- MCP Registry name:") and marker in line for line in (root / "README.md").read_text(encoding="utf-8").splitlines()):
    raise SystemExit("Cargo README must contain the visible MCP ownership marker")
if args.require_publishable and package.get("publish") is False:
    raise SystemExit("Publication blocked: package publishing is explicitly disabled")
for specification in manifest.get("dependencies", {}).values():
    if args.require_publishable and isinstance(specification, dict) and ("git" in specification or "path" in specification) and not specification.get("version"):
        raise SystemExit("Publication requires registry-compatible dependencies, including optional native dependencies")
if args.check_package_size:
    archive = root / "target" / "package" / f'{package["name"]}-{version}.crate'
    if not archive.is_file():
        raise SystemExit("Package archive missing; run cargo package --no-verify --all-features --locked first")
    size = archive.stat().st_size
    limit = 10 * 1024 * 1024
    if size > limit:
        raise SystemExit(f"Package archive is {size} bytes; crates.io allows at most {limit} bytes")
    print(f"Package archive size: {size} / {limit} bytes")
print(f'Validated {package["name"]} {version} and Registry metadata')
