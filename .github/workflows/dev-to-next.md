---
description: |
  When validation passes on dev, open a PR from dev into the next branch that
  updates release documentation and version metadata. Gated: it does not auto-merge unless the PR
  is approved.

on:
  workflow_run:
    workflows: [Validation]
    types: [completed]
    branches: [dev]

engine: copilot

permissions:
  contents: read
  issues: read
  pull-requests: read
  copilot-requests: write

network: defaults

tools:
  github:
    lockdown: false
    min-integrity: none

safe-outputs:
  create-pull-request:
    title-prefix: "[release] "
    labels: [release]
    draft: false
    base-branch: next
    # Blast radius: auto-merge is intentionally OFF. A human approves, then the repo's
    # squash-merge setting takes over. To fully automate squash-merge on green, add:
    #   auto-merge: true
---

# Dev → Next

The `Validation` workflow finished on `dev`. Promote only when the integration branch is green.

## Guard

1. If the triggering run's conclusion is not `success`, call `noop` and stop.
2. If there are no changes on `dev` ahead of the release branch, call `noop` and stop.

## Prepare the release PR

Open a PR from `dev` into `next`. On the PR branch, update the following so the promotion is
self-describing:

1. **CHANGELOG** — add/update an entry summarizing the features and fixes merged into `dev` since
   the last release (group by feature; reference the issues/PRs). Create `CHANGELOG.md` if absent.
   Use a top-level `## [Unreleased]` heading for the in-progress entry. This workflow must rename
   that heading to `## [vX.Y.Z] - YYYY-MM-DD` in the `dev → next` PR before it is merged. Write
   commit bullets using conventional-commit prefixes (`feat:`, `fix:`, `docs:`, `chore:`) since
   the tag workflow derives the version bump from those same prefixes.
2. **README.md** — update only what the new features actually changed (setup steps, feature list,
   screenshots references, version). Do not rewrite unrelated sections.
3. **Version metadata** — determine the SemVer bump from conventional commits since the latest
   stable `vX.Y.Z` tag (breaking change: major; `feat`/`feature`: minor; otherwise patch). Update
   every existing version manifest that the repository uses, such as `package.json`, `pyproject.toml`,
   or a release config. Never create a stack-specific manifest just for this workflow.
4. **CLAUDE.md files** — update the root `CLAUDE.md` and any package/app `CLAUDE.md` so agent
   guidance reflects new modules, commands, or conventions introduced this cycle. Skip files that
   need no change.

Keep edits scoped to release documentation and existing version metadata; do not touch source in this PR.

## Output

- Open the release PR with a description that lists what's included, links the closed issues, and
  states the validation status (validation passed on `dev`).
- The PR is created **ready for review but not auto-merged**. A maintainer approves; squash-merge
  to `next` is then performed via the repo's merge settings (or auto-merge if you enabled the
  toggle above).

If there is nothing to release or document, call `noop`.
