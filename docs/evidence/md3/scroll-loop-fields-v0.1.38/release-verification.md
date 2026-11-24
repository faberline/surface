# Surface v0.1.38 release verification

Main commit `0f31cab083d9a2f425501422197eff724b366c40`. The tree matches PR head `d308ae0058b4e3f8a60e98096f5b39fb1c7a5d73`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy, and wasm32 checks passed. 1185 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 14,005 checks. Material passed 221 steps and 13,757 checks.
- Ten offscreen Metal renderer regressions passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 14382 checks. Material passed 221 steps and 13757 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45300185002) · [Browser job](https://github.com/faberline/surface/actions/runs/45300185002/job/138130491162)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45300178406)
- [Linux report](web/hosted-scroll-loop.json) · [Linux screenshot](web/hosted-scroll-loop.png)
