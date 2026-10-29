# One package, two uses

The package name is `gpui-agentation`; the native Rust library remains
`gpui_agentation`. Version 0.3.0 bundles pinned Ely source and assets in
the same package. See [provenance and build adaptation](ely-distribution.md).
There is no dependency on gpui-agentation-ely. The MCP-only default does not
compile bundled Ely or enable its native dependencies.

| Features | Result |
| --- | --- |
| default (`mcp`) | Portable stdio binary; no native UI compilation |
| `--no-default-features --features native` | macOS-only native debug annotation library; no bridge dependencies |
| `--no-default-features --features mcp` | Same portable binary as the default install |
| `--all-features` | Native library, bridge, and native test support |
| `--no-default-features` | Empty library; no binary |

The native library supports macOS only; other native targets report a compile
error. The MCP-only executable has no native platform restriction, but this
repository verifies it locally on macOS.

The library keeps `cfg(debug_assertions)` as well as its feature gate. Even
with `native` enabled, release builds expose no annotation API. The MCP binary
works in release builds because it is a separate binary target, not part of
the debug-gated library.

## Native app

Use the native feature explicitly:

```toml
[dependencies]
gpui-agentation = { version = "0.6.0", default-features = false, features = ["native"] }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
```

Gate imports and annotation fields/mounting with `cfg(debug_assertions)`.
Ordinary Rust imports are still `use gpui_agentation::...`; changing the
package name does not require renaming Rust modules. The Ely source remains at revision
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721`. Bundled Ely is exposed as
`gpui_agentation::ely`; its types and theme/focus globals differ from any host
Ely crate, even if that crate uses the same upstream revision. A host's own
Ely initialization does not initialize Agentation's bundled components.

Initialize `gpui_agentation::Assets` and call `gpui_agentation::init(cx)` before
mounting `Agentation` once at the screen root. It owns bundled Ely initialization,
assets, focus and frame registration. Ordinary controls need no wrappers.
See the [consumer setup guide](consumer-setup.md) and complete example.
The lower-level `AnnotationOverlay` remains available for existing integrations.

When migrating from 0.2.0, remove the direct gpui-agentation-ely dependency if it
was used only for annotations. Code using `gpui_agentation::ely` retains that
namespace; externally aliased Ely types must be updated to the bundled types.
This type-identity change is why the bundled release starts at 0.3.0.

## MCP client

For now, install from a checkout containing this candidate:

```sh
cargo install --path . --locked
```

Once the combined package is actually published, the registry command will be:

```sh
cargo install gpui-agentation --locked
```

Do not use that registry command yet. The existing public
`cargo install gpui-agentation-mcp --version 0.1.0 --locked` remains supported
through the old package. Existing clients using that executable keep working;
new candidate configs use the command `gpui-agentation` with the same `--url`
and `--timeout-ms` arguments. The host app must be running in debug mode.

The MCP feature avoids compiling GPUI/Ely. Cargo still resolves optional
dependencies; every native dependency now has a registry version.

Registry Cargo metadata describes the installed executable. The binary is enabled by default, so a standard
Cargo installation works without install-feature metadata. Do not pass Cargo
feature flags as executable arguments. The bridge can still be excluded by
consumers using `default-features = false`.
