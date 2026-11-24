# Surface v0.1.47 release verification

Main commit `7526fac5d0546b17afe7110d923b8d490a5e6484` and candidate `6b86ea2aaf7e9ce94d9fb563bca09b2f861992b6` share tree `a07a1a636ab1082f2b30c43fe2148b036e6e36f1`.
The exact 594-file source manifest is `1b4c1cc1acc3719c5064fd678f91cf99a98c0dc8d1c2204b8d77eb08e732ac76`. It verified against the released checkout.

## Verification

- 1317 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 16 offscreen Metal cases ran.
- Two native headless replays each produced two matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 19131 checks and verified four Navigation PNGs.
- Eight stock Linux cases passed 19508 checks on the PR and 19508 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/aae8b6f1f47060f3f6fc2d988dc881a739defab6/docs/evidence/md3/flexible-navigation-bar-v0147-candidate)

Coverage is limited to the compact vertical variant. Live screen-reader speech and native settings remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45492207942) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45492187763)
