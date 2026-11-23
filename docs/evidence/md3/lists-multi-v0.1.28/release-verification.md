# Surface v0.1.28 release verification

Main commit `aa0cc1290068a0022fe5d98ac9c7091f3cbf1da2`.
The source tree matches PR head `ca12957dc6767ac23dee54e4e3425891024363e2` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1087 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 10,181 checks. Material passed 170 steps and 9,933 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 10558 total and 9933 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45152741688) · [Browser job](https://github.com/faberline/surface/actions/runs/45152741688/job/137617944134)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45152731373)
  [Linux Lists report](web/hosted-lists-multi.json) · [Linux headless capture](web/hosted-lists-multi.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
