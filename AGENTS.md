# Agent operating guide for `repo-template`

This repository is a **template for other repositories**, so changes should optimize for reusable defaults, not one-off product assumptions.

## Mission

Keep the template generic, secure, and immediately usable: branch model, workflow automation, and agent guidance should work before downstream application code exists.

## Non-negotiable rules

1. Read the relevant workflow and policy files before editing automation.
2. Prefer reusable defaults over stack-specific assumptions.
3. Update `README.md`, `CLAUDE.md`, and `.github/copilot-instructions.md` when workflow behavior changes.
4. Recompile edited `.github/workflows/*.md` files with `gh aw compile`.
5. Do not commit local-only agent state such as `.claude/`, `.agents/`, or workflow log artifacts.
6. Never commit a credential, token, or API key — not in code, tests, fixtures, or examples. Use
   placeholder values and keep real values in `.env`, which is gitignored.
7. Do not bypass the secret-scanning hook with `git commit --no-verify`. If the hook fires, the
   correct response is to remove the secret and rotate it, never to skip the check. CI runs the
   same scan and will fail the pull request regardless.

## Branch and PR policy

- `dev` is the default branch; lifecycle branches (`feature/*`, `bug/*`, `doc/*`, `chore/*`) target it.
- Promotion happens from `dev` to `next`.
- Releases land from `next` to `main`.
- `main` is read-only except for the generated repository's explicitly configured owner and Actions bypasses.

If this file and the automation diverge, treat `.github/LIFECYCLE.md` and `.github/BRANCH-AND-RELEASE-POLICY.md` as the source of truth.

## Expected workflow

1. Make the smallest complete change that improves the template itself.
2. Preserve template-safe validation even when no app stack has been chosen yet.
3. Run the existing repo validation commands for touched workflow surfaces.
4. Leave downstream teams with clear starter docs rather than hidden assumptions.

## Template details to customize in generated repos

- **Core apps/services:** replace placeholders with the real repo surface area
- **Validation commands:** replace `gh aw`-only defaults with the actual build/test commands
- **Secrets/integrations:** document only the GitHub Actions secrets the generated repo truly needs
- **Release process:** keep `main/dev/next` or update the policy docs and workflows together

## Agentic workflow note

If you edit any agentic workflow source file, regenerate the lock files before finishing:

```sh
gh aw compile
```

Do not hand-edit `.lock.yml` files unless the repository explicitly documents that as acceptable.
