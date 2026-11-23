# Surface v0.1.19 release verification

Main commit `c64c49f0d5c0c13a92c6ee3b9fc8272e8e103560`.
The source tree matches PR head `5f9a098774791e5b9ade36cf0d851fe37ebcebd5` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, fmt, Clippy and wasm32 checks passed. All 919 active Rust tests passed. The manual system clipboard check did not run.
- Native headless Metal reproduced six PNGs and the report. Page.screenshot, CLI and MCP agreed. Both profiles observed zero visible windows.
- Five offscreen Metal renderer tests passed.
- Headless Chrome WebGPU. All five cases passed 6,902 checks. Material passed 120 steps and 6,654 checks.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 7279 total and 6654 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45093544518) · [Browser job](https://github.com/faberline/surface/actions/runs/45093544518/job/137428215822)
  [Linux date picker report](web/hosted-pr-docked-date-picker.json) · [Linux compact screenshot](web/hosted-compact-dark-rtl-200.png)
- The branch CI passed after a browser rerun at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45093541721)
  Attempt 1 stopped at the existing narrow Rename dialog case M20. [Timeout geometry](web/hosted-rename-dialog-retry.json) · [First attempt](https://github.com/faberline/surface/actions/runs/45093541721/attempts/1)
