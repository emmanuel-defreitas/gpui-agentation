---
description: |
  When a lifecycle PR is merged into dev, move the linked issue to status:qa and create a follow-up
  PR with acceptance coverage or a scoped QA plan for the changed behavior.

on:
  pull_request:
    branches: [dev]
    types: [closed]

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
    title-prefix: "[qa] "
    labels: [qa, e2e]
    draft: false
    base-branch: dev
  add-labels:
    target: "*"
  remove-labels:
    target: "*"
  add-comment:
    target: "*"
---

# PR Merged → QA Coverage

A PR into `dev` closed. Only act on merged `feature/**` PRs.

## Guard

1. If the PR was **not merged** (`merged == false`), call `noop` and stop.
2. If the head branch does not start with one of `feature/`, `bug/`, `doc/`, `chore/`, call
   `noop` and stop.
3. Parse the linked issue number from the branch name (digits after the prefix
   `feature/`/`bug/`/`doc/`/`chore/`, fallback first integer).

## Status

- On the linked issue: add `status:qa`, remove `status:in-review`.

## Write the acceptance coverage

Translate the feature's behavior into acceptance coverage. Start from
`tests/scenarios/issue-<number>.feature` (when present) and any acceptance criteria in the issue.

- Prefer the repo's existing integration / acceptance / e2e framework.
- Each scenario should map to observable behavior: set up the state, perform the user or system
  action, then assert the expected outcome.
- If there is no suitable automation harness yet, do **not** introduce a large new testing stack
  automatically. Instead, create a small QA-plan artifact (for example `tests/scenarios/issue-<number>.md`)
  that captures the runnable scenarios and clearly notes the missing automation gap.

## Output

- Open a **follow-up PR into `dev`** titled `qa coverage for #<number>` containing only the
  coverage or QA-plan files. Its description should list the scenarios covered and note whether the
  coverage is automated or still manual.
- Comment on the linked issue linking to the QA PR.

If the feature genuinely has no acceptance-observable behavior (pure internal refactor), skip the
PR, still set `status:qa`, and comment explaining why no QA coverage was added.
