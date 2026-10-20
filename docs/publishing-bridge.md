# Publish the combined gpui-agentation package

The candidate is one package, version 0.2.0: the debug-only native library and
a default-enabled `gpui-agentation` binary behind the optional `mcp` feature.
Native consumers explicitly enable `native` and disable default features. It replaces the
previous source layout containing an independent bridge crate; it does not remove or yank the
already published `gpui-agentation-mcp` 0.1.0.

## Pinned Ely distribution

The unpublished Git-only dependency is replaced by the attributable
`gpui-agentation-ely =0.1.1` distribution in `vendor/ely`. Its source, assets,
licenses, normal dependencies and GPUI pins match upstream revision
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721`. See its README for Cargo identity
and existing-host migration requirements. This is a support dependency; the
native library and optional bridge remain one `gpui-agentation` package.

Run `python3 .github/scripts/check-ely-snapshot.py --upstream /path/to/pinned/ely`
before releasing. The checker rejects source/asset drift and dependency or pin
changes. Root Cargo packaging replaces the local path with the exact registry
version, so the snapshot **must be published first**.

## Release sequence

1. Review changes against `dev`; promote `dev → next → main`.
2. Update Cargo.toml, Cargo.lock and server.json together. The candidate Registry
   version must be new: 0.1.0 already identifies the old package in the live entry.
3. Check identity with `python3 .github/scripts/check-bridge-release.py`.
   Use `--require-publishable` for the additional publication guard.
4. Bootstrap the support dependency first from clean promoted main:

   ```sh
   cargo publish --manifest-path vendor/ely/Cargo.toml --dry-run
   cargo publish --manifest-path vendor/ely/Cargo.toml
   ```

   Wait for registry indexing, then verify and bootstrap the combined package:

   ```sh
   cargo publish --dry-run --all-features --locked
   cargo publish --all-features --locked
   ```

   Use a fresh secure Cargo credential for first uploads. The old package's
   trusted publisher does not automatically apply to the new package.
5. Configure the new crate's trusted publisher with owner `emmanuel-defreitas`,
   repository `gpui-agentation`, workflow `publish-bridge.yml`, and environment
   `crates-io`. That environment is restricted to `main`; its private-repository
   billing plan does not support additional required environment reviewers.
   For future Ely snapshot updates, repeat provenance verification and publish
   the support dependency first. Its version is immutable for this revision;
   the combined-package workflow never silently publishes a changed snapshot.
6. Subsequent releases dispatch `Publish MCP Bridge` from `main` with matching
   version and `publish=true`. It publishes this package with the MCP feature;
   there is no static-token fallback. Release-tag events do not publish.
7. Publish the new matching Registry version only after the crate exists:

```sh
mcp-publisher login github
mcp-publisher publish server.json
```

The Registry identity stays `io.github.emmanuel-defreitas/gpui-agentation`, while
its Cargo identifier becomes `gpui-agentation`. Native clients retain the same
loopback endpoint. Keep authentication state outside the repository and keep
the visible `mcp-name` marker in the root README.
