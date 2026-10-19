# Claude guidance for `repo-template`

This repository is a **bootstrap template**, not an application. Optimize for good defaults that downstream repositories can safely inherit.

## Quick summary

- **Repository type:** GitHub template repository
- **Primary stack:** Markdown policy docs + GitHub Actions + GitHub Agentic Workflows
- **Entrypoints:** `.github/workflows/`, `.github/BRANCH-AND-RELEASE-POLICY.md`, `.github/LIFECYCLE.md`
- **Owners:** `@exegia`

## Commands

Run commands from the repository root.

| Task | Command |
| --- | --- |
| Compile agentic workflows | `gh aw compile` |
| Compile + lint generated workflows | `gh aw compile --actionlint` |
| Inspect workflow status | `gh aw status` |
| Install the secret-scanning hook | `pre-commit install` |
| Scan working tree for secrets | `gitleaks dir .` |

There is intentionally no app-specific build or test command in this template. Generated repositories must replace these defaults with their real stack commands.

## Template architecture

The important flows in this repository are operational:

1. `.github/workflows/*.md` are the agentic workflow sources; `.lock.yml` files are generated artifacts.
2. `pr-base-policy.yml`, `issue-start-branch.yml`, and `release-tag.yml` enforce the branch lifecycle around `main`, `dev`, and `next`.
3. `claude.yml`, `claude-code-review.yml`, and `copilot-setup-steps.yml` wire hosted AI integrations into the template.
4. `secret-scan.yml` blocks credentials from entering history; `.pre-commit-config.yaml` is its local counterpart.
5. `README.md`, `AGENTS.md`, and `.github/copilot-instructions.md` are starter guidance that downstream repos are expected to customize immediately.

## Working principles

1. **Keep the template generic.** Avoid baking in a product stack, package manager, or framework unless the file is explicitly meant to be replaced downstream.
2. **Keep workflow sources authoritative.** Edit `.md` files for agentic workflows, then recompile instead of hand-editing lock files.
3. **Surface required setup clearly.** Secrets, GitHub features, and branch protections should be documented, not implied.
4. **Preserve safe defaults.** Protected branches, reviewed promotions, and validation-before-release should remain intact unless the template policy is intentionally changed.
5. **Never commit secrets.** A `gitleaks` pre-commit hook and the `Secret Scan` workflow both check
   for credentials. Do not bypass the hook with `--no-verify`; if it fires, remove the secret and
   rotate it. Keep real values in `.env` (gitignored) and commit `.env.example` instead.
6. **Update docs with workflow changes.** If the lifecycle or automation changes, update this file, `README.md`, and `AGENTS.md` in the same change.

## Branching and release workflow

The default template lifecycle is:

- lifecycle branches → `dev` (the default branch)
- `dev` → `next`
- `next` → `main`

Use conventional PR titles so the `dev` candidate and `main` release tags can infer semantic version
bumps. Treat `main` as read-only except for explicitly configured owner and GitHub Actions bypasses.

If you edit agentic workflow source files in `.github/workflows/*.md`, recompile before finishing:

```sh
gh aw compile
```

## Implementation expectations

- Keep changes scoped to the template itself.
- Prefer reusable automation over stack-specific assumptions.
- Add or update validation when workflow behavior changes.
- Do not commit local Claude/Copilot working directories or generated workflow logs.

## Template notes

- **Generated files:** `.github/workflows/*.lock.yml`
- **Local-only artifacts:** `.claude/`, `.agents/`, `.github/aw/logs/`, `.env`
- **Secret scanning:** `.pre-commit-config.yaml` (local) and `.github/workflows/secret-scan.yml` (CI), configured by `.gitleaks.toml`
- **Required release branches:** `main`, `dev`, `next`
- **Preferred validation strategy:** compile and lint workflow sources before merge
