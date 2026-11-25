# Surface v0.1.64 release verification

Main commit `fa64f5bd7119b5f0c3245e9ce4c24b0fbc70e1bf` and candidate `daf04cbedb78aff4778813b447f54e8e80396ab6` share tree `eeef2dc049409f7df3a67773d8e72e76b298faf6`.
The exact 678-file source manifest is `fe7349b293de3078ca44fd6b1fa1885a5fa156ba559503c795cd9d6daab5e4a3`. It verified against the released checkout.

## Verification

- 1543 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 7 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 24442 checks and verified 7 Text Field Character Counter PNGs.
- Eight stock Linux cases passed 24819 checks on the PR and 24819 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/8eb75eba19191584a00ab9a33e59bf627de2512c/docs/evidence/md3/text-field-counter-v0164-candidate)

This release adds a soft grapheme counter, a related accessible description and overflow validation to single-line Filled and Outlined fields. The composition evidence uses Chrome CDP and trusted input. Live assistive speech and operating-system candidate selection remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45593154016) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45593139339)
