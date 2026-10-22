# Publish the combined gpui-agentation package

The 0.4.2 candidate contains the debug-only native library, bundled pinned Ely
source/assets and a default-enabled stdio binary. There is one Cargo package to
publish. Previously published releases remain usable and are not removed/yanked.

When bumping the version, update `Cargo.toml` and `server.json`, then fix every
reference that `make version-check` reports. Leave CHANGELOG history alone.

## Bundled Ely verification

Run `python3 .github/scripts/check-ely-snapshot.py --upstream /path/to/pinned/ely`.
This checks the immutable upstream revision, source/asset/license hashes, GPUI
0.3.8 pins and optional native dependency mapping. There is no Ely path or
registry dependency. Native builds generate namespace/asset adaptations from
unchanged upstream files; MCP-only builds skip that work.

Verify Cargo's extracted package with all features, and independently verify
the default portable install. Check the archive contains the full vendor source,
assets, upstream manifest, provenance and license notices. See
[distribution details](ely-distribution.md).

## Release sequence

1. Review changes against `dev`; promote `dev → next → main`.
2. Update Cargo.toml, Cargo.lock and server.json together. The candidate Registry
   version must be new: 0.1.0 identifies the old bridge and 0.2.0 the prior
   combined package in the live Registry.
3. Check identity with `python3 .github/scripts/check-bridge-release.py`.
   Use `--require-publishable` for the additional publication guard.
4. From clean promoted main, verify and publish the one package:

   ```sh
   cargo publish --dry-run --all-features --locked
   cargo publish --all-features --locked
   ```

   There is no support-crate publication prerequisite.
5. Retain the configured trusted publisher with owner `emmanuel-defreitas`,
   repository `gpui-agentation`, workflow `publish-bridge.yml`, and environment
   `crates-io`. That environment is restricted to `main`; its private-repository
   billing plan does not support additional required environment reviewers.
6. Subsequent releases dispatch `Publish MCP Bridge` from `main` with matching
   version and `publish=true`. It publishes this package with the MCP feature;
   there is no static-token fallback. Release-tag events do not publish.
7. Publish the new matching Registry version only after the crate exists:

```sh
mcp-publisher login github
mcp-publisher publish server.json
```

The Registry identity stays `io.github.emmanuel-defreitas/gpui-agentation`, while
its Cargo identifier remains `gpui-agentation`. Native clients retain the same
loopback endpoint. Keep authentication state outside the repository and keep
the visible `mcp-name` marker in the root README.
