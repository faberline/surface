# Surface v0.1.57 release verification

Main commit `074186f2268601f721df4fa24b209af930ef6416` and candidate `644d1e5faa1c25241306423c709ef58ca4c936be` share tree `72cf21a76d284e3a299aedff91b953eae232db07`.
The exact 652-file source manifest is `5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1`. It verified against the released checkout.

## Verification

- 1481 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 8 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 22606 checks and verified 7 Slider Refinements PNGs.
- Eight stock Linux cases passed 22983 checks on the PR and 22983 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/e44e6ebb009292c1822c70fd9b6fd5b97ae69119/docs/evidence/md3/slider-refinements-v0157-candidate)

Slider coverage stays partial. Remaining work includes the broader Material accessibility matrix, live assistive speech, and physical system accessibility settings.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45544464596) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45544453586)
