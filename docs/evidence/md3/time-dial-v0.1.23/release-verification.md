# Surface v0.1.23 release verification

Main commit `70c9453751f8bfc595d49382ff9c6f7db4314f89`.
The source tree matches PR head `4117de94f29e074cd923888373c6238b215e1385` exactly.
Both local source manifests verified against the release tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1009 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs and the report in two separate runs. Page.screenshot, CLI and MCP agreed. CoreGraphics reported zero visible Gallery windows.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 8,220 checks. Material passed 143 steps and 7,972 checks.
- Five offscreen Metal renderer tests passed.

- PR build and behavior checks passed. PR browser CI passed eight cases.
  Checks: 8597 total and 7972 Material. The DevTools headless case passed 76 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45114941665) · [Browser job](https://github.com/faberline/surface/actions/runs/45114941665/job/137495530760)
- The branch CI also passed at the same source head. [Branch CI](https://github.com/faberline/surface/actions/runs/45114936456)
  [Linux time dial report](web/hosted-time-dial.json) · [Linux headless capture](web/hosted-time-dial.png)
- The exact local workspace check passed with `RUST_TEST_THREADS=1`.

The current Hour and Minute controls support arrow-key entry at the current value and exact one-step movement on the face. Material's Enter navigation to the clock face is still open. Its starting marks are hour 12 in 12-hour format, hour 00 in 24-hour format, and minute 00. Animated selection movement is also open.
