# Material 3 badges on the adaptive navigation scaffold

Source commit `758b0af489e659fb871bdf2b238340c4f1ff479c` uses tree `af4ac05e6bbffb92fbad3d89d9c5da3d58675ab5`.
The manifest covers all 717 source files.
`Theme::adaptive_navigation_scaffold_with_badges` uses the same badge data
in the compact bar, horizontal bar and wide rail. The app controls selection
and clears its localized badge content when the user acknowledges it.

Workspace checks passed 1,633 Rust tests and three Wasm builds.
Eighteen separate offscreen Metal tests produced 21 original PNGs.
Two native replays reproduce 12 PNGs through `Page.screenshot`, CLI and MCP.
All 10 Gallery processes exited. The observer recorded
22 samples with zero visible windows.

Headless Chrome passed 389 Material steps and 32,700 checks.
The reader confirmed RGB badge pixels and geometry in four original PNGs.
The journey checked focus and selection on both sides of each breakpoint. Chrome reports
names and the disabled control through its accessibility tree. The DOM projection records
the current page. The separate focused run passed five steps with 637 checks.

The initial full run stopped at M264 because the new Gallery launch row pushed the Divider
launcher below the viewport. The final source uses the existing wrapping row for both badge
launchers. The focused replay verified Divider activation and focus return before testing badges.

Use Python 3.12 and Pillow to read back the original evidence without starting an app.

```sh
python3 -E -B portable/readback.py --manifest-sha f90ae7a09b12c3043e3fd822b19da08fe0cb9ac4e757d3998657683197bde7b6
```

Restore `portable/source/q83-source.bundle` on branch `codex/md3-adaptive-navigation-badges-v0183`.
Run the checks from that source with the repository's Rust build instructions.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
cap cargo build --locked -p surface-material-gallery -p surface-devtools -p surface-e2e --features surface-material-gallery/devtools
cap cargo xtask web e2e --artifacts "$evidence" material
```

The native and browser folders contain the replay tools and original configs.
Set their source and binary paths to the rebuilt checkout and regenerate the file manifests.
Choose a fresh output directory under `/tmp`. Run `adaptive_native.py` and `browser_q83.py`
with `--config`, `--config-sha256`, `--run`, and `--out`.

The evidence covers headless rendering, accessibility metadata, and the Gallery's keyboard path.
VoiceOver speech and physical touch did not run.
