# Surface v0.1.65 release verification

Main commit `b40bc212fb32ac97b3f207cdfd002814bf48db66` and candidate `00a0ef834bc7b975b36f611c384c962ffcc10d86` share tree `2341dc506a206fe1e892c35a32d51386702958b3`.
The exact 681-file source manifest is `6a420b56fc090457f9841fc8fda7c1bf0602e27138f586ca3f6b251652b580f2`. It verified against the released checkout.

## Verification

- 1561 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 7 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 24689 checks and verified 12 Multiline Text Field PNGs.
- Eight stock Linux cases passed 25066 checks on the PR and 25066 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/b61875ff57bdfde3c8cb384ed0134ea9ea4a939e/docs/evidence/md3/multiline-text-field-v0165-candidate)

This release adds growing and fixed-row multiline Filled and Outlined fields. The browser evidence covers trusted keyboard, wheel and touch events, simulated composition and inspected accessibility. Live assistive speech and operating-system candidate selection remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45600098360) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45600084496)
