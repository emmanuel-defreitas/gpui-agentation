#!/usr/bin/env python3
"""Assemble the example app into a macOS .app bundle with the project icon.

A bare `cargo run` launches an unbundled binary, so the Dock shows a generic
icon. This builds `agentation-consumer`, turns `example/assets/icon.png` into an
`.icns`, wraps the binary in `target/<name>.app`, and opens it.

Usage: python3 scripts/bundle-example.py [--release] [--no-open]

Only `sips` and `iconutil` (shipped with macOS) are required; cargo-bundle is
not. The `[package.metadata.bundle]` block in example/Cargo.toml mirrors this
layout for anyone who does have cargo-bundle installed.
"""

import argparse
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICON_SOURCE = ROOT / "example/assets/icon.png"
BINARY = "agentation-consumer"
APP_NAME = "Agentation Example"
IDENTIFIER = "com.agentation.consumer-example"

# Standard iconset entries whose pixel size does not upscale the 512px source.
ICONSET = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
]


def crate_version() -> str:
    text = (ROOT / "example/Cargo.toml").read_text()
    match = re.search(r'(?m)^\s*version\s*=\s*"([^"]+)"', text)
    return match[1] if match else "0.1.0"


def build_icns(destination: Path) -> None:
    if not ICON_SOURCE.exists():
        raise SystemExit(f"Missing icon source: {ICON_SOURCE}")
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for name, size in ICONSET:
            subprocess.run(
                ["sips", "-z", str(size), str(size), str(ICON_SOURCE),
                 "--out", str(iconset / name)],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(destination)],
            check=True,
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true",
                        help="build the release profile (annotations are stripped)")
    parser.add_argument("--no-open", action="store_true",
                        help="assemble the bundle without launching it")
    args = parser.parse_args()

    if sys.platform != "darwin":
        raise SystemExit("The macOS .app bundle can only be built on macOS.")

    profile = ["--release"] if args.release else []
    subprocess.run(
        ["cargo", "build", "-p", BINARY, "--locked", *profile],
        cwd=ROOT, check=True,
    )
    built = ROOT / "target" / ("release" if args.release else "debug") / BINARY

    app = ROOT / "target" / f"{APP_NAME}.app"
    macos = app / "Contents" / "MacOS"
    resources = app / "Contents" / "Resources"
    if app.exists():
        shutil.rmtree(app)
    macos.mkdir(parents=True)
    resources.mkdir(parents=True)

    shutil.copy2(built, macos / BINARY)
    build_icns(resources / "AppIcon.icns")

    info = {
        "CFBundleName": APP_NAME,
        "CFBundleDisplayName": APP_NAME,
        "CFBundleIdentifier": IDENTIFIER,
        "CFBundleExecutable": BINARY,
        "CFBundleIconFile": "AppIcon",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": crate_version(),
        "CFBundleVersion": crate_version(),
        "LSMinimumSystemVersion": "11.0",
        "NSHighResolutionCapable": True,
        "NSPrincipalClass": "NSApplication",
    }
    with (app / "Contents" / "Info.plist").open("wb") as plist:
        plistlib.dump(info, plist)

    print(f"Bundled {app.relative_to(ROOT)}")
    if not args.no_open:
        subprocess.run(["open", str(app)], check=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
