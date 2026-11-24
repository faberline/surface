# Surface v0.1.39 release verification

Main commit `1ad7aece8b23f0367e8398b43787534efa4323ae`. The tree matches PR head `cafd78d18b384cb0aae0c2da0f1072cc012054e2`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy, and wasm32 checks passed. 1188 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 14,279 checks. Material passed 227 steps and 14,031 checks.
- Ten offscreen Metal renderer regressions passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 14656 checks. Material passed 227 steps and 14031 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45323151154) · [Browser job](https://github.com/faberline/surface/actions/runs/45323151154/job/138206241643)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45323141103)
- [Linux report](web/hosted-scroll-pair.json) · [Linux screenshot](web/hosted-scroll-pair.png)
