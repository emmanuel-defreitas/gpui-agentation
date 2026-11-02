# External inspector

Inspect opens its panel in the toolbar window, not inside your app, on stock
`gpui-pre =0.3.8`. No patched GPUI, build script, or Cargo override is
involved; `cargo run` and the published crate behave the same way.

## How it works

GPUI always draws its inspector as a fixed sidebar (30 rem) inside the window
it inspects and lays the app out in the width that remains. Nothing public
turns that off, so Agentation keeps the host window's native content view that
much wider than the window shows. GPUI's viewport gains width that lies past
the window's right edge, GPUI lays its sidebar out there where it is never
seen, and the app keeps its whole visible width. The companion window renders
the same `Inspector` entity as its own panel, so the toolbar grows by the panel
while inspecting and the consumer window does not move or resize.

AppKit gives the content view its plain size back on every window resize; a
frame-change observer restores the margin synchronously, before the next frame
draws. Stop releases the margin and the observer.

Picking the same element in the host outlines it in your accent color; the
box model, sliders, and color pickers live in the companion panel.

## When the panel stays inside the app

The companion does not show when the toolbar is hidden or the window fills its
display (fullscreen, zoomed, or Fill). Inspect then keeps GPUI's sidebar inside
the window and the app yields the width until inspection ends. The same
fallback applies on platforms without the native view, and in GPUI's headless
tests.

## Limits

- Reopening a saved inspection from its pin or the Comments list restarts
  picking; pick the same element again to continue the note with its saved
  overrides. Stock GPUI exposes no way to reselect an element by identity.
- Pins for inspections sit where the element was when it was inspected. They
  do not follow later layout changes, unlike pins on registered targets.
- While picking, GPUI swallows every mouse event in the host window, so the
  Inspect button in the toolbar reads **Stop inspecting** and ends it; the
  panel's own **Stop** works once an element is picked.
