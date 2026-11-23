# Surface v0.1.36 release verification

Main commit `1256b9b9c3f3154c84a67146a6b5b3766bd6e2d9`. The tree matches PR head `6a6c1e0a462c447031acd9a3b4a0194edf336f02`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy, and wasm32 checks passed. 1157 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 13,411 checks. Material passed 209 steps and 13,163 checks.
- Nine offscreen Metal renderer regressions passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 13788 checks. Material passed 209 steps and 13163 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45255976162) · [Browser job](https://github.com/faberline/surface/actions/runs/45255976162/job/137977779612)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45255957387)
- [Linux report](web/hosted-scroll-fields.json) · [Linux screenshot](web/hosted-scroll-fields.png)
