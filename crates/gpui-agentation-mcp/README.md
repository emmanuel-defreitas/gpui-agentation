# GPUI Agentation MCP bridge

A standalone stdio MCP executable connecting an agent to development annotations
in a running local GPUI app. It has no GPUI/Ely dependency, starts no native UI,
and works in release builds. The app still needs the native Agentation library.

## Run from source

```sh
cargo run --manifest-path crates/gpui-agentation-mcp/Cargo.toml --locked -- --url http://127.0.0.1:4750/mcp
```

The host must be running in debug mode with MCP enabled. The consumer example
uses port 4750, the playground uses 4749, and the default host setup uses 4748.
Copy the actual URL from the app's Settings. `--url` overrides
`GPUI_AGENTATION_URL`; the default is `http://127.0.0.1:4748/mcp`.

After this package is published, install it with:

```sh
cargo install gpui-agentation-mcp --locked
```

For an MCP client accepting JSON configuration:

```json
{
  "mcpServers": {
    "gpui_agentation": {
      "command": "gpui-agentation-mcp",
      "args": ["--url", "http://127.0.0.1:4750/mcp"]
    }
  }
}
```

Use the absolute path to the installed binary if the client doesn't inherit
your shell's PATH. Before publication, use the built executable at
`crates/gpui-agentation-mcp/target/release/gpui-agentation-mcp` instead.

## Behavior

- Reads newline-delimited JSON-RPC from stdin and writes only compact JSON-RPC
  responses to stdout. Diagnostics go to stderr. EOF ends the process.
- Initializes locally, even when the app is offline. Tool discovery/calls
  forward to the app; start the app before discovering tools. An offline or
  paused app returns an actionable error. The bridge stays alive for retries.
- Negotiates MCP 2025-03-26, 2025-06-18, or 2025-11-25. The app's stateless
  JSON Streamable HTTP endpoint supplies the tool schemas/results; the bridge
  does not duplicate the tool catalog or annotation storage.
- Accepts only literal loopback HTTP URLs with `/mcp`, an explicit port, and
  no credentials/query/fragment. The current native listener uses IPv4.
  Proxies and redirects are disabled; no API token is needed.
- Limits stdin requests to 32 KiB and backend responses to 8 MiB. HTTP requests
  time out after 10 seconds (`--timeout-ms` accepts 1–60000). Writes are never
  automatically retried, avoiding duplicate replies or status mutations.
- Sequential request forwarding; no server-initiated requests, SSE stream,
  or task cancellation. This is an adapter for Agentation's stateless HTTP API,
  not a general-purpose MCP HTTP proxy.

The bridge does not launch the app, edit source, change client settings, or
publish anything. Native direct-HTTP MCP configurations remain supported.

## Verify and prepare Registry publishing

```sh
cargo fmt --manifest-path crates/gpui-agentation-mcp/Cargo.toml -- --check
cargo clippy --manifest-path crates/gpui-agentation-mcp/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path crates/gpui-agentation-mcp/Cargo.toml --locked
cargo publish --manifest-path crates/gpui-agentation-mcp/Cargo.toml --dry-run --locked
```

`server.json` prepares a Cargo/stdio listing for the official MCP Registry.
Publish the crate first, verify namespace ownership and matching versions, then
validate/publish that metadata with `mcp-publisher`. The visible ownership marker
below is required for Cargo README verification; do not hide it in an HTML comment.

- MCP Registry name: `mcp-name: io.github.emmanuel-defreitas/gpui-agentation`
- Repository: https://github.com/emmanuel-defreitas/gpui-agentation
- Registry publishing guide: https://github.com/modelcontextprotocol/registry/blob/main/docs/modelcontextprotocol-io/quickstart.mdx

Original MIT license and attribution are retained in LICENSE.
