---
name: gpui-agentation website
description: Native feedback explained through quiet documentation and clear feature motion.
colors:
  bg: "#fafafa"
  surface: "#fff"
  sunken: "#f2f2f2"
  text: "#242424"
  muted: "#666666"
  line: "#dfdfdf"
  amber: "#f4b942"
  amber-ink: "#523600"
  dark-bg: "#171717"
  dark-surface: "#242424"
  dark-sunken: "#262626"
  dark-text: "#ededed"
  dark-muted: "#a8a8a8"
  dark-line: "#3b3b3b"
  focus: "#986300"
typography:
  display:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "clamp(42px, 5.3vw, 66px)"
    fontWeight: 600
    lineHeight: 1.05
    letterSpacing: "-2.5px"
  body:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.65
  code:
    fontFamily: "JetBrains, monospace"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.85
rounded:
  control: "7px"
  container: "8px"
  feature: "10px"
  film: "12px"
spacing:
  wrapper: "40px"
  wrapper-mobile: "20px"
  feature-gap: "28px"
  reference-separation: "64px"
components:
  button-primary:
    backgroundColor: "{colors.text}"
    textColor: "{colors.bg}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "10px 16px"
---

## Overview

A reading-focused Astro site explains native GPUI feedback with an illustrated
GIF and three original SVG feature sequences. The illustrations are explanatory,
not evidence of a running native app. The example-app page is intentionally absent.

## Colors

All page and control neutrals are achromatic, including the dark palette. Amber
connects selection outlines, numbered annotation pins, and handoff motion.

**The Annotation Accent Rule.** Amber identifies annotation state; ordinary
reference surfaces remain neutral. Syntax highlighting and the resolved-state
check may use meaningful secondary colors.

## Typography

Bundled Inter carries headings, prose, navigation, and controls. Bundled
JetBrains Mono identifies commands, source references, and code. The existing
font pairing stays consistent with the native product.

**The Source Voice Rule.** Code and technical metadata use the mono family;
prose and actions use Inter. Kirari adds semantic token color without changing
source content, indentation, or clipboard output.

## Layout

The centered wrapper remains 1180px wide including 40px horizontal padding,
reducing to 28px at 950px and 20px at 700px. The hero retains its wide two-line
heading. A full-width illustrated film leads into three equal feature columns
with 28px gaps; the panels stack on mobile.

Documentation retains its sticky 190px navigation column, 55px gutter, and
content column capped at 760px. At the intermediate breakpoint these become
145px and 32px. Mobile section links wrap above the reference text. Code wraps
long lines and uses 11px text on mobile, 12px elsewhere.

## Elevation & Depth

The film and feature art use flat fills, quiet borders, and restrained corner
radii. Documentation containers rely on surface tone and thin separators.

**The Working Layer Rule.** Show feature behavior through selection, cursor,
path drawing, and status changes; do not add decorative motion to reference prose.

## Shapes

Use compact rounded rectangles for controls and source containers. Feature
art uses 10px corners, the film 12px. Numbered annotation pins stay circular.
SVG strokes are consistent, with amber reserved for meaningful annotations.

## Components

- **Illustrated film:** self-hosted GIF, static poster, explicit illustration
  label, and keyboard-accessible Play/Pause control. Source provenance and the
  reproducible conversion command live in `site/docs/motion-assets.md`.
- **Feature SVGs:** inspect/source reference, box/arrow/pen, and agent handoff.
  GSAP animates only the relevant shapes. Loops pause offscreen. The global
  pause control stops the film and feature loops together.
- **Source containers:** build-time Kirari highlighting for Rust, TOML, shell,
  and JSON. Neutral background; theme-specific readable token colors. Copy
  shows a green “Copied!” state for two seconds and copies original source text.
- **Navigation:** compact header links and sticky reference navigation. Current
  location uses both stronger typography and `aria-current`.
- **Accessibility:** visible focus, semantic button labels, and descriptive
  illustration text. Reduced motion starts with the static poster/SVGs and
  disables scroll entrances. JavaScript-free content remains readable.

## Do's and Don'ts

- Do keep all neutral colors free of green or other hue casts.
- Do make motion explain a concrete step in the annotation workflow.
- Do keep static compositions meaningful and source text copyable.
- Don't present generated illustrations as native screenshots or runtime proof.
- Don't restore the removed playground or complete-example download section.
