# GitHub Pages site

Astro generates a static documentation and preview site at
https://emmanuel-defreitas.github.io/gpui-agentation/.

Requires Node.js 22.12+ (or a newer version supported by Astro).

```sh
cd site
npm ci
npm run dev
npm run build
npm run preview
```

The configured project base is `/gpui-agentation/`; use that path in local
preview URLs. `dist/` is generated and excluded from source control.
Kirari highlights Rust, TOML, shell, and JSON at build time, so code snippets
ship as escaped token spans without a client highlighter. GSAP handles the three
inline SVG feature sequences. The complete example-app page and downloads have
been removed.

See [motion asset provenance](docs/motion-assets.md) for the original vector dashboard
illustration, SVG feature work, and optional GIF regeneration command.

## Deploy

The site can publish from a dedicated `gh-pages` branch with `.nojekyll`.
This avoids adding third-party Actions or changing repository protections.
Run `npm run deploy` to build, copy only `dist/` into a temporary checkout,
and push `gh-pages` without force. It preserves deployment history and leaves
your working tree untouched. In GitHub Settings → Pages, use **Deploy from a branch**, `gh-pages`,
root. Never publish the repository root, private application files, or native
build artifacts. Site source changes target `dev` under the normal PR policy.

The preview is an illustrated GIF with original animated SVG feature panels,
not an executable GPUI app. A pause control stops the GIF and SVG loops. Visitors
who request reduced motion receive a still poster and static SVGs by default.
The site never calls the native loopback API or sends analytics; assets are
self-hosted. Theme preference uses optional local storage. Clipboard writes
happen only when a visitor clicks a Copy button.

Fonts are self-hosted from bundled Ely assets. See `public/assets/Inter-LICENSE.txt`
and `public/assets/JetBrainsMono-LICENSE.txt` for their licenses.
The site and native implementation credit https://www.agentation.com/ for
inspiration; they do not reuse Agentation's implementation or brand assets.
