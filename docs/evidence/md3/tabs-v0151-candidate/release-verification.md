# Surface v0.1.51 release verification

Main commit `de11edf089a273c794875d54597a08e4ff1d9c0f` and candidate `e8163ad77a063adbbcc6df4fae99eb9cb9b1c406` share tree `187930ac394178c821649c76b0f29c2f9c751b9e`.
The exact 616-file source manifest is `f928486f4fd04824a503424b0ebf53caa5b2784ba363dae928c7fca17c202cd0`. It verified against the released checkout.

## Verification

- 1368 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 16 offscreen Metal cases ran.
- Two native headless replays each produced two matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 20664 checks and verified four Tabs PNGs.
- Eight stock Linux cases passed 21041 checks on the PR and 21041 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/8f5831055d0e413070180874def608c8b6727ff7/docs/evidence/md3/tabs-v0151-candidate)

Tabs coverage stays partial. Optional badges, page swipe, system high contrast and live screen-reader speech remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45515434800) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45515420949)
