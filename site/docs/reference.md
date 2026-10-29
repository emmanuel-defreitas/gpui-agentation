# Development reference

Detailed native integration and API behavior for gpui-agentation 0.6.0.
Start with the [README](../../README.md) or [consumer setup guide](consumer-setup.md).

Native feedback for GPUI development builds: select a registered element, write
a note, copy Markdown, and exchange status updates and replies with an agent.
The interaction and HTTP resource model follow [Agentation](https://www.agentation.com/api).
This is an independent GPUI implementation, not the React package or a complete
Agentation protocol implementation.

## Install

Start with the [consumer setup guide](consumer-setup.md) and
[complete consumer app](../../example/README.md). Its app-local adapter
keeps debug gates in one module and registers one screen at a time, so ordinary
controls need no annotation wrappers. A reusable
[setup skill](../../skills/gpui-agentation-setup/SKILL.md) helps agents apply the same
pattern to another GPUI app. The consumer can use plain GPUI: Agentation owns
its internal Ely setup and focus handling.

This library is a native GPUI integration using Ely internally. It is not a Codex plugin bundle.
Its loopback server exposes both REST/SSE and a Streamable HTTP MCP endpoint.

```toml
[dependencies]
gpui-agentation = { version = "0.6.0", default-features = false, features = ["native"] }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
```

Enable `native` and disable default features for a host app:

```toml
gpui-agentation = { version = "0.6.0", default-features = false, features = ["native"] }
```

Keep `gpui-pre =0.3.8`. Source and assets for Ely revision
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721` are bundled. Agentation’s internal
Ely globals are independent of host Ely globals. Development uses Rust 1.98.1.

## Toolbar and editor

Run `make example` for a native playground with the real
annotation overlay. See the [consumer example](../../example/README.md).

The toolbar always occupies a separate vertical companion window. It docks to the
consumer's right edge with a 14-logical-pixel gap, shares its top edge and height,
and reserves no space inside the consumer. Tools and color swatches occupy the
upper scrollable section; MCP status and **Comments**, **Settings**, and **Close**
stay at the bottom. Short windows scroll the tools while keeping the footer reachable.

On macOS, an AppKit child-window relationship makes the toolbar follow consumer
dragging without replacing its GPUI window. Resizing updates the dock and height.
Minimizing, fullscreen, or filling the usable screen hides the companion until the
consumer is restored. Closing the consumer closes the companion. Other backends
use GPUI window placement and may reopen the companion on a position change.

**Inspect** uses the consumer's native picker and sidebar. It becomes **Stop**
while inspecting, and drawing controls hide. The toolbar stays outside the
consumer's inspectable element tree in every mode. **Box**, **Arrow**, and **Pen**
draw over the live consumer; the selecting click does not activate its controls.
Inspect uses the normal arrow cursor; drawing tools use a crosshair with an Ely glyph.

**Close** hides the toolbar without stopping inspection. **Debug → Show Toolbar**
or its shortcut restores it. **Esc** closes the editor or inspector while leaving
the companion available. There is no draggable grip or embedded Banner/Floating layout.

Each note has a stable number within its route. Deleting a note does not
renumber other notes or reuse its number. Every pin is a round button with its
number centered. Canvas pins, Comments drawer number badges, and the Comments
count badge share the amber theme color.
Element pins sit at the component center; drawing pins sit at their bounding
box center. Drawings retain their selected stroke color. Outside picking mode,
click a badge or a shape edge to edit its note. Clicks away from
shape edges continue to reach the application. The editor opens below the annotation's bounding
box, flips above when there is insufficient room below, and stays inside the
window. It shows the note number with icon-and-label **Delete** and **Edit/Save**
actions. Existing annotations open in read mode; **Edit** activates the textarea
and changes the action to **Save**. Saving keeps the note open in read mode
and restores **Edit**, including the first save of a new annotation. New
annotations start in edit mode. The note and borderless textarea use the amber
accent surface with contrasting text, icons, and controls. The textarea starts at one line and grows as text wraps, up to twelve
lines before scrolling; its panel hugs the content.

Clicking outside saves valid uncommitted changes and dismisses the note. Clicking
Comments, Settings, or another annotation also opens that item in the same click.
If saving fails, navigation is blocked and the current draft stays open. Empty
new notes are discarded. If validation or persistence fails, the editor stays
open with the draft and error message. **Escape** cancels without saving.
**Delete** deletes a saved note or discards an unfinished note. Saved unresolved annotations remain
visible after leaving annotation mode; resolved notes remain in **Notes**.

The pinned Ely ImageAnnotator combines an opaque image canvas with its toolbar
and has no transparent-surface API. Live drawing therefore adapts its public
`media::Mark` and `media::Tool` model to a transparent GPUI surface; it does not
mount the standalone image widget or capture a screenshot. Tool buttons, color
swatches, inputs, and the floating ActionBar are Ely components.

**Notes** counts and lists every annotation across the app, including resolved
and dismissed notes. Its Ely SelectableList shows each note's type icon and
route, with trailing edit and delete icon buttons. Select a row with the mouse
or arrow keys; double-click or Enter opens its editor. The edit button opens
the saved text, including notes from other routes, and preserves its original
route and number. Delete removes only that note and keeps the popover open.

**Add** in the Annotations popover closes the list and starts element picking.
**Copy** includes all the app's notes as Markdown, grouped by route,
with their metadata and threads. Clicking Comments saves and dismisses an unfinished editor and opens the list
in the same click.
The popover closes on an outside click or Escape.
The compact editor does not display source metadata, lifecycle controls, or
conversation threads. Agents can still read metadata and threads, reply,
change status, and retrieve Markdown through the API. Status changes appear
within 300 ms. There is no Copy button on the toolbar.

Notes, numbers, and drawing geometry survive development rebuilds in
`target/agentation/feedback.json`.
The watcher ignores this file. Route changes cancel unfinished comments;
saved notes stay with their route. Keep one app process per store file.

## Settings

The companion has one fixed dock position, so Settings has no Position control.
Comments opens a sheet inside the consumer window. On macOS, Settings opens
from a process-wide menu-bar icon; the companion button, native Debug menu, and
shortcut target that same popup. Other platforms retain the Settings sheet.
The Comments sheet shares the compact sunken header used by Settings. Its rows
fill the sheet width and show numbered badges; a full-width Add button follows
the list, and Copy stays in the bottom footer.

On macOS, Settings uses a separate GPUI popup below a native menu-bar icon.
There is one icon per consumer process, not one per window. The last active
consumer supplies the settings controller; its window title appears in the header.
The popup owns its focus scope and configuration input, while MCP
continues to use the consumer's existing state. It updates live, preserves the
consumer's inspection/draft, and closes with Esc, its close button, an outside
click, a second icon click, or closure/unmounting of its selected consumer.
Outside clicks are passed through and do not reactivate the consumer. No global
keyboard monitor or additional service is used. Multiple consumer processes each
have their own icon.

The icon is removed when the last mounted consumer is gone. A surviving consumer
is selected by activation order. Native Debug → Settings and the existing shortcut
remain alternate entry points. Where native tray support is unavailable, Settings
continues to open an Ely **Sheet** inside the consumer, with a Settings icon and
**Overline** section labels and typography components for field labels and
descriptions. Its content scrolls independently; Escape, the close button, or
a click on the scrim dismisses it and restores focus.
**MCP** shows read-only, pretty-printed JSON
in an Ely JsonInput that wraps and grows to fit its content without internal
scrolling using the actual listening endpoint. The collapsible header has a
left-aligned MCP label, a trailing **Copy** action, and a disclosure chevron.
Copy remains visible when collapsed and does not expand the section. It copies
that exact JSON and shows green **copied!** feedback for two seconds; another
click restarts the timer. The listener must be available to copy a configuration.
The field is not editable, and copying does not modify any client's settings.

The **McpServerList** row shows connection state, tool count, and an on/off
switch. Turning it off pauses new `/mcp` requests with HTTP 503 while keeping
REST/SSE and local annotations available; in-flight calls may finish. Turning
it back on resumes the same endpoint. If the listener failed at startup, the
switch retries the configured port and shows any failure. The pause state is
per server and resets to on when the host restarts.

**About** includes a disabled documentation link until the site is available.
**Star on GitHub** opens the repository so you can star it yourself.

## Mount in a GPUI app

For the minimal integration, use the consumer guide's app-local `Devtools`
adapter. Configure `gpui_agentation::Assets` before application launch, call
`gpui_agentation::init(cx)` in the launch callback, and retain one
`gpui_agentation::Agentation` on the root view:

```rust,ignore
let feedback = gpui_agentation::Agentation::mount(
    "My app", "gpui://my-app/home", "target/feedback.json", 4748, window, cx,
)?;
```

Render through `feedback.root("screen-id", "Screen name", content, window, cx)`.
It owns root focus traversal/restoration, starts frame registration, inserts the
banner, and registers the screen. No Ely initializer, theme import, or focus
scope is required. Use `Assets::before(HostAssets)` to retain existing app assets;
repeated `init` calls preserve an already-initialized Ely theme.

Place this root below native/custom chrome, outside content padding and
scrolling. Do not add another Ely root focus scope around it. Call
`feedback.set_route(url, window, cx)` on navigation to switch feedback buckets.
The screen target records the Rust caller's file/line/column. Optional finer
targets use `gpui_agentation::target("stable-id", "Readable name", component)`
at meaningful view boundaries, with unique stable IDs within the window.

The existing `AnnotationOverlay`, `begin_frame`, and `target` APIs remain
available for hosts that already manage their own Ely initialization and root
focus scope. Those are lower-level integrations; don't mount both root paths.

**Inspect** uses GPUI's native picker. Selecting an element creates or updates a
numbered inspection annotation (IA) with its source and a snapshot of the
displayed properties, including unchanged values. The IA's border follows its
element; clicking the border reopens the inspector. Width, height, padding,
margin, border, and opacity can be previewed with Ely sliders and save into the
same annotation as they change. Clicking its numbered pin opens the optional
comment editor. Both the comment and property snapshot appear in the notes,
Markdown clipboard, and MCP response. Saved `gpuiConfiguration` retains style
overrides and suggested Rust builder calls. Re-picking an existing IA restores
those overrides before updating its snapshot; Reset clears the preview.

The Inspector header **Stop** button exits inspection. While an element is
selected, Esc or clicking elsewhere on the consumer resumes picking. The rail's
Annotation mode switch enables drawing tools; when off, they stay visible but
muted and disabled.
Comments opens in the companion and shares the inspector panel's expand and
collapse animation. Undo, Redo, Save, and Delete remain available through
shortcuts and the native Debug menu; there is no power icon.

The source path uses Ely's breadcrumb component and is relative to the process
working directory. Generated bundled Ely paths resolve back to `vendor/ely/src`.
Only the final segment is actionable: it opens the file at its source line using
`$EDITOR`, defaulting to Zed. Zed, VS Code, Cursor, and VSCodium CLI conventions
are supported, including quoted command paths and arguments. Commands run
without a shell. Unknown IDEs, missing source files, and failed editor launches
copy the relative `path:line:column` and show **Path copied to clipboard** inline.

Registered targets remain available for host-defined feedback regions and the
drawing tools. Their bounds follow layout and scrolling; their markers hide
when the registered target is absent. Inspect feedback can identify native
GPUI elements without additional target wrappers. Coordinates use logical
pixels, with `x` expressed as a percentage of viewport width.

The entire crate is gated by `cfg(debug_assertions)`. Gate imports, entity fields,
mounting, and wrappers in host code the same way. Release profiles, including
release with all Cargo features, expose no annotation types, toolbar, or server.
Cargo may still build transitive dependencies; no annotation code is linked.

## Local API for MCP or Codex tools

Base URL: `http://127.0.0.1:4748`. REST/SSE uses the endpoints below.
MCP clients connect to `/mcp` on the same listener; no plugin is installed or
configured automatically. Existing third-party `agentation-mcp` tools are not
assumed to be wire-compatible.

| Method | Endpoint | Response |
| --- | --- | --- |
| GET | `/health`, `/status` | Health or server revision/session count |
| GET, POST | `/sessions` | List sessions or create/reuse `{appName, url}` |
| GET | `/sessions/:id` | Session including all annotations and threads |
| POST | `/sessions/:id/annotations` | Create feedback from `AnnotationInput` |
| GET | `/annotations/:id` | One annotation |
| PATCH | `/annotations/:id` | Update `{comment?, status?}` |
| DELETE | `/annotations/:id` | Delete one note |
| POST | `/annotations/:id/thread` | Append `{role, message}` |
| GET | `/pending`, `/sessions/:id/pending` | `{count, annotations}` for pending notes |
| GET | `/sessions/:id/markdown` | Copyable Markdown |
| GET | `/events`, `/sessions/:id/events` | SSE stream |

Statuses are `pending`, `acknowledged`, `resolved`, and `dismissed`. Acknowledge
after reading, reply with progress or questions, and resolve after verifying the
change. To record a resolution summary or dismissal reason, append a thread
message as well as patching status. Acknowledged notes remain in session history
but are excluded from `/pending`.

```sh
curl http://127.0.0.1:4748/pending
curl -X PATCH http://127.0.0.1:4748/annotations/NOTE_ID \
  -H 'Content-Type: application/json' -d '{"status":"acknowledged"}'
curl -X POST http://127.0.0.1:4748/annotations/NOTE_ID/thread \
  -H 'Content-Type: application/json' -d '{"role":"agent","message":"Checking the spacing."}'
curl -N http://127.0.0.1:4748/events
```

Events use `{id, type, sessionId, data}`. Types are `session.created`,
`annotation.created`, `annotation.updated`, `annotation.deleted`, and
`thread.message`. Streams begin with future events; reconnect with
`Last-Event-ID` to replay retained events. The latest 512 events are persisted.
An expired or future cursor returns 400; a slow consumer receives `reset`.
In either case, reload sessions and reconnect without a cursor. SSE heartbeats
arrive every 15 seconds. Dropping `Server` closes active connections and releases
its listener, including during a development restart.

Annotations include a server-owned `index` and optional `shape`. Shapes use
`type: "box" | "arrow" | "pen" | "pin"`, `color: 0..3`, and coordinates in
viewport shares from zero to one. Boxes and arrows carry `from` and `to`, pen
strokes carry `points`, and pins carry `at`. Existing stores without these fields
are read as element feedback and assigned stable numbers. Markdown includes the
shape JSON so copied feedback retains the drawing context.

The listener binds IPv4 loopback only, rejects browser `Origin` headers and
non-local `Host` headers, and limits JSON bodies to 32 KiB. It is intended for
native local tools under the same user, not remote or browser clients. Storage
failures preserve the previous state and the editor's draft.

Screenshot capture, placement/rearrange design modes, webhooks, and browser DOM
metadata are outside this API.

## Connect an MCP client

For clients that launch stdio servers, use the optional
[GPUI Agentation MCP bridge](package-layout.md#mcp-client). `make bridge`
builds its release executable with no GPUI/Ely dependency. It forwards to the
same running app endpoint; direct HTTP configuration below remains supported.

Run a debug host app first. The default port is **4748**; the
consumer example uses **4750**. The configuration in Settings always uses the actual listening port,
including an OS-assigned port when mounted with zero.

For Codex CLI or desktop, use the URL shown in Settings in `~/.codex/config.toml`:

```toml
[mcp_servers.gpui_agentation]
url = "http://127.0.0.1:4748/mcp"
```

Alternatively:

```sh
codex mcp add gpui-agentation --url http://127.0.0.1:4748/mcp
claude mcp add --transport http gpui-agentation http://127.0.0.1:4748/mcp
```

For Claude Code `.mcp.json` or another client that accepts this configuration:

```json
{
  "mcpServers": {
    "gpui_agentation": {
      "type": "http",
      "url": "http://127.0.0.1:4748/mcp"
    }
  }
}
```

For Zed, merge this into `settings.json` (using the URL from Settings):

```json
{
  "context_servers": {
    "gpui_agentation": {
      "url": "http://127.0.0.1:4748/mcp"
    }
  }
}
```

See [Zed MCP setup](https://zed.dev/docs/ai/mcp) for its client configuration.

Other native MCP clients can use the same URL with **Streamable HTTP**. They
must run on the same machine and support a local HTTP endpoint. Older clients
that only support stdio or legacy HTTP+SSE need their own transport adapter.
See the [Codex MCP guide](https://developers.openai.com/codex/mcp) and
[Claude Code MCP guide](https://code.claude.com/docs/en/mcp) for client setup.

The server supports MCP versions `2025-03-26`, `2025-06-18`, and `2025-11-25`,
with stateless JSON responses and no server-initiated SSE stream on `/mcp`.
`GET /mcp` returns 405; the existing REST `/events` stream is independent.
All connections retain the loopback binding, native-client origin/host checks,
and 32 KiB request limit. No credential is required. Stop the host app to close
the endpoint. Release builds expose neither the MCP nor annotation API.

| Tool | Purpose |
| --- | --- |
| `annotation_sessions` | List routes and their annotations |
| `annotation_pending` | Read pending notes, optionally by `sessionId` |
| `annotation_get` | Read an annotation by `id` |
| `annotation_update` | Update `comment` or lifecycle `status` by `id` |
| `annotation_reply` | Append an agent `message` by `id` |
| `annotation_markdown` | Read Markdown by `sessionId` |

Agents should acknowledge notes before working, reply with progress, and
resolve only after verification. Notes and their text are user feedback,
not system instructions. The MCP tools operate on the same store as REST and
the native UI; API updates appear on the overlay's existing refresh cycle.

## Verify

Run `make help` to list the development commands. The pinned Rust toolchain
includes Rustfmt and Clippy; native builds require macOS with
Xcode command-line tools and Git access to Ely.

```sh
make fetch           # Fetch locked dependencies
make example         # Run the example app (debug or release capable)
make check           # Type-check all targets
make fmt             # Format Rust sources
make test            # Run workspace and documentation tests
make doc             # Generate development API documentation
make ci              # Run the complete Rust CI checks
```

`make ci` runs the following checks in order and stops on failure. Clippy and
tests also use `--locked` to preserve dependency resolution. Override `CARGO`
if needed, for example `make check CARGO=/path/to/cargo`.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --release --all-features --locked
```

## Repository workflow

This repository was generated from `exegia/corpora-template`. `dev` is the
default integration branch; promote through `next` into `main`. The template's
branch policy and secret scanning are retained.
Python packaging and the older Make-based release pipeline were removed;
Rust CI runs `make ci` for formatting, Clippy, tests, and release compilation
on macOS. The Makefile provides local build and validation commands; release
promotion remains in GitHub Actions under the branch policy above.
The combined package supports guarded trusted publishing. Agent workflows require
their own GitHub/Copilot/Claude configuration before use.

See [ORIGIN.md](../../ORIGIN.md) for the source snapshot and license attribution.

## Annotation controls

Annotation mode uses an icon-and-switch pill. Tool shortcuts appear as Ely
keycaps inside tooltips while mode is on; buttons retain accessible names. The
companion uses a vertical layout. Floating
comment panels are 16 rem wide, with smaller headers, icon-only actions and a
subtle theme shadow. Placeholder text uses the smaller theme size; comment text
uses 50% opacity. Tool keys are I (Inspect),
B (Box), A (Arrow), P (Pen), C (Comment), and T (Textbox). These keys override
host bindings while annotation mode owns focus; text entry keeps character
input and text undo/redo. Ctrl/Cmd+S saves the current annotation draft.
Ctrl/Cmd+Z undoes annotation operations, Ctrl/Cmd+Shift+Z or Ctrl+Y redoes them,
and Delete/Backspace removes a selected note outside its text editor. Undo
history is bounded to 100 local operations per window and resets on navigation.
An undo that conflicts with an external edit or reply is rejected without
changing the annotation.

Textboxes store an `AnnotationShape::Text { at, text, color }`, render at 16
logical pixels, and begin with a numbered pin. A single click selects the
bounding box; double-click or Enter edits the text. Dragging the body or its pin
moves both together. Saving does not open a separate comment. Other numbered
pins move independently of the underlying shape. Position changes preserve note
identity, update percentage-x/logical-y fields and support undo/redo.

Typing `@` in a floating comment opens up to six matching notes from the current
session. Search by number or comment text; an exact number ranks first. Hovering
or navigating suggestions previews the corresponding pin with a short scale
transition. Typing `#12` also previews note 12. Click, Enter or Tab inserts `@#12 `;
Escape closes the list while keeping the comment open. Motion respects the
theme's reduced-motion setting.

Right-click any annotation for Edit…, Delete and a checked Color submenu.
macOS uses an AppKit system context menu; other platforms use the themed Ely
context menu. Color and Delete actions participate in local undo/redo. The
optional persisted `Annotation.color` field retains element-note colors without
changing their annotation type; older records fall back to their shape color.
Adding this field and the Text shape variant changes public Rust struct/enum
construction, which is part of this major release.

Compose the native menu with the host's menus:

```rust
#[cfg(debug_assertions)]
cx.set_menus(vec![
    app_menu, // The host's existing application menu, including its Quit action.
    gpui_agentation::debug_menu(),
]);
```

Include that menu alongside existing menus rather than replacing them.
Agentation updates only its Debug menu when window state or active window changes.
`Agentation::root` handles `AnnotationCommand` automatically. Hosts mounting
`AnnotationOverlay` directly must capture that action on their window root and
forward it to `AnnotationOverlay::dispatch_command`.

Close hides the companion; Show Toolbar restores it. Visibility is stored per
app in the annotations file. Windows mounting the same path share preference
changes; independent processes should use separate stores. Native menu commands
remain available regardless of toolbar visibility.

Undo/Redo reflect each window's history. New and Color are disabled while mode
is off. Settings also exposes a checked Annotation Mode command. The public
`ToolbarPosition::Banner` and `Floating` values and their former shortcuts are
retained as compatibility aliases that show the same companion; `None` hides it.

Additional menu shortcuts use Command+Option on macOS and Control+Alt elsewhere:

| Keys after those modifiers | Command |
| --- | --- |
| A / H | Annotation mode / toolbar visibility |
| F / B | Show companion (legacy placement shortcuts) |
| C / S | Comments / Settings |
| M | Start/stop MCP |

Comments and Settings show/hide with the same command. Report an Issue opens the
repository's bug-report template; Star on GitHub opens its repository page.
