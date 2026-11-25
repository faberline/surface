# Surface v0.1.66 textbox-value candidate proof

This archive records local verification of the exact candidate tree
3de0c143c43d35911593cfc95d3ea5931016c308, with 681 source files and an eight-path patch.

- The exact workspace passed 1,561 Rust tests and three WebAssembly
  checks. The GPU gate ran 18 tests on Apple Metal.
- The exact native Gallery replay returned 14 original PNGs in two
  runs. All match the prior v0.1.65 captures. Page.screenshot, CLI and
  MCP matched. The guard saw zero visible windows across 24 samples.
  The separate native development preview supplies four matching PNGs.
- The paired Chrome probe found the six inactive values missing in
  v0.1.65. The same six values match exactly after the v0.1.66 change,
  including whitespace, a combining accent and a terminal newline.
- The focused run passed 308 checks from M341 through M346. It keeps
  the 12 original PNGs, six inactive accessibility values, and the
  active textarea handoff. All four Gallery Web files match stock.
- The exact stock run passed 24,750 checks in five
  headless macOS suites. It ran 345 Material steps through M346, the
  retained I11 regressions, and verified the six inactive values
  including the single-line field. The guard found zero visible
  windows and no surviving processes.

The accessibility receipts cover the Chromium tree. Spoken output
requires a separate screen-reader test.

## Offline recheck

Use Python 3.12 with Pillow. Supply the reviewed manifest digest.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-textbox-values.py --verify-package . --package-sha "$manifest_sha"
~~~

The gzip files reproduce the original patch and log bytes. This
command reads the original evidence and starts no application.
