# Add Agentation to a GPUI Kit app

> **Version: 0.7.0.** Registry GPUI Kit integration.

This migration uses GPUI Kit 0.11.0 and the standard GPUI 0.3.14 backend.
The inspector compatibility fix is inside Agentation; Kit is unmodified.
See [Kit distribution](kit-distribution.md).

## 1. Share the exact Kit dependency

For a crates.io installation:

```toml
[dependencies]
gpui-agentation = { version = "0.7.0", default-features = false, features = ["native"] }
gpui = { package = "gpui-kit", version = "=0.11.0", default-features = false, features = ["component", "assets", "inspector"] }
```

Both dependencies must resolve to the same registry Kit version. Do not add
Git/path overrides for only one dependency: component entities and globals must match.
The debug-only `gpui_agentation::kit` re-export also exposes the exact dependency.
A host that uses Kit in release needs the direct dependency shown above because
Agentation exposes no library API in release builds.

## 2. Configure assets and initialization

```rust,ignore
let app = gpui::application();
#[cfg(debug_assertions)]
let app = app.with_assets(gpui_agentation::Assets::before(HostAssets));
#[cfg(not(debug_assertions))]
let app = app.with_assets(HostAssets);
app.run(|cx| {
    // A standalone app lets Agentation initialize Kit in debug.
    #[cfg(debug_assertions)]
    gpui_agentation::init(cx).expect("initialize Agentation");
    #[cfg(not(debug_assertions))]
    gpui::init(cx);
    // Open your windows here.
});
```

Host assets win path collisions. Agentation supplies missing Kit icons and its
own marks. Missing paths return `Ok(None)`. Keep your normal fonts and assets.
Repeated Agentation initialization is harmless.

If the host already calls `gpui::init(cx)`, make ownership explicit instead:

```rust,ignore
gpui::init(cx);
#[cfg(debug_assertions)]
gpui_agentation::init_with_options(
    gpui_agentation::InitOptions {
        host: gpui_agentation::HostMode::KitInitialized,
        inspector: gpui_agentation::RendererPolicy::Replace,
    },
    cx,
).expect("initialize Agentation");
```

`Replace` explicitly grants Agentation the inspector renderer. `Preserve` leaves
the host renderer and element factories untouched; Agentation's inspector
annotation workflow requires its renderer. Kit-initialized hosts keep their
existing theme, key bindings, Root and focus. A Theme global alone does not
establish full Kit initialization.

## 3. Keep the Kit Root and mount one annotation owner per window

Construct your content entity, then wrap it in the shared Kit Root:

```rust,ignore
cx.open_window(options, |window, cx| {
    let content = cx.new(|cx| AppView::new(window, cx));
    cx.new(|cx| gpui::base::Root::new(content, window, cx))
})?;
```

An existing Kit Root stays in place; do not add another one. Root provides Kit's
focus traversal, popup and notification layers. Agentation creates Kit roots
for its own toolbar and Settings windows.

Inside `AppView::new`, retain the mounted value in a debug-only field:

```rust,ignore
#[cfg(debug_assertions)]
let feedback = gpui_agentation::Agentation::mount(
    "My app", "gpui://my-app/home", "target/agentation/feedback.json", 4748,
    window, cx,
)?;
```

In render, wrap the screen once:

```rust,ignore
#[cfg(debug_assertions)]
let content = self.feedback.root("home", "Home screen", content, window, cx);
content
```

Put content padding and scrolling inside that screen boundary. Keep
`#[track_caller]` on any app-local wrapper so source references reach the host.
Add optional `target(id, label, child)` wrappers only for stable, useful targets.
Use stable IDs rather than array indexes or random values.

Keep all annotation imports, fields and calls behind `cfg(debug_assertions)`.
The [complete example](../../example/src/devtools.rs) centralizes these gates
in an app-local `Devtools` module and runs the same host in debug and release.

## 4. Connect committed navigation and MCP

After the host commits a route change:

```rust,ignore
#[cfg(debug_assertions)]
self.feedback.set_route("gpui://my-app/details", window, cx);
```

Saved notes stay on their route; unfinished edits are cancelled. Choose an
ignored store path and a distinct loopback port per application. If the port
is busy, local annotations remain usable and Settings reports the server error.
Keep Host/Origin checks and request-size limits enabled.

Open Settings, enable MCP and copy its live configuration. The example listens
on port 4750 and stores notes at `target/consumer/feedback.json`. The app must
remain running for clients to read annotations. The optional stdio bridge uses
the portable `mcp` feature and does not compile native UI.

## 5. Verify the real app

```sh
cargo run -p agentation-consumer --locked
cargo run -p agentation-consumer --release --locked
make verify
```

Check normal host controls, inspection, drawing, comment/textbox editing,
source opening, style reset, Settings, window resize and close/reopen. Verify
notes survive route changes and reload. Headless tests can enable
`test-support` and call `gpui_agentation::init_for_tests(cx)`.

Native support is macOS only. Release builds have no annotation API. The new
host Kit dependency and removal of the former public component namespace are
pre-1.0 breaking changes requiring a minor version release when published.
