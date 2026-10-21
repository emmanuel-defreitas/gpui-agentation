# Bundled pinned Ely source

This directory preserves **Ely GPUI Components** runtime source, assets and
license notices byte-for-byte from upstream Ely 0.1.1 at commit
`da3fdcaf39f2fa85ce1dd9cad07c3c81cf1dd721`:
https://git.zacharyzhang.com/ZacharyZhang-NY/Ely-GPUI-Components

Copyright (c) 2026 Ely GPUI Component contributors. The original MIT license
is in LICENSE; font and icon licenses remain in assets. Cargo.upstream.toml
preserves the upstream manifest. UPSTREAM.json records the revision and SHA-256
hashes for every runtime file and the original manifest.

There is no Cargo package in this directory and no dependency on the separately
published gpui-agentation-ely crate. gpui-agentation includes these files in its
own crate archive. All Ely runtime dependencies are optional and activated only
by the native feature, with GPUI and platform pinned to =0.3.8.

## Build adaptation

For native builds, build/ely.rs copies Rust files into OUT_DIR and makes three
limited adaptations without editing this snapshot:

- Internal crate:: paths become crate::ely:: paths, including paths in macros.
- cfg(test) predicates become false, keeping upstream unit tests disabled as
  they were when Ely was an external dependency. The test-support feature is
  preserved for native test initialization.
- RustEmbed's asset folder points to vendor/ely/assets inside the package.

The adapter uses Rust tokens and byte offsets; comments, string literals,
formatting and line numbers are preserved. Adapted output is never committed.
Dependency lint allowances are scoped to the bundled module. MCP-only builds
skip source generation and do not activate Ely dependencies or its tokenizer.

Run python3 .github/scripts/check-ely-snapshot.py from the repository root.
Add --upstream /path/to/pinned/ely to compare against the actual Git commit.

## Consumer compatibility

Bundled components are available as gpui_agentation::ely in debug native builds.
They have their own Rust type identity and theme/focus state, independently of
any Ely crate used by the host. A host's Ely initialization does not initialize
these components. Do not pass external Ely types into APIs that expect bundled
types. Initialize the bundled Assets/theme/key bindings for native annotations.
Agentation's release library remains empty, so release UI must use the host's
own component library.

The previously published gpui-agentation 0.2.0 still uses gpui-agentation-ely
0.1.1; those immutable releases remain available. Bundling starts in 0.3.0.
