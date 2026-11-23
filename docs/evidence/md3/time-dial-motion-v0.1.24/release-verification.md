# Surface v0.1.24 release verification

Main commit `19b48eab43ce1b472b8438fab02063877f8c84e4`.
The source tree matches PR head `fd60de1a7f7c21d683bd2e379c4d9aa02e6ea1ba` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1022 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 8,524 checks. Material passed 147 steps and 8,276 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 8901 total and 8276 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45122291212) · [Browser job](https://github.com/faberline/surface/actions/runs/45122291212/job/137518593101)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45122284907)
  [Linux dial keys and motion report](web/hosted-time-dial-motion.json) · [Linux headless capture](web/hosted-time-dial-motion.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
