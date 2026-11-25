# Surface v0.1.52 release verification

Main commit `05ad50a57be8491d619182400d915e2814027c2d` and candidate `a01bbaa71bce622603239dee9abcd98951306827` share tree `48d560ab127cc1c692a5ba0eba6fb5a8949234a8`.
The exact 624-file source manifest is `8d2548019eca22046b20992b27f48f4404cb0f5af84f26d6d864d6de498509b1`. It verified against the released checkout.

## Verification

- 1390 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 16 offscreen Metal cases ran.
- Two native headless replays each produced two matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 20949 checks and verified four Chips PNGs.
- Eight stock Linux cases passed 21326 checks on the PR and 21326 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/bf47e8e996f092d82f52e91ad3feee10e4e94592/docs/evidence/md3/chips-v0152-candidate)

Chips coverage stays partial. Live screen-reader speech, system high contrast and optional drag reordering remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45520724951) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45520711073)
