# Surface v0.1.7 release verification

The main commit is `40dd3535b3797c7372ed5d9231eb8a44228f6fe6`.
Its source tree matches PR head `e12cc72ea4bd850971ae7114e292355971ad7d25`.
Both source manifests passed against the main commit.

- Both jobs passed on the PR workflow and the branch push workflow.
  [PR #181](https://github.com/faberline/surface/pull/181) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45040014377) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45040007099)
- The macOS workspace check passed 747 Rust tests, Clippy and three WASM
  checks. Six tests were ignored in the workspace run. All five GPU cases
  passed separately on Apple M1 Max Metal. The clipboard hand check did
  not run.
- Two independent native headless replays passed all six cases and
  produced the same 19 PNGs and JSON report. Compact cases verified
  identical PNG bytes and metadata through raw TCP, CLI and MCP.
  [Native report](native/native-bottom-sheet-report.json)
- Headless Chrome on Apple Metal 3 passed all 54 Material steps and
  2,314 checks. M50–M55 exercised the new sheet and ran 214 checks.
  [Browser report](web/browser-material-bottom-sheet.json)
- Both jobs passed on the exact main commit. The hosted E2E runner
  passed 8 suites and 2,939 checks. The Material runner passed 54
  steps and 2,314 checks. DevTools headless passed 3 steps and 76
  checks.
  [Main CI](https://github.com/faberline/surface/actions/runs/45040755518) ·
  [Hosted E2E](https://github.com/faberline/surface/actions/runs/45040755518/job/137264435670)
