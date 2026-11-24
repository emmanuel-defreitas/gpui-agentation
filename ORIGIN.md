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

## Ely components

`vendor/ely/` is [Ely GPUI Components](https://elygpui.com/) at commit
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721` (MIT, Copyright (c) 2026 Ely GPUI
Component contributors), preserved byte for byte with its license. Source:
https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components.

## GPUI inspector configuration

`vendor/ely/src/tooling/inspector.rs` adapts the temporary style editing and
style-export approach from
[`joris-gallot/gpui-devtools`](https://github.com/joris-gallot/gpui-devtools)
at commit `e967606d18709e89cad002db49f7d3b6eae19646` (MIT). The controls and
published configuration model are rewritten against the bundled Ely components
and Agentation note pipeline.

### Inspection service extraction (#115)

`src/inspection/source.rs` moves the path resolution, editor command/launch,
and clipboard fallback logic from the locally patched
`vendor/ely/src/tooling/inspector/source.rs` at Agentation commit
`2a370c4a420ccfb8c1e68dea5467baf18161026c`. The temporary Ely file retains only
breadcrumb presentation and calls the owned service. The original MIT licenses
and inspector attribution above remain in place; `UPSTREAM.json` is unchanged,
and `LOCAL_PATCHES.json` records the updated adapter hashes.

`src/inspection/mod.rs` and `src/inspection/style.rs` extract selection snapshots,
style baselines, live edits, configuration export/restore and reset from the same
locally patched `vendor/ely/src/tooling/inspector.rs`. The owned style service also
retains Ely's pure hex-color parsing/formatting helpers from
`vendor/ely/src/forms/color.rs`, without importing its controls. These files retain
the Ely and gpui-devtools MIT provenance above. The temporary inspector adapter
keeps presentation and delegates operations to the owned service.

## GPUI Kit inspector presentation (cutover branch)

`src/inspection/kit/box_model.rs` retains the box-model drawing from the locally
patched Ely inspector described above, restyled with GPUI Kit theme values.
The panel and Kit widget wiring in `src/inspection/kit/` are Agentation-owned;
the retained service and source-navigation MIT attribution remains applicable.

The isolated `migration/kit-inspector` consumer uses GPUI Kit 0.11.0 and its
GPUI 0.3.14 backend. Kit Base and Component are Apache-2.0; their upstream license
files remain in the generated dependency copies. The checked-in compatibility
patches adapt transparent overlay containers for inspector picking. Original
edited-file hashes and patched hashes are recorded in `patches/upstream.json`.
No upstream acceptance or release of these local patches is implied.

## Model Context Protocol mark

`src/assets/mcp.svg` is the official MCP mark from
https://github.com/modelcontextprotocol/docs/blob/573dc60c2e7aab2605b29d0bf27194aa7b02e4fb/favicon.svg
(2024-11-21), distributed under that repository's MIT license.
Copyright (c) 2024–2025 Anthropic, PBC and contributors.
The original SVG and full license (`src/assets/MCP-LICENSE`) are preserved.
