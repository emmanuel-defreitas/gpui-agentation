# Kit inspector cutover consumer

This is the first presentation port on `migration/kit-cutover` (#116/#119).
It compiles the actual Agentation inspection services, model and drawing source
against **Kit 0.11.0 / GPUI 0.3.14**, with no Ely code or dependency. Production's
Ely/GPUI 0.3.8 workspace remains separate until the remaining UI ports are ready.
Do not add this harness as a production dependency or merge the root draft early.

The inspector uses Kit sliders, ColorPicker popovers, buttons, breadcrumbs and
focus trapping. Widget entities/subscriptions are per inspector; existing owned
services still handle snapshots, style baselines, edits/reset/restore and source
activation. Box-model drawing retains its MIT provenance (see ORIGIN.md).

## Run

From the repository root:

```sh
make kit-inspector-check
make kit-inspector-example
# Existing Kit host, external companion, native lifecycle assertions:
python3 migration/kit-inspector/cargo.py run --locked -- --host-initialized --smoke
# Switch an already-open in-host inspector into the companion:
python3 migration/kit-inspector/cargo.py run --locked -- --from-in-host --smoke
# Explicit in-host fallback:
python3 migration/kit-inspector/cargo.py run --locked -- --in-host --smoke
```

`--smoke` checks actual native windows: visible host size, existing Root and
focus, companion Root, close/reopen, and host-close cleanup. It is separate from
manual visual QA and the final annotation integration matrix. Omit it to leave
the consumer running. Ordinary `make example-app` still launches the Ely example.

## Kit picking compatibility patch

Stock 0.11.0 places transparent full-window overlay containers above host
content. GPUI 0.3.14 creates inspector hitboxes for those containers, causing a
normal hover/click to select an invisible notification/root layer instead of the
host control. Scrolling past these layers is not acceptable migration behavior.

`patches/` contains a narrow compatibility patch for Kit Base and Component:
layout-only overlay wrappers suppress their own inspector identity while
forwarding layout, child rendering, element identity and accessibility hooks.
Actual toast/dialog/sheet children remain inspectable. This patch is local to the
cutover consumer; it is not an upstream release or an upstream-accepted fix.

`cargo.py` locates the exact stock packages through the existing locked probe,
verifies the original edited-file SHA-256 hashes, and applies the patch to
ignored copies under this consumer's `target/kit-patches/`. It verifies generated
contents on reuse and rebuilds altered copies. The Cargo registry cache is never
modified. Use the launcher for Cargo checks/builds; its local patch overrides are
part of this consumer's lockfile resolution. Cargo fmt needs no launcher.

Before production cutover, explicitly resolve the patch's delivery: adopt a
reviewed upstream release/revision carrying the fix, or package and document an
intentional compatible patch. Do not silently drop it or mix GPUI snapshots.
Kit's Apache-2.0 licenses remain in the generated source copies; no upstream
copyright/license files are removed.

## Remaining integration gates

The consumer proves the inspector presentation in isolation. It does not yet
replace the production overlay's Ely adapter, toolbar/comments/text editors
(#117), Settings/menu bar (#118), or final dependency/export cleanup (#120).
The annotation-selection synchronization and complete scroll/keyboard/IME/native
visual matrix must run after those lanes are assembled (#119/#121).

`RendererPolicy::Preserve` leaves a host's renderer/factories untouched.
`Replace` is explicit, runs after Kit initialization, and installs once. The
companion owns its own Base Root, reuses the existing AppKit docking/margin code,
and uses the host's rem size. Full-screen/fill/unsupported windows report false
from `companion::open` so callers can choose in-host fallback. Runtime transitions
into full-screen/fill and toolbar-driven visibility remain integration work.

## Recorded checks

On macOS arm64, November 23, 2026:

- `make kit-inspector-check`: formatting, strict Clippy, 19 tests and active
  dependency graph verification passed. Tests include actual pointer selection,
  slider/reset clicks, Kit color event routing, visible toast/host picking,
  renderer preservation, custom rem, and the shared two-host service lifecycle.
- Native `--smoke`, `--host-initialized --from-in-host --smoke`, and
  `--in-host --smoke` passed the lifecycle assertions described above.
- Production `make verify` passed separately: 146 workspace tests, 34 release
  script tests, seven MCP tests, release disappearance and bridge build/install.
- Independent reviews of the patch and companion lifecycle found no remaining
  blocking issues after fixes. Manual visual, OS IME, and final integrated
  annotation acceptance are not established by these checks.
