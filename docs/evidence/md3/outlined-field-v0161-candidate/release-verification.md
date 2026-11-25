# Surface v0.1.61 release verification

Main commit `00d9a97c83e8245fe857f8a03f24f9a1a4a0db3f` and candidate `c6d7d06861d3f6221d06c1c6dd682692fa1984a5` share tree `7c7881f78c3dc44129f52688a079b6874c9a4991`.
The exact 668-file source manifest is `3c79ae5ee15bf3b9b786d37452f1e0e5929ac0d31276ca7f6506897462dd9cc8`. It verified against the released checkout.

## Verification

- 1528 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 4 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 23656 checks and verified 5 Outlined Text Field R2 PNGs.
- Eight stock Linux cases passed 24033 checks on the PR and 24033 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/eba8429b2f390ef905286fe2ccf7f3203897e979/docs/evidence/md3/outlined-field-v0161-candidate)

This release covers the single-line Outlined text field. The composition evidence uses Chrome CDP and trusted Enter. Live assistive speech and operating-system candidate selection remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45569894616) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45569881486)
