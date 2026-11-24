# One package, two uses

> **Version: 0.7.0.** Registry GPUI Kit integration.

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

Follow [consumer setup](consumer-setup.md). Host and Agentation share registry
Kit 0.11.0 on GPUI 0.3.14; initialization and inspector ownership are explicit.
The inspector workaround is part of Agentation, with no patched dependencies.
See [Kit distribution](kit-distribution.md).

## MCP client

Install the portable bridge from crates.io:

```sh
cargo install gpui-agentation --locked
```

The executable is `gpui-agentation`; use `--url` for the running debug app's
loopback MCP endpoint and `--timeout-ms` for the request timeout. The default
feature does not activate Kit, GPUI or native assets. Native consumers can
exclude the bridge with `default-features = false`.

## Earlier packages

Version 0.3.0 bundles the previous native UI implementation. Existing published
archives are immutable; the Kit cutover applies to subsequent versions.
