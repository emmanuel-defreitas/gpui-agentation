# Feature lifecycle workflows

Automation for the `feature/** → dev → next → main` lifecycle, built on plain GitHub Actions workflows. The GitHub Agentic Workflows (`gh-aw`) that used to open promotion PRs, write tests and review PRs were removed; the steps they covered are manual and are marked below.

## Conventions assumed

| Concern | Decision | Where to change |
|---|---|---|
| Issue status | `status:todo` and `status:in-progress` are automated; `status:in-review` and `status:qa` are set by hand | `issue-start-branch.yml`, `issue-status-sync.yml` |
| Branch → issue | `<prefix>/123-slug` where prefix is one of `feature`, `bug`, `doc`, `chore` | `issue-start-branch.yml` |
| Base branches | `dev` (default/integration), `next` (preview), `main` (production) | triggers + `pr-base-policy.yml` |
| Versioning | Conventional-commit SemVer candidate on `dev` merge; production tag on `next → main` | `dev-release-candidate.yml`, `release-tag.yml` |
| Release docs | Edited by hand on the `dev → next` and `next → main` PR branches | `CHANGELOG.md`, `README.md` |

Create these labels once via `.github/scripts/create-triage-labels.sh`: `status:todo`,
`status:in-progress`, `status:in-review`, `status:qa`, `release:approved`, `needs-base-fix`,
`wiki-update`.

## The stages

| # | Trigger | File | What it does |
|---|---|---|---|
| 1 | issue gets `status:in-progress` | `issue-start-branch.yml` | Create the `<prefix>/<n>-slug` branch linked to the issue |
| 2 | PR `feature/** → dev` opened | `pr-feature-draft.yml` | Convert the new PR to draft |
| 3 | PR `feature/** → dev` opened as draft | `issue-status-sync.yml` | Keep the linked issue at `status:in-progress` |
| 4 | PR `feature/** → dev` | `ci.yml`, `pr-semver.yml` | Validate the template and require a conventional SemVer PR title |
| 5 | PR `feature/** → dev` merged | `dev-release-candidate.yml` | Tag the squash merge as a release candidate |
| 6 | push to `dev` | `validation.yml` | Promotion gate |
| 7 | PR `* → *` opened | `pr-base-policy.yml` | Enforce the `dev → next → main` base-branch policy |
| 8 | PR `<non-lifecycle> → *` opened | `pr-branch-enforcement.yml` | Open a tracking issue for non-lifecycle branches |
| 9 | push to `next` | `next-preview.yml` | Run configured install, test, build, image, and preview deployment steps |
| 10 | `Preview CI` passed on `next` | `next-to-main-release-pr.yml` | Open the `next → main` production PR |
| 11 | PR `next → main` merged | `release-tag.yml`, `production-deploy.yml` | Tag/release, then run configured production deployment |

Manual steps (formerly agentic):

- After `Validation` passes on `dev`, open the `dev → next` promotion PR yourself and update `CHANGELOG.md` and any version manifests on it. Do not merge without review.
- Update release documentation on the `next → main` PR branch.
- Set `status:in-review` when a PR is ready for review and `status:qa` after merge, if you use those labels.

## Branch & release policy

See [`BRANCH-AND-RELEASE-POLICY.md`](./BRANCH-AND-RELEASE-POLICY.md) for the definitive policy. In short:

- lifecycle branches go to `dev`
- only `dev` promotes to `next`
- only `next` promotes to `main`
- `main`, `dev`, and `next` should stay protected and non-deletable

## Notes

- `validation.yml` is a plain check on `dev`; nothing triggers from it any more.
- Workflows triggered by `issues` or `workflow_run` events run from the repository's default branch.
