# Surface v0.1.14 release verification

Main commit `b555e7ed53751d020a543abeba4c3a347708cab8`.
The source tree matches PR head `80d0bc2f66b0f6ab2731b1551fdd1755e9a2bc4b` exactly.
Both local source manifests verified against the release tree.

## Local verification

- The Surface workspace gate covers provenance, formatting, Clippy,
  unit and integration tests, and the wasm32 consumer contract with
  both feature configurations.
- All five ignored renderer tests ran on native Metal and passed.
- Native Gallery replay checked window count at startup and after tests.
  It stayed at zero. The two replays produced the same PNG and JSON bytes.
- Chrome opened with an explicit headless flag and used WebGPU. Split
  Button M89–M94 ran as part of the full Material case.

See the [native and browser receipts](README.md) for exact local
commands and artifacts.

## Release verification

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 5497 total and 4872 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45070005671) · [Browser job](https://github.com/faberline/surface/actions/runs/45070005671/job/137354627535)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45069998836)
