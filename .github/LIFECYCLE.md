# Feature lifecycle workflows

Automation for the `feature/** → dev → next → main` lifecycle, built on [GitHub Agentic Workflows (`gh-aw`)](https://github.com/githubnext/gh-aw) plus deterministic GitHub Actions workflows where an agent would add no value.

## How this is wired

Agentic workflows are authored as Markdown (`*.md`) with YAML frontmatter and compiled into hardened Actions workflows (`*.lock.yml`). The agent job stays read-only; mutations happen through safe outputs. Deterministic enforcement and validation stay in plain `*.yml`.

> [!NOTE]
> After editing any agentic workflow source file, re-run:
>
> ```sh
> gh aw compile
> ```

## Conventions assumed

| Concern | Decision | Where to change |
|---|---|---|
| Issue status | `status:in-progress`, `status:in-review`, `status:qa` | workflow safe outputs |
| Branch → issue | `<prefix>/123-slug` where prefix is one of `feature`, `bug`, `doc`, `chore` | `issue-start-branch.yml` + prompts |
| AI engine | `copilot` for agentic workflows | `engine:` frontmatter |
| Base branches | `dev` (default/integration), `next` (preview), `main` (production) | triggers + `pr-base-policy.yml` |
| Versioning | Conventional-commit SemVer candidate on `dev` merge; production tag on `next → main` | `dev-release-candidate.yml`, `release-tag.yml` |
| Release docs | Updated on the `dev → next` and `next → main` PR branches | `dev-to-next.md`, `next-to-main-wiki.md` |

Create these labels once via `.github/scripts/create-triage-labels.sh`: `status:todo`,
`status:in-progress`, `status:in-review`, `status:qa`, `release:approved`, `needs-base-fix`,
`wiki-update`.

## The stages

| # | Trigger | File | Kind | What it does |
|---|---|---|---|---|
| 1 | branch `feature/**` created | `feature-branch-context-prime.md` | agentic | Load compact context for the new branch |
| 2 | branch `feature/**` created | `feature-branch-scenarios.md` | agentic | Parse the issue and write behavior scenarios |
| 3 | PR `feature/** → dev` opened | `pr-feature-draft.yml` | plain | Convert the new PR to draft |
| 4 | PR `feature/** → dev` opened | `pr-auto-unit-tests.md` | agentic | Add targeted tests when the repo already has a suitable harness |
| 5a | PR `feature/** → dev` | `ci.yml`, `pr-semver.yml` | plain | Validate the template and require a conventional SemVer PR title |
| 5b | `ci.yml` failed | `ci-failure-diagnose.md` | agentic | Diagnose CI failure and open a sub-issue |
| 6 | PR `feature/** → dev` ready for review | `pr-feature-review.md` | agentic | Perform pragmatic review and move the issue to `status:in-review` |
| 7 | PR `feature/** → dev` merged | `pr-merged-qa-scenarios.md`, `dev-release-candidate.yml` | agentic/plain | Move the issue to `status:qa`, propose acceptance coverage, and tag the squash merge |
| 7e | push to `dev` | `validation.yml` | plain | Promotion gate for the template repo |
| 8 | `Validation` passed on `dev` | `dev-to-next.md` | agentic | Open the `dev → next` promotion PR and update release-facing docs |
| 9 | PR `* → *` opened | `pr-base-policy.yml` | plain | Enforce the `dev → next → main` base-branch policy |
| 10 | PR `<non-lifecycle> → *` opened | `pr-branch-enforcement.yml` | plain | Open a tracking issue for non-lifecycle branches |
| 9a | push to `next` | `next-preview.yml` | plain | Run configured install, test, build, image, and preview deployment steps |
| 10 | `Preview CI` passed on `next` | `next-to-main-release-pr.yml` | plain | Open the `next → main` production PR |
| 11 | PR `next → main` opened | `next-to-main-wiki.md` | agentic | Update release documentation on the PR branch |
| 12 | PR `next → main` merged | `release-tag.yml`, `production-deploy.yml` | plain | Tag/release, then run configured production deployment |

## Branch & release policy

See [`BRANCH-AND-RELEASE-POLICY.md`](./BRANCH-AND-RELEASE-POLICY.md) for the definitive policy. In short:

- lifecycle branches go to `dev`
- only `dev` promotes to `next`
- only `next` promotes to `main`
- `main`, `dev`, and `next` should stay protected and non-deletable

## Notes

- Stages 1 and 2 both fire on branch creation and self-guard when the branch is outside the lifecycle prefixes.
- Stage 4 is intentionally conservative: it should reuse an existing test harness, not invent a stack.
- Stage 8 is gated: it opens the `dev → next` PR but does not silently merge it.
- `validation.yml` is the promotion gate on `dev`, so the workflow name must stay `Validation` unless `dev-to-next.md` is updated too.
