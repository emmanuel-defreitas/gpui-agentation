# Inspection services — #115

Status: first extraction slice; #115 remains open for inspector state and style
services. Production still uses the pinned Ely / GPUI 0.3.8 graph.

`src/inspection/source.rs` owns source paths, quoted editor command construction,
editor launch, and clipboard fallback. Its boundary uses only standard-library
types and GPUI App/Entity primitives. It has no Ely or Kit component dependency.

The temporary `vendor/ely/src/tooling/inspector/source.rs` adapter renders the
breadcrumb and notice with Ely controls. Its explicit relative import reaches
the owned service without weakening the build generator's namespace rewriting.
Source navigation has one implementation; it is no longer implemented in Ely.
The generated-Ely-to-editable-source mapping stays until the later vendor removal.

Existing integration tests exercise compile-root and relative path resolution,
quoted editor arguments, unsupported editor and failed-launch clipboard fallback,
and breadcrumb component click behavior. Navigation tests now call the owned
service directly; existing inspector render tests exercise the presentation
adapter, but do not directly click its editor-launch callback.

Remaining #115 work: extract selection/configuration/target state, style editing,
reset/restore and snapshot collection; define host/companion cleanup and renderer
ownership, retaining existing public tooling exports until their planned removal.
Source extraction alone does not satisfy those acceptance gates.
