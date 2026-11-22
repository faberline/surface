# Surface v0.1.11 release verification

Main commit `37adcff739e3057c5d70220554fdd151e5f09e17`.
The source tree matches PR head `4d842defd4fb667e97144969124566dbe689ee33` exactly.
Both local source manifests verified against the release tree.

- The pull request and branch workflows passed on the candidate source.
  [PR #185](https://github.com/faberline/surface/pull/185) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45054717519) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45054706499)
- The macOS workspace check passed 803 tests, formatting, provenance,
  Clippy and three WASM checks. Six tests were ignored in the workspace
  run. All five GPU cases passed separately on Apple M1 Max Metal.
  The clipboard hand check did not run.
- Two native headless replays produced eight identical PNGs and JSON.
  Raw API, CLI and MCP bytes matched. The 320 dp profile verified all
  six layout controls fit at 200% text.
  [Native report](native/native-carousel-report.json)
- Two headless Chrome runs passed 76 Material steps and 3,863 checks.
  M73–M77 contributed 359 checks. The runner and source hashes verified
  before and after each run. Both WASM files had hash
  `a8f3f46e1180f38f98ede643fff889be68fafdce3eb81edd1a09492dee659ee6`.
  [Browser report](web/browser-material-carousel.json)

- Main build and behavior checks passed. Main browser CI passed eight
  cases and 4,488 checks, including 3,863 Material checks and 76
  checks in the DevTools headless case.
  [Main CI](https://github.com/faberline/surface/actions/runs/45055837233) · [Browser job](https://github.com/faberline/surface/actions/runs/45055837233/job/137310740631)
