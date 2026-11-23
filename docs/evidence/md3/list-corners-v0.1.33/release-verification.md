# Surface v0.1.33 release verification

Main commit `289574bb438cd88fc10b62d915f8a93552554548`. The tree matches PR head `ec2eecef5468f31fbd998a6fb1cad253e98675a3`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1127 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 12,383 checks. Material passed 194 steps and 12,135 checks.
- Six offscreen Metal integration tests and one raster texture cache test passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 12760 checks. Material passed 194 steps and 12135 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45208147747) · [Browser job](https://github.com/faberline/surface/actions/runs/45208147747/job/137809395198)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45208129195)
- [Linux report](web/hosted-list-corners.json) · [Linux screenshot](web/hosted-list-corners.png)
- [Current Google catalog check](catalog-refresh.md)
