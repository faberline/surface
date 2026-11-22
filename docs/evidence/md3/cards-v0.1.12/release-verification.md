# Surface v0.1.12 release verification

Main commit `7fec533c20c53fe3ee2ac50f89b231896936c08a`.
The source tree matches PR head `4d78f9036ad0ca913af1daa2bf2145ef0a23a493` exactly.
Both local source manifests verified against the release tree.

- The pull request and branch workflows both passed.
  [PR #186](https://github.com/faberline/surface/pull/186) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45058694371) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45058688947)
- The macOS workspace check passed 809 tests, formatting, provenance,
  Clippy and three WASM checks. Six tests were ignored in the workspace
  run. All five GPU cases passed separately on Apple M1 Max Metal.
  The clipboard hand check did not run.
- Two native headless replays produced seven identical PNGs and JSON.
  Raw API, CLI and MCP bytes and metadata matched. CoreGraphics counted
  zero onscreen Gallery windows in the four final runs.
  [Native report](native/native-card-report.json)
- Headless Chrome 157 on Apple Metal 3 passed 81 Material steps and
  4,086 checks. M78–M82 contributed 223 checks. The source, runner,
  browser asset and evidence hashes verified. The card page test
  recorded zero errors.
  [Browser report](web/browser-material-card.json)

- Main build and behavior checks passed. Main browser CI passed eight
  cases and 4,711 checks, including 4,086 Material checks and 76
  checks in the DevTools headless case.
  [Main CI](https://github.com/faberline/surface/actions/runs/45059894236) · [Browser job](https://github.com/faberline/surface/actions/runs/45059894236/job/137323244314)
