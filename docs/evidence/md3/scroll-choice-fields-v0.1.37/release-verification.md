# Surface v0.1.37 release verification

Main commit `1f85afea87291f75f1a88c5b45bbee8d5f0def9b`. The tree matches PR head `8a08a87c4c33996302d54a25e221cbaf52da3ff8`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy, and wasm32 checks passed. 1175 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 13,711 checks. Material passed 215 steps and 13,463 checks.
- Nine offscreen Metal renderer regressions passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 14088 checks. Material passed 215 steps and 13463 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45279439069) · [Browser job](https://github.com/faberline/surface/actions/runs/45279439069/job/138059702099)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45279422993)
- [Linux report](web/hosted-scroll-choices.json) · [Linux screenshot](web/hosted-scroll-choices.png)
