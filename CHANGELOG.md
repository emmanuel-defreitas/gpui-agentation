# Changelog

## 0.4.2

- Refine Settings with a larger icon, tighter header spacing, clearer hot reload
  labels, and a crates.io link in About. The displayed version follows the package.
- Make MCP configuration collapsible and initially closed, with a left-aligned
  disclosure and a trailing Copy button that also works while collapsed.
- Expose the MCP config disclosure’s expanded state to assistive technology.
- Move MCP status before Comments in the development banner and hide annotation
  count badges when there are no notes.
- Update setup instructions to match the new Settings labels. Keep the public
  API, loopback protections, GPUI pin, and bundled Ely snapshot unchanged.

## 0.4.1

- Rewrite the README around what the agent receives, the pre-release status, a
  "Why this" section and a release-gated integration example.
- Add crates.io keywords, categories and homepage.
- Add `make version-check`, which fails when a documented version disagrees
  with `Cargo.toml`.
- Remove the GitHub agentic workflows and the Claude Code Review workflow;
  fix committer identities for release tags and the branch-tag enforcement job.
- No API or behaviour changes: nothing under `src/` changed since 0.4.0.

## 0.4.0

- Add a minimal Agentation integration API with internal assets, initialization,
  focus handling and screen registration, plus a consumer app and setup skill.
- Add banner mode, settings and comments sheets, amber notes and pins,
  auto-growing note editors, click-away saving and inspector hover details.
- Add an opt-in Settings Hot reload switch backed by a rebuild/restart supervisor.
- Simplify the README and reuse the website logo.
- Add website Markdown copying and Claude Code, Codex and Cursor handoff links.
- Keep GPUI 0.3.8 and the bundled Ely snapshot unchanged. MCP-only installs
  continue to skip native UI compilation.
