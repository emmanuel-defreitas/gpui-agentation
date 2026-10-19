#!/usr/bin/env bash
# create-triage-labels.sh
#
# Creates the labels used by the issue-triage workflows:
#   - issue-triage.md       (assigns type:* and priority:*, seeds status:todo, flags duplicate)
#   - issue-start-branch.yml (fires when status:in-progress is added; removes status:todo)
#   - issue-status-sync.yml  (ensures status:in-progress on draft PR open)
#
# Idempotent: `--force` creates the label or updates it if it already exists, so this is safe to
# re-run and will also correct drifted colors/descriptions. Run from the repo root:
#
#   ./.github/scripts/create-triage-labels.sh
#
set -euo pipefail

mklabel() { gh label create "$1" --color "$2" --description "$3" --force; }

# ── Type (issue-triage assigns exactly one) ──────────────────────────────
mklabel "type:bug"          "d73a4a" "Something is broken or behaves incorrectly"
mklabel "type:feature"      "0e8a16" "New capability or enhancement request"
mklabel "type:docs"         "0075ca" "Documentation only"
mklabel "type:chore"        "cfd3d7" "Build, CI, deps, refactor, maintenance (no user-facing change)"
mklabel "type:question"     "d876e3" "Question or support request, not actionable work"

# ── Priority (issue-triage assigns exactly one) ──────────────────────────
mklabel "priority:critical" "b60205" "Data loss, security, build/release broken, or app unusable"
mklabel "priority:high"     "d93f0b" "Major feature broken or blocked, no workaround"
mklabel "priority:medium"   "fbca04" "Important but has a workaround, or a valuable enhancement (default)"
mklabel "priority:low"      "0e8a16" "Minor, cosmetic, or nice-to-have"

# ── Status (todo seeds the workflow; in-progress triggers branch creation) ─
mklabel "status:todo"        "ededed" "Triaged, not yet started"
mklabel "status:in-progress" "1d76db" "Work has started; feature branch created"

# ── Duplicate (flagged by triage, confirmed/closed by a human) ───────────
mklabel "duplicate"         "cccccc" "Likely duplicate of another issue"

# ── Policy / release-doc flags (pr-base-policy, pr-branch-enforcement, next-to-main-wiki) ────
mklabel "needs-base-fix"    "b60205" "PR targets the wrong base branch (policy reassigns/closes)"
mklabel "wiki-update"       "0075ca" "Release docs pending/created by the next-to-main-wiki workflow"

echo "✓ Triage labels created/updated."
