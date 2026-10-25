# Branch & Release Policy

This is the source of truth for the template's branch model, PR targeting rules, protection expectations, and release promotion flow. Read [`LIFECYCLE.md`](./LIFECYCLE.md) alongside it.

## Branch model

| Branch / prefix | Purpose | Created by | Lifecycle? |
|---|---|---|---|
| `feature/<n>-<slug>` | New capability or enhancement | `issue-start-branch.yml` | yes |
| `bug/<n>-<slug>` | Defect fix | `issue-start-branch.yml` | yes |
| `doc/<n>-<slug>` | Documentation-only change | `issue-start-branch.yml` | yes |
| `chore/<n>-<slug>` | CI, build, refactor, tooling | `issue-start-branch.yml` | yes |
| `ai/v6-01-annotation-model`, `ai/v6-02-annotation-tools`, `release/v6.0.0` | Approved v6 release stack | maintainer | exact-name stack exception |
| `dev` | Integration and default branch | protected long-lived branch | yes |
| `next` | Staging / pre-release branch | protected long-lived branch | yes |
| `main` | Production branch | protected long-lived branch | yes |
| `hotfix/*`, `dependabot/*`, `copilot/*`, `claude/*` | Bot and exception flows | respective bots / maintainers | exempt from branch-tag enforcement |

The lifecycle prefix is chosen from the issue's `type:*` label when `status:in-progress` is applied, so branch prefix and issue type stay aligned.

## PR targeting rules (enforced by `pr-base-policy.yml`)

1. **Lifecycle work targets `dev`.** PRs opened against unsupported base branches are reassigned to `dev`.
2. **Only `dev` promotes into `next`.** Any other head branch targeting `next` is redirected back to `dev`.
3. **Only `next` releases into `main`.** PRs into `main` from any head other than `next` are closed.

The approved same-repository v6 stack preserves these dependent PR bases:

```text
ai/v6-01-annotation-model → dev
ai/v6-02-annotation-tools → ai/v6-01-annotation-model
release/v6.0.0 → ai/v6-02-annotation-tools
```

Only those two dependent edges are exceptions to the normal `dev` target.
Forks with matching branch names receive no exception. Base policy rechecks on
opening, edits, reopening and new commits. These three exact branch names are
exempt from automatic tracking-issue creation and require conventional PR titles,
with stack numbering after the prefix, such as `feat: add textboxes [1/3]`.
No general `ai/*` or `release/*` exemption is granted.

Review and merge bottom-up. After each parent merges, retarget its dependent PR
to `dev` and rebase it onto the integrated parent before merging. With squash
merges, use `git rebase --onto origin/dev <old-parent-tip> <child>` to avoid
replaying the parent's commits. Never merge a dependent PR into its feature
parent. Production promotion remains `dev → next → main`.

## Branch protection and ruleset expectations

`main`, `dev`, and `next` should all be non-deletable and non-force-pushable. `main` is read-only:

1. changes must arrive through an approved PR with required checks;
2. direct updates are allowed only for explicitly configured owner and GitHub Actions bypass actors;
3. the template does not hard-code bypass IDs because they belong to the generated repository.

Run `.github/scripts/apply-branch-guardrails.sh` after creating a repository from the template. It
creates `dev` and `next`, makes `dev` the default branch, and applies the rulesets. Supply
`MAIN_BYPASS_ACTORS_JSON` with the generated repository's owner/team and GitHub App actor IDs when
the `main` ruleset is applied.

## Branch-tag enforcement (`pr-branch-enforcement.yml`)

When a PR is opened from a branch that is not one of the lifecycle prefixes and not an exempt automation branch, the workflow:

1. infers work type from the PR title's conventional-commit prefix
2. opens a tracking issue under the `⚙️ EPIC: Workflow` milestone
3. comments on the PR with the tracking issue link

Type inference uses the same conventional-commit vocabulary as `release-tag.yml`, so issue labels and semver signals stay aligned.

## Dynamic issue title prefixes

| Prefix | Label | When |
|---|---|---|
| `[BUG]` | `type:bug` | fix work inferred from `fix:` / `bug:` titles |
| `[FEATURE]` | `type:feature` | feature work inferred from `feat:` titles |
| `[DOC]` | `type:docs` | docs work inferred from `docs:` titles |
| `[CHORE]` | `type:chore` | default for everything else |

## Release flow: `dev → next → main`

1. **PR → `dev`** — CI and SemVer-title validation run. Lifecycle PRs
   are squash-merged, then `dev-release-candidate.yml` tags the squash merge.
2. **Prepare and promote** — run `make release-prepare` in a branch targeting `dev`.
   It synchronizes Cargo.toml, Cargo.lock, server.json, current docs, skills, and
   site references. Settings, CLI, and MCP versions derive from Cargo. Review the
   generated diff and update `CHANGELOG.md`, merge through review, then open
   `dev → next` after validation passes. No command commits or pushes changes.
3. **Preview** — merging to `next` runs `next-preview.yml`, which runs configured install, test,
   build, container, and preview deployment commands.
4. **Release PR** — a successful preview opens a `next → main` PR. Promotion
   checks reject stale metadata or a version that disagrees with commit history.
5. **Release** — merging `next → main` creates the SemVer tag and GitHub Release; the published
   release triggers the configured production deployment.

### Semantic versioning (`release-tag.yml`)

| Signal | Bump |
|---|---|
| `BREAKING CHANGE` in any body, or a `<type>!:` subject | **major** |
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
  "to": "6.0.0",
  "reason": "Use the explicitly requested 6.0.0 release numbering"
}
```

Commit this declaration on the release preparation branch, alongside the
`make release-prepare VERSION=6.0.0` diff and changelog. The shared version
script reads it from the requested Git revision, so uncommitted edits cannot
authorize a release. `from` must match the latest reachable stable tag; `to`
must be a stable increase and satisfy at least the conventional-commit bump.
Metadata consistency, Settings derivation, package checks and the reviewed
merge-commit tag remain mandatory. Malformed declarations and baseline drift
fail validation. Once `to` or a later stable version is tagged, the declaration
is consumed and ordinary version calculation resumes.

This is an input to the existing `release-tag.yml` workflow, not another release
trigger. It does not create a tag, release, merge, or deployment by itself.

## Cheat sheet

```text
issue (type:feature) ──status:in-progress──▶ feature/123-slug
                                              │
                                              ▼
                                  PR feature/123 → dev (squash + candidate tag)
                                              │
                                              ▼  Validation green on dev
                                  PR dev → next (docs + version metadata)
                                              │
                                              ▼
                                  preview CI → PR next → main
                                              │  merge
                                              ▼
                                  tag vX.Y.Z + Release + production deploy
```

## Related files

- `.github/LIFECYCLE.md`
- `.github/workflows/pr-base-policy.yml`
- `.github/workflows/pr-branch-enforcement.yml`
- `.github/workflows/release-tag.yml`
- `.github/workflows/issue-start-branch.yml`
- `.github/scripts/create-triage-labels.sh`
