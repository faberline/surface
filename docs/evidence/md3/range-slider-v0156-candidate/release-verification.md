# Surface v0.1.56 release verification

Main commit `87dbf51ca5d552c6ad929b6fce5ee9439bc320a0` and candidate `40c5a5f0aa24bb2bae748c53d96039c8af962255` share tree `6e7c09a6eda249eb046babf3f2a4b392b0aaa99c`.
The exact 646-file source manifest is `de92395ee2ab7b8d7d49760adb2722d5b44ca270c2002deb6fefbecd809e8647`. It verified against the released checkout.

## Verification

- 1464 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 18 offscreen Metal cases ran.
- Two native headless replays each produced 5 matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 22116 checks and verified four Range Slider PNGs.
- Eight stock Linux cases passed 22493 checks on the PR and 22493 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/af7aab9cb3de3a6146cbf19517697d2ba8e8cf00/docs/evidence/md3/range-slider-v0156-candidate)

Slider coverage stays partial. Vertical Centered, Standard inset icons, live assistive speech, and physical system accessibility settings have separate gates.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45539652578) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45539639588)
