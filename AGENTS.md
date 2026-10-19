# gpui_agentation

Native development annotations for GPUI and Ely. Read README.md before editing.

- Keep the public API independent of Corpora and other host applications.
- Keep `cfg(debug_assertions)` on the crate; release builds expose no annotation API.
- Match the pinned Ely revision and `gpui-pre =0.3.8`; do not mix GPUI sources.
- Preserve the original MIT license and attribution.
- Use Ely components and theme values. Keep UI state in GPUI entities.
- Keep API traffic on loopback and preserve origin/host checks and body limits.
- Never commit credentials, local annotation stores, or generated build artifacts.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --release --all-features --locked
```

`dev` is the default branch. Changes target `dev`; promotion runs through `next`
into `main`. See `.github/BRANCH-AND-RELEASE-POLICY.md` for the inherited policy.
Use conventional commits. Keep edits scoped and preserve public API behavior.

Agentic workflow Markdown is the source of truth. Recompile edited workflow
sources with `gh aw compile`; do not hand-edit their generated lock files.
