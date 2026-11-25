# Surface v0.1.60 release verification

Main commit `98c62d31d1646878b1a5393f84aa896e52aa2985` and candidate `2797abe42104ef73e667d2efe4dde35f1cc83778` share tree `b9539a68d5993b8c29a9dc17da571b31078c6874`.
The exact 662-file source manifest is `4874c29831d6a87a60c1e3b98071cc522d2ab9a933deab60b6f33091bcfd25b9`. It verified against the released checkout.

## Verification

- 1515 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 4 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 23478 checks and verified 7 Segmented Button touch ownership PNGs.
- Eight stock Linux cases passed 23855 checks on the PR and 23855 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/65bbfe891edcfd4ee18b721db67d6e62641d33e9/docs/evidence/md3/segmented-touch-v0160-candidate)

This release covers segmented-button touch ownership in the browser. The proof also includes native headless keyboard and screenshot replay. Live assistive speech remains outside this evidence.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45563988472) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45563977813)
