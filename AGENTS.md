# gpui_agentation

Native development annotations for GPUI and Ely. Read README.md before editing.

- The native library supports macOS only. Run Rust and website checks locally; GitHub Actions is limited to promotion and release operations.
- Keep the public API independent of Corpora and other host applications.
- Keep `cfg(debug_assertions)` on the crate; release builds expose no annotation API.
- Match the pinned Ely revision and `gpui-pre =0.3.8`; do not mix GPUI sources.
- Preserve the original MIT license and attribution.
- Use Ely components and theme values. Keep UI state in GPUI entities.
- Keep API traffic on loopback and preserve origin/host checks and body limits.
- Never commit credentials, local annotation stores, or generated build artifacts.

## Ely to Kit migration (#112)

- The production library stays on the pinned Ely/GPUI 0.3.8 graph until the
  atomic cutover. Preparatory work must keep that graph buildable.
- `migration/kit-probe` is an independent, unpublished workspace evaluating
  exactly GPUI Kit 0.11.0 on its standard GPUI 0.3.14 backend. This is the
  preparation exception to the Ely/0.3.8 rule; do not add the probe as a
  production workspace member or dependency, and do not enable `gpui-fast`.
- The cutover replaces Ely completely with Kit. Follow the contract in
  `migration/kit-probe/README.md`: preserve host init/theme/assets/Root and
  inspector ownership; keep annotation and inspector services Agentation-owned.
- Keep the default MCP-only graph free of native UI dependencies. Removal of
  the public `ely` module and the coordinated host GPUI upgrade require a
  pre-1.0 minor release and migration instructions.
- Run `make kit-probe-check` for probe changes. Its success does not establish
  that the production Agentation library has migrated or passed native QA.
- On `migration/kit-cutover`, `migration/kit-inspector` compiles the actual
  owned inspector services and Kit presentation in another isolated workspace.
  Use `make kit-inspector-check` and its documented Cargo launcher, which applies
  version-checked Kit overlay-picking patches to ignored dependency copies.
  Keep the root PR draft: production UI integration and patch delivery remain
  explicit gates in `migration/kit-cutover.md`.

## Verification

```sh
make verify
make site-check  # when website files change
```

The core Rust checks included in `make verify` are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --release --all-features --locked
```

`dev` is the default branch. Changes target `dev`; promotion runs through `next`
into `main`. See `.github/BRANCH-AND-RELEASE-POLICY.md` for the inherited policy.
Use conventional commits. Keep edits scoped and preserve public API behavior.
