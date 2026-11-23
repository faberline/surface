# Surface v0.1.18 release verification

Main commit `f72a23b8422341f979605ebc605f3a2a00269551`.
The source tree matches PR head `b2fa97351d866ed793e50d1e2a7d1757672d056c` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, fmt, Clippy and wasm32 checks passed. All 905 active Rust tests passed. The manual system clipboard check did not run.
- Native headless Metal reproduced six PNGs and the report. Page.screenshot, CLI and MCP agreed. Both profiles observed zero visible windows.
- Five offscreen Metal renderer tests passed.
- Headless Chrome WebGPU. All five cases passed 6,631 checks. Material passed 114 steps and 6,383 checks.
- A trusted browser wheel event scrolls the compact picker body over a date cell.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 7008 total and 6383 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45087608989) · [Browser job](https://github.com/faberline/surface/actions/runs/45087608989/job/137409684039)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45087601343)
