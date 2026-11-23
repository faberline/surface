# Surface v0.1.21 release verification

Main commit `b9fcf9443a95f42db7b67b0132b2374fa5d3b27e`.
The source tree matches PR head `7213b7432751c82ef615cfe9dee8ce172d0c686b` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, fmt, Clippy and wasm32 checks passed. All 961 active Rust tests passed. Manual clipboard checks did not run.
- Native headless Metal reproduced 4 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 7,575 checks. Material passed 132 steps and 7,327 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 7952 total and 7327 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45105488819) · [Browser job](https://github.com/faberline/surface/actions/runs/45105488819/job/137465674236)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45105483159)
  [Linux gesture report](web/hosted-date-picker-gestures.json) · [Linux compact swipe](web/hosted-compact-swipe.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1` after an earlier loopback test did not exit.
