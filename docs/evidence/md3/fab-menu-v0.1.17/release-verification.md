# Surface v0.1.17 release verification

Main commit `e004097015d1545b96b209ec7bf4012044bbc8b9`.
The source tree matches PR head `339335629805f3a5fce40fdf5bf99929ba89c924` exactly.
Both local source manifests verified against the release tree.

- Local workspace provenance, fmt, Clippy and wasm32 checks passed. All 885 active Rust tests passed. The manual system clipboard check did not run.
- The native headless Metal Gallery matched all ten PNGs and the report across two replays. Page.screenshot, CLI and MCP agreed on each capture. Both profiles observed zero visible windows.
- The five separate offscreen Metal renderer tests passed.
- Local headless Chrome WebGPU. Five cases passed 6,532 checks. Material passed 110 steps and 6,284 checks.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 6909 total and 6284 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45081648952) · [Browser job](https://github.com/faberline/surface/actions/runs/45081648952/job/137390924267)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45081638437)
