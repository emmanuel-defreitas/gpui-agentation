"""Verify the pinned Kit bundle and the production dependency boundary."""
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def verify_bundle(root=ROOT):
    provenance = json.loads((root / "vendor/KIT-UPSTREAM.json").read_text())
    assert provenance["version"] == "0.11.0"
    assert provenance["backend"] == "gpui-pre =0.3.14"
    for name, package in provenance["packages"].items():
        directory = root / "vendor" / name
        actual = {p.relative_to(directory).as_posix() for p in directory.rglob("*") if p.is_file()}
        assert actual == set(package["files"]), f"Bundle inventory changed: {name}"
        for path, digest in package["files"].items():
            assert hashlib.sha256((directory / path).read_bytes()).hexdigest() == digest, f"Bundle file changed: {name}/{path}"
        manifest = tomllib.loads((directory / "Cargo.toml").read_text())
        assert manifest["package"]["version"] == "0.11.0"
        assert manifest["dependencies"]["gpui"]["version"] == "=0.3.14"
        for dependency in ("gpui-base", "gpui-component"):
            if dependency in manifest["dependencies"]:
                assert manifest["dependencies"][dependency]["path"] == f"../{dependency}"
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    assert manifest["features"]["default"] == ["mcp"]
    assert manifest["dependencies"]["gpui"]["path"] == "vendor/gpui-kit"
    assert not (root / "build.rs").exists()
    assert not (root / "vendor/ely").exists()


def verify_graph(root=ROOT):
    def graph(*features):
        return subprocess.check_output([
            "cargo", "tree", "--locked", "--target", "aarch64-apple-darwin",
            "--prefix", "none", "--format", "{p}", "-e", "normal,build", *features,
        ], cwd=root, text=True)
    native = graph("--no-default-features", "--features", "native")
    assert "ely" not in native.lower(), "Legacy component dependency in native graph"
    assert "gpui-fast" not in native, "Mixed GPUI backend"
    for line in native.splitlines():
        if line.startswith("gpui-pre "):
            assert line.split()[1] == "v0.3.14", line
        if line.startswith(("gpui-kit ", "gpui-base ", "gpui-component ")):
            assert "vendor/" in line, f"Mixed Kit source: {line}"
    portable = graph("--no-default-features", "--features", "mcp")
    assert not any(line.startswith(("gpui-kit ", "gpui-pre ", "gpui-base ", "gpui-component ")) for line in portable.splitlines()), "MCP graph contains native UI"


if __name__ == "__main__":
    verify_bundle()
    verify_graph()
    print("Verified Kit 0.11.0 bundle, one GPUI 0.3.14 backend, and portable MCP graph")
