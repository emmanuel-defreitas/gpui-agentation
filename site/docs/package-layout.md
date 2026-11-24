# One package, two uses

> **Version: 0.6.0.** Unreleased Git/path migration candidate.

The `gpui-agentation` package provides a debug-only native annotation library
and a portable stdio MCP executable.

| Features | Result |
| --- | --- |
| default (`mcp`) | Portable stdio binary; no native UI compilation |
| `--no-default-features --features native` | macOS-only debug annotation library using GPUI Kit |
| `--all-features` | Native library, bridge and native test support |
| `--no-default-features` | Empty library; no binary |

The library keeps both `cfg(debug_assertions)` and its native feature gate.
Even with `native` enabled, release builds expose no annotation API. The MCP
binary is a separate target and works in release builds.

## Native app

Follow [consumer setup](consumer-setup.md). This migration uses the pinned
Kit 0.11.0 bundle on GPUI 0.3.14. Host and Agentation share that exact dependency
family; initialization and inspector ownership are explicit. See
[Kit distribution](kit-distribution.md) for the Git/path consumer instructions
and the unresolved registry publication boundary.

## MCP client

Install the portable bridge from this checkout:

```sh
cargo install --path . --locked
```

The executable is `gpui-agentation`; use `--url` for the running debug app's
loopback MCP endpoint and `--timeout-ms` for the request timeout. The default
feature does not activate Kit, GPUI or native assets. Native consumers can
exclude the bridge with `default-features = false`.

Crates.io publication of this migration is not claimed. The patched native Kit
family must be available from a publishable source before releasing the new
native library. Existing immutable package releases are unchanged.

## Earlier packages

Version 0.3.0 bundles the previous native UI implementation. Existing published
archives are immutable; the Kit cutover applies to this source candidate.
