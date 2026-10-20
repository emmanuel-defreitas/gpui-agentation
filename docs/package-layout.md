# One package, two uses

The package name is `gpui-agentation`; the native Rust library remains
`gpui_agentation`. Version 0.2.0 is a source-only candidate until the pinned Ely
revision can be distributed through crates.io. Do not change the GPUI source or
substitute a different Ely revision to get a successful upload.

| Features | Result |
| --- | --- |
| default (`native`) | Native debug annotation library; no binary |
| `--no-default-features --features mcp` | Optional stdio binary; no native UI compilation |
| `--all-features` | Native library, bridge, and native test support |
| `--no-default-features` | Empty library; no binary |

The library keeps `cfg(debug_assertions)` as well as its feature gate. Even
with `native` enabled, release builds expose no annotation API. The MCP binary
works in release builds because it is a separate binary target, not part of
the debug-gated library.

## Native app

Until a registry-compatible Ely dependency exists, use Git:

```toml
[dependencies]
gpui-agentation = { git = "https://github.com/emmanuel-defreitas/gpui-agentation", branch = "dev" }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
```

Gate imports and annotation fields/mounting with `cfg(debug_assertions)`.
Ordinary Rust imports are still `use gpui_agentation::...`; changing the
package name does not require renaming Rust modules. Retain the pinned Ely
revision `da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721` in existing Ely hosts.

## MCP client

For now, install from a checkout containing this candidate:

```sh
cargo install --path . --no-default-features --features mcp --bin gpui-agentation --locked
```

Once the combined package is actually published, the registry command will be:

```sh
cargo install gpui-agentation --no-default-features --features mcp --bin gpui-agentation --locked
```

Do not use that registry command yet. The existing public
`cargo install gpui-agentation-mcp --version 0.1.0 --locked` remains supported
through the old package. Existing clients using that executable keep working;
new candidate configs use the command `gpui-agentation` with the same `--url`
and `--timeout-ms` arguments. The host app must be running in debug mode.

The MCP feature avoids compiling GPUI/Ely, but Cargo still resolves optional
dependencies and may fetch their Git repositories. This feature does not make
the combined package publishable while Ely is Git-only.

Registry Cargo metadata describes the installed executable. Its schema has no
Cargo install-feature field, so clients must follow the explicit install command
above; do not pass Cargo feature flags as executable arguments. The registry
entry does not make `cargo install gpui-agentation` enable the optional binary.
