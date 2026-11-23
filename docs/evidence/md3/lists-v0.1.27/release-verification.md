# Surface v0.1.27 release verification

Main commit `18575735454e4a175e3a2af43d34c24eda4a78f8`.
The source tree matches PR head `beb82c6f844e10f09b16c600d9d6d9b9a8dbeb5c` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1080 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 9,815 checks. Material passed 165 steps and 9,567 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 10192 total and 9567 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45144198803) · [Browser job](https://github.com/faberline/surface/actions/runs/45144198803/job/137589312873)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45144181206)
  [Linux Lists report](web/hosted-lists.json) · [Linux headless capture](web/hosted-lists.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
