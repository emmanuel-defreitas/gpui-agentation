# Atomic Kit cutover

Active root branch: `migration/kit-cutover`, based on dev
`2a370c4a420ccfb8c1e68dea5467baf18161026c`.

At the user's request to continue presentation work, this isolated branch locally
integrates the reviewed preparation heads #129 (`7e2e598`) and #130 (`0233eca`).
They remain separate PRs targeting dev; this is not a claim that they have merged.
Refresh/reconcile their final merge commits before the root is ready for review.

The first presentation implementation is the Kit inspector consumer described in
`kit-inspector/README.md`. It uses one selected snapshot, Kit 0.11.0 / GPUI 0.3.14,
and compiles real owned services. The full production native library deliberately
still builds on its existing Ely/GPUI graph. A Kit-only harness is the transitional
compile gate allowed by #116; full-native Kit compilation remains pending the
other presentation ports. No process or consumer mixes the two GPUI families.

The root PR stays draft until #117–#121 are satisfied. Pending gates:

- Land/reconcile #114 and #115 preparation PRs; complete bootstrap integration.
- Port toolbar/comments/text editors and Settings/menu bar; wire this inspector
  into the production overlay and annotation synchronization.
- Resolve delivery of the version-checked Kit picking patch; remove Ely and its
  generator, dependencies, public exports and assets with consumer migration docs.
- Run full production/site/package checks and native acceptance on the final head.
- Merge the reviewed atomic root into dev and retire temporary branches afterward.

Owner: @emmanuel-defreitas, with implementation in this chat. No release or
promotion is part of this branch's current work.
