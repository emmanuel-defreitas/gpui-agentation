---
description: |
  When an issue is opened, reopened, or edited, triage it: apply a type label and a priority
  label from the fixed taxonomy, set the initial status to status:todo if it has none, and flag
  likely duplicates by commenting + labelling (never auto-closing). Reasoning step, so agentic.

on:
  issues:
    types: [opened, reopened, edited]

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
  add-labels:
    target: "*"
  remove-labels:
    target: "*"
  add-comment:
    target: "*"
---

# Issue Triage

An issue was just opened, reopened, or edited. Triage it accurately and cheaply, then stop.

## Guard (stop early when not applicable)

1. This runs on the issue in the event context. If the event is for a **pull request**
   (pull requests are issues in the API), call `noop` and stop.
2. If the issue is already closed, call `noop` and stop.

## Cost discipline

Be token-frugal: read the issue (title + body + existing labels) and, for duplicate detection,
search at most ~20 recent/related issues. Do not crawl the whole repo or read source files.

## 1. Type label (exactly one)

Read the title and body and choose the single best-fitting **type** label. Add it; if a different
`type:` label is already present and clearly wrong, remove it. Taxonomy:

- `type:bug` — something is broken or behaves incorrectly.
- `type:feature` — a new capability or enhancement request.
- `type:docs` — documentation only.
- `type:chore` — build, CI, deps, refactor, or maintenance with no user-facing behavior change.
- `type:question` — a question or support request, not actionable work.

If the issue genuinely fits none, prefer `type:chore` and note the ambiguity in your comment.

## 2. Priority label (exactly one)

Assign one **priority** based on impact and urgency expressed in the issue. Add it; remove any
existing, contradicting `priority:` label. Taxonomy:

- `priority:critical` — data loss, security, build/release broken, or app unusable.
- `priority:high` — major feature broken or blocked, no workaround.
- `priority:medium` — important but has a workaround, or a valuable enhancement (default).
- `priority:low` — minor, cosmetic, or nice-to-have.

When in doubt, choose `priority:medium`.

## 3. Initial status

- If the issue has **no** `status:` label at all, add `status:todo` (this is what the
  branch-creation workflow watches for the `todo → in-progress` transition).
- If a `status:` label is already present, leave it untouched — later lifecycle stages own it.

## 4. Duplicate detection (flag, never close)

- Search existing issues for ones describing the same problem/request (match on title keywords and
  the core symptom — not superficial word overlap).
- If you find a **likely** duplicate that is still open or recently closed:
  - Add the `duplicate` label.
  - Post one comment: `🔁 **Possible duplicate of #<N>** — <one line on why>. A maintainer should
    confirm and close if so.`
  - **Do not close the issue.** Closing is a human decision.
- If nothing matches, do not comment about duplicates.

## Output

Apply the labels via the label safe-outputs and post **at most one** comment (the duplicate note,
if any). If the issue was already well-labelled and is not a duplicate, just ensure the taxonomy is
correct and call `noop` for the comment. Keep everything proportional and quiet.
