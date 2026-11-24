# Inspection services — #115

The service extraction is implemented. Production retains the pinned Ely / GPUI
0.3.8 graph until the later Kit presentation cutover.

## Ownership boundary

`src/inspection/mod.rs` owns selection snapshots, per-host configurations,
companion target routes, renderer claims and window-close cleanup.
`src/inspection/style.rs` owns per-inspector edit baselines, numeric/color edits,
configuration export/restore and reset. `src/inspection/source.rs` owns source
paths, quoted editor commands, editor launch and clipboard fallback. These
private service modules use only Agentation data, standard-library types and
GPUI primitives; they import no Ely or Kit component types.

The temporary `vendor/ely/src/tooling/inspector.rs` and `inspector/source.rs`
adapters retain presentation. Color-picker expansion is presentation state;
original element styles are service state. Legacy public tooling names reexport
owned operations and configuration types until their planned removal in #120.
Property labels and serialized configuration shapes remain compatible. Explicit
relative imports preserve the build generator's namespace rules. Generated-Ely-
to-editable-source mapping stays until vendor removal.

## Renderer and lifecycle contract

The element renderer is registered once. Default inspector presentation may
initialize once, and Agentation may explicitly take ownership once. Repeated
default or Agentation installation cannot replace the selected renderer. This
contract governs these two installers; GPUI exposes no getter for detecting an
unrelated application's direct renderer replacement.

Each inspector owns its edit-state entity; snapshots and configurations are
keyed by host window. Companion edit, reset and restore operations route to that
host. Closing a companion removes its route without clearing a live host's
configuration. Closing a host removes snapshots, configurations and incoming
routes, and clears any retained edit baseline through a weak entity reference.

Route and sidebar removal check entity ownership so a replaced owner's late
release cannot erase its successor. Overlay release and host close both remove
the owned sidebar and release its reserved margin. Native inspector closure is
deferred when necessary to avoid re-entering a window update; replacement
registration cancels that deferred closure. Cleanup is idempotent.

## Verification

`make verify` is the acceptance command. Service tests cover two hosts with
independent baselines, companion width/opacity/color edits, reset and saved
configuration restore, retained companion and host closure, stale route cleanup,
and renderer claims. Extracted style tests cover box-model values and changed
property export. Existing native GPUI tests cover inspector controls, padding,
configuration attachment, host/companion behavior and breadcrumb rendering.

Source tests exercise compile-root/relative paths, quoted editor arguments,
unsupported editors and failed-launch clipboard fallback. They call the owned
service; the adapter's editor-launch callback is not newly clicked directly.
Automated GPUI tests are distinct from manual native visual observations. No
new manual appearance acceptance is claimed for this extraction.

The remaining Kit presentation and dependency removal belong to later migration
issues. They consume this service boundary instead of copying inspection logic.
