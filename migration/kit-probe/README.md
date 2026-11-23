# Kit compatibility contract — #113

Status: compatibility implementation and initial native evidence ready for review;
the complete native acceptance matrix remains open. This is
the foundation for [#112](https://github.com/emmanuel-defreitas/gpui-agentation/issues/112),
not a migrated Agentation release. The probe deliberately does **not** link
`gpui-agentation`, Ely, or the production workspace.

## Version and dependency decision

Evaluate and pin `gpui-kit =0.11.0`, using its standard `gpui-pre =0.3.14`
backend. The dependency is named `gpui` in this probe so macros and all GPUI,
Base, Component and asset imports resolve through Kit. Enable only `component`,
`assets`, `inspector`, and (for tests) `test-support`; macOS does not need X11.
Do not enable the alternate Fast backend.

Kit 0.10.0 / GPUI 0.3.12 was the original candidate. Kit 0.11.0 was released
on November 23, 2026, before implementation began. Use the current release
for the probe rather than require a second snapshot migration immediately
after cutover. Its API changes are explicit migration work, not a compatible
dependency bump. If the probe fails, resolve the incompatibility or record a
revised decision; never mix GPUI 0.3.8 with 0.3.14 in a native consumer graph.

The checked-in lockfile is reproducible. It contains inactive optional packages
from upstream (including Fast); `check-graph.py` checks the **resolved active
graph**, not names merely present in the lockfile.

Production retains Ely and GPUI 0.3.8 during #114/#115. This nested workspace
is intentionally separate from the production Cargo workspace and package
include list. The single migration integration branch performs the native
switch atomically in #116–#121. The default MCP feature must remain independent
of both Kit and Ely. Do not add a production Kit dependency in #113.

## Host contract

| Concern | Required behavior at cutover | Probe / remaining evidence |
| --- | --- | --- |
| Initialization | Agentation initialization is idempotent. A host that fully initialized Kit explicitly retains ownership; Agentation skips Kit init. Standalone hosts initialize Kit exactly once. A Theme global alone is not proof of complete initialization. | `prepare` exercises explicit ownership modes; production API design must expose/document that ownership. |
| Theme | Retain host colors, appearance mode and registry. Follow system appearance only when Agentation owns initialization. | Dark-mode preservation regression tests; system appearance/native transitions remain acceptance work. |
| Assets | Keep Agentation assets under `agentation/`. Preserve host assets and unknown-path fallback; choose and document collision precedence. | Probe chooses host-first precedence and filters Kit's inventory before loading, because Kit returns an error for an unknown path. Port Agentation's own namespace in #116. |
| Root | A Kit host keeps its actual Base Root; annotation content mounts inside it. Agentation-owned companion windows get their own Base Root. Do not nest Roots or replace host roots. | Root identity regression test and two-window native probe. Root plugins must register before windows are constructed. |
| Focus and keyboard | Preserve host focus on mount and after close; keep shortcuts scoped so normal typing remains usable. | Focus identity test; input/IME, dialogs, menus and multi-window keyboard acceptance remain required. |
| Inspector | Preserve the host's renderer by default. External-inspector replacement requires explicit ownership/opt-in; do not silently overwrite a host renderer. | Sentinel renderer test. GPUI exposes a setter but no getter/restore/composition API. #115/#119 must settle external-window rendering and fallback explicitly. |

Kit's full `init` is not idempotent: it resets theme state and installs an
inspector renderer. Therefore an unconditional replacement of `ely::init`
with `gpui::init` is incorrect. This probe's private `prepare` function models
the ownership contract; it is not an implementation of Agentation's public API.

## Public compatibility and removal boundary

Removing public `gpui_agentation::ely` and changing GPUI entity/window types
requires a **pre-1.0 minor release**. Do not bump or publish a version in this
preparation PR. Consumers must upgrade their complete GPUI family together;
imports may use Kit re-exports. Existing Agentation entrypoints should remain
host-independent, with explicit migration instructions for any init changes.

Keep note IDs, shape JSON, palette indices, persisted text frames, lifecycle
and MCP/REST/SSE payloads compatible. Agentation owns annotation types and
inspection/source services; Kit owns presentation primitives. Preserve
`cfg(debug_assertions)`, native macOS gating, loopback and Host/Origin/body guards.

At #120, remove `pub mod ely`, `vendor/ely`, `build/ely.rs`, generator wiring,
Ely asset fallback and snapshot checks, plus dependencies used only by Ely.
Replace checks with dependency/feature/provenance checks; do not simply remove
failing gates. Preserve upstream MIT attribution for retained logic and Kit's
Apache-2.0 notices. A shared crate such as `usvg` may remain transitively through
Kit/GPUI; “Ely-free” does not mean banning every crate Ely also used.

Two existing custom elements implement both `Element` and `IntoElement`:
`FollowMark` in `src/overlay/followed.rs` and `Registered` in `src/target.rs`.
GPUI 0.3.14 adds a blanket `IntoElement` implementation, so remove the redundant
implementations **during cutover**, not while production remains on 0.3.8.
Revalidate inspector geometry; do not carry forward the hard-coded 30-rem
GPUI 0.3.8 sidebar assumption without measurement.

## Baseline and inspector work disposition

Base: `f04ecf0d925ac737195713b5e25d573653f3f8c9` (`origin/dev`, November 23).
PR #127 repaired the usvg/Ely manifest mismatch and Axum 0.8 route captures.
Its merged tree exactly matches the verified head `220cdf6`: `make verify`
passed (141 workspace tests, 34 release-script tests, seven MCP-only tests,
Clippy, formatting, snapshot, release and bridge build/install checks).
Use Python 3.11+; macOS `/usr/bin/python3` is 3.9.6 and lacks `tomllib`.
The #113 environment uses Python 3.12.14 and Rust 1.98.1. `make site-check`
also passed with Node 26.13.1, resolving the previously recorded install failure.

`make verify` was rerun successfully in the #113 worktree. `make kit-probe-check`
passed formatting, Clippy, three compatibility tests, and the resolved graph
check. Both native smoke modes exited successfully after opening two Root
windows. The graph check currently targets **Apple Silicon macOS**; Intel
native compatibility is not established by this evidence. The production
default `cargo tree --locked --edges normal,build` contains no GPUI/Kit graph.

### Native evidence captured November 23

The existing Ely example was built and launched from this baseline. The
following screenshots are observations of the actual app, separate from the
headless test results:

- [Host and companion toolbar](evidence/ely-host.png): original 900×652 host
  window with the 50-pixel companion toolbar.
- [External inspector](evidence/ely-inspector.png): Overview selection, source
  breadcrumb and property controls; the host kept its size while the companion
  expanded. Property edit/reset correctness also has existing automated coverage.
- [Textbox editing](evidence/ely-textbox.png): typed text, resize handles, and
  Save/Cancel controls. Saving returned the text to the canvas.
- [Drawing and mentions](evidence/ely-drawing-mentions.png): box drawing around
  the Tools card, saved textbox and note picker opened with `@`. Selected Note #2
  and saved the box comment afterward.
- [Settings](evidence/ely-settings.png): native Settings popup opened from the
  companion toolbar. MCP configuration was not changed.
- [Kit host-owned dark appearance](evidence/kit-host-initialized.png): actual
  Kit companion window after host initialization; no Agentation/Ely linked.

These establish initial baseline references, not exhaustive native QA. Small
windows, native dark/light transitions, IME, keyboard-only navigation, drawing
tool variants, scroll/undo, and multi-window close/reopen remain in the matrix.
Native screenshots contain only the dedicated example/probe windows. Local
feedback stores and build artifacts are excluded from this change.

The original inspector checkout is clean at `dab472b`, two commits behind
`origin/fix/external-inspector` (`9bc9a85`). The remote tip's entire tree equals
PR #99's `9dd51d4` (tree `72218a13d85769d93084d769f740aacbdef15db7`). Its committed
behavior is integrated; no cherry-pick is needed. Preserve it and the subsequent
scroll/undo fixes on dev. The earlier eight-file uncommitted WIP report could
not be reproduced; no recovery of absent uncommitted changes is claimed. The
original checkout remains untouched.

## Acceptance matrix

| Consumer / behavior | Automated baseline | Native visual / interaction gate |
| --- | --- | --- |
| Current Ely example: drawing, text editing, mentions, scroll/undo, inspector/source/reset, settings and companion geometry | Existing workspace tests pass on repaired baseline; fixtures remain in `src/overlay/tests`, `src/tests`, `example` | Initial native captures above pass; complete dark/light, small-window and interaction matrix remains pending |
| Standalone Kit host | Probe initializes once and creates Kit controls under Base Root | Native two-window smoke passes; this does not exercise Agentation annotations |
| Already initialized Kit host | Probe preserves dark theme, root, focus and sentinel inspector renderer | Native two-window smoke and dark screenshot pass; full focus/input and close/reopen interaction matrix remains pending |
| Production MCP-only client | Existing bridge tests/install pass; no production Cargo changes in #113 | No native UI dependency allowed |
| Production release consumer | Existing release check passes with annotation API excluded | Recheck after cutover, including actual example release build |

Do not close #113 or unblock #114/#115 solely because the small probe compiles.
Attach the native evidence and review the contract first. Full annotation
behavior in a Kit host cannot be demonstrated until the later UI ports; record
that as a cutover gate rather than claim the probe is a migrated library.

## Reproduce

```sh
# Python 3.11+ must be the python3 on PATH.
make kit-probe-check
cargo run --manifest-path migration/kit-probe/Cargo.toml --locked -- --smoke
cargo run --manifest-path migration/kit-probe/Cargo.toml --locked -- --host-initialized --smoke
# Omit --smoke to leave both native windows open for inspection.
```

The smoke mode creates two native Root windows, checks their type and host
appearance, allows the event loop to run, then quits. It proves startup only;
it is not visual QA, annotation integration, or the final multi-window lifecycle
matrix. `make verify` remains the production gate; the probe is checked separately.

## Source references

- [Kit 0.11 release / upstream API changes](https://github.com/longbridge/gpui-kit/releases/tag/v0.11.0)
- [Exact snapshot pins](https://github.com/longbridge/gpui-kit/blob/v0.11.0/Cargo.toml)
- [Kit init and open_window](https://github.com/longbridge/gpui-kit/blob/v0.11.0/crates/kit/src/lib.rs)
- [Theme initialization](https://github.com/longbridge/gpui-kit/blob/v0.11.0/crates/component/src/theme/mod.rs)
- [Inspector initialization](https://github.com/longbridge/gpui-kit/blob/v0.11.0/crates/component/src/inspector.rs)
- [Base Root ownership](https://github.com/longbridge/gpui-kit/blob/v0.11.0/crates/base/src/root.rs)
- [Native asset behavior](https://github.com/longbridge/gpui-kit/blob/v0.11.0/crates/assets/src/native_assets.rs)
