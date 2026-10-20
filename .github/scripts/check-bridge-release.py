"""Validate package/Registry release identity; no credentials are read here."""
import json
import os
from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parents[2] / "crates/gpui-agentation-mcp"
manifest = tomllib.loads((root / "Cargo.toml").read_text())
package = manifest["package"]
registry = json.loads((root / "server.json").read_text())
candidate = os.environ.get("BRIDGE_RELEASE_VERSION", package["version"])
version = candidate.removeprefix("v")
if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
    raise SystemExit("Expected a stable version such as 0.1.0 or v0.1.0")
if not version == package["version"] == registry["version"] == registry["packages"][0]["version"]:
    raise SystemExit("Release version, package version, and Registry versions must match")
if package["name"] != "gpui-agentation-mcp" or package.get("publish") is False:
    raise SystemExit("Only the standalone gpui-agentation-mcp crate may be published")
entry = registry["packages"][0]
if entry["registryType"] != "cargo" or entry["identifier"] != package["name"] or entry["transport"]["type"] != "stdio":
    raise SystemExit("Registry package must identify the Cargo/stdio bridge")
marker = f'mcp-name: {registry["name"]}'
if not any(line.startswith("- MCP Registry name:") and marker in line for line in (root / "README.md").read_text().splitlines()):
    raise SystemExit("Cargo README must contain the visible MCP ownership marker")
for specification in manifest.get("dependencies", {}).values():
    if isinstance(specification, dict) and ("git" in specification or "path" in specification):
        raise SystemExit("Bridge publishing requires registry dependencies only")
print(f'Validated {package["name"]} {version} and Registry metadata')
