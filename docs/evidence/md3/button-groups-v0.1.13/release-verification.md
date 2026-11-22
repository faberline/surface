# Surface v0.1.13 release verification

Main commit `abfdd046a09b9b987a47d3be23e42c2484096743`.
The source tree matches PR head `3679f4168b911016fc9e401d2f5446336f595110` exactly.
Both local source manifests verified against the release tree.

- The pull request and branch workflows passed on the final documented
  source. The evidence manifests also match that source.
  [PR #187](https://github.com/faberline/surface/pull/187) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45063560823) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45063555894)
- The macOS workspace check passed 824 tests, formatting, Clippy,
  provenance and three WASM checks. Six tests were ignored in the
  workspace run. All five GPU cases passed separately on Apple M1 Max.
  The clipboard hand check did not run.
- Two native headless replays produced nine identical PNGs and JSON.
  Raw API, CLI and MCP bytes matched. CoreGraphics counted zero onscreen
  Gallery windows across four final processes.
  [Native report](native/native-button-group-report.json)
- Headless Chrome 157 on Apple Metal 3 passed 87 Material steps and
  4,436 checks. M83–M88 contributed 350 checks. The source, runner,
  browser asset and evidence hashes verified. No browser process or
  page error remained.
  [Browser report](web/browser-material-button-group.json)

- Main build and behavior checks passed. Main browser CI passed eight
  cases and 5,061 checks, including 4,436 Material checks and 76
  checks in the DevTools headless case.
  [Main CI](https://github.com/faberline/surface/actions/runs/45064772618) · [Browser job](https://github.com/faberline/surface/actions/runs/45064772618/job/137338361322)
