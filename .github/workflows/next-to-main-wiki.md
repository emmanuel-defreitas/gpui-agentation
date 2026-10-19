---
description: |
  When the next branch PR is opened into main, update release-facing documentation on the PR branch
  so the release is self-describing and future agents have durable context.

on:
  pull_request:
    types: [opened]
    branches: [main]

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
  add-comment:
    target: "*"
  push-to-pull-request-branch:
---

# Next → Main Release Docs

A `next` PR into `main` was just opened. This is the moment to make the release self-describing.
Update release-facing docs on the PR branch so they ship with the release.

## Guard (stop early when not applicable)

1. Read the PR's head and base from the event context. If the head branch is **not** `next`, call
  `noop` and stop (non-release PRs into `main` are handled by the base-branch
  policy workflow).
2. If the PR is a draft with no describable changes yet, call `noop` ("release PR is empty/too
  early to document") and stop.

## Read the release

- Read the PR description and the diff (or `gh pr diff`). Identify the user-facing features, fixes,
  and chore-level changes included in this release.
- Read `CHANGELOG.md` if it exists — it is the authoritative list of what's in the release.
- Read `README.md`, `CLAUDE.md`, and any release notes or docs directory that already exists.

## Update release-facing docs

For each meaningful capability in the release (skip pure refactors with no behavior change):

1. Prefer updating existing docs in place rather than creating duplicates.
2. If there is a release notes or docs area in the repo, add or update the smallest useful page.
3. Keep all edits documentation-only; do not touch source code in this PR.

If the repo has no dedicated docs area yet, update only `README.md`, `CHANGELOG.md`, and
`CLAUDE.md` where needed.

## Output

1. Push all documentation changes to the **release PR branch** via the `push-to-pull-request-branch`
   safe output with a clear commit message like `docs(release): update release docs for <version>`.
2. Post **one** comment on the PR summarizing what documentation changed.

If there is nothing meaningful to document, call `noop`.
