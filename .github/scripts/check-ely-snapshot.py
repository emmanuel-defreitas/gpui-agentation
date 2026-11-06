"""Verify the pinned Ely baseline and explicitly recorded local patches."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

REVISION = "da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--upstream", type=Path, help="Git checkout containing the pinned commit")
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
snapshot = root / "vendor/ely"
provenance = json.loads((snapshot / "UPSTREAM.json").read_text())
patches = json.loads((snapshot / "LOCAL_PATCHES.json").read_text())
assert patches["base_revision"] == REVISION, "Local patches target a different Ely revision"
assert patches["description"].strip(), "Local patches require a description"
assert all(name.startswith("src/") and ".." not in Path(name).parts for name in patches["files"]), "Local patches must name Ely source files"
assert provenance["revision"] == REVISION, "Unexpected Ely revision"
original = (snapshot / "Cargo.upstream.toml").read_bytes()
assert hashlib.sha256(original).hexdigest() == provenance["manifest_sha256"], "Original manifest changed"
upstream = tomllib.loads(original.decode())
agentation = tomllib.loads((root / "Cargo.toml").read_text())
assert provenance["repository"] == upstream["package"]["repository"]
assert agentation["package"]["metadata"]["bundled-ely"] == {"repository": provenance["repository"], "revision": REVISION, "version": upstream["package"]["version"]}
assert not (snapshot / "Cargo.toml").exists(), "Ely must be bundled, not a nested Cargo package"
assert "ely-gpui-component" not in agentation["dependencies"]
assert not any(spec.get("package") == "gpui-agentation-ely" for spec in agentation["dependencies"].values() if isinstance(spec, dict))
native = agentation["features"]["native"]

def check_dependency(name, expected, actual):
    if isinstance(expected, str):
        expected = {"version": expected}
    assert actual.get("optional") is True, f"Ely dependency must be native-only: {name}"
    for key in ("version", "package", "default-features"):
        assert actual.get(key, True if key == "default-features" else None) == expected.get(key, True if key == "default-features" else None), f"Changed Ely dependency {name}: {key}"
    for feature in expected.get("features", []):
        assert feature in actual.get("features", []) or f"{name}/{feature}" in native, f"Missing pinned Ely feature: {name}/{feature}"
    assert f"dep:{name}" in native, f"Ely dependency not enabled by native: {name}"

for name, expected in upstream["dependencies"].items():
    check_dependency(name, expected, agentation["dependencies"][name])
for target, table in upstream["target"].items():
    for name, expected in table.get("dependencies", {}).items():
        check_dependency(name, expected, agentation["target"][target]["dependencies"][name])
assert "gpui_platform/runtime_shaders" in native, "Preserve Ely's default runtime shaders"
assert "dep:proc-macro2" in native
assert agentation["build-dependencies"]["proc-macro2"]["optional"] is True
assert agentation["features"]["default"] == ["mcp"]
assert agentation["features"]["mcp"] == ["dep:reqwest", "dep:serde_json"]
assert agentation["dependencies"]["gpui"]["version"] == "=0.3.8"
assert agentation["dependencies"]["gpui_platform"]["version"] == "=0.3.8"
files = {p.relative_to(snapshot).as_posix() for folder in ("src", "assets") for p in (snapshot / folder).rglob("*") if p.is_file()} | {"LICENSE"}
expected_files = provenance["files"] | patches["files"]
assert files == set(expected_files), "Snapshot file inventory changed"
for name, expected in expected_files.items():
    assert hashlib.sha256((snapshot / name).read_bytes()).hexdigest() == expected, f"Changed bundled file: {name}"
if args.upstream:
    def git(*arguments):
        return subprocess.check_output(["git", "-C", str(args.upstream), *arguments])
    assert git("show", f"{REVISION}:Cargo.toml") == original, "Upstream manifest mismatch"
    names = set(git("ls-tree", "-r", "--name-only", REVISION, "src", "assets", "LICENSE").decode().splitlines())
    assert names == set(provenance["files"]), "Upstream inventory mismatch"
    # One archive operation avoids spawning a process per source file.
    import io
    import tarfile
    with tarfile.open(fileobj=io.BytesIO(git("archive", REVISION, "src", "assets", "LICENSE"))) as archive:
        for entry in archive:
            if entry.isfile():
                assert hashlib.sha256(archive.extractfile(entry).read()).hexdigest() == provenance["files"][entry.name], f"Upstream mismatch: {entry.name}"
print(f"Verified {len(files)} bundled Ely files against {REVISION} and {len(patches['files'])} recorded local patches; native-only dependencies and GPUI pins preserved")
