# Material Typography Bold verification

Source `50c84895b631114df5773769ff69b01d00a7859b` uses the bundled Noto Sans CJK TC 700 face for Bold. The 400 face remains the source for Regular and the current 500 fallback. The Gallery compares one English and Traditional Chinese sample at the same 16 dp size and 24 dp line height. The layout and paint use the same font selection.

- The exact workspace passed 1,628 Rust tests and three WebAssembly checks. The workspace run reported 20 ignored tests. A separate Apple M1 Max offscreen run passed 18 Metal tests and produced 21 original PNGs.
- Native Gallery ran at 960×800 in light mode and 320×720 in dark mode. Two replays produced four original screenshots. Page.screenshot, the CLI and MCP agreed byte for byte. Eight window samples showed zero visible Gallery windows. All four Gallery processes exited.
- Headless Chrome 158 WebGPU passed 383 ordered Material steps and 31,830 checks. The two new steps verify visible text, keyboard focus return and unclipped Regular and Bold samples. The reader counted more Bold ink in both themes from the original PNGs. The 4,953 process samples showed zero visible test windows. The run left no observed processes.
- The 30 size, line height, spacing and weight values match the pinned AndroidX reference. Medium 500, rendered letter spacing, separate Brand and Plain families, locale-specific scaling and a Gallery sample for every role remain open. The browser Gallery package gained 3,717,054 uncompressed WebAssembly bytes over the previous source.

[Native light](native/originals/repeat-1/wide-light.png) · [Native dark](native/originals/repeat-1/compact-dark.png) · [Chrome light](browser/artifacts/material-typography-font-weight-wide-light.png) · [Chrome dark](browser/artifacts/material-typography-font-weight-compact-dark.png)

[Source reconstruction](source/README.md) · [Reference and font inventory](spec/README.md)

Recreate the source from the bundle and set `CARGO_TARGET_DIR` to `/tmp/cargo-target/surface-<worktree-name>`. Put the Cargo binaries first in `PATH` and set `CARGO_BUILD_JOBS=2`. Run these commands from the checkout.

```sh
cap sh scripts/check.sh
cap cargo build --locked -p surface-material-gallery --features surface-material-gallery/devtools -p surface-devtools
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
cap cargo xtask web build material-gallery --release --no-compress
evidence=$(mktemp -d /tmp/surface-typography-browser.XXXXXXXX)
cap cargo xtask web e2e --artifacts "$evidence" material
```

The native and browser tool folders hold the exact test harnesses and receipts from this Mac. The original browser command and exact flags are in `browser/invocation.json`. Set paths and build checksums in the configs when running on a new machine. Use Python 3.12 and Pillow to read back the recorded evidence from this directory.

```sh
python3 -E -B readback.py --manifest-sha REVIEWED_MANIFEST_SHA
```

The earlier Browser run found that the Typography entry was clipped below the Badge examples at 800 dp. The earlier snapshot is in `history/before-entry-fix.json`. The final run uses the relocated control at the top of the section.
