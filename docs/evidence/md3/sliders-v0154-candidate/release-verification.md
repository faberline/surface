# Surface v0.1.54 release verification

Main commit `ae898054cd8412d7cdd09a88166bbf39c7a1a7af` and candidate `1d5d47994d256e1b7486052e1fe7feafd51ddac3` share tree `a92d66c4998656b1749f322723aa4324028afdc4`.
The exact 635-file source manifest is `1e1c1be43dbd652ff26dbaab2f264651c2c811915bc04e41dfbedd425ba830d8`. It verified against the released checkout.

## Verification

- 1421 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 4 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 21459 checks and verified four Horizontal Standard Slider PNGs.
- Eight stock Linux cases passed 21836 checks on the PR and 21836 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/4f39a60c7fce497bd7d474fb601a63e5a084085f/docs/evidence/md3/sliders-v0154-candidate)

Slider coverage stays partial. Centered, vertical, Range, and inset icons remain open. Live screen-reader speech and the optional system clipboard test did not run.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45530199487) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45530187217)
