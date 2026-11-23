# Surface v0.1.20 release verification

Main commit `7567409f9c21f09b5eb0004ca631379f59459375`.
The source tree matches PR head `fb3bc8ecc32a65055107e4b312eae864585a0988` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, fmt, Clippy and wasm32 checks passed. All 950 active Rust tests passed. Manual clipboard checks did not run.
- Native headless Metal reproduced 6 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 7,238 checks. Material passed 126 steps and 6,990 checks.
- Five offscreen Metal tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 7615 total and 6990 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45100968210) · [Browser job](https://github.com/faberline/surface/actions/runs/45100968210/job/137451480804)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45100962930)
  [Linux range report](web/hosted-date-range-picker.json) · [Linux short viewport](web/hosted-compact-short.png)
