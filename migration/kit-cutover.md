# Production Kit cutover (historical acceptance record)

The release now uses unmodified registry Kit, with picking compatibility inside
Agentation. The record below describes the earlier vendored development stage.
See [current distribution](../site/docs/kit-distribution.md).

Draft root PR #131 targets dev. The production library, consumer example,
inspector, toolbar, comment/textbox editors and Settings now use Kit 0.11.0
on GPUI 0.3.14. The old source bundle and generated component namespace are
removed. The standalone prototype workspaces have been retired in favor of
production tests.

The branch began at dev `2a370c4a420ccfb8c1e68dea5467baf18161026c` and includes
preparation heads #129 (`7e2e598`) and #130 (`0233eca`). Reconcile their final
merge state before merging this root. No release or promotion is implied.

Run `make verify` and `make site-check`. `snapshot-check` verifies the complete
Kit inventory, one active GPUI backend, and the portable MCP dependency graph.
Native interaction acceptance must be recorded separately from compilation.

The source bundle fixes inspector picking through transparent Kit overlays.
Git/path consumers share its direct dependency edges; registry publication
requires a published Kit family with that fix. See
[distribution](../site/docs/kit-distribution.md) and
[consumer migration](../site/docs/consumer-setup.md).

## Verified production acceptance

- `make verify`: formatting, strict Clippy, 141 library tests, two consumer
  tests, eight bridge tests, 29 release-script tests, release compilation,
  seven portable MCP tests and the default bridge install smoke all pass.
- `make site-check`: Astro build, documentation export and link checks pass.
- Native production consumer smoke exercises standalone and already-initialized
  Kit hosts, an already-open inspector, retained Root/content and visible host
  size, Inspect/Stop, Settings, and host-close cleanup.
- A separate consumer outside the workspace passes debug and release checks
  using direct paths, with no consumer-level patch table.
- Headless regressions cover editing/reset, annotation persistence and input,
  asset composition, preserved host renderer, picked-inspector cleanup, and
  fullscreen/hidden-toolbar placement without losing selection.

Native smoke commands (macOS):

```sh
cargo run -p agentation-consumer --features native-smoke -- --smoke
cargo run -p agentation-consumer --features native-smoke -- --host-initialized --from-in-host --smoke
```

Use `GPUI_AGENTATION_STORE` to select an isolated annotation store for these runs.
Automated native checks do not constitute manual visual/IME acceptance. The
OS-level appearance switch was skipped at the user's request. Publishing is
explicitly disabled until the registry Kit dependency carries the picking fix.
