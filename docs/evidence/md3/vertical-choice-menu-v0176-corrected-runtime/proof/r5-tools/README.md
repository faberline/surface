# Surface v0.1.76 R5 stock browser replay

The R5 runner changes one browser test file. The patch samples the logical
leading corner of an RTL row. Its old trailing sample fell on the scrollbar.
The R5 source tree is `b6dc18c4baf8e6b65bddfbc6f59b7b07b4a0b5b9`.
The product and Web producer use tree
`37e4454bc7d3f63f7111c8264d4e59e451c8ef9e`. Preflight checks both 703-file
source trees, the exact one-file patch bytes, the rebuilt R5 runner and its
original build log, and the unchanged R4 Web bundle.

```sh
lane=/tmp/surface-md3-choice-menu-v0176-r5-guard.7wvCho7W
run=/tmp/surface-md3-choice-menu-v0176-browser-corner.3ixT0mHe/guarded-stock-material
sh "$lane/run-guarded.sh" --check-inputs
sh "$lane/run-guarded.sh" --run "$run"
sh "$lane/run-guarded.sh" --verify "$run"
```

The same stock guard checks for one private headless Chrome launch, zero
visible owned windows and no surviving processes. A passing run must
record all 374 ordered steps through M375, with M23 retired. M373 must
show that a real mouse click on the disabled owner row leaves the menu
open and preserves focus and selection. M375 must show a dark rounded
leading corner, the selected fill, scrolling, the full wrapped label,
toggling and Escape focus. The offline reader also decodes the original
640×1048 compact PNG and checks all three leading, inner and center samples.

The terminal output uses the existing `verified-stock-material.json`
name. Its `menus` section adds the independent M375 receipt alongside
the M373 disabled-row receipt. The runner invocation, guard ledger,
headless ownership report, exact source/build pins, and original reports
and PNGs form the complete run evidence.

The exact build-input check and offline fixtures passed. The R5 stock
terminal evidence comes from a fresh controller run. The R4 native
receipt has its own product source and binary pins.
