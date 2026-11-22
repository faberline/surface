# Surface v0.1.9 release verification

The main commit is `d57fcf47d57f8f4e6e13d55e0b0e6c7547548a0c`.
Its source tree matches PR head `e6ef0dd967ec75e8ee45777f38d40a7c7db87217`.
Both source manifests passed against the main commit.

- The pull request and branch push workflow both passed.
  [PR #183](https://github.com/faberline/surface/pull/183) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45046501591) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45046494188)
- The final macOS workspace check passed 771 tests, Clippy and three
  WASM checks. Six tests were ignored in the workspace run. All five
  GPU cases passed separately on Apple M1 Max Metal. The clipboard
  hand check did not run.
- Two native headless Metal replays passed both profiles and produced
  12 identical PNGs and matching JSON. The replay checked raw TCP,
  CLI and MCP parity on compact captures, including the paired FAB.
  [Native report](native/native-toolbar-report.json)
- Headless Chrome 157 on Apple Metal 3 passed all 67 Material steps
  and 3,313 checks. The six toolbar steps ran 594 checks.
  [Browser report](web/browser-material-toolbar.json)
- Both jobs passed on the exact main commit. The hosted E2E runner
  passed 8 suites and 3,938 checks. Material passed 67 steps and 3,313
  checks. DevTools headless passed 3 steps and 76 checks.
  [Main CI](https://github.com/faberline/surface/actions/runs/45047531525) ·
  [Hosted E2E](https://github.com/faberline/surface/actions/runs/45047531525/job/137285288013)
