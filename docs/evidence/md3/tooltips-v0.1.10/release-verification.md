# Surface v0.1.10 release verification

Main commit `239243c35b3b311f37f531e91183aa4b70be6769`.
The source tree matches PR head `41536a48b2b76875bb049ee0a774e12058374cdf` exactly.
Both local source manifests verified against the release tree.

- The pull request and branch push workflows both passed.
  [PR #184](https://github.com/faberline/surface/pull/184) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45050413083) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45050406323)
- The final macOS workspace check passed 789 tests, Clippy and three
  WASM checks. Six tests were ignored in the workspace run. All five
  GPU cases passed separately on Apple M1 Max Metal. The clipboard
  hand check did not run.
- Native headless Metal replay passed both text and action tooltip
  profiles. Independent runs produced five identical PNGs and
  JSON, and the API, CLI and MCP bytes matched.
  [Native report](native/native-tooltip-report.json)
- Headless Chrome 157 on Apple Metal 3 passed 71 Material steps
  and 3,504 checks. The four tooltip steps ran 191 checks.
  [Browser report](web/browser-material-tooltip.json)

- Main build and behavior checks passed. Main browser CI passed eight
  cases and 4,129 checks, including 3,504 Material checks and 76
  checks in the DevTools headless case.
  [Main CI](https://github.com/faberline/surface/actions/runs/45051497446) · [Browser job](https://github.com/faberline/surface/actions/runs/45051497446/job/137297393213)
