# Surface v0.1.35 release verification

Main commit `212e52b128784200993b38c2a4cded0e82bf9331`. The tree matches PR head `cc3d4fa382ac56d73a4b3043697138914ec699a5`.
Both local source manifests verified against the released tree.

- Workspace provenance, format, Clippy and wasm32 checks passed. 1145 active Rust tests passed.
- Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.
- Chrome/157.0.8103.79 used Apple Metal 3. Five headless cases passed 13,094 checks. Material passed 203 steps and 12,846 checks.
- Nine offscreen Metal tests passed. The loading indicator test measured all seven silhouettes and an intermediate paint frame.
- The macOS workspace check used `DEVELOPER_DIR=/Library/Developer/CommandLineTools` and `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` in its test process.
- Linux PR browser CI passed eight cases and 13471 checks. Material passed 203 steps and 12846 checks.
  [PR CI](https://github.com/faberline/surface/actions/runs/45238698007) · [Browser job](https://github.com/faberline/surface/actions/runs/45238698007/job/137917185115)
- Branch CI passed at the same head. [Branch CI](https://github.com/faberline/surface/actions/runs/45238681599)
- [Linux report](web/hosted-loading.json) · [Linux screenshot](web/hosted-loading.png)
