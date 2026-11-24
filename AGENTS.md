# gpui_agentation

Native development annotations for GPUI Kit. Read README.md before editing.

- Native support is macOS only. Rust and website checks run locally; GitHub Actions handles promotion/release.
- Keep the public API independent of Corpora and other host applications.
- Keep `cfg(debug_assertions)` on the crate; release builds expose no annotation API.
- Use the pinned Kit 0.11.0 source bundle and its standard `gpui-pre =0.3.14` backend. Do not mix registry and bundled Kit types or enable `gpui-fast`.
- Use Kit controls and theme values. Annotation state, persistence, inspection snapshots and style editing remain Agentation-owned.
- Host initialization is explicit: `HostMode::KitInitialized` preserves its Kit globals; `RendererPolicy` controls inspector replacement. Preserve host assets, Root, focus and custom rem size.
- Keep the default MCP graph free of native UI dependencies.
- Preserve licenses and source provenance in LICENSE, ORIGIN.md and vendor/KIT-UPSTREAM.json.
- Keep API traffic on loopback and preserve origin/host checks and body limits.
- Never commit credentials, annotation stores or build artifacts.

## Verification

Run `make verify`. When website files change, also run `make site-check`.
Core Rust checks: formatting, strict Clippy for all workspace targets/features,
workspace tests, and locked release compilation.

`dev` is the default and PR target. Promotion runs through `next` into `main`;
see `.github/BRANCH-AND-RELEASE-POLICY.md`. The Kit cutover is a pre-1.0 minor
API change. Its local Kit patches must be available from a publishable source
before the native crate can be released on crates.io. Git/path consumers use
the bundled dependency edges without root-level Cargo patches.

Use conventional commits. Keep edits scoped and preserve public behavior outside the documented migration API changes.
