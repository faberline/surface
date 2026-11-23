# Surface v0.1.31 release verification

Main commit `28106f254bf686587bf239fb7da59560fcd164fa`. The tree matches PR head `3bdde947590499fb2e5b23278b523aaad410c201`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1112 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI, and MCP returned identical PNGs.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 11,482 checks. Material passed 185 steps and 11,234 checks.
- Five offscreen Metal renderer tests passed.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 11859 checks. Material passed 185 steps and 11234 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45184577406) · [Browser job](https://github.com/faberline/surface/actions/runs/45184577406/job/137726738024)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45184560348)
- [Linux report](web/hosted-list-reorder.json) · [Linux screenshot](web/hosted-list-reorder.png)
