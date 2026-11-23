# Surface v0.1.32 release verification

Main commit `d110e9e7b80c0c4281f28443c9a07e0781decc71`. The tree matches PR head `939505901b31af24af7df151e0b75fe77ddb0e0a`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1122 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 11,802 checks. Material passed 190 steps and 11,554 checks.
- Six offscreen Metal integration tests and one raster texture cache test passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 12179 checks. Material passed 190 steps and 11554 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45196188412) · [Browser job](https://github.com/faberline/surface/actions/runs/45196188412/job/137767138076)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45196170145)
- [Linux report](web/hosted-list-media.json) · [Linux screenshot](web/hosted-list-media.png)
