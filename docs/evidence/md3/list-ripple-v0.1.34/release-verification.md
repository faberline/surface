# Surface v0.1.34 release verification

Main commit `cc5dfc81419d6f5702fbd9764bf0bb311fd5bc4f`. The tree matches PR head `33266d04f67264e19147980d8f1d3f59afcd852c`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1138 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 12,641 checks. Material passed 199 steps and 12,393 checks.
- Eight offscreen Metal tests passed, including a clocked photo ripple.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 13018 checks. Material passed 199 steps and 12393 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45221833311) · [Browser job](https://github.com/faberline/surface/actions/runs/45221833311/job/137857850847)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45221811745)
- [Linux report](web/hosted-list-ripple.json) · [Linux screenshot](web/hosted-list-ripple.png)
