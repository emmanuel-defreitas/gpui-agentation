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
