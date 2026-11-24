# Surface v0.1.43 Search native motion proof

The 557-file stock source seal has SHA-256 `4452974627bf7d8e9d54518a3b114b7c37dd7c94d9f1cd6e7f69ed62f3e1ffd9`. The diagnostic changes only the scratch offscreen host through [clock.patch](clock.patch). A private authenticated DevTools method sets the test clock. Native Metal paints the actual Gallery in both runs. The offscreen host creates no window.

| Diagnostic artifact | SHA-256 |
| --- | --- |
| Source manifest | `e7c41f159ca047909ff418bb2259d6c66282a96b0cbc18d505208bf9ec96ce6e` |
| Clock patch | `4ab28ef4250320850b5b4e7b7ffd3b938018a01f8d116df9f57f55762deda0be` |
| Binary manifest | `192c138eabd6f78c8c034c754a1a846fbaefad5bcda67d24f5fc1c519cff7237` |
| Motion report | `f51664c2793ade4f741b13439b875d5b844b709d3750f4cd28aac7a46c728b4c` |
| Repeat manifest | `2355c0cf3ca56ab68e7370e6fa20c23aad3fde7fb8c9c7dbb813b77303b3d6f3` |

Two independent diagnostic runs produced eight PNGs and a report with identical bytes. Each image matched across Page.screenshot, CLI and MCP. The light docked LTR and dark compact RTL 200% profiles cover opening at 0 and 60 ms, settling at 1000 ms, and an inert exit 60 ms after Escape. Focus stays on the editor during opening and returns to the opener during closing. Both profiles kept visible aligned hint and caret pixels at all three opening samples. All 24 diagnostic visible-window checks returned zero.

The 60 ms docked result height is 187 dp with scrim alpha 41. The light editor has a 3 px hint gap and a 26 px visible caret segment. The 320×400 compact editor has an 8 px gap and a 36 px visible caret segment. The exit geometry and all eight PNG bytes also match the prior integration diagnostic. The prior stock frames serve only as the initial settle reference.

## Exact stock confirmation

Neither stock binary was rebuilt for this check. The supplied stock pins matched the release build's manifest before and after capture. Its two settled frames matched the diagnostic settled PNGs and editor ink bounds exactly. The stock host rejected `Test.setClock`. Two further window checks returned zero, for 26 zero-window checks across both builds.

| Stock confirmation | SHA-256 |
| --- | --- |
| Upstream binary manifest | `bcb1c1dadc6781c4770a04bd017c34869f110d18183cd7754d5339a181cfd2de` |
| Direct stock-to-motion comparison | `8a1e8925fa077d3c1f76a8412b0281209ed87dd8caad16ef1aa9b9167118d6f5` |
| Stock capture report | `757692737ffc8e58605b96fada0710746d33574200158db4ae0408e056da6e46` |

The stock report and both PNGs are in [exact-stock/stock](exact-stock/stock), and the byte comparison is in [exact-stock/exact-stock-comparison.json](exact-stock/exact-stock-comparison.json). Diagnostic evidence is in [exact-motion/motion-1](exact-motion/motion-1) and [exact-motion/motion-2](exact-motion/motion-2). This evidence covers native Metal pixels and semantic keyboard actions in the no-window host. Real browser interaction and live system input methods are separate checks.

## Reproduce

Use a fresh Cargo target to build a private copy of the sealed source with `clock.patch` applied. Run `cap cargo build --locked -p surface-material-gallery -p surface-devtools --features surface-material-gallery/devtools` with `CARGO_BUILD_JOBS=2`. Put the binaries and hash receipt in `pins`.

Run `PYTHON_BIN=/opt/homebrew/bin/python3.12 sh replay-motion-pair.sh /tmp/surface-md3-search-combined-clock.HTLZX5E0/stock-1 exact-motion` in a fresh diagnostic evidence directory. The path `exact-motion` must be new. Keep that run under `exact-motion` when verifying the exact stock pin. This runner verifies its source and binary digests before and after.

Run `python3 native-scripts/verify-v0143-stock.py --pin-stage STOCK_STAGE --out exact-stock` from this directory, with the prepared stock stage from the release build. It verifies all 557 per-path source hashes and the Gallery/CLI digests before capture. It compares the exact settled bytes and editor pixels. Stock and diagnostic binaries remain separate.
