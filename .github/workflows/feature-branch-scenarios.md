---
description: |
  When a feature/** branch is created, fetch the issue it references, read the task description,
  and derive Gherkin-style user scenarios that will drive the unit tests for the feature.

on:
  create:

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

# Feature Branch Scenarios

Turn the issue behind a new `feature/**` branch into concrete user scenarios that unit tests can
target.

## Guard (stop early when not applicable)

1. Read the created ref from the event context (`github.ref` / `github.ref_type`).
2. If it is not a branch, or the branch name does not start with one of `feature/`, `bug/`,
   `doc/`, `chore/`, call `noop` and stop.

## Identify the issue

- Parse the issue number from the branch name: the digits right after the prefix
  (`feature/`, `bug/`, `doc/`, or `chore/` — e.g. `feature/123-login` → **123**). Fallback: the
  first integer anywhere in the branch name.
- If no number can be found, call `noop` ("no issue number in branch name") and stop.

## Read the task

- Fetch the linked issue's title and body. Treat the body as the requirements source.
- If the body is empty or has no testable behavior, call `noop` ("issue lacks testable
  description") and stop.

## Write the scenarios

Produce **Gherkin-style** user scenarios that capture the feature's behavior — the contract the
unit tests must satisfy. Cover the happy path plus the obvious edge/error cases the description
implies.

```gherkin
Feature: <feature name from the issue>

  Scenario: <behavior>
    Given <precondition>
    When <action>
    Then <observable outcome>
```

Rules:
- Keep scenarios **behavioral and implementation-agnostic** (no function/class names).
- One scenario per distinct behavior; prefer 3–7 focused scenarios over one giant one.
- Make each `Then` an assertion a unit test could check directly.

## Output

1. **Comment** the scenarios on the linked issue, prefixed `🧪 **Test scenarios for \`<branch>\`**`,
   so they are visible and reviewable. These scenarios are the spec that the unit-test workflow
   (`pr-auto-unit-tests`) will implement against when the PR opens.
2. If the branch already exists remotely and you can push, also commit the scenarios as
   `tests/scenarios/issue-<number>.feature` to the feature branch with message
   `test: add scenarios for #<number>`. If pushing is not possible (e.g. fork), skip the commit
   and rely on the comment.
