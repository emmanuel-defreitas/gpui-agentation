# Consumer app

A complete plain GPUI app using Agentation through its public API, with no
direct Ely dependency, initialization, or focus scope. It belongs to
the root Cargo workspace and runs in both debug and release builds.

```sh
# From the repository root:
cargo run -p agentation-consumer --locked
cargo run -p agentation-consumer --release --locked
```

Debug adds the banner, screen-level inspection, drawing, Comments, Settings,
and MCP at `http://127.0.0.1:4750/mcp`. Notes live in
`target/consumer/feedback.json`. Release runs the same app without annotations.

- `src/main.rs`: normal controls, two routes, and one `devtools.frame` call.
- `src/devtools.rs`: Agentation assets/init, root mounting, route hook, and all
  debug gates. Agentation owns its internal UI and focus setup.

Inspect selects the screen as a whole. Controls have no individual wrappers.
Optional finer targets can be added at shared component roots.

Follow the [consumer setup guide](../site/docs/consumer-setup.md) to adapt the
example, choose dependencies and storage, and connect an agent.
