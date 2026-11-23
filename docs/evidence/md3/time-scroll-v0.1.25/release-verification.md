# Surface v0.1.25 release verification

Main commit `6191afb45b8c484d90359697b87815361fa900ef`.
The source tree matches PR head `0cd2973edf032817eb550fc2aba8ff8637165788` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1055 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 8,882 checks. Material passed 153 steps and 8,634 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 9259 total and 8634 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45129425581) · [Browser job](https://github.com/faberline/surface/actions/runs/45129425581/job/137541252350)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45129256367)
  [Linux Scroll time-picker report](web/hosted-time-scroll.json) · [Linux headless capture](web/hosted-time-scroll.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
