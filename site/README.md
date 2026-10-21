# GitHub Pages site

Astro generates a static documentation and preview site at
https://emmanuel-defreitas.github.io/gpui-agentation/.

Requires Node.js 22.22+ (or a newer version supported by Astro).

```sh
cd site
npm ci
npm run dev
npm run check
npm run preview
```

The configured project base is `/gpui-agentation/`; use that path in local
preview URLs. `dist/` is generated and excluded from source control.
`npm run check` builds the production site and checks local links, assets,
highlighted JSON, and annotation media. CI runs the same command.
Kirari highlights Rust, TOML, shell, and JSON at build time, so code snippets
ship as escaped token spans without a client highlighter. GSAP handles the three
inline SVG feature sequences. The complete example-app page and downloads have
been removed.

See [motion asset provenance](docs/motion-assets.md) for the original vector dashboard
illustration, SVG feature work, and optional GIF regeneration command.

## Deploy

The site can publish from a dedicated `gh-pages` branch with `.nojekyll`.
This avoids adding third-party Actions or changing repository protections.
Run `npm run deploy` to build and validate, copy only `dist/` into a temporary checkout,
and push `gh-pages` without force. It preserves deployment history and leaves
your working tree untouched. In GitHub Settings → Pages, use **Deploy from a branch**, `gh-pages`,
root. Never publish the repository root, private application files, or native
build artifacts. Site source changes target `dev` under the normal PR policy.

The preview is an illustrated GIF with original animated SVG feature panels,
not an executable GPUI app. A pause control stops the GIF and SVG loops. Visitors
who request reduced motion receive a still poster and static SVGs by default.
The site never calls the native loopback API or sends analytics; assets are
self-hosted. Theme preference uses optional local storage. Clipboard writes
happen only when a visitor clicks a Copy button or Open in Codex.

Fonts are self-hosted from bundled Ely assets. See `public/assets/Inter-LICENSE.txt`
and `public/assets/JetBrainsMono-LICENSE.txt` for their licenses.
The site and native implementation credit https://www.agentation.com/ for
inspiration; they do not reuse Agentation's implementation or brand assets.

## Copy this page

The page menu copies a plain Markdown export generated from the rendered main
content during every build. `index.md` retains the original setup snippets and
absolute links. Claude Code and Cursor use their documented prompt deep links.
Codex copies the context prompt and opens its web workspace, where visitors paste
to start; no undocumented Codex prompt URL is assumed. Desktop link handlers
require the corresponding app to be installed.

Link references: [Claude Code](https://code.claude.com/docs/en/deep-links),
[Cursor](https://cursor.com/docs/reference/deeplinks).
