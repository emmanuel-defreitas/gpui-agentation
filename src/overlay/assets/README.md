# Client logos

These marks identify MCP clients in Settings. They remain trademarks of their
respective owners and do not imply endorsement. The crate's original MIT
license and attribution remain unchanged.

- `zed-black.svg`, `zed-white.svg`: unmodified square logomarks extracted from
  https://zed.dev/brand on 2026-10-19. Use the official pure-black and pure-white
  variants for light and dark themes, preserving their proportions.
- `claude.svg`: the square `logos:claude-icon` mark from the SVG Logos collection,
  https://api.iconify.design/logos/claude-icon.svg, corresponding to the Claude
  logo requested at https://www.shadcn.io/icon/logos-claude. The collection is
  listed as CC0 on that page. The original brand color is preserved.
- `codex.svg`: supplied by the user as `codex-openai-color.svg`. Its path and
  currentColor source are preserved. The renderer resolves currentColor to
  black or white for the active appearance.

SVG data is embedded in the library, so consumers do not need to register
another asset source or fetch logos at runtime.
