"""Verify the immutable Ely runtime snapshot and its packaging-only changes."""
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
assert provenance["revision"] == REVISION, "Unexpected Ely revision"
original = (snapshot / "Cargo.upstream.toml").read_bytes()
assert hashlib.sha256(original).hexdigest() == provenance["manifest_sha256"], "Original manifest changed"
upstream = tomllib.loads(original.decode())
manifest = tomllib.loads((snapshot / "Cargo.toml").read_text())
assert provenance["repository"] == upstream["package"]["repository"]
assert manifest["package"]["name"] == "gpui-agentation-ely"
assert manifest["package"]["version"] == upstream["package"]["version"] == "0.1.1"
assert manifest["package"]["license"] == upstream["package"]["license"] == "MIT"
assert manifest["package"].get("publish") is not False
assert manifest["lib"]["name"] == "ely_gpui_component"
assert manifest["package"]["metadata"]["upstream"]["revision"] == REVISION
assert manifest["lints"] == upstream["lints"], "Ely lint configuration changed"
assert manifest["features"] == upstream["features"], "Ely features changed"
assert manifest["dependencies"] == upstream["dependencies"], "Ely dependencies changed"
for target in upstream["target"]:
    assert manifest["target"][target].get("dependencies", {}) == upstream["target"][target].get("dependencies", {}), "Ely target dependencies changed"
assert set(manifest["target"]) == set(upstream["target"])
for dependency in ("gpui", "gpui_platform"):
    assert manifest["dependencies"][dependency]["version"] == "=0.3.8"
files = {p.relative_to(snapshot).as_posix() for folder in ("src", "assets") for p in (snapshot / folder).rglob("*") if p.is_file()} | {"LICENSE"}
assert files == set(provenance["files"]), "Snapshot file inventory changed"
for name, expected in provenance["files"].items():
    assert hashlib.sha256((snapshot / name).read_bytes()).hexdigest() == expected, f"Changed upstream file: {name}"
if args.upstream:
    def git(*arguments):
        return subprocess.check_output(["git", "-C", str(args.upstream), *arguments])
    assert git("show", f"{REVISION}:Cargo.toml") == original, "Upstream manifest mismatch"
    names = set(git("ls-tree", "-r", "--name-only", REVISION, "src", "assets", "LICENSE").decode().splitlines())
    assert names == files, "Upstream inventory mismatch"
    # One archive operation avoids spawning a process per source file.
    import io
    import tarfile
    with tarfile.open(fileobj=io.BytesIO(git("archive", REVISION, "src", "assets", "LICENSE"))) as archive:
        for entry in archive:
            if entry.isfile():
                assert hashlib.sha256(archive.extractfile(entry).read()).hexdigest() == provenance["files"][entry.name], f"Upstream mismatch: {entry.name}"
agentation = tomllib.loads((root / "Cargo.toml").read_text())
assert agentation["dependencies"]["ely-gpui-component"] == {
    "package": "gpui-agentation-ely", "version": "=0.1.1", "path": "vendor/ely", "optional": True,
}, "Agentation must use the exact snapshot"
assert agentation["dependencies"]["gpui"]["version"] == "=0.3.8"
print(f"Verified {len(files)} unchanged Ely files at {REVISION}; GPUI pins preserved")
