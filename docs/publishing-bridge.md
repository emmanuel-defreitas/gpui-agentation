# Publish the standalone MCP bridge

This guide covers CI/CD for `crates/gpui-agentation-mcp` only. The native
`gpui_agentation` library stays `publish = false` while it depends on pinned Git
Ely. The bridge has its own workspace/lockfile and only registry dependencies,
so publishing it does not resolve or compile GPUI/Ely.

## Review and promotion

The bridge and its publishing workflows are reviewed together in a scoped PR
against `dev`. Follow `dev → next → main`; do not bypass required reviews or
publish an unpromoted candidate. Native UI changes are independent of this release.

`MCP Bridge CI` formats, lints, tests, and verifies the packaged crate on Linux,
macOS, and Windows. `Publish MCP Bridge` verifies the candidate commit belongs
to main history, requires matching release/Cargo/Registry versions, and dry-runs
the package before the publishing job gets credentials. Other crate paths are
never published. Existing agentic workflow sources and locks are unchanged.

## Configure trusted publishing

Configure crates.io trusted publishing for the `gpui-agentation-mcp` package:

- GitHub owner: `emmanuel-defreitas`
- Repository: `gpui-agentation`
- Workflow filename: `publish-bridge.yml`
- GitHub environment: `crates-io`

Crates.io currently requires the crate to exist before adding a trusted
publisher. For the first release, after promotion and successful checks, run:

```sh
cargo login # enter a fresh short-lived token interactively; never in chat
cargo publish --manifest-path crates/gpui-agentation-mcp/Cargo.toml --locked
cargo logout
```

Verify the new crate version, then revoke the bootstrap token on crates.io.
Create/configure the matching GitHub environment and its intended reviewers
and restrict deployments to `main`. On crates.io, open the crate's Settings →
Trusted Publishing → Add GitHub and use the exact four values above.
Do not reuse any token shared in chat or store a credential in source. The workflow has no static
token fallback: it uses `rust-lang/crates-io-auth-action` to obtain and revoke
a short-lived publishing token via GitHub OIDC.

No secrets or external environment/publisher settings have been configured by
this branch. These account-side prerequisites must be completed before an
actual publish can succeed.

## Run publishing

Update the bridge's Cargo.toml, server.json and lockfile in the release PR.
Use a version not already published. From promoted `main`, dispatch
`Publish MCP Bridge` with the matching `version`; leave `publish=false` for
the initial dry run. Set `publish=true` only when ready to publish. Published
stable GitHub releases also run the workflow; their `vX.Y.Z` tag must match
the bridge's version exactly. Releasing another component with a different
version fails validation rather than publishing the wrong bridge version.

After crates.io publication succeeds, authenticate and publish the package's
`server.json` using the official `mcp-publisher`:

```sh
mcp-publisher login github
mcp-publisher publish crates/gpui-agentation-mcp/server.json
```

Authorize the `emmanuel-defreitas` account, which owns the
`io.github.emmanuel-defreitas` namespace. If that namespace becomes an
organization, the authorizing user must be an organization owner.
Keep publisher authentication state outside the repository. Registry publishing
is separate from crates.io publishing and is not automated here. Verify the
entry through the Registry API after publishing; do not treat schema validation
as proof that package ownership passed. The ownership marker is visible in the package
README because crates.io removes HTML comments during rendering.

## Local checks

```sh
python3 .github/scripts/check-bridge-release.py
actionlint .github/workflows/bridge-ci.yml .github/workflows/publish-bridge.yml
cargo test --manifest-path crates/gpui-agentation-mcp/Cargo.toml --locked
cargo publish --manifest-path crates/gpui-agentation-mcp/Cargo.toml --dry-run --locked
```

References:

- https://github.com/rust-lang/crates-io-auth-action
- https://crates.io/docs/trusted-publishing
- https://github.com/modelcontextprotocol/registry/blob/main/docs/modelcontextprotocol-io/package-types.mdx
