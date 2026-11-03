Use short, plain sentences in user-facing reports.

Surface is a standalone Rust MicroDOM application framework.
Keep semantic state, layout and display lists independent of platform APIs.
Keep wgpu types in surface-wgpu and platform types in surface-platform.
Keep the surface and ui-runtime APIs imported from Core compatible until the
migration described in docs/migration.md is complete.

Run cargo fmt --all --check, affected Rust tests, native cargo check, and the
wasm32-unknown-unknown check before reporting a cross-platform change as built.
A successful build does not prove browser or GPU runtime behavior.
Use cap and CARGO_BUILD_JOBS=2 for local Rust builds.
Never turn a skipped GPU test into passing acceptance evidence.
