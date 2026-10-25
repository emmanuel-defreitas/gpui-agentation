# Changelog

## 6.0.0 (unreleased)

- Add an explicit annotation mode switch, visible tool shortcuts, save, undo/redo,
  and undoable deletion; ordinary text entry keeps its editing shortcuts.
- Add plain textboxes with inline editing and no automatic comment popup.
- Color comment editors from their annotation and use a comment icon for Comments.
- Reposition numbered pins without changing their drawings; pin moves are undoable.
- Persist Banner, Floating, None, and floating coordinates across routes, windows,
  and restarts using the same app store.
- Use a transparent banner, theme-aware buttons, a horizontal palette on wide
  windows, and an MCP toggle prefixed by the official MCP mark.
- Expose a host-composable Debug menu and route its commands through Agentation.
- Keep the GPUI/Ely pins, loopback API guards, and debug-only crate boundary.

## 0.5.1

- Exclude README demonstration GIFs from the Cargo archive to keep it below
  crates.io's 10 MiB upload limit. The media remains in the repository and site.
- Check the actual package archive size before compiling publication candidates,
  with regression coverage for missing archives and the size boundary.
- Preserve all native sources, bundled Ely assets, licenses, examples, and skills.

## 0.5.0

- Add an MCP exploration skill covering tool discovery, feedback context,
  native annotation features, and verified note resolution.
- Document both agent skills, their installation, and the roadmap in the
  README and site; add native inspection and annotation demonstration GIFs.
- Add one release preparation command that synchronizes Cargo, the lockfile,
  Registry metadata, current documentation, skills, and site references.
- Validate promotion and preview versions against conventional commits and
  check the exact reviewed commit before creating a stable release tag.
- Add release regression tests, validate Settings before writing release files,
  and read release metadata as UTF-8 across platforms, including Windows.
- Restore the MCP config button's expanded/collapsed accessibility state by
  using its disclosure wrapper in the Settings UI.
- Align contribution examples with lifecycle branches and conventional commits;
  keep the advisory dev version preview separate from production versioning.
- Preserve the native public API, loopback guards, GPUI pin, bundled Ely snapshot,
  and the empty release-build annotation API.

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
