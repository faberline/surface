# Surface v0.1.29 release verification

Main commit `408c58d944758f53a458b0c0433691076b1ef58e`.
The source tree matches PR head `a6b36bad94c43cea4ccfa995166a8bbd049154ff` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1097 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 10,636 checks. Material passed 176 steps and 10,388 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 11013 total and 10388 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45161969993) · [Browser job](https://github.com/faberline/surface/actions/runs/45161969993/job/137649361410)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45161960735)
  [Linux Lists report](web/hosted-list-actions.json) · [Linux headless capture](web/hosted-list-actions.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
