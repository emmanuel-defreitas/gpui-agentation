# Local verification and promotion workflows

The native library supports macOS only. Run `make verify` locally before
merging and on the candidate before promotion. `make ci` is an alias.
Run `make site-check` for website changes and
`cargo publish --dry-run --all-features --locked` before publication.
Maintainers review these results; GitHub does not run the test suite.

## Retained GitHub Actions

| Trigger | Workflow | Operation |
| --- | --- | --- |
| Lifecycle PR merged into `dev` | `dev-release-candidate.yml` | Tag the merge as a release candidate |
| PR into `next`/`main` or push to `next` | `next-preview.yml` | Require the promotion lane for PRs and validate release metadata as `Promotion Preview` |
| Successful push-triggered `Promotion Preview` | `next-to-main-release-pr.yml` | Open the `next → main` release PR |
| `next → main` merged | `release-tag.yml` | Validate metadata, tag the reviewed merge, create a GitHub Release |
| Published release | `production-deploy.yml` | Run the configured production deployment |
| Explicit dispatch from `main` | `publish-bridge.yml` | Check package identity/size and perform trusted publication when requested |

All retained jobs use standard Linux runners. No hosted Rust compilation,
Rust tests, website tests, cross-platform matrices, or AI/issue automation runs.
The publishing workflow packages and publishes with `--no-verify`; local macOS
verification replaces its former hosted compilation checks.

## Manual development workflow

Create the work branch and tracking issue, use a conventional PR title, manage
labels/draft state, and review local verification results before merging to
`dev`. Prepare release versions with `make release-prepare`, review the generated
diff and changelog, and promote `dev → next → main`.

The promotion preview retains its existing filename and owns the PR and push
checks in one workflow. Its job is named
`promotion-metadata` so its status can satisfy the production release PR even
when that PR is created with `GITHUB_TOKEN` and does not trigger new PR runs.
Workflows using `workflow_run` are loaded from the default branch; this change
must reach that branch before the renamed preview chain takes effect.

See [BRANCH-AND-RELEASE-POLICY.md](./BRANCH-AND-RELEASE-POLICY.md) for protections,
versioning, and release requirements.
