# Surface v0.1.44 Dialog resize candidate

Source commit `9b8e02b8668eba8da49c1fdcdf1832dee05865fb` has tree
`50c3a5646a033b04280f7050748ca92c42154ad5`. Start from the previous
candidate `9c841f6e0654cc43ab778d6660994ad6b63fa28c` and apply
`git apply --whitespace=error-all remeasure-fab.patch`. Run
`shasum -a 256 -c source.sha256` in the resulting source directory.
The 579-file manifest SHA-256 is
`f69a406b6b86be6a0f24fb951a413b89312a729c5275862446912ba2039be161`.

The earlier Linux branch run found the FAB at the right place but still
reported its old animation origin. The Gallery now discards this
measurement when the viewport or direction changes. It requests the
frame that can read the new layout, even with Reduce Motion enabled.
The unmodified browser check still waits for the measured origin to
match the drawn FAB.
[Prior Linux receipts](https://github.com/faberline/surface/tree/adb1901a8dbce236e5d74bdb2c1a670dc4cbf27d/docs/evidence/md3/dialog-adaptive/v0144-hosted-resize-failure)

The focused regression first failed on the old Gallery. The fixed
Gallery passed all 28 local tests. Two native headless replays then
produced 14 PNGs each. Each PNG matched across the screenshot API, CLI
and MCP. None of the 44 native window samples found a visible window.
[Regression fail](resize-before.log) · [Gallery pass](resize-after.log) · [Native verification and replay](native/README.md)

This snapshot records the local regression and native replays. It does
not record the new hosted Linux or the full macOS browser results.
