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
The source in `example/` is checked against the published 0.3.0 crate. The
prepare script copies only its manifest and Rust source to the public download
folder; it never copies Cargo's target directory or local feedback stores.

```sh
cargo check --manifest-path example/Cargo.toml --all-targets
cargo check --manifest-path example/Cargo.toml --all-targets --release
```

## Deploy

The site can publish from a dedicated `gh-pages` branch with `.nojekyll`.
This avoids adding third-party Actions or changing repository protections.
Run `npm run deploy` to build, copy only `dist/` into a temporary checkout,
and push `gh-pages` without force. It preserves deployment history and leaves
your working tree untouched. In GitHub Settings → Pages, use **Deploy from a branch**, `gh-pages`,
root. Never publish the repository root, private application files, or native
build artifacts. Site source changes target `dev` under the normal PR policy.

The interactive preview is a browser simulation with fictional sample content,
not a GPUI executable. All annotations remain in tab memory. It never calls the
native loopback API, sends analytics, or loads third-party assets. Theme
preference uses local storage when available. Clipboard writes happen only
when a visitor clicks a Copy button.

Fonts are self-hosted from bundled Ely assets. See `public/assets/Inter-LICENSE.txt`
and `public/assets/JetBrainsMono-LICENSE.txt` for their licenses.
The site and native implementation credit https://www.agentation.com/ for
inspiration; they do not reuse Agentation's implementation or brand assets.
