# Branch & Release Policy

The native library supports macOS only. Rust and website verification runs
locally; GitHub Actions is reserved for promotion and release operations.
Read [LIFECYCLE.md](./LIFECYCLE.md) for the workflow inventory.

## Branch model and local verification

Work branches target `dev`. Maintainers create branches and tracking issues,
review conventional PR titles, manage labels, and merge changes manually.
Use conventional commit subjects and squash titles because release versioning
reads the integrated commit history. Issue creation, automatic drafts, AI reviews,
Dependabot auto-merge, hosted secret scanning, and routine PR CI are removed.

Before merging, run `make verify` on macOS. `make ci` remains an alias.
For website changes, also run `make site-check`. Before promotion, rerun the
local suite on the candidate commit and review the results with the release PR.
A green hosted check confirms release metadata, not successful native tests.

## Promotion and branch protection

Only same-repository `dev → next` and `next → main` PRs are promotions.
`next-preview.yml` runs the `promotion-metadata` check for PRs targeting `next`
or `main`, validating both the lane and reviewed release metadata. It also
validates metadata on pushes to `next`. Invalid lanes
fail; ordinary PRs targeting `dev` do not start Actions.

Keep `dev`, `next`, and `main` protected against deletion and force pushes.
Checked-in rulesets require `promotion-metadata` on `next` and `main`, with
no hosted test contexts on `dev` or temporary release branches. The promotion
preview reports the same metadata context on the `next` commit, so a release
PR created using `GITHUB_TOKEN` does not depend on a new PR event to run it.

`.github/scripts/apply-branch-guardrails.sh` applies these expectations and
sets `dev` as the default branch. Its updates to GitHub are a separate
maintainer operation; editing these files does not change live repository
settings. Existing live protections must not require retired CI contexts.
Supply `MAIN_BYPASS_ACTORS_JSON` when owner/team or GitHub App bypass actors
are needed. Keep existing review and signature requirements in the live rules.

## Release flow: `dev → next → main`

1. Run `make verify` locally, review the results, and merge work into `dev`.
   Lifecycle merges can still create release candidate tags through
   `dev-release-candidate.yml`; no tests run in that workflow.
2. Run `make release-prepare` on a preparation branch targeting `dev`, review
   the version diff and `CHANGELOG.md`, and merge through review. No local
   command commits or pushes changes. Verify the candidate locally on macOS,
   including `cargo publish --dry-run --all-features --locked`, then open
   `dev → next`. Hosted promotion checks require synchronized metadata and the
   version inferred from committed history.
3. Merging to `next` runs `Promotion Preview`, which validates metadata only.
   On success, `next-to-main-release-pr.yml` opens the `next → main` PR.
4. After local verification and production review, merging `next → main`
   creates the SemVer tag and GitHub Release. Configured production deployment
   remains release-triggered.
5. Dispatch `Publish MCP Bridge` explicitly from promoted `main`. Linux jobs
   verify package identity and size and publish with short-lived crates.io
   credentials. They use `--no-verify`: compilation and tests must already
   have passed locally. Release tags do not automatically publish the crate.

### Semantic versioning (`release-tag.yml`)

| Signal | Bump |
|---|---|
| `BREAKING CHANGE` in any body, or a `<type>!:` subject | **major** at 1.0.0 or later; **minor** during 0.x development |
| any `feat:` / `feature:` subject | **minor** |
| any `fix:` subject (or default) | **patch** |

Version preparation, promotion checks, and final tagging share
`.github/scripts/release_version.py`. Only stable tags reachable from the promoted
commit establish the baseline. With no stable tag, the reviewed Cargo version
bootstraps the first release. `make release-prepare VERSION=x.y.z` accepts an
explicit stable version. Ordinary promotion checks it against commit history.
Emoji prefixes on commit subjects are accepted; PR titles retain their plain
conventional prefix for squash merges and title validation.
Historical versions and dependency pins are preserved. Release candidates use
`vX.Y.Z-dev.<PR-number>` and do not replace the production tag.

### Explicit release versions

A maintainer-approved version jump is declared in `.github/release-version.json`:

```json
{
  "from": "0.5.1",
  "to": "0.6.0",
  "reason": "Use the intended pre-1.0 annotation tools release numbering"
}
```

Commit this declaration on the release preparation branch, alongside the
`make release-prepare VERSION=0.6.0` diff and changelog. The shared version
script reads it from the requested Git revision, so uncommitted edits cannot
authorize a release. `from` must match the latest reachable stable tag; `to`
must be a stable increase and satisfy at least the conventional-commit bump.
Metadata consistency, Settings derivation, package checks and the reviewed
merge-commit tag remain mandatory. Malformed declarations and baseline drift
fail validation. Once `to` or a later stable version is tagged, the declaration
is consumed and ordinary version calculation resumes.

Before 1.0.0, a breaking change advances the minor version (for example,
`0.5.1 → 0.6.0`). Breaking changes at 1.0.0 or later advance the major version.
The withdrawn `6.0.0` release does not establish a baseline: its stable tag was
removed and its crate version was yanked. The authorized correction to `0.6.0`
updates current metadata once; ordinary preparation still refuses downgrades.

This is an input to the existing `release-tag.yml` workflow, not another release
trigger. It does not create a tag, release, merge, or deployment by itself.

## Related files

- `.github/LIFECYCLE.md`
- `.github/workflows/next-preview.yml`
- `.github/workflows/release-tag.yml`
- `.github/workflows/publish-bridge.yml`
- `CONTRIBUTING.md`
