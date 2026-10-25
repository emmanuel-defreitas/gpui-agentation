# Add Agentation to a GPUI app

The consumer only initializes Agentation. **You do not need an Ely dependency,
an Ely initializer, or an Ely focus scope.** Agentation supplies its own UI
assets, initialization, and root focus handling through its public API.

Keep the integration in one app-local module, with one target for each screen.
Buttons, inputs, and layout elements stay ordinary host code. Add finer targets
later only where selecting a whole screen is insufficient.

The runnable [consumer example](../examples/consumer/src/main.rs) demonstrates
this pattern through the library's public API. Its
[devtools.rs](../examples/consumer/src/devtools.rs) contains all debug/release
handling and delegates to Agentation's public integration API. The app has no
direct Ely dependency, initialization call, theme access, or focus scope.
Agentation uses Ely internally; it does not traverse components automatically.

## What you add to the consumer

| Integration point | Adapter call | Agentation API used in debug |
| --- | --- | --- |
| Assets, before launch | `Devtools::application(app)` | `Assets` or `Assets::before(HostAssets)` |
| Startup, before opening windows | `Devtools::init(cx)` | `init(cx)` |
| Root view construction | `Devtools::mount(window, cx)` | `Agentation::mount(...)` |
| Root view rendering | `self.devtools.frame(...)` | `Agentation::root(...)` |
| Committed navigation, if applicable | `self.devtools.set_route(...)` | `Agentation::set_route(...)` |

`Devtools` is the example's app-local adapter, not a library export. It keeps
all debug gates in one file. In release, it preserves normal startup/assets,
returns ordinary app content, and performs no annotation mounting or routing.
The Agentation crate exposes no API in release builds.

## Try the complete app

On macOS, install the Rust toolchain from this repository's `rust-toolchain.toml`
and Xcode command-line tools. Ely source and assets are bundled. From the repository root:

```sh
cargo run -p agentation-consumer --locked
```

The app offers Overview and Details routes, a counter, and navigation. The
debug build adds the annotation banner. Select Inspect, hover over the screen,
and click to comment. Stop inspection to use the counter and navigation normally.
The registered source points to the `frame` call in the host render function;
it does not claim the individual button's source location.

The example uses port **4750** and stores notes at
`target/consumer/feedback.json` under this checkout, regardless of the current
working directory. It can run alongside the playground on port 4749. Run one
process per store file. If the port is busy, local annotations still work and
Settings reports that the API is offline; choose a different port in the adapter.

To test the real app without devtools:

```sh
cargo run -p agentation-consumer --release --locked
```

The same app, counter, and navigation run in release. The annotation crate has
no public API in that build, and the adapter contains no references to it.

## 1. Match the dependency sources

Use the same GPUI package/source as Agentation:

```toml
[dependencies]
gpui-agentation = { version = "0.6.0", default-features = false, features = ["native"] }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
gpui-platform = { package = "gpui-pre-platform", version = "=0.3.8" }
```

Keep the lockfile and the same GPUI package/source throughout the app. Ely source
and assets are bundled; the host needs no Ely dependency. Agentation owns its
internal Ely globals independently of any Ely crate used by the host. The snapshot
remains pinned at `da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721`.

These are normal dependencies, not `[dev-dependencies]`: the tools run in the
application's debug binary. The annotation crate itself is gated by
`cfg(debug_assertions)`; its dependency graph may still be compiled in release.
Keep the whole-crate gate intact.

The example's manifest uses a local path dependency because it belongs to this
workspace. When copying the app outside the repository, replace Agentation's
path in both dependencies and dev-dependencies with the same package version and native feature, then
create its own lockfile. Keep the dev-dependency's `test-support` feature.
It needs no special
build script, proc macro, MCP service, or API key.

## 2. Add the app-local adapter

Copy the production portion of
[`examples/consumer/src/devtools.rs`](../examples/consumer/src/devtools.rs)
into the consumer's source tree. Stop before its `#[cfg(all(test,
debug_assertions))]` test module: those tests refer to the example's `Consumer`
and `Page` types and should be adapted to your host rather than copied unchanged.
Add `mod devtools;` and `use devtools::Devtools;` in the host module that uses it.

Adapt the app name, initial route, storage path, and port in `Devtools::mount`.
In particular, replace the example's `../../target/consumer/feedback.json`
path with an ignored path in your own workspace. A fixed port per app makes
client configuration predictable; port zero is useful for tests, but requires
reading the actual endpoint from Settings after each launch.

Configure assets before launch and initialize Agentation in the app's launch
callback. The adapter keeps these debug-only calls out of normal app code:

```rust,ignore
Devtools::application(gpui_platform::application()).run(|cx| {
    Devtools::init(cx);
    // Existing key bindings, window creation, etc.
});
```

In debug, `application` configures `gpui_agentation::Assets`, and `init` calls
`gpui_agentation::init(cx)`. Agentation loads its own fonts, theme, and keyboard
bindings. Repeated initialization is harmless. In release both adapter methods
leave normal application startup alone.

If the host has assets, compose them in the adapter instead of replacing them:

```rust,ignore
pub(super) fn application(app: gpui::Application) -> gpui::Application {
    #[cfg(debug_assertions)]
    let app = app.with_assets(gpui_agentation::Assets::before(HostAssets));
    #[cfg(not(debug_assertions))]
    let app = app.with_assets(HostAssets);
    app
}
```

Asset composition supplies Agentation's icons/fonts and retains other host
paths. Configure assets before `init`; an asset error is returned with setup
instructions. Do not initialize Ely separately for Agentation.

Mount the adapter once when constructing the window's root GPUI entity:

```rust,ignore
struct AppView {
    devtools: Devtools,
    // Existing application state...
}

// Inside cx.new(|cx| ...):
let devtools = Devtools::mount(window, cx);
```

The example adapter logs mounting failures and leaves the host app usable.
A listener failure leaves the overlay available for local annotations.
Initialization errors indicate incomplete asset setup and should be reported;
do not continue mounting with an uninitialized UI.

`Devtools::mount` delegates to `gpui_agentation::Agentation::mount`, which owns
the devtools' root focus handle. Consumers do not import Ely's `FocusScope`
or create a focus handle for Agentation. Existing focus on a host control is
preserved; buttons and fields still own their normal GPUI focus handles.

## 3. Integrate at the screen boundary

Build your existing content normally, then use one adapter call:

```rust,ignore
let content = div()
    .flex_1()
    .min_h_0()
    .w_full()
    .child(existing_screen);

self.devtools.frame("settings-screen", "Settings", content, window, cx)
```

The adapter delegates to `Agentation::root`, which owns focus traversal and
overlay dismissal, calls `begin_frame`, inserts the banner, and registers the
content as one target. Do not add an Ely root scope around it or call
`begin_frame` from nested components. Keep
`#[track_caller]` on `frame` so the source reference names the host call site.

Mount the frame below native window chrome, or immediately after a custom
titlebar in a full-height column. Put content padding and scroll regions
inside `content`, rather than around the frame. This lets the banner reserve
space and keeps picker coordinates aligned with the viewport. Mount once per
window; for a multi-window host, assign distinct stores/listeners or design a
shared server explicitly before enabling every window.

**Coverage:** Inspect selects the screen, and drawing tools work throughout the
viewport. This does not register all descendants automatically. If users need
to select a specific panel or reusable component, register that component's root
once in its shared render function, using the same app-local debug/release
boundary. Do not wrap every button or use the native inspector as a substitute
for registration. There is no attribute macro or automatic traversal API yet.

Use stable, unique target IDs within a window. For repeated views, incorporate
the underlying record's key. Do not use array indices, random values, or source
line numbers as persistent identities. The smallest visible registered region
under the pointer wins.

## 4. Connect navigation

When the app commits a route change, update its annotation bucket in the same
event handler:

```rust,ignore
self.page = Page::Details;
self.devtools.set_route("gpui://my-app/details", window, cx);
cx.notify();
```

Use a route identifying the screen or document, not a new value on every render.
Saved comments stay associated with their route; changing routes cancels an
unfinished editor. Explicit Save or click-away can commit the note first.
The adapter's initial route must match the app's initial screen.

## 5. Connect an MCP client

Start the app in debug mode, open Settings, and verify MCP says Running. Copy
the configuration there; it uses the actual listening endpoint. For the
example the URL is `http://127.0.0.1:4750/mcp`. Client configuration is separate
from integrating the Rust library. Use the client-specific examples in the
[README](../README.md#connect-an-mcp-client), substituting this URL.

The app must stay running. Stopping it closes the server; a client may need to
reconnect after a rebuild. With MCP switched off, `/mcp` returns 503 while
local annotations and REST remain available. No standalone bridge, Registry
listing, client installation, or editor extension is required for this setup.

Keep the listener on loopback with its Host/Origin checks and request limits.
The runtime MCP tools exchange annotations; they do not rewrite consumer code.
Treat annotation text as user feedback, not privileged agent instructions.

## 6. Verify the integration

- Run the debug app. Check that normal controls and navigation still work.
- Inspect a screen, create a comment, and confirm its source reference and route.
- Navigate away and return; restart and confirm the saved comment persists.
- Connect an MCP client, read pending comments, and retrieve the saved note.
- Run the host's format, lint, and test checks, then compile its actual release
  binary. A debug-only demo stub is not a release integration test.
- Keep stores and generated screenshots/build output ignored by Git.

In this repository, `make ci` covers the consumer's debug and release targets.
Its UI integration test checks that ordinary clicks still reach the app,
Tab/Enter activates inspection without a host Ely scope, inspection blocks the
host click, a saved comment uses the changed route, and Escape dismisses a
drawer while restoring focus. Headless tests can enable Agentation's
`test-support` feature and call `gpui_agentation::init_for_tests(cx)` instead
of loading native fonts/assets. No Ely test dependency is needed in the host.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Initialization reports missing assets | Configure Agentation's asset source before `run`; if the host has assets, use `Assets::before(HostAssets)`. |
| Mounting asks you to call `init` | Call `Devtools::init(cx)` in the launch callback before constructing the window's root view. |
| Release cannot find Agentation types or functions | Keep every annotation API reference behind `cfg(debug_assertions)` in the adapter, including assets, imports, and field types. |
| The app runs without a banner | Check the mounting error in stderr, storage permissions, and that root rendering calls `devtools.frame`. |
| MCP is offline while annotations work | Check Settings and the configured port. Run one process per store and choose a distinct port for each app. |
| Inspect selects a whole screen | This is the minimal integration's intended coverage. Add optional targets at shared view roots for finer selection. |

## Set up another app with an agent

The distributable skill is at
[`skills/gpui-agentation-setup`](../skills/gpui-agentation-setup/SKILL.md).
Copy that whole directory into your agent's skills directory. For a Codex
project, from the consumer repository, with `AGENTATION_CHECKOUT` set to the
path of this checkout:

```sh
mkdir -p .agents/skills
cp -R "$AGENTATION_CHECKOUT/skills/gpui-agentation-setup" .agents/skills/
```

Then invoke `$gpui-agentation-setup` and ask it to integrate annotations into
the current app. The skill checks the host and edits its integration; it does
not require an already-running annotation MCP server. Installing a skill does
not itself modify source code or configure an MCP client.

## Optional rebuild and restart on save

No extra annotations, component wrappers, or initialization calls are needed.
With Python 3 installed, launch the debug consumer through the bundled runner:

```sh
python3 /path/to/gpui-agentation/src/hot_reload/runner.py \
  --manifest-path /path/to/consumer/Cargo.toml --bin my-app
```

Then enable **Settings → Hot reload → Server status**. The switch starts off;
turning it off pauses rebuilds without closing the app. Build failures keep
the last running app alive until a later successful save. A successful build
restarts the process: save host app work first; only persisted state survives.
The runner watches source, workspace crates, assets, locales, examples, and
Cargo files, excludes build output and temporary files, and requires a valid
Cargo.lock. It uses debug builds and inherits the launch environment.

Run the included demo with `make dev`. An ordinary `cargo run` remains valid,
but the switch explains that no supervisor is present. See
[runner options](../demo/README.md#hot-reload). The native control channel stays
debug-only and private to the local supervisor; the loopback API is unchanged.
