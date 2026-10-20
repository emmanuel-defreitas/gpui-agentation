# gpui_agentation

Native feedback for GPUI development builds: select a registered element, write
a note, copy Markdown, and exchange status updates and replies with an agent.
The interaction and HTTP resource model follow [Agentation](https://www.agentation.com/api).
This is an independent GPUI implementation, not the React package or a complete
Agentation protocol implementation.

## Install

This library is a native GPUI/Ely integration. It is not a Codex plugin bundle
or an MCP server. Its loopback API can be used by an external MCP adapter.

```toml
[dependencies]
gpui_agentation = { git = "https://github.com/emmanuel-defreitas/gpui-agentation", branch = "dev" }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
```

The repository is private, so Git must authenticate with GitHub. Pin a commit
with `rev` for repeatable builds. Use the same Ely revision as this crate:
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721`. A different GPUI package or source
has incompatible types. The minimum Rust version is 1.95; development and CI
use Rust 1.98.1. Ely is a Git dependency, so crates.io publishing remains disabled.

## Toolbar and editor

After mounting the overlay in your app, choose the round **Plus** icon in the floating Ely ActionBar.
Use **Inspect element** (crosshair) to anchor a note to a registered GPUI
component, or choose **Box**, **Arrow**, or **Pen** to draw directly over the
live window. Select one of four Ely chart colors before drawing. The native
cursor uses GPUI's crosshair or arrow style; a matching Ely drawing-tool glyph
follows it because GPUI has no native pencil or rectangle cursor.

The bar animates its width with Ely motion duration, easing, and interpolation,
including reduced-motion handling. Expanded mode shows tools and **X** on the
right; collapsed mode shows **Plus** and the **Notes** map-pin/count badge.
The selecting click does not activate the application control. **Esc** cancels
the editor; a second **Esc** exits picking. **X** returns to the application.

Drag the grip to move the ActionBar. Keyboard users can Tab to the grip and use
the arrow keys. The bar stays inside the viewport when moved or resized.

Each note has a stable number within its route. Deleting a note does not
renumber other notes or reuse its number. Every pin is a round button with its
number centered, using the same Ely accent color as the Notes count badge.
Element pins sit at the component center; drawing pins sit at their bounding
box center. Drawings retain their selected stroke color. While the bar is collapsed, click a badge or a shape
edge to edit its note. The bar stays collapsed while editing. Clicks away from
shape edges continue to reach the application. The editor opens below the annotation's bounding
box, flips above when there is insufficient room below, and stays inside the
window. It shows the note number, textarea, and check/trash icon buttons.
Reopening a note fills the textarea with its saved text; check updates it.
Trash deletes a saved note or discards an unfinished note. Saved unresolved annotations remain
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

**Copy Markdown for agent** includes all the app's notes, grouped by route,
with their metadata and threads. Opening Notes closes an unfinished editor.
The popover closes on an outside click or Escape.
The compact editor does not display source metadata, lifecycle controls, or
conversation threads. Agents can still read metadata and threads, reply,
change status, and retrieve Markdown through the API. Status changes appear
within 300 ms. There is no Copy button on the toolbar.

Notes, numbers, and drawing geometry survive development rebuilds in
`target/agentation/feedback.json`.
The watcher ignores this file. Route changes cancel unfinished comments;
saved notes stay with their route. Keep one app process per store file.

## Mount in an Ely app

Initialize Ely and the window's root `FocusScope` normally. Keep one overlay
entity on the window's root view:

```rust,ignore
let feedback = gpui_agentation::AnnotationOverlay::mount(
    "My app", "gpui://my-app/home", "target/feedback.json", 4748, window, cx,
)?;
```

At the beginning of the root render call `gpui_agentation::begin_frame(window,
cx)`. Register meaningful components with `gpui_agentation::target("stable-id",
"Readable name", component)` and add the overlay entity as the last root child.
The wrapper preserves its child's layout and records the Rust caller's
file/line/column. Use unique IDs within each window. Call `set_route(url,
window, cx)` on navigation to switch feedback buckets.

The picker uses the smallest visible registered region under the pointer.
Bounds follow layout, scrolling, display scaling, and cached GPUI views;
markers are hidden when their target is absent. Coordinates use logical pixels,
with `x` expressed as a percentage of viewport width. Registration is explicit:
GPUI has no browser DOM to inspect. Register the components that should be selectable in your host app.

The entire crate is gated by `cfg(debug_assertions)`. Gate imports, entity fields,
mounting, and wrappers in host code the same way. Release profiles, including
release with all Cargo features, expose no annotation types, toolbar, or server.
Cargo may still build transitive dependencies; no annotation code is linked.

## Local API for MCP or Codex tools

Base URL: `http://127.0.0.1:4748`. This is a REST/SSE API, not an MCP JSON-RPC
transport. An MCP adapter or Codex plugin can wrap these endpoints. No plugin
is installed or configured automatically, and existing `agentation-mcp` tools
are not assumed to be wire-compatible.

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

## Verify

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --release --all-features --locked
```

## Repository workflow

This repository was generated from `exegia/corpora-template`. `dev` is the
default integration branch; promote through `next` into `main`. The template's
branch policy, secret scanning, and agentic workflow sources are retained.
Python packaging and the older Make-based release pipeline were removed;
Rust CI checks formatting, Clippy, tests, and release compilation on macOS.
The standalone bridge has a dedicated publishing workflow. Agent workflows require
their own GitHub/Copilot/Claude configuration before use.

See [ORIGIN.md](ORIGIN.md) for the source snapshot and license attribution.

## Standalone bridge publishing

The stdio bridge lives in `crates/gpui-agentation-mcp` and is independent of
the native library. See [publishing instructions](docs/publishing-bridge.md)
for the dedicated CI matrix, dry-run workflow, and crates.io trusted publishing.
The native library remains unpublished.

The native debug library serves `/mcp` on its existing loopback HTTP listener.
The bridge forwards its tool calls to that endpoint, sharing the same store as
REST/SSE. Origin/Host checks and the 32 KiB request limit apply to both.
