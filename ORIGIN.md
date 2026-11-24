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

Agentation depends on the unmodified crates.io GPUI Kit 0.11.0 family from
https://github.com/longbridge/gpui-kit, under Apache-2.0, using GPUI 0.3.14.
The dependency packages retain their upstream license and attribution.

The development cutover used a local source bundle with inspector-transparent
containers. That historical bundle and its original/modified hashes remain in
Git history at commit `72adf7703e6d23222371d2b45f3c7bb068f89539`.
The release instead handles picking inside Agentation through public GPUI APIs;
no modified Kit packages are shipped or published by this repository.

## Model Context Protocol mark

`src/assets/mcp.svg` is the official MCP mark from
https://github.com/modelcontextprotocol/docs/blob/573dc60c2e7aab2605b29d0bf27194aa7b02e4fb/favicon.svg
(2024-11-21), distributed under that repository's MIT license.
Copyright (c) 2024–2025 Anthropic, PBC and contributors.
The original SVG and full license (`src/assets/MCP-LICENSE`) are preserved.
