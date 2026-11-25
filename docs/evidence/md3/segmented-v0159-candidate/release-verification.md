# Surface v0.1.59 release verification

Main commit `b57bd90f1a6945a973e142cbaffa1ce9569b0310` and candidate `1d50bb918a8b9f244b01750445249858017d2e4d` share tree `757179e260a331b4488c6be1b49410263097d675`.
The exact 660-file source manifest is `9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8`. It verified against the released checkout.

## Verification

- 1502 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 4 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 23315 checks and verified 5 Baseline Segmented Button PNGs.
- Eight stock Linux cases passed 23692 checks on the PR and 23692 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/e18d16e38aaf8b26522bac34382dbf5e16394386/docs/evidence/md3/segmented-v0159-candidate)

This release covers baseline segmented buttons. The follow-up touch ownership slice and live assistive speech remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45558476857) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45558464958)
