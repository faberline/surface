# Native screenshot fault tests

The unchanged Surface end-to-end runner ran against a disposable build of
`36ea9b655a12f71c0620bd6500ec7aa8487d8e2b`. The build used Apple M1 Max Metal
on macOS 27.2.1 (26B131) and Rust 1.96.1. The injected code was confined to
`crates/surface-platform/src/screenshot.rs`. The app, runner, and CLI
binaries were identical across all four runs. The first run had the fault
selector unset. [Source diff](injected-source.patch) · [Source digests](source-digests.txt)

With faults disabled the app passed all 17 steps and 192 checks.
The decoded native screenshots were 960 × 800 px before and after the
theme change. Raw, CLI JSON, CLI file, and MCP returned the same PNG bytes
at the requested 400 px limit. Each deliberate fault made the unchanged
runner fail at DN15. [Passing run](results/baseline.log) · [Audited results](results/receipts.json)

| Injected fault | DN15 assertion | Exit |
| --- | --- | --- |
| Swap red and blue after native GPU readback | Pixel (70, 332) had RGBA `[143,85,101,255]` instead of `[101,85,143,255]`. [Run](results/swapped-red-blue.log) | 1 |
| Return the first default PNG after the theme change | Pixel (1, 1) had light RGBA `[253,247,255,255]` instead of dark `[20,18,24,255]`. [Run](results/stale-default-image.log) | 1 |
| Drop `max_side` after validation | The requested longer side was 400 px and the PNG had a 960 px longer side. [Run](results/ignored-max-side.log) | 1 |

## Replay

Use a graphical macOS session that permits AppKit, Metal, and loopback.
Place a clean checkout of the verified source in a new scratch directory
at `$proof/repo`. Copy `inject.py` and `run-controls.py` into `$proof`.
Verify that `git rev-parse HEAD^{tree}` in the checkout returns
`4a86a2265106643fefd7ea6bcf1d58bb66837ffb`.

```sh
python3 "$proof/inject.py" "$proof/repo"
cd "$proof/repo"
test "$(git diff --name-only)" = "crates/surface-platform/src/screenshot.rs"
git diff --check
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${proof##*/}"
printf '%s\n' "$CARGO_TARGET_DIR" > "$proof/target-dir.txt"
cap cargo build --locked -p surface-material-gallery -p surface-devtools -p surface-e2e \
  --features surface-material-gallery/devtools \
  --bin surface-material-gallery --bin surface-devtools --bin surface-e2e
shasum -a 256 "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
  "$CARGO_TARGET_DIR/debug/surface-devtools" "$CARGO_TARGET_DIR/debug/surface-e2e" \
  > "$proof/binaries.sha256"
python3 "$proof/run-controls.py"
```

The reported result and test artifacts are in `$proof/evidence`.
