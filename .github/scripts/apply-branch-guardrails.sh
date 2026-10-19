#!/usr/bin/env bash

set -euo pipefail

repo="${1:-$(gh repo view --json nameWithOwner --jq '.nameWithOwner')}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"

main_sha="$(gh api "repos/${repo}/git/ref/heads/main" --jq '.object.sha')"

ensure_branch() {
  local branch="$1"

  if gh api "repos/${repo}/git/ref/heads/${branch}" >/dev/null 2>&1; then
    echo "Branch ${branch} already exists."
    return 0
  fi

  gh api -X POST "repos/${repo}/git/refs" \
    -f ref="refs/heads/${branch}" \
    -f sha="${main_sha}" >/dev/null

  echo "Created branch ${branch} from main."
}

apply_branch_protection() {
  local branch="$1"

  gh api -X PUT "repos/${repo}/branches/${branch}/protection" \
    --input - >/dev/null <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "validate-template"
    ]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "require_last_push_approval": false,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": true,
  "lock_branch": false
}
EOF

  echo "Applied branch protection to ${branch}."
}

apply_ruleset() {
  local branch="$1"
  local file="${repo_root}/.github/rulesets/${branch}.json"
  local ruleset_id

  ruleset_id="$(gh api "repos/${repo}/rulesets" --jq ".[] | select(.name==\"Protect ${branch} branch\") | .id" 2>/dev/null || true)"

  local payload
  payload="$(mktemp)"
  cp "${file}" "${payload}"

  # GitHub requires numeric IDs for ruleset bypass actors. A generated repository
  # supplies its owner/team and GitHub App IDs as JSON, rather than inheriting IDs
  # that belong to this template repository.
  if [[ "${branch}" == "main" && -n "${MAIN_BYPASS_ACTORS_JSON:-}" ]]; then
    jq --argjson actors "${MAIN_BYPASS_ACTORS_JSON}" '.bypass_actors = (.bypass_actors // []) + $actors' \
      "${payload}" > "${payload}.next"
    mv "${payload}.next" "${payload}"
  fi

  if [[ -n "${ruleset_id}" ]]; then
    gh api -X PUT "repos/${repo}/rulesets/${ruleset_id}" --input "${payload}" >/dev/null
    echo "Updated ruleset for ${branch}."
  else
    gh api -X POST "repos/${repo}/rulesets" --input "${payload}" >/dev/null
    echo "Created ruleset for ${branch}."
  fi

  rm -f "${payload}"
}

ensure_branch dev
ensure_branch next
gh repo edit "${repo}" --default-branch dev
echo "Set dev as the default branch."

for branch in main dev next; do
  apply_branch_protection "${branch}"
done

if gh api "repos/${repo}/rulesets" >/dev/null 2>&1; then
  for branch in main dev next; do
    apply_ruleset "${branch}"
  done
else
  echo "Repository rulesets are unavailable for ${repo}; branch protection was still applied." >&2
fi
