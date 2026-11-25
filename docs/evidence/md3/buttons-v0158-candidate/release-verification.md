# Surface v0.1.58 release verification

Main commit `454fa53e33bf8a86be79ce24a757a5f45666e4c2` and candidate `72e0ed4075e47ec103b461022cabc15cea77adaf` share tree `8ba487496a2816f3bff3363328ba0dc40a059c30`.
The exact 656-file source manifest is `b03623198811629a68a2dfe5620d5705c55ea7744157ff2ac732d174833eedbf`. It verified against the released checkout.

## Verification

- 1492 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 5 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 23004 checks and verified 7 Common Buttons PNGs.
- Eight stock Linux cases passed 23381 checks on the PR and 23381 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/65d5d71fe188104f76e0ec8282eaed5e6889814b/docs/evidence/md3/buttons-v0158-candidate)

This release covers standalone common buttons. Wider Material accessibility coverage, live assistive speech and physical system accessibility settings remain outside this evidence.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45549556601) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45549546176)
