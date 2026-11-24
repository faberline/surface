# Surface v0.1.41 release verification

Main commit `5a90a6fd12717207c934f2b0f27b37aff4e0a2c2`. The tree matches PR head `4f80d36334325d4e8bebf64bd63a1147e54fa052`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy, and wasm32 checks passed. 1207 active Rust tests passed.
- Native headless Metal reproduced 8 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 15,298 checks. Material passed 239 steps and 15,050 checks.
- Ten offscreen Metal renderer regressions passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 15675 checks. Material passed 239 steps and 15050 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45386749058) · [Browser job](https://github.com/faberline/surface/actions/runs/45386749058/job/138419539439)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45386738006)
- [Linux report](web/hosted-scroll-modal.json) · [Linux screenshot](web/hosted-scroll-modal.png) · [Linux shortest editor](web/hosted-shortest-editor.png)
