# Publish the combined gpui-agentation package

The 0.6.0 candidate contains the debug-only native library, bundled pinned Ely
source/assets and a default-enabled stdio binary. There is one Cargo package to
publish. Previously published releases remain usable and are not removed/yanked.

Before promotion, run `make release-prepare`. It infers the stable version from
conventional commits and updates Cargo.toml, Cargo.lock, server.json, current
documentation, setup skill, and site references together. Use
`make release-prepare VERSION=x.y.z` to prepare an explicit stable version; the
promotion gate still requires it to agree with conventional-commit history.
Settings, CLI, and MCP versions derive from Cargo at build time. Review the diff,
add release notes to CHANGELOG.md without rewriting history, and run
`make verify release-version-check` on macOS before promotion. Run
`cargo publish --dry-run --all-features --locked` on the exact release candidate
to verify the extracted package locally.

The Cargo archive excludes the README demonstration GIFs under `site/docs/assets/`;
they remain available in the repository and site. The publishing workflow runs
on Linux, packages without compilation, and checks crates.io's 10 MiB upload
limit with `check-bridge-release.py --check-package-size`. Compilation and tests
are local macOS responsibilities; hosted publication uses `--no-verify`.

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

1. Prepare the version and review changes against `dev`; promote `dev → next → main`.
2. Include the synchronized version files in that review. The candidate Registry
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
