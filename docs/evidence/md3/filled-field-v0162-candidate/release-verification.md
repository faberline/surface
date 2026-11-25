# Surface v0.1.62 release verification

Main commit `288218dbe3461bd9465139befd3a4471a9191df6` and candidate `d51897142953740795b35d7517b15caa50f852dc` share tree `7266ce0588d5bc2d83a4151ca2e13c780153e279`.
The exact 670-file source manifest is `78ae815e1c6407c91c65b13219e07b2c6f2298eb5218530670fad26f70a3dabe`. It verified against the released checkout.

## Verification

- 1531 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 4 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 23891 checks and verified 6 Filled Text Field PNGs.
- Eight stock Linux cases passed 24268 checks on the PR and 24268 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/daab50af8f9c36d559ad1e52bf39be66b9fdeba5/docs/evidence/md3/filled-field-v0162-candidate)

This release covers the single-line Filled text field and retains the Outlined variant. The composition evidence uses Chrome CDP and trusted Enter. Live assistive speech and operating-system candidate selection remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45580940619) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45580923600)
