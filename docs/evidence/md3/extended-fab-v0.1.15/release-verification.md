# Surface v0.1.15 release verification

Main commit `b3a9d824da58e3e65acd6a27674fa934e6bf0707`.
The source tree matches PR head `afda0949b718a178a0f97ce779bf2235102fbcc5` exactly.
Both local source manifests verified against the release tree.

## Local verification

- The Surface workspace gate covers provenance, formatting, Clippy,
  unit and integration tests, and the wasm32 consumer contract with
  both feature configurations. The workspace passed 852 tests. The
  manual clipboard test did not run.
- All five ignored renderer tests ran on native Metal and passed.
- Native Gallery replay checked window count at startup and after tests.
  It stayed at zero. The two replays produced the same 18 PNGs and JSON.
- Chrome opened with an explicit headless flag and used WebGPU. Extended FAB
  M95–M99 ran as part of the full Material case.

See the [native and browser receipts](README.md) for exact local
commands and artifacts.

## Release verification

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 5921 total and 5296 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45073495119) · [Browser job](https://github.com/faberline/surface/actions/runs/45073495119/job/137365493610)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45073489554)
