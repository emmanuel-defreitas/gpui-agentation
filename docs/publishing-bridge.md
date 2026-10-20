# Publish the combined gpui-agentation package

The candidate is one package, version 0.2.0: the debug-only native library and
an optional `gpui-agentation` binary behind the `mcp` feature. It replaces the
source layout with an independent bridge crate; it does not remove or yank the
already published `gpui-agentation-mcp` 0.1.0.

## Publication blocker

The exact Ely revision remains a Git dependency. Cargo requires registry
versions even for optional Git dependencies during publication. Therefore
`publish = false` remains set, and publishing validation fails before requesting
OIDC credentials. `--no-default-features` does not bypass this restriction.

Resolve the upstream registry distribution of the pinned Ely revision before
removing the guard. Cargo permits a Git-plus-version dependency during local
development and uses the registry version in the published package; verify that
the registry artifact matches the required pinned source before adding that fallback. Do not silently change renderer sources, vendor a different
revision, or publish a broken native feature. Then run the full feature matrix
and Cargo's actual package verification before promotion/publication.

## Release sequence

1. Review changes against `dev`; promote `dev → next → main`.
2. Update Cargo.toml, Cargo.lock and server.json together. The candidate Registry
   version must be new: 0.1.0 already identifies the old package in the live entry.
3. Check identity with `python3 .github/scripts/check-bridge-release.py`.
   Use `--require-publishable` for the additional publication guard.
4. Once publishable, bootstrap the new `gpui-agentation` package from clean
   promoted main with a fresh secure Cargo credential. The old package's
   trusted publisher does not automatically apply to the new package.
5. Configure the new crate's trusted publisher with owner `emmanuel-defreitas`,
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
its Cargo identifier becomes `gpui-agentation`. Native clients retain the same
loopback endpoint. Keep authentication state outside the repository and keep
the visible `mcp-name` marker in the root README.
