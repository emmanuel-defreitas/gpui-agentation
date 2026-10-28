# Native annotation demo

Run from the repository root on macOS:

```sh
make dev
```

This example uses the library's actual `AnnotationOverlay`, the pinned Ely
components and the same `gpui-pre =0.3.8` source. It intentionally does not add
`gpui-kit`, which would introduce incompatible GPUI types into this Ely app.

The separate vertical toolbar docks 14 logical pixels to the consumer window's
right edge and matches its height. It follows dragging and resizing, hides with
minimize/fullscreen/fill, and closes with the consumer. Click **Inspect** to pick
a component and edit its properties or add an optional comment in the sidebar.
**Stop** exits inspection; **Close** hides the toolbar. Drawing tools return when
inspection ends. Comments, Settings, and MCP status stay near the bottom.
Comments opens a sheet inside the consumer. On macOS, Settings opens a rich
popup from the menu-bar icon; the toolbar's gear and Debug menu open the same
popup. Restore a hidden toolbar
with **Debug → Show Toolbar**. Undo, Redo, Save, and Delete remain available in
the native menu and through shortcuts.
Use Tab / Shift-Tab to navigate and Escape to dismiss the editor or picker.
**Switch appearance** changes between the Ely light and dark themes.

The loopback API uses port **4749**. Notes are stored in
`target/demo/feedback.json`, separate from host application feedback. A busy
port leaves local annotations usable. Release builds show only a terminal
message because the annotation API is intentionally debug-only.

## Export a GIF preview

The demo can export four actual GPUI/Metal-rendered frames (light and dark,
before and after incrementing the counter), then exit. This uses renderer
readback rather than recording the desktop. From the repository root:

```sh
cargo run --no-default-features --features native --example demo -- target/demo/preview
ffmpeg -y -framerate 1 -i target/demo/preview/frame-%02d.png \
  -filter_complex 'fps=10,scale=900:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse' \
  -loop 0 target/demo/preview.gif
```

This preview shows static states, not a recording of pointer interaction.

## Hot reload

`make dev` launches the bundled Python 3 supervisor and a debug Cargo build.
Open **Settings → Hot reload → Server status** to start watching source saves.
Switch it off to pause watching without closing the app. The switch is off by
default, and disabled when launched without the supervisor.

A successful build restarts the app; a build error keeps the current app open
and shows a recovery message. This is a rebuild/restart, not in-process Rust
code replacement. Saved annotations survive; in-memory host app state does not.
Save application work before enabling it. Ctrl+C stops the supervisor and app;
closing the app also stops the supervisor.

For another consumer app (no additional Agentation initialization or wrappers):

```sh
python3 src/hot_reload/runner.py --manifest-path /path/to/app/Cargo.toml --bin my-app
```

Use `--example name` instead of `--bin`, `--package name` for workspace members,
and `--features names` / `--no-default-features` to match the app's debug build.
Pass application arguments after `--`. The runner uses Cargo's reported
executable path and workspace root, so custom target directories work. It
ignores build output (including annotation stores under `target/`), editor
temporary files, and symlinks. It does not watch dependencies outside the
consumer workspace.

Run watcher regression tests with
`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s src/hot_reload -p 'test_*.py'`.
