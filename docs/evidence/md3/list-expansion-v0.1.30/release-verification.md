# Surface v0.1.30 release verification

Main commit `e206353e491d241fb67455e74d60a6bb4dd21eb7`. The tree matches PR head `67fa60d6b3912365645c54d70b408cb8e1a50d49`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1106 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 11,050 checks. Material passed 180 steps and 10,802 checks.
- Five offscreen Metal renderer tests passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 11427 checks. Material passed 180 steps and 10802 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45171318042) · [Browser job](https://github.com/faberline/surface/actions/runs/45171318042/job/137681289419)
- Branch CI passed at the same head after one retry of its browser job. The initial attempt timed out while opening the existing narrow dialog in M20. [Initial attempt](https://github.com/faberline/surface/actions/runs/45171306957/attempts/1) · [Branch CI](https://github.com/faberline/surface/actions/runs/45171306957)
- [Linux report](web/hosted-list-expansion.json) · [Linux screenshot](web/hosted-list-expansion.png)
