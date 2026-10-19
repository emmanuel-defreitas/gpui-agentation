---
description: |
  When the CI workflow fails on a feature/** PR, diagnose the root cause from the failed run logs
  and open a sub-issue (linked to the feature's parent issue) describing the failure and a
  suggested fix.

on:
  workflow_run:
    workflows: [CI]
    types: [completed]
    branches: ["feature/**", "bug/**", "doc/**", "chore/**"]

engine: copilot

permissions:
  contents: read
  issues: read
  pull-requests: read
  copilot-requests: write
  actions: read

network: defaults

tools:
  github:
    lockdown: false
    min-integrity: none

safe-outputs:
  create-issue:
    title-prefix: "[ci-failure] "
    labels: [ci-failure, bug]
  link-sub-issue:
  add-comment:
    target: "*"
---

# CI Failure Diagnosis

The `CI` workflow completed for a `feature/**` PR. Diagnose only genuine failures.

## Guard

1. If the triggering run's conclusion is not `failure`, call `noop` and stop.
2. Identify the PR and head branch behind the run. If the head branch does not start with one of
   `feature/`, `bug/`, `doc/`, `chore/`, call `noop` and stop.
3. Parse the parent issue number from the branch name (digits after the prefix
   `feature/`/`bug/`/`doc/`/`chore/`, fallback first integer). Keep it for linking; if none,
   proceed without linking.

## Diagnose

- Read the failed jobs/steps and their logs for the triggering run.
- Determine which stage failed (lint / typecheck / unit tests / web build / Tauri bundle) and the
  most likely root cause. Quote the smallest decisive log excerpt (a few lines), not whole logs.
- Distinguish a **real code defect** from **infra flakiness** (network, runner, cache). For
  flakiness, recommend a re-run rather than a code change.

## Output

1. Open a **sub-issue** titled after the failing stage, e.g.
   `CI failed: typecheck on feature/123-login (PR #45)`. Body must include:
   - **What failed** — stage + the key log excerpt.
   - **Likely cause** — concise root-cause analysis with file:line references where identifiable.
   - **Suggested fix** — concrete, minimal steps to resolve it.
   - **Confidence** — high / medium / low.
2. **Link** the new issue as a sub-issue of the parent feature issue (when a parent number was
   found).
3. Post a short comment on the PR linking to the new sub-issue.

If you cannot determine anything actionable, call `noop` rather than filing a low-value issue.
