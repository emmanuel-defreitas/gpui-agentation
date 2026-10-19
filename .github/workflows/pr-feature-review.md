---
description: |
  When a feature/** PR into dev is marked ready for review (requesting merge), run a pragmatic
  code review focused on best practices, consistency, types, and comments — efficient, not
  nitpicky — and move the linked issue to status:in-review.

on:
  pull_request:
    branches: [dev]
    types: [ready_for_review, review_requested]

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
  submit-pull-request-review:
    allowed-events: [COMMENT, REQUEST_CHANGES]
  create-pull-request-review-comment:
  add-labels:
    target: "*"
  remove-labels:
    target: "*"
---

# PR Feature Review

A `feature/**` PR into `dev` is requesting merge. Review it, then mark the linked issue in review.

## Guard

- If the PR head branch does not start with one of `feature/`, `bug/`, `doc/`, `chore/`, call
  `noop` and stop.

## Review philosophy

Be **efficient, not strict**. The goal is to catch common, real problems — not to gold-plate.
Only raise things a competent reviewer would flag in a normal review.

Focus areas (in priority order):

1. **Correctness / common bugs** — obvious logic errors, unhandled error/`null`/`undefined`
   paths, off-by-one, swallowed errors, misuse of async/await or React hooks.
2. **Types** — `any` where a real type is known, unsafe casts, missing/incorrect generics,
   public APIs without types.
3. **Consistency** — deviations from patterns already established in the touched package/app
   (naming, file layout, idioms).
4. **Comments** — comments that are wrong/stale, or genuinely missing on non-obvious logic.
   Do **not** demand comments on self-explanatory code.

Explicitly **skip**: subjective style the linters/formatter already own, bikeshedding,
speculative "you could also…" suggestions, and anything not in the diff.

## Output

- Leave **inline review comments** only where there is a specific, actionable issue, each tied to
  the exact line.
- Submit one review:
  - `request_changes` only if there is at least one **real correctness or type** problem.
  - otherwise `comment` with a brief summary (and "looks good" if clean).
- Keep total comments proportional to the diff — prefer the few that matter.

## Status

- On the linked issue (number parsed from the branch name — digits after the prefix
  `feature/`/`bug/`/`doc/`/`chore/`, fallback first integer): add `status:in-review` and remove
  `status:in-progress`.
