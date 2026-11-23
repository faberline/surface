# Surface v0.1.26 release verification

Main commit `9fac6a0f7995f788d54757cf6a3a033d45585671`.
The source tree matches PR head `117db94b97a57a59f9cbdf71163706063ac14302` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1066 active Rust tests passed.
- Native headless Metal reproduced 7 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 9,427 checks. Material passed 159 steps and 9,179 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 9804 total and 9179 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45136834177) · [Browser job](https://github.com/faberline/surface/actions/runs/45136834177/job/137565017490)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45136826052)
  [Linux Baseline time-picker report](web/hosted-time-baseline.json) · [Linux headless capture](web/hosted-time-baseline.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.
