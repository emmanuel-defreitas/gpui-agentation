---
description: |
  When a pull request targeting the dev branch is opened, analyze the PR diff and
  add corresponding unit tests directly to the PR branch.

on:
  pull_request:
    branches: [dev]
    types: [opened]

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
  push-to-pull-request-branch:
  add-comment:
---

# PR Auto Unit Tests

When a pull request is opened against `dev` from a `feature/**` branch, decide whether the
change needs unit tests and, if so, author them and iterate until they pass.

## Guard

1. If the PR head branch does not start with one of `feature/`, `bug/`, `doc/`, `chore/`, call
   `noop` and stop.
2. If the PR head repository is a fork, do not push commits; post a PR comment explaining that
   automatic test commits are only supported for same-repository branches, then stop.

## Step 1 — Decide whether a unit test is needed

Inspect the PR diff and decide. A test **is** warranted when the change adds or alters
behavior-impacting logic: functions, modules, components, state transitions, validation paths,
parsing/serialization, or bug fixes.

A test is **not** warranted (call `noop` with a one-line reason and stop) when the diff is purely:
docs/comments, formatting, config/lockfile bumps, generated code, asset/style-only changes, or
test files only.

If `tests/scenarios/issue-<number>.feature` exists for the linked issue, treat those scenarios as
the behavioral spec your tests must satisfy.

## Step 2 — Author the tests

1. Add or update unit tests that directly cover the changed behavior, mapping each test to a
   scenario where scenarios exist.
2. Follow the existing testing conventions in the touched package/app: framework, file naming,
   directory structure, helpers, and style.
3. If the repo does **not** already have a suitable unit-test harness for the changed area, do not
   introduce a brand-new framework automatically. Post a comment explaining that automated test
   coverage should be added after the project chooses a test stack, then stop.
4. Keep changes test-only unless a tiny testability refactor is strictly required.
5. Do not modify unrelated files.

## Step 3 — Run until green

1. Run the most relevant existing test command for the changed area.
2. If tests fail, fix the **tests** (not the feature) and re-run. Repeat until they pass or you
   have made a reasonable number of attempts (cap ~5).
3. If they still cannot pass — likely because the feature itself is incomplete or broken — do not
   force them green. Post a comment describing exactly which scenario fails and why, and leave the
   failing tests in place so the developer sees the gap.

## Step 4 — Commit & report

1. Commit passing test changes to the PR branch: `test: add unit tests for PR #<number>`.
2. Push to the PR branch.
3. Post a short PR comment summarizing: tests added/updated, behavior covered, the final test
   result (pass/fail), and any gaps not covered automatically.
