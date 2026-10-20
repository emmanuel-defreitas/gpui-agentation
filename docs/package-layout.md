# One package, two uses

The package name is `gpui-agentation`; the native Rust library remains
`gpui_agentation`. Version 0.2.0 uses the registry-compatible `gpui-agentation-ely =0.1.1`
snapshot of the unchanged pinned Ely source. See [provenance](ely-distribution.md).
Both packages must be published before installing this candidate from crates.io.

| Features | Result |
| --- | --- |
| default (`mcp`) | Portable stdio binary; no native UI compilation |
| `--no-default-features --features native` | Native debug annotation library; no bridge dependencies |
| `--no-default-features --features mcp` | Same portable binary as the default install |
| `--all-features` | Native library, bridge, and native test support |
| `--no-default-features` | Empty library; no binary |

The library keeps `cfg(debug_assertions)` as well as its feature gate. Even
with `native` enabled, release builds expose no annotation API. The MCP binary
works in release builds because it is a separate binary target, not part of
the debug-gated library.

## Native app

Use the native feature explicitly:

```toml
[dependencies]
gpui-agentation = { version = "0.2.0", default-features = false, features = ["native"] }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
```

Gate imports and annotation fields/mounting with `cfg(debug_assertions)`.
Ordinary Rust imports are still `use gpui_agentation::...`; changing the
package name does not require renaming Rust modules. The Ely source remains at revision
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721`. Existing Ely hosts must switch their
Ely dependency to the **same distribution**, retaining their Rust imports:

```toml
ely-gpui-component = { package = "gpui-agentation-ely", version = "=0.1.1" }
```

Before the snapshot is published, point that dependency at `vendor/ely` in a
local checkout. For a Git-based host during development, point both
Agentation and Ely at that checkout to keep one Cargo identity. Do not mix
upstream Git Ely with the registry snapshot: identical source does not unify
Cargo types or theme/focus globals. New debug-only integrations can use
`gpui_agentation::ely`; release Ely UI needs the direct dependency above.

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
