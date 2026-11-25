# Surface v0.1.53 release verification

Main commit `3913703a699289229088f25aac2092565b8fd2d2` and candidate `5742b68640e482a97d7382a8e5333da0b1299145` share tree `c694ddfc095f5cc18e3c27b99f7d02985b49abbc`.
The exact 628-file source manifest is `b957fec84eb0c5f3574df79c9d7756d1263f7667d0ac0d3b0c3ec8915d49775f`. It verified against the released checkout.

## Verification

- 1399 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 3 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 21241 checks and verified four Icon Buttons PNGs.
- Eight stock Linux cases passed 21618 checks on the PR and 21618 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/fe557622a5614d6fdb16af5af564873296ed9b42/docs/evidence/md3/icon-buttons-v0153-candidate)

Icon Buttons coverage stays partial. System contrast and live screen-reader speech remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45525793286) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45525781046)
