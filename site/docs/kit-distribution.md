# Pinned GPUI Kit source

Agentation uses GPUI Kit 0.11.0 with the standard GPUI 0.3.14 backend.
The three local crates in `vendor/` form one dependency family with direct path
edges. The consumer and Agentation must share this family: registry Kit and
bundled Kit have different Rust types and globals despite matching versions.

The bundle contains a small inspector-picking fix. Transparent overlay
containers do not become inspector targets; actual controls, notifications,
focus, layout and accessibility remain intact. `vendor/KIT-UPSTREAM.json`
records every upstream and modified hash. `make snapshot-check` verifies the
inventory and the active native/MCP dependency graphs.

## Distribution

This unreleased migration supports repository and Git/path consumers. Use the
same checkout's `vendor/gpui-kit` dependency in your host, or the debug-only
`gpui_agentation::kit` re-export. A host that uses Kit in release needs its own
direct dependency on that same bundled package because Agentation's entire
library is absent in release builds.

Do not mix this source with registry Kit or a different GPUI backend. No
consumer-level `[patch.crates-io]` section is required.

Crates.io publication requires a published Kit release containing the picking
fix, or a published coherent fork family. Cargo normalizes path dependencies
to registry versions when packaging; publishing this native candidate before
that dependency is available would discard the fix. No release is claimed.

See [source provenance](../../ORIGIN.md) and [consumer setup](consumer-setup.md).
