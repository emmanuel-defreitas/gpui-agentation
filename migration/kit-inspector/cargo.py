"""Run this isolated consumer with the reviewed Kit 0.11.0 picking patch.

Never modifies the registry cache. Generated dependency copies live in target/.
Usage: python3 migration/kit-inspector/cargo.py test --locked
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
PATCHES = ROOT / "patches"


def prepare():
    # The existing stock probe supplies a locked, unpatched source inventory.
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--manifest-path", str(ROOT.parent / "kit-probe/Cargo.toml"),
        "--locked", "--format-version", "1",
    ]))
    upstream = json.loads((PATCHES / "upstream.json").read_text())
    flags = []
    for name, expected in upstream.items():
        package = next(p for p in metadata["packages"]
                       if p["name"] == name and p["version"] == expected["version"])
        source = Path(package["manifest_path"]).parent
        patch = PATCHES / f"{name}.patch"
        digest = hashlib.sha256(patch.read_bytes()).hexdigest()
        destination = ROOT / "target" / "kit-patches" / f"{name}-{digest[:16]}"
        stamp = destination / ".agentation-patch"
        generated_hashes = {
            str(path.relative_to(source)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in source.rglob("*") if path.is_file()
        }
        generated_hashes.update(expected["patched_files"])
        valid = stamp.is_file() and stamp.read_text() == digest and all(
            (destination / filename).is_file()
            and hashlib.sha256((destination / filename).read_bytes()).hexdigest() == expected_hash
            for filename, expected_hash in generated_hashes.items()
        )
        if not valid:
            for filename, expected_hash in expected["files"].items():
                original = source / filename
                actual = hashlib.sha256(original.read_bytes()).hexdigest() if original.exists() else None
                if actual != expected_hash:
                    raise RuntimeError(f"Unexpected upstream source: {name}/{filename}")
            if destination.exists():
                shutil.rmtree(destination)
            shutil.copytree(source, destination)
            subprocess.run(["patch", "--batch", "-p1", "-i", str(patch)], cwd=destination, check=True, stdout=sys.stderr)
            stamp.write_text(digest)
        flags += ["--config", f"patch.crates-io.{name}.path={json.dumps(str(destination))}"]
    return flags


def main():
    if len(sys.argv) < 2:
        raise SystemExit("Usage: cargo.py <cargo command> [args ...]")
    flags = prepare()
    command, *arguments = sys.argv[1:]
    args = ["cargo", command, "--manifest-path", str(ROOT / "Cargo.toml"), *flags, *arguments]
    raise SystemExit(subprocess.call(args, env=os.environ.copy()))


if __name__ == "__main__":
    main()
