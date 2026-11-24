# Surface v0.1.47 exact local navigation bar gates

Start with the Menu v0.1.46 source export. Verify `menu-base.sha256`, then
apply `surface-v0.1.47.patch` with `git apply --whitespace=error-all`.
Run `shasum -a 256 -c source.sha256` in the resulting tree. A second copy
already reproduced all 594 source files.

- 1317 Rust tests passed with provenance, formatting, Clippy and WebAssembly checks. [Workspace log](workspace.log)
- Sixteen offscreen Metal cases ran. [Metal log](gpu.log)
- Two native headless replays each captured two matching PNGs through Page.screenshot, CLI and MCP. [Native proof](native)
- The five macOS headless Chrome suites passed 19131 checks. The verifier decoded four navigation screenshots and confirmed the title, focus and bar remained fixed during page scroll. CoreGraphics found zero visible windows. [Results](stock/verified-navigation.json)

Read the [preflight](../flexible-navigation-bar-preflight) for the official design references and the earlier title clip regression. The full hosted Linux runs use the separate exact PR and branch receipts.
