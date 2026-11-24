"""Verify registry Kit identity, one GPUI backend, and the portable MCP graph."""
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
KIT = ("gpui-kit", "gpui-base", "gpui-component", "gpui-kit-assets", "gpui-component-macros")
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"


def verify_manifests(root=ROOT):
    for name in ("Cargo.toml", "example/Cargo.toml"):
        manifest = tomllib.loads((root / name).read_text())
        for kind in ("dependencies", "dev-dependencies"):
            dependency = manifest.get(kind, {}).get("gpui")
            if dependency:
                assert dependency["package"] == "gpui-kit"
                assert dependency["version"] == "=0.11.0"
                assert "path" not in dependency and "git" not in dependency
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    assert manifest["features"]["default"] == ["mcp"]
    for name in ("gpui-base", "gpui-component"):
        dependency = manifest["dependencies"][name]
        assert dependency == {"version": "=0.11.0", "default-features": False, "optional": True}
        assert f"dep:{name}" in manifest["features"]["native"]
    assert not (root / "build.rs").exists()
    packages = tomllib.loads((root / "Cargo.lock").read_text())["package"]
    for name in (*KIT, "gpui-pre"):
        found = [package for package in packages if package["name"] == name]
        assert len(found) == 1, f"Missing or duplicate dependency: {name}"
        package = found[0]
        assert package["source"] == REGISTRY, f"Non-registry dependency: {name}"
        assert package["version"] == ("0.3.14" if name == "gpui-pre" else "0.11.0")
        assert package.get("checksum"), f"Missing registry checksum: {name}"


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
        name = line.split()[0]
        if name in (*KIT, "gpui-pre"):
            assert "(/" not in line and "(https:" not in line, f"Non-registry source: {line}"
            assert line.split()[1] == ("v0.3.14" if name == "gpui-pre" else "v0.11.0"), line
    portable = graph("--no-default-features", "--features", "mcp")
    assert not any(line.split()[0] in (*KIT, "gpui-pre") for line in portable.splitlines()), "MCP graph contains native UI"


if __name__ == "__main__":
    verify_manifests()
    verify_graph()
    print("Verified registry Kit 0.11.0, one GPUI 0.3.14 backend, and portable MCP graph")
