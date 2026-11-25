# Surface v0.1.50 release verification

Main commit `fceb6a45b10eca20d94fb04e0f77ec949407d3ac` and candidate `1b2938759053adac90663f618a7c6b2dcc79aa4d` share tree `4966df4360616d1156d5227aba72b5b6228091cd`.
The exact 607-file source manifest is `a031c8b5dc2700f69cb131182842722976d57726dd01c2e0a2d99e643e406544`. It verified against the released checkout.

## Verification

- 1349 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 16 offscreen Metal cases ran.
- Two native headless replays each produced two matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 20293 checks and verified four Radio PNGs.
- Eight stock Linux cases passed 20670 checks on the PR and 20670 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/19c572612cc8e5b6660f83f308fdc3c4943b0ab6/docs/evidence/md3/radio-v0150-candidate)

Live screen-reader speech and platform high contrast remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45510067863) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45510049248)
