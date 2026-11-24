# Surface v0.1.40 release verification

Main commit `ccfc9c0e3c6f47c6e01ca045877d34e79256f285`. The tree matches PR head `99bffd824575bc4febe236e4ab3c03c2c100a7fa`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy, and wasm32 checks passed. 1193 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 14,633 checks. Material passed 233 steps and 14,385 checks.
- Ten offscreen Metal renderer regressions passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 15010 checks. Material passed 233 steps and 14385 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45346607474) · [Browser job](https://github.com/faberline/surface/actions/runs/45346607474/job/138282568565)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45346597362)
- [Linux report](web/hosted-scroll-popup.json) · [Linux screenshot](web/hosted-scroll-popup.png)
