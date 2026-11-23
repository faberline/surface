# Surface v0.1.22 release verification

Main commit `31974c893f2f14d490ad536d1c45e2cb7795b90e`.
The source tree matches PR head `e73cd2809859fa9277b4b84361f873abeea86468` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. All 978 active Rust tests passed.
- Native headless Metal reproduced 5 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 7,932 checks. Material passed 137 steps and 7,684 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 8309 total and 7684 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45109535047) · [Browser job](https://github.com/faberline/surface/actions/runs/45109535047/job/137478471065)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45109528814)
  [Linux time input report](web/hosted-time-input.json) · [Linux headless capture](web/hosted-time-input.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
