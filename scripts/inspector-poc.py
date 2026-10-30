#!/usr/bin/env python3
"""Run Cargo against an isolated, patched copy of the pinned GPUI 0.3.8.

Usage: python3 scripts/inspector-poc.py run -p agentation-consumer
       python3 scripts/inspector-poc.py test --lib external_inspector
The registry source and ordinary Cargo configuration are never modified.
"""

import fcntl
import os
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "target/inspector-poc/gpui-pre"
PATCH = ROOT / "patches/gpui-pre-0.3.8-external-inspector.patch"


def restore_registry_entry(original_lock):
    """Undo only our GPUI source change, preserving unrelated concurrent edits."""
    pattern = r'(?ms)^\[\[package\]\]\nname = "gpui-pre"\nversion = "0\.3\.8"\n.*?(?=^\[\[package\]\]|\Z)'
    original = re.search(pattern, original_lock)
    if not original or 'source = "registry+' not in original[0]:
        return
    lock = ROOT / "Cargo.lock"
    current = lock.read_text()
    patched = re.sub(r"^(?:source|checksum) = .*\n", "", original[0], flags=re.M)
    found = re.search(pattern, current)
    if found and found[0] == patched:
        lock.write_text(current[: found.start()] + original[0] + current[found.end() :])


def prepare():
    marker = DEST / ".agentation-patch"
    if marker.exists() and marker.read_bytes() == PATCH.read_bytes():
        return
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    registry = cargo_home / "registry"
    sources = list((registry / "src").glob("*/gpui-pre-0.3.8"))
    if DEST.exists():
        shutil.rmtree(DEST)
    DEST.parent.mkdir(parents=True, exist_ok=True)
    if sources:
        shutil.copytree(sources[0], DEST)
    else:
        archives = list((registry / "cache").glob("*/gpui-pre-0.3.8.crate"))
        if not archives:
            raise SystemExit("Run cargo fetch --locked first: GPUI 0.3.8 is not cached")
        with tarfile.open(archives[0]) as archive:
            archive.extractall(DEST.parent, filter="data")
        (DEST.parent / "gpui-pre-0.3.8").rename(DEST)
    subprocess.run(["patch", "--batch", "-p1", "-i", str(PATCH)], cwd=DEST, check=True)
    marker.write_bytes(PATCH.read_bytes())


if __name__ == "__main__":
    args = sys.argv[1:] or ["build", "-p", "agentation-consumer"]
    DEST.parent.mkdir(parents=True, exist_ok=True)
    guard = (DEST.parent / "runner.lock").open("w")
    try:
        fcntl.flock(guard, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        raise SystemExit(
            "Another inspector POC command is running; wait for it to finish"
        )
    # Cargo's own build lock serializes compilation with normal Cargo commands.
    # Do not overwrite their unrelated lockfile changes during cleanup.
    original_lock = (ROOT / "Cargo.lock").read_text()
    prepare()
    # Package-scoped builds (such as the example app) enable the library's
    # features through their own dependency; only the root package needs them.
    features = (
        []
        if {"-p", "--package"} & set(args[1:])
        else ["--features", "native,test-support"]
    )
    try:
        result = subprocess.run(
            [
                "cargo",
                args[0],
                "--config",
                f'patch.crates-io.gpui-pre.path="{DEST}"',
                *features,
                *args[1:],
            ],
            cwd=ROOT,
            env={**os.environ, "AGENTATION_EXTERNAL_INSPECTOR": "1"},
        )
    finally:
        restore_registry_entry(original_lock)
    sys.exit(result.returncode)
