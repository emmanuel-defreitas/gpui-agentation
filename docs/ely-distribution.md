# Pinned Ely distribution for gpui-agentation

This is an independently packaged snapshot of **Ely GPUI Components**, not an
upstream release. All runtime source, assets, and their license notices are
byte-for-byte from upstream commit
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721` (Ely 0.1.1):
https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components

Copyright (c) 2026 Ely GPUI Component contributors. Distributed under the
original MIT license in LICENSE. Font and icon licenses remain in assets.

Only Cargo packaging changes: the registry package name is
`gpui-agentation-ely`, package links identify this independent distribution, the original library name is retained, publishing is
enabled, and upstream gallery/dev-only dependencies and examples are omitted.
`Cargo.upstream.toml` preserves the original manifest. `UPSTREAM.json` records
SHA-256 hashes for every upstream runtime file and the original manifest.
Normal dependencies and features are unchanged, including the exact
`gpui-pre =0.3.8` and `gpui-pre-platform =0.3.8` pins.

This package is maintained for Agentation's pinned dependency distribution.
Version `=0.1.1` always identifies this exact revision; any source update requires
a new package version and explicit revision review.

## Shared Ely state

Cargo treats this registry package and the upstream Git package as different
crates. Do not use both in one host: theme globals, focus state and Ely types
would be distinct. Existing Ely apps should alias this package:

```toml
ely-gpui-component = { package = "gpui-agentation-ely", version = "=0.1.1" }
```

Rust imports stay `ely_gpui_component`. New debug-only integrations can instead
use `gpui_agentation::ely`, which is the same crate. Apps using Ely in release
builds need the direct dependency because Agentation's release library is empty.

Verify the snapshot from the repository root with
`python3 .github/scripts/check-ely-snapshot.py`. Pass `--upstream PATH` to also
compare the recorded inventory and hashes against the pinned Git commit.
