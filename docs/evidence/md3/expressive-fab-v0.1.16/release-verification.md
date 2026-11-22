# Surface v0.1.16 release verification

Main commit `c2c2f3de02d791d7557d9db15830f35d4f422236`.
The source tree matches PR head `4a43f87629d846973c8e913a438c6576a8341b32` exactly.
Both local source manifests verified against the release tree.

## Local verification

- The Surface workspace gate passed 869 tests and both wasm32 consumer
  configurations. The manual clipboard test did not run.
- All five offscreen renderer tests ran on native Apple Metal and passed.
- Native Gallery replays reported zero visible windows. The two replays
  matched all 17 PNGs and the JSON report. The API, CLI and MCP matched
  every PNG's bytes and metadata.
- Headless Chrome on Apple Metal 3 passed 5,909 checks across five suites.
  M100–M105 verify the Expressive FAB. The Rust motion suite verifies reversal.
- The final PR includes a coverage wording correction. Both source
  manifests match the final PR and the release tree.

See the [native and browser receipts](README.md) for the exact commands
and screenshots.

## Release verification

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 6286 total and 5661 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45077614347) · [Browser job](https://github.com/faberline/surface/actions/runs/45077614347/job/137378263702)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45077608561)
