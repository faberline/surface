# Surface v0.1.46 verified command menu

Start from the Divider v0.1.45 source. Verify `divider-base.sha256`, apply
`surface-v0.1.46.patch` using `git apply --whitespace=error-all`, and run
`shasum -a 256 -c source.sha256`. A separate copy reproduced all 590 files.
This candidate includes the Divider endpoint sampling correction.

- 1308 Rust tests passed with formatting, provenance, Clippy and WebAssembly checks. [Workspace log](stock-mac/workspace.log)
- Sixteen offscreen Metal cases ran. [Metal log](stock-mac/gpu.log)
- Two native headless replays each captured seven matching PNGs through Page.screenshot, CLI and MCP. [Native proof](native)
- Five stock macOS headless suites passed 18673 checks. The Menu verifier decoded all three PNGs. CoreGraphics observed zero visible windows. [Browser results](stock-mac/verified-browser.json) · [Menu verification](stock-mac/independent-menu-check.json)
