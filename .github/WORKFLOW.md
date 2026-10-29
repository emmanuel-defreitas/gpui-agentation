# Development and release workflow

The native library supports macOS only. Run `make verify` locally before
merging; `make ci` remains an alias. Website changes also require
`make site-check`. GitHub Actions handles promotion and release operations;
it does not compile native code or run Rust/website tests.

Work branches target `dev`, then promotion follows `dev → next → main`.
Maintainers create branches/issues, review conventional commit titles and
local verification results, and merge through review.

Before promotion, prepare and review the version with `make release-prepare`,
update the changelog, and verify the candidate locally. Promotion checks validate
metadata only. Publishing uses Linux with `--no-verify`; first run
`cargo publish --dry-run --all-features --locked` locally on macOS.

The workflow inventory is in [LIFECYCLE.md](./LIFECYCLE.md). Branch protections,
version rules, and trusted publishing are defined in
[BRANCH-AND-RELEASE-POLICY.md](./BRANCH-AND-RELEASE-POLICY.md).
