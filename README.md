<img src="docs/assets/mark.svg" width="48" height="48" alt="gpui-agentation logo">

# gpui-agentation

Point at a running GPUI app, leave a note, and your coding agent gets the
element, its bounds, and its Rust source location.

> **Status: pre-release.** 0.4.1 is on [crates.io](https://crates.io/crates/gpui-agentation);
> `dev` is heading to 0.5. Pre-1.0: the API can change between minor releases, so
> pin the version. Debug builds only: release builds expose no annotation API.
> Pinned to `gpui-pre =0.3.8`. [MIT](LICENSE).

[Website](https://emmanuel-defreitas.github.io/gpui-agentation/) · [Reference](docs/reference.md)

## Why this

[Agentation](https://www.agentation.com/) annotates web pages through the DOM.
A native GPUI window has no DOM, so this registers screens (and optional finer
targets) with an ID, label, bounds and Rust call site, and attaches your note to
that.

Use it when you would otherwise describe a UI bug to your agent in words or paste
a screenshot. Notes persist to a local JSON file, carry a status (pending,
acknowledged, resolved, dismissed) and a reply thread, and an agent reads them
over a loopback MCP server or REST/SSE.

It is a development tool. It annotates registered targets, not arbitrary pixels
(by default, the whole screen), and release builds do not include it. For
production telemetry or session replay, use something else.

## Add to your app

```sh
cargo add gpui-agentation --no-default-features --features native
```

Initialize once at launch, mount once per window, and wrap your screen once. The
crate does not exist in release builds, so every call sits behind
`#[cfg(debug_assertions)]` and release returns your content unchanged:

```rust
// Launch: serve Agentation's assets, then initialize.
let app = gpui_platform::application();
#[cfg(debug_assertions)]
let app = app.with_assets(gpui_agentation::Assets);
app.run(|cx| {
    #[cfg(debug_assertions)]
    gpui_agentation::init(cx).expect("initialize Agentation");
    // ...open your window
});

// Window setup: app name, route, notes file, local port. Keep `feedback`
// in a field of your root view that carries the same cfg.
#[cfg(debug_assertions)]
let feedback = gpui_agentation::Agentation::mount(
    "My app", "gpui://my-app/home", "target/agentation/feedback.json", 4748, window, cx,
)?;

// Render: wrap the screen in debug builds; release keeps `content` as is.
#[cfg(debug_assertions)]
let content = self.feedback.root("home", "Home screen", content, window, cx);
content
```

The [consumer example](examples/consumer/README.md) keeps all of this in one
`devtools` module. Ordinary controls need no wrappers; add
`target(id, label, child)` only where you want finer inspection. Agentation
handles its Ely setup, assets and focus internally. More: [setup guide](docs/consumer-setup.md),
[setup skill](skills/gpui-agentation-setup/SKILL.md) for agents,
[customization](docs/reference.md#settings).

## Connect your agent

1. Keep the debug app running.
2. Open **Settings → MCP** and turn the server on.
3. Copy the config into your MCP client.

The endpoint runs on loopback. Clients that require stdio can use the
[MCP bridge](docs/package-layout.md#mcp-client).

## Try the demo

Requires macOS, Xcode command-line tools, Rust via rustup, and Python 3.
The repository pins the Rust toolchain, and Ely is bundled.

```sh
git clone https://github.com/emmanuel-defreitas/gpui-agentation
cd gpui-agentation
make dev
```

1. Choose **Inspect**, **Box**, **Arrow**, or **Pen**.
2. Select an element or draw, then write a comment and save.
3. Open **Comments → Copy** to copy your feedback as Markdown.

For rebuilds on save, enable **Settings → Development → Hot reload**. It rebuilds
and restarts the app after a successful build; in-memory state resets, saved
annotations survive. [Demo details](demo/README.md)

## Development

Run `make ci` for checks or `make help` for available commands.
Changes target `dev`, then promote through `next` to `main`.

Inspired by [Agentation](https://www.agentation.com/). Independent GPUI
implementation. Original MIT attribution is preserved in [ORIGIN.md](ORIGIN.md).

- MCP Registry name: `mcp-name: io.github.emmanuel-defreitas/gpui-agentation`
