# Surface v0.1.55 release verification

Main commit `648f48dc4bb126e63ead17a88489d2d0a1c730a5` and candidate `97290b617955ab2c84de1594e1ce02f9202f059b` share tree `18bf4098eefd03ff562e058ceb5d7f99a34447a2`.
The exact 640-file source manifest is `93fbdb8c80cfaf7698ae702a8bbc155b15411f2fe427048e1040100a672fc2d4`. It verified against the released checkout.

## Verification

- 1439 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 5 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 21745 checks and verified four Slider Forms PNGs.
- Eight stock Linux cases passed 22122 checks on the PR and 22122 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/a8c41f3ada057121961a52a7535dab8fa656161c/docs/evidence/md3/slider-forms-v0155-candidate)

Slider coverage stays partial. Vertical Centered, two-thumb Range, Standard inset icons, and live assistive speech have separate gates.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45535423703) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45535411013)
