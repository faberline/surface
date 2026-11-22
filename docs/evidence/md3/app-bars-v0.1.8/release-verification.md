# Surface v0.1.8 release verification

The main commit is `9557229b4409a7ce0aa92e88d32e601751945c41`.
Its source tree matches PR head `d98b5cf27c5e1287bfa33ee9dc8ca0b87452bb37`.
Both source manifests passed against the main commit.

- The pull request and branch push workflow both passed.
  [PR #182](https://github.com/faberline/surface/pull/182) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45043169140) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45043162286)
- The final macOS workspace check passed 760 tests, Clippy and the
  three WASM checks. Six tests were ignored in the workspace run.
  All five GPU cases passed separately on Apple M1 Max Metal. The
  clipboard hand check did not run.
- The two native headless profiles passed on two independent runs.
  Their 14 PNGs and JSON reports were byte-identical. The native
  replay checked raw TCP, CLI and MCP parity for the compact frames.
  [Native report](native/native-app-bar-report.json)
- Headless Chrome 157 on Apple Metal 3 passed all 61 Material
  steps and 2,719 checks. The seven app-bar steps ran 405 checks.
  [Browser report](web/browser-material-app-bar.json)
- Both jobs passed on the exact main commit. The hosted E2E runner
  passed 8 suites and 3,344 checks. Material passed 61 steps and 2,719
  checks. DevTools headless passed 3 steps and 76 checks.
  [Main CI](https://github.com/faberline/surface/actions/runs/45044121807) ·
  [Hosted E2E](https://github.com/faberline/surface/actions/runs/45044121807/job/137274855768)
