# Motion assets

The main banner uses original vector artwork following the Agentation dashboard
reference explicitly selected by the user. No upstream source, animation files,
logos, or brand assets are copied. The existing Agentation inspiration credit
remains on the site. This is an illustration, not a native runtime recording.

## Main dashboard sequence

- Artwork and deterministic frame state: `src/illustrations/banner.mjs`
- Exporter: `scripts/render-banner.mjs`
- Published assets: `public/assets/motion/dashboard-demo.gif` and
  `dashboard-demo-poster.svg`
- Regenerate: `npm run render:banner`
- Requires FFmpeg only for asset regeneration. Normal Astro builds use the
  committed public assets and do not require FFmpeg.

The sequence moves a crosshair among dashboard components, highlights a target,
opens a dark rounded note popover, types a comment, clicks Add, and leaves a
numbered pin. Four annotations demonstrate a card, activity row, export action,
and sidebar icons. Blue pins, a green Add action, the skeleton dashboard, and
floating charcoal toolbar follow the supplied visual reference. Titles and
comments use fictional Fieldwork sample content.

Resvg renders the authored SVG frames using the bundled Inter fonts. FFmpeg
encodes a 21-second, 12 fps loop with a 128-color palette. The static SVG poster
shows the sidebar note with previously saved pins. All user-facing text is
rendered from source rather than inferred by a generative video model.

The former generated video, source clip, raster poster, and conversion script
have been removed because they are no longer used.

## Smaller SVG features

`src/components/FeatureMotion.astro` retains three original SVG illustrations:
component inspection/source reference, box/arrow/pen annotation, and MCP handoff.
`src/scripts/motion.ts` animates their cursor, paths, and handoff state with GSAP.
Offscreen loops pause. The visible pause button stops the GIF and feature loops;
reduced-motion visitors receive a still poster and static SVGs by default.
