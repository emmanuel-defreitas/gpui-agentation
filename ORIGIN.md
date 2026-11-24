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

## Retained inspector service attribution

The former presentation library has been removed. Agentation-owned inspection
services in `src/inspection/` retain code derived from Ely GPUI Components at
commit `da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721` and its local adaptations at
Agentation commit `2a370c4a420ccfb8c1e68dea5467baf18161026c`.
Copyright (c) 2026 Ely GPUI Component contributors. Its full MIT notice is
preserved in LICENSE. Original source:
https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components.

Style editing/export and the box model also derive from
[`joris-gallot/gpui-devtools`](https://github.com/joris-gallot/gpui-devtools)
commit `e967606d18709e89cad002db49f7d3b6eae19646` (MIT).
These retained source notices do not indicate a runtime dependency.

## GPUI Kit

`vendor/gpui-kit`, `vendor/gpui-base` and `vendor/gpui-component` are the
0.11.0 crates.io sources from https://github.com/longbridge/gpui-kit,
under Apache-2.0. Each package retains its original license. The inventory,
original hashes, final hashes and modified paths are recorded in
`vendor/KIT-UPSTREAM.json`.

Local changes make transparent overlay scaffolding invisible to inspector
picking while preserving visible descendants, layout and accessibility. The
manifests use direct local paths between the three packages and omit upstream
standalone test/example targets. Assets and macros stay at registry 0.11.0;
the standard GPUI backend stays at 0.3.14. No upstream acceptance or release
of these patches is implied. This source bundle is for Git/path consumers;
registry publication awaits a published Kit family containing the fix.

## Model Context Protocol mark

`src/assets/mcp.svg` is the official MCP mark from
https://github.com/modelcontextprotocol/docs/blob/573dc60c2e7aab2605b29d0bf27194aa7b02e4fb/favicon.svg
(2024-11-21), distributed under that repository's MIT license.
Copyright (c) 2024–2025 Anthropic, PBC and contributors.
The original SVG and full license (`src/assets/MCP-LICENSE`) are preserved.
