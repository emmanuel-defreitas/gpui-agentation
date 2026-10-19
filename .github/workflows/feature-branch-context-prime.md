---
description: |
  When a feature/** branch is created, prime a compact, cost-friendly context note for the
  work ahead: consult repository memory and recent activity, summarize the linked issue, and
  store a short context comment so later lifecycle steps (and humans) start with shared context.

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
---

# Feature Branch Context Prime

A Git ref was just created. Your job is to load context cheaply and leave a compact note on the
related issue so the rest of the lifecycle starts informed.

## Guard (stop early when not applicable)

1. Read the created ref from the event context (`github.ref` / `github.ref_type`).
2. If the ref is **not a branch**, or the branch name does **not** start with one of `feature/`,
   `bug/`, `doc/`, `chore/`, call `noop` with a one-line reason and stop. Do no further work.

## Identify the issue

- Parse the issue number from the branch name: the digits immediately after the prefix
  (`feature/`, `bug/`, `doc/`, or `chore/` — e.g. `feature/123-login` → issue **123**).
- If no leading number is present, scan the branch name for the first integer and use that.
- If still none, call `noop` ("no issue number in branch name") and stop.

## Cost discipline (important)

This step must be **token-frugal**. Do the minimum:

- Read **only** the linked issue (title + body) and its labels.
- Read at most the 5 most recent issues/PRs for nearby context — do not crawl the whole repo.
- Do not read source files unless the issue explicitly references specific paths.
- Prefer one concise pass; do not iterate.

## Produce the context note

Post a single comment on the linked issue containing a tight briefing (keep it under ~200 words):

- **Task summary** — one or two sentences restating the issue's goal.
- **Acceptance signals** — the observable outcomes that mean "done".
- **Relevant areas** — packages/apps likely touched (infer from the issue; do not over-claim).
- **Open questions** — anything ambiguous a developer should clarify first.

Prefix the comment with `🧭 **Context primed for \`<branch-name>\`**` so it is easy to find.

If there is genuinely nothing useful to say, call `noop` instead of posting an empty note.
