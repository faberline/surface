Use short, plain sentences in user-facing reports.

# Surface

Surface is a standalone Rust MicroDOM application framework.
See `README.md` and `docs/architecture.md`.

## Layers

- Keep semantic state, layout and display lists independent of platform APIs.
  `crates/surface` and `crates/ui-runtime` must not depend on wgpu, winit or
  a platform crate.
- Keep wgpu types in surface-wgpu and platform types in surface-platform.
- Keep the surface and ui-runtime APIs imported from Core compatible until
  the migration described in `docs/migration.md` is complete.
  Add APIs. Do not change or remove imported ones.
- Do not edit `crates/ui-runtime/tests/consumer_contract.rs`. It pins the
  API that Jet and Core use.

## Build and check

- Use `cap` and `CARGO_BUILD_JOBS=2` for local Rust builds.
  Put `~/.cargo/bin` first in `PATH`.
- Run `sh scripts/check.sh` before you report a cross-platform change as
  built. It runs provenance, `cargo fmt --all --check`, clippy, the
  workspace tests, and the `wasm32-unknown-unknown` checks.
  For a small change, run at least fmt, the affected Rust tests, the native
  `cargo check` and the wasm32 check.
- A successful build does not prove browser or GPU runtime behavior.
- GPU tests are `#[ignore]`. Run them with `sh scripts/check-gpu.sh` on a
  real adapter.
- Never turn a skipped or ignored GPU test into passing acceptance evidence.
  If it did not run, say "did not run".

## Pull requests

- Start the PR body with `Closes #<n>` or `Refs #<n>`.
- Do not write fix, close or resolve next to an issue number or link that the
  PR does not finish. GitHub closes that issue when the PR merges.
- If you change a file imported from Core or a pinned font, update
  `docs/provenance.json`. `cargo xtask provenance` checks it.
- Do not commit `artifacts/` or `examples/*/web/pkg/`. They are generated.
