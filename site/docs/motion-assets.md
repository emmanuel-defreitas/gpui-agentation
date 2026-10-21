# Motion assets

The illustrated loop was generated specifically for this site with the user's
explicitly selected HeyGen connector. It is a conceptual illustration, not a
recording of the native application. No real user data or application capture
was supplied.

- Tool: HeyGen `text_to_video`, model `heygen-video-1`
- Video ID: `86e5269e8420436d9a73b45257835fcc`
- Request: 5 seconds, 16:9, 768p, prompt enhancement disabled
- Original: `media-source/annotation-demo.mp4` (kept outside public assets)
- Published: `public/assets/motion/annotation-demo.gif` and poster PNG
- Processing: centered crop, neutral-pixel color correction retaining amber,
  12 fps, 128-color palette, no audio; `scripts/render-demo.py` reproduces it
- Reproduce: `python3 scripts/render-demo.py media-source/annotation-demo.mp4`
- FFmpeg is needed only to regenerate assets, not to build the Astro site.

## Exact generation prompt

Create a silent 5-second seamless loop of a precise, minimal flat vector illustration for gpui-agentation, a native developer annotation tool. Pure neutral charcoal background #171717, gray app window with a tiny simplified code panel, thin off-white outlines, amber #f4b942 numbered annotation pin and cursor. Show a cursor pointing at a UI card; an amber rectangular selection traces around the card, a small amber note appears with two horizontal text-like strokes, then an amber dot travels to a small code bracket symbol to represent agent handoff. Return smoothly to the initial state. All shapes should be clean SVG-like geometry, no real faces, no avatars, no narration, no music, no gradients, no green tint, no photorealism, no marketing logos, no fabricated legible text. Locked orthographic composition, ample breathing room, deliberate gentle easing. This is an abstract illustrative motion study, not a recording of a real application.

## SVG features

`src/components/FeatureMotion.astro` contains three original SVG illustrations:
component inspection/source reference, box/arrow/pen annotation, and MCP handoff.
`src/scripts/motion.ts` animates their cursor, paths, and handoff state with GSAP.
Offscreen loops pause. The visible pause button stops the GIF and feature loops;
reduced-motion visitors receive a still poster and static SVGs by default.
