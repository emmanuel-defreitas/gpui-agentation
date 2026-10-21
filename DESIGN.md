---
name: gpui-agentation website
description: A native developer workbench for annotation previews and installation reference.
colors:
  bg: "#fafbfb"
  surface: "#fff"
  sunken: "#f1f3f3"
  text: "#202626"
  muted: "#616c6c"
  line: "#dde3e3"
  amber: "#f4b942"
  amber-ink: "#523600"
  toolbar: "#202626"
  toolbar-ink: "#f5f7f7"
  focus: "#986300"
  dark-bg: "#171c1c"
  dark-surface: "#202626"
  dark-sunken: "#282f2f"
  dark-text: "#edf1f1"
  dark-muted: "#a4afaf"
  dark-line: "#394343"
  dark-toolbar: "#eff3f3"
  dark-toolbar-ink: "#202626"
  dark-focus: "#f4b942"
  note-bg: "#ffdda0"
  note-text: "#392700"
  note-line: "#c58a15"
  note-field: "#fff4db"
  note-placeholder: "#786237"
  drawing: "#b77a00"
typography:
  display:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "clamp(42px, 5.3vw, 66px)"
    fontWeight: 600
    lineHeight: 1.05
    letterSpacing: "-2.5px"
  headline:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "34px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-1.3px"
  body:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.65
  reference-body:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.85
  label:
    fontFamily: "Inter, Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 600
  code:
    fontFamily: "JetBrains, monospace"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.85
rounded:
  field: "5px"
  icon: "6px"
  control: "7px"
  container: "8px"
  dialog: "11px"
  workbench: "12px"
spacing:
  compact: "8px"
  control: "10px 16px"
  panel: "18px 19px 16px"
  page: "40px"
  section: "64px"
components:
  button-primary:
    backgroundColor: "{colors.text}"
    textColor: "{colors.bg}"
    rounded: "{rounded.control}"
    padding: "{spacing.control}"
    typography: "{typography.label}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "{spacing.control}"
    typography: "{typography.label}"
  button-secondary-hover:
    backgroundColor: "{colors.sunken}"
  note-field:
    backgroundColor: "{colors.note-field}"
    textColor: "{colors.note-text}"
    rounded: "{rounded.field}"
    padding: "11px"
  annotation-pin:
    backgroundColor: "{colors.amber}"
    textColor: "{colors.amber-ink}"
    width: "27px"
    height: "27px"
  code-block:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.text}"
    rounded: "{rounded.container}"
  toolbar:
    backgroundColor: "{colors.toolbar}"
    textColor: "{colors.toolbar-ink}"
    padding: "9px 13px"
---

# Design System: gpui-agentation website

## Overview

**Creative North Star: "Native developer workbench"**

The native developer workbench carries the visual vocabulary of the annotation product into a reading-focused website. Cool neutral surfaces, graphite controls, and amber annotation marks make the interactive preview feel related to the native interface.

Inter provides practical reading and control text; JetBrains Mono identifies source, commands, and technical metadata. Spacious documentation sections surround a denser preview. This specification describes the Astro website only; it does not define the native crate’s API or native UI tokens.

**Key Characteristics:**

- Neutral reading surfaces with amber annotation state.
- Bundled Inter and JetBrains Mono for continuity with the native product.
- Quiet borders and restrained shadows; compact working controls.
- Responsive documentation with a functional, explicitly simulated preview.

## Colors

The palette pairs cool paper-like neutrals with warm annotation amber. Frontmatter records the exact source colors; CSS custom properties in `site/public/styles.css` remain implementation authority.

### Primary

- **Annotation Amber** (`amber`): numbered pins, comment counts, and selected preview tools.
- **Amber Ink** (`amber-ink`): readable labels on amber fills.
- **Focus Amber** (`focus`): keyboard outlines, with its brighter dark-theme counterpart.
- **Drawing Ochre** (`drawing`): box, arrow, and pen strokes in the browser simulation.
- **Warm note colors** (`note-*`): a deliberately distinct editing surface, retained across both themes.

### Neutral

- **Cool Page** (`bg`), **Reference White** (`surface`), and **Sunken Gray** (`sunken`): page, containers, and source blocks.
- **Graphite** (`text`) and **Muted Slate** (`muted`): primary and secondary reading hierarchy.
- **Quiet Divider** (`line`): borders and section separators.
- **Toolbar Graphite** (`toolbar`) and **Toolbar Ink** (`toolbar-ink`): high-contrast preview controls.
- **Dark counterparts** (`dark-*`): replace semantic neutrals and focus color through the root theme attribute. Amber and the warm note palette remain shared.

**The Annotation Accent Rule.** Amber identifies annotation state; ordinary reference surfaces remain neutral.

## Typography

**Display and Body Font:** bundled Inter, with Arial and sans-serif fallbacks. The source maps the bundled regular face to weight 400 and the semibold face to the 600–800 range.

**Source and Metadata Font:** bundled JetBrains Mono, registered under the CSS family name `JetBrains`, with monospace fallback.

This is an intentionally practical pairing inherited from the native product. The type hierarchy depends on size, weight, spacing, and context rather than a separate decorative display face.

### Hierarchy

- **Display:** the fluid frontmatter role is used by the main heading; at the mobile breakpoint it becomes (46px) with tracking (-1.9px).
- **Headline:** documentation section headings use the frontmatter role; mobile headings reduce to (30px) with tracking (-1px).
- **Body:** the base role applies globally; reference prose uses the smaller, more open `reference-body` role. Reference prose reduces to (12px) on mobile.
- **Label:** semibold Inter identifies actions and section labels. Navigation uses regular Inter (12–13px).
- **Code:** source blocks use the `code` role, reducing to (10px) on mobile; inline code uses (.88em). Compact preview metadata is smaller and contextual, not a general reading-text scale.

**The Source Voice Rule.** Use the mono family for commands, source references, and technical metadata; use Inter for prose and actions.

## Layout

The centered page wrapper has a maximum outer width (1180px), including horizontal padding (40px per side). At (950px) and below the padding becomes (28px); at (700px) and below it becomes (20px).

The desktop preview places a flexible canvas beside a comment rail (265px). The rail narrows to (225px) at the intermediate breakpoint. At the mobile breakpoint, the rail stacks below the canvas. Hiding comments gives the canvas the full preview width.

Documentation uses a sticky navigation column (190px), a gap (55px), and a content column capped at (760px). At the intermediate breakpoint these become (145px) and (32px). Mobile documentation becomes one column with wrapping section links above the text. Header navigation is hidden on mobile; documentation links remain available.

Reference sections use equal bottom padding and separation (64px), reducing to (43px) on mobile. The preview and reference area have a larger separating interval (94px desktop; 52px mobile). A wide-screen adjustment begins at (1200px). Tables of options and the example callout stack on mobile. Code wraps long lines rather than forcing the page wider.

## Elevation & Depth

Most reference surfaces are flat, separated by tonal steps and thin borders. The workbench has a quiet ambient shadow (`0 12px 32px #17212108`); floating pins use a small locating shadow (`0 1px 5px #0002`). The note dialog is the strongest layer (`0 20px 60px #0003`) above a dimmed backdrop (`#17212166`). Static explanatory pins and comment-rail pins omit the shadow.

**The Working Layer Rule.** Reserve floating depth for the interactive workbench, annotations, and dialog; documentation containers rely on borders and tone.

## Shapes

Rounded rectangles carry a compact desktop-control character. The frontmatter distinguishes fields, icon buttons, ordinary buttons, source containers, dialog, and outer workbench corners. Numbered annotation pins and counts are circular. Dividers use single-pixel borders. The workbench and code containers clip their children to the outer silhouette.

## Components

### Buttons

Primary actions use text/background inversion; secondary actions use a surface fill and quiet border. Both use the `control` radius and padding tokens. Hover darkens buttons slightly through a brightness filter; secondary buttons also move to the sunken surface. Background and color transitions take (.15s). Keyboard focus has a visible outline (2px) with offset (4px). Icon-only controls use a compact square (32px) and the `icon` radius.

### Annotation Pins and Toolbar

Amber circular numbered pins connect locations in the canvas to comments in the rail. Main pins use the frontmatter dimensions; rail pins are smaller (23px), borderless, and flat. The toolbar uses the paired toolbar colors; a pressed tool switches to amber and amber ink. Toolbar focus stays amber. Pin arrival scales and fades over (.3s) with the source easing curve.

### Source Containers

Sunken backgrounds, quiet borders, and the `container` radius group source text. A divided header holds a language/context label and copy action. Source padding is (17px 18px 20px), becoming (14px 12px 17px) on mobile. Copy feedback briefly changes to a check and “Copied!” state.

### Note Dialog and Field

The warm editor uses the `note-*` palette with a darker amber border, `dialog` radius, and padding (20px). Its width is (420px), capped to the viewport minus (32px). The textarea uses the `field` radius, padding (11px), a minimum height (96px), and vertical resizing. Keep a programmatic label even when its visual presentation is hidden. Closing or cancelling restores the preview; saving updates the matching pin and comment.

### Navigation

Desktop header links are compact muted text. Documentation navigation is sticky on desktop and wraps inline on mobile. The current documentation link becomes primary text and semibold; hover links underline with offset (4px). The current location is also represented by `aria-current`.

### Feedback and Motion

A bottom-centered toast fades and rises into view using the source easing curve, then clears after (2400ms). The clipboard action remains temporarily disabled during its copied state. Reduced-motion preferences disable transitions and animations and remove smooth scrolling. Theme choice follows the system until explicitly changed and can be stored locally.

## Do's and Don'ts

### Do:

- Do use the semantic surface and text variables so light and dark appearance stay coherent.
- Do reserve amber for annotation identity, selection, and related focus states.
- Do pair readable Inter prose with JetBrains Mono source and technical metadata.
- Do retain visible keyboard focus and honor reduced-motion preferences.
- Do keep the browser preview visibly identified as a simulation.

### Don't:

- Don't use the browser preview as evidence of native runtime behavior.
- Don't replace the bundled font pairing solely to create a separate marketing identity.
- Don't spread annotation amber across ordinary documentation containers.
- Don't introduce large decorative shadows into reference content.
