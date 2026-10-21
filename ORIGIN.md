# Source provenance

Extracted from `exegia/gpui-corpora`, commit
`327ec8bb8c42bc7a6db0dd6811a611b2f52da5d9`, directory
`crates/gpui_agentation`, on 2026-10-19.

The initial export copies all Rust source and tests without changes. Package
metadata, dependency versions, and lint settings are expanded from the source
workspace so this repository builds independently. The original MIT license
and copyright notice are preserved in LICENSE.

The GitHub repository was generated from `exegia/corpora-template`. Its initial
commit is retained as the parent of the export commit.

## Development reload supervisor

`src/hot_reload/runner.py` adapts `scripts/dev.py` from
`exegia/gpui-corpora` at `a240711296727d8379dfb9477d3cff99f162bab5`.
Its polling snapshot, save debounce, build-before-restart, failed-build recovery,
and process cleanup approach are retained. Cargo target discovery and the
settings control channel are Agentation adaptations. The original copyright
notice and full MIT license are preserved in `src/hot_reload/LICENSE`.
The crate does not depend on Corpora.
