<img src="docs/assets/mark.svg" width="48" height="48" alt="gpui-agentation logo">

# gpui-agentation

Annotate a running GPUI app and send feedback to your coding agent.
Inspect an element, draw a box or arrow, and add a comment. Copy the feedback as
Markdown or let an agent read and respond through the local MCP server.

[Preview & docs](https://emmanuel-defreitas.github.io/gpui-agentation/) · [MIT license](LICENSE)

Development builds only. Release builds expose no annotation API.

## Try the demo

Requires macOS, Xcode command-line tools, Rust via rustup, and Python 3.
The repository pins the Rust toolchain. Ely source and assets are bundled.

From this checkout:

```sh
make dev
```

1. Choose **Inspect**, **Box**, **Arrow**, or **Pen**.
2. Select an element or draw, then write a comment and save.
3. Open **Comments → Copy** to copy your feedback as Markdown.

For rebuilds on save, enable **Settings → Development → Hot reload**.
It rebuilds and restarts the app after a successful build. In-memory app state
resets; saved annotations survive. [Demo details](demo/README.md)

## Add to your app

Enable the native library:

```toml
[dependencies]
gpui-agentation = { version = "0.4.0", default-features = false, features = ["native"] }
```

Keep your app on `gpui-pre =0.3.8`. Follow the
[setup guide](docs/consumer-setup.md) then initialize
Agentation and mount it once at your screen root. Agentation handles Ely setup,
assets, and focus internally; your app does not need to use Ely.

Ordinary controls need no wrappers. Register finer inspection targets only
where useful. Keep the integration behind `#[cfg(debug_assertions)]`.

See the [consumer example](examples/consumer/README.md),
[setup skill](skills/gpui-agentation-setup/SKILL.md), and
[customization reference](docs/reference.md#settings).

## Connect your agent

1. Keep the debug app running.
2. Open **Settings → MCP** and turn the server on.
3. Copy the config into your MCP client.

The endpoint runs on loopback. Clients that require stdio can use the
[MCP bridge](docs/package-layout.md#mcp-client).

## Development

Run `make ci` for checks or `make help` for available commands.
See the [API and integration reference](docs/reference.md) for details.
Changes target `dev`, then promote through `next` to `main`.

Inspired by [Agentation](https://www.agentation.com/). Independent GPUI
implementation. Original MIT attribution is preserved in [ORIGIN.md](ORIGIN.md).

- MCP Registry name: `mcp-name: io.github.emmanuel-defreitas/gpui-agentation`
