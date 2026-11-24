# Surface Search integrated headless verification

The same pinned Chrome test failed before the inactive input host repair and passed afterward. Before the repair, the test found the hidden input host in the browser accessibility tree after Search closed. It still had the name 搜尋文件. After the repair, the inactive host has no layout box or accessible textbox. Reopening uses the same host with the current name and browser focus. [Failure trace](web/exit-before.json) · [Repaired run](web/search-after.json) · [Verified receipt](web/verified-search.json)

Chrome 158 headless used Apple Metal 3 for WebGPU. Search M248 through M255 passed eight steps and 1,172 checks with zero page errors. Showcase I1 through I20 passed 20 steps and 57 checks. Search checks include both style and layout pairs, the 599 to 600 dp resize with a pending composition, pointer selection, caret visibility at both logical starts, Chrome forced colors, and entering and closing with and without Reduce Motion. [Search log](web/run-repaired-final.log) · [Showcase log](web/run-showcase-final.log) · [RTL caret](web/caret-rtl-divided.png)

The integrated native diagnostic produced eight identical PNGs and identical reports in two independent runs. Both settled frames matched the separately built stock Gallery. At 60 ms, the light Docked frame showed 187 dp of results and a distinct caret. The dark RTL 200% full-screen frame also kept the hint and caret on the right. Closing restored focus while the shell still painted. There were 26 zero-window observations. Page.screenshot, CLI and MCP agreed on image bytes and metadata. [Native report](native/native-search-motion-report.json) · [Native frames](native)

These checks exercise synthetic composition, the Chrome accessibility tree and offscreen rendering. Real system input methods and spoken screen-reader output remain open.

## Source and replay

Start from source `ab8615c8495d99a652f4fac38d4db37f08c82c7f` in a scratch copy. Apply the [assembled preflight patch](../preflight/source/assembled.patch) and verify the [555-file manifest](../preflight/source/assembled.sha256).

- Add [the input host repair](source/web-inactive-host.patch) for the [browser replay source](source/web-inactive-host.sha256). The native replay above uses the assembled preflight source and a [clock overlay](source/integrated-clock.patch) in a diagnostic-only copy. The stock binary did not expose that clock.
- Add [the remaining browser cases](source/combined-search-chrome-e2e.patch) and [the component guide and coverage update](source/search-docs.patch). The result matches the [unversioned manifest](source/unversioned.sha256).
- Add [the v0.1.43 version patch](source/version-v0143.patch) to obtain the [staged release source](source/v0143-source.sha256). These integration results precede the full release checks.

Use the [headless replay commands](../preflight/README.md#replay) and a fresh Cargo target for each source. Run `shasum -a 256 -c SHA256SUMS` here to check the archived inputs and outputs.
