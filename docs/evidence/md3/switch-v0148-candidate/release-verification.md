# Surface v0.1.48 release verification

Main commit `dc85a3714fc11c1708155c48f23cfa6c637b3498` and candidate `1346beff8a2dfcc124e0cde4d78c4d91608d46a8` share tree `4b1a812f2d384adeede0e1619dc0cfc9c1e7c960`.
The exact 598-file source manifest is `a4ec5bc9d9c0da42fe004f6f7d6877f59edc08e5503ae15c3bbefb6c73f4d086`. It verified against the released checkout.

## Verification

- 1326 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 16 offscreen Metal cases ran.
- Two native headless replays each produced two matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 19486 checks and verified four Switch PNGs.
- Eight stock Linux cases passed 19863 checks on the PR and 19863 on the branch with Chrome SwiftShader.
- The corrected browser test checks canvas focus after a pointer click. The keyboard test checks projected focus. The original failing run remains in the evidence.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/559029c799a829e084f8faac6d36e5f217538600/docs/evidence/md3/switch-v0148-candidate)

Drag selection, live screen-reader speech, and system high contrast remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45499168058) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45499148999)
