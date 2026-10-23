# gpui-agentation

Source: https://emmanuel-defreitas.github.io/gpui-agentation/

Visual feedback for your native GPUI app. Annotate the interface. Give your agent the context.

[Add to your app ↗](https://emmanuel-defreitas.github.io/gpui-agentation/#installation) [Rust crate v0.5.1](https://crates.io/crates/gpui-agentation/0.5.1) 

## See feedback take shape.

Illustrated demo

![Illustrated dashboard annotation sequence: a cursor selects cards, activity rows, an export button, and sidebar icons; comments are typed into amber popovers and saved as numbered pins.](https://emmanuel-defreitas.github.io/gpui-agentation/assets/motion/dashboard-demo-poster.svg?v=35d6cdd052b5)

An illustrated look at the workflow. The installed tools run inside your native GPUI app.

### Point to the exact place.

Select a registered component. Keep its bounds and Rust source reference with the comment.

### Make the change visible.

Box an area, draw an arrow, or sketch with the pen. Give a visual idea a precise anchor.

### Carry the context forward.

Copy structured Markdown or connect an MCP client to read, reply, and resolve feedback.

## Small package. Native feedback.

Add the library to your GPUI app. Install the bridge separately if you want your coding agent to read and respond to annotations through MCP.

Rust 1.95+ · GPUI 0.3.8 · Debug builds · MIT

### 01 Add the native library

In your app’s `Cargo.toml`, enable `native` and turn off default features. Ely’s pinned source and assets come bundled in the package.

Cargo.toml

```toml
[dependencies]
gpui-agentation = { version = "0.5.1", default-features = false, features = ["native"] }
gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
```

Use the same `gpui-pre` version and registry source throughout your app. Other GPUI sources produce incompatible Rust types.

### 02 Install the optional MCP bridge

Terminal

```shell
cargo install gpui-agentation --version 0.5.1 --locked
```

The default install builds the portable bridge executable. It does not compile GPUI or Ely and does not launch your app.

## Wire it in once.

Keep the integration at your application root. Register meaningful component boundaries you want to select, rather than wrapping every widget.

These instructions target the 0.5.1 Agentation API. Gate all annotation imports, initialization, fields, and render hooks with `#[cfg(debug_assertions)]`: release builds expose no annotation API.

1.  ### Initialize Agentation
    
    Configure `gpui_agentation::Assets` on your Application, then call `gpui_agentation::init(cx)` in the launch callback. Agentation handles its bundled Ely setup and focus internally.
    
2.  ### Mount once per window
    
    Root initialization · debug only
    
    ```rust
    let feedback = gpui_agentation::Agentation::mount(
        "My app", "gpui://my-app/home",
        "target/agentation/feedback.json", 4748, window, cx,
    )?;
    ```
    
3.  ### Render your screen
    
    Root render · debug only
    
    ```rust
    self.feedback.root(
        "home", "Home screen", content, window, cx,
    )
    ```
    
    Ordinary controls need no wrappers. Add `target(id, label, child)` only for finer inspection targets. Keep IDs stable and unique.
    

## Give your agent the whole comment.

Run your native app in debug mode, then add the installed bridge to an MCP client that supports stdio servers. The bridge forwards requests to the app’s local listener.

MCP client configuration · JSON

```json
{
  "mcpServers": {
    "gpui_agentation": {
      "command": "gpui-agentation",
      "args": ["--url", "http://127.0.0.1:4748/mcp"]
    }
  }
}
```

Use your client’s equivalent server configuration if its schema differs. The agent can read pending annotations, acknowledge them, reply, and resolve or dismiss feedback. Keep the native app running while using these tools.

Loopback only. No API key. Browser origins are rejected by the native API. The site’s illustrations do not connect to your local server.

[View the MCP Registry entry ↗](https://registry.modelcontextprotocol.io/v0.1/servers/io.github.emmanuel-defreitas%2Fgpui-agentation/versions/0.5.1) 

## Give your agent a working guide.

Two reusable skills help your agent install gpui-agentation and explore feedback from a running app. Choose the workflow you need.

**[Setup skill ↗](https://github.com/emmanuel-defreitas/gpui-agentation/blob/dev/skills/gpui-agentation-setup/SKILL.md) **

`gpui-agentation-setup` guides native library integration, screen registration, and debug and release verification.

**[Exploration skill ↗](https://github.com/emmanuel-defreitas/gpui-agentation/blob/dev/skills/gpui-agentation-explore/SKILL.md) **

`gpui-agentation-explore` discovers MCP tools, reads sessions and pending notes, exports Markdown, and guides requested fixes through acknowledgment, replies, and verified resolution. It also explains the native drawing tools and API limits.

From a repository clone, copy the chosen directory under `skills/` into your host project’s `.agents/skills/`, or `~/.agents/skills/` for personal use across projects. In Zed, start a new conversation and invoke `/gpui-agentation-explore` or `/gpui-agentation-setup`. Other agents can load the linked `SKILL.md` directly or use their supported skill directory.

Try: “Use gpui-agentation-explore to discover my app’s MCP tools and summarize pending feedback without changing notes.” Connect the running debug app’s MCP server first; installing a skill provides instructions, not a server connection.

The linked skills track the `dev` branch. Use the skill from your host’s gpui-agentation revision when working with a pinned version.

## Make it fit your app.

The integration points are small and explicit. Start with the defaults and adjust the context your agent needs.

**App name & route**

Name the app when mounting. Call `set_route(url, window, cx)` on navigation to group saved feedback by screen.

**Selectable components**

Choose which component roots to register with `target(id, label, child)`. Keep IDs stable and unique within the window.

**Storage**

Set a local JSON path when mounting. Use one running app process per store file; keep feedback files out of version control.

**Local port**

Change the mount port if `4748` is in use, and match it in your MCP client’s `--url`. A busy port leaves local notes usable.

**Drawing tools**

Use Inspect, Box, Arrow, and Pen in the native toolbar. Four built-in color swatches distinguish drawing strokes.

**Build features**

Use `native` for the debug library, default `mcp` for the bridge, or both when developing this package.

Version 0.5.1 does not expose a public toolbar layout or custom palette builder.

## A familiar idea. A native implementation.

Inspired by [Agentation](https://www.agentation.com/)  and its approach to giving agents precise visual feedback. Thank you for the inspiration.

gpui-agentation is an independent Rust implementation for GPUI. It is not affiliated with Agentation and does not implement its complete protocol. The native interface uses bundled [Ely components](https://elygpui.com/) ; upstream licenses and attribution are preserved.

[Project source ↗](https://github.com/emmanuel-defreitas/gpui-agentation) [MIT license ↗](https://github.com/emmanuel-defreitas/gpui-agentation/blob/main/LICENSE) [Agentation ↗](https://www.agentation.com/)
