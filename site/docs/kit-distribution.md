# Registry GPUI Kit dependency

Agentation uses the published GPUI Kit 0.11.0 family with the standard
GPUI 0.3.14 backend. Host applications and Agentation must resolve the same
registry Kit version so component types and globals are shared.

Only `gpui-agentation` is released by this repository. No Kit fork packages,
vendor directories, or consumer-level Cargo patches are required.

## Inspector compatibility

Agentation skips the known invisible overlay containers in Kit 0.11.0 using
GPUI public inspector depth controls. Visible notification and dialog content
stays rendered and inspectable. Correction runs only during inspection;
synthetic depth changes never dispatch host scroll or click handlers.

The source-location filter is deliberately tied to the exact pinned Kit
version. When upgrading Kit, review those call sites and run the host, toast,
dialog, fast-click, wheel traversal, and style-editing regression tests.

Run `make snapshot-check` to verify the registry dependency identities and
checksums, one GPUI backend, and the portable MCP graph. Run the complete
`make verify` suite and a Cargo publication dry run before release.

See [source provenance](../../ORIGIN.md) and [consumer setup](consumer-setup.md).
