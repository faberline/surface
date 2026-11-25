# Surface v0.1.63 release verification

Main commit `a8933082a0141198bb0c0452216a570325693aca` and candidate `b33fbaf63b147e1fedfb05c26a41f0279937ba65` share tree `3e2f280ed159c28f5b939813c4d4fd88d26803ff`.
The exact 675-file source manifest is `2d671c1f3c4b450a21ab728a22ba0f8126af4790291a1b718d9005ab4f29eec2`. It verified against the released checkout.

## Verification

- 1540 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 6 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 24134 checks and verified 6 Text Field Adornments PNGs.
- Eight stock Linux cases passed 24511 checks on the PR and 24511 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/42863b52510bf56e0ac0eb710ec63a5f9b291477/docs/evidence/md3/adorned-field-v0163-candidate)

This release adds adornments to the single-line Filled and Outlined fields. The composition evidence uses Chrome CDP and trusted input as well as native headless input. Live assistive speech and operating-system candidate selection remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45586958568) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45586940228)
