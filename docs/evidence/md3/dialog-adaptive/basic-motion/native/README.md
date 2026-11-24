# Basic Dialog fade

`BasicDialogMotion` retains the opacity for one Basic Dialog. The caller owns the open flag, actions, and any draft. Rebuild when the open flag changes, call `motion.advance(&frame, open)` from the animation callback, and use `Theme::basic_dialog_with_motion(options, content, open, &motion)` in the view. Keep the same stable IDs and authored layer position while a dialog retires. Retain the content that should remain painted until `should_mount(false)` is false.

The panel fades as one isolated group. The backdrop fades on its own layer with the existing scrim ink and 82/255 resting alpha. Both follow the same StandardDefaultEffects spring. A finite first frame starts at the presented opacity. Switching direction keeps the current spring pose and velocity. Missing time and Reduce Motion snap to rest. A panel closed before any opening paint has no exit.

The live modal scope starts immediately. A closing view loses pointer, focus, and accessibility interaction at once. The Runtime restores eligible focus before the fade ends. A paint sample does not rebuild the view or change accessibility generation. The last exit sample removes the retired view.

The Gallery applies this to the Basic preview, the explicit-action Alert, and the nested discard prompt. Its wide and full-screen editor layouts use the existing steady wrappers. Full-screen container motion is a separate component change. Existing `Theme::basic_dialog` callers still get the existing immediate presentation. Use the extended display list for the animated builder.

## Exact sources and build

- Input Runtime source manifest `9a1378fc2e976c0a8850d3515a5280c21d5537056ea46180e01baf0a86e2cfc8` with 570 files.
- Patch `1d58925b1fef6484df173c73d6c65102292722e87ab0e58376dc6283b003dae5` applies cleanly to that input.
- Output source manifest `6e9faca3b07a7057f652c18967a9109a23c7ad4f0124859789dc4696e1f7f044` with 572 files.

```sh
batch="$stage"
cd "$batch/source"
shasum -a 256 -c ../source.sha256
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:?set a fresh target under /tmp/cargo-target}"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-false}"
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
cap cargo test --locked -p surface-material --test basic_dialog
cap cargo test --locked -p surface-material --lib basic_dialog::motion
cap cargo test --locked -p surface-material-gallery --features devtools --lib dialog_demo
sh scripts/check.sh
SURFACE_GPU_EVIDENCE="$batch/gpu/gpu-offscreen.png" sh scripts/check-gpu.sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
```

The pinned stock native replay uses the normal no-window Gallery host. It uses Reduce Motion by design, so it checks actual dark RTL compact and light wide settled paint, focus, and application transactions. For a fresh run with the pinned binaries, use:

```sh
cd "$batch"
PYTHON_BIN=python3 PYTHONDONTWRITEBYTECODE=1 \
    sh replay-native.sh "$batch/native-reproduction"
```

Each run checks equality between Page.screenshot, the CLI, and the MCP screenshot. It samples the Gallery process for visible windows. The two reports and eight PNGs in `native-sealed/` match byte for byte, with all 12 window samples per run zero. All eight settled captures also match the aligned Dialog baseline. See `native-before-after-settled.log`.

The separate ignored GPU integration test uses the real Basic builder, a deterministic Runtime clock, and offscreen Metal. It compares the actual scrim and panel pixels at opening, settled, and retiring frames. The retired state has no accessibility node, the opener has focus, and Reduce Motion clears the paint immediately. The `gpu-sealed/gpu-basic-dialog-*.png` captures illustrate those states. On the pinned test binary from the workspace gate, run `gpu_offscreen --ignored --nocapture` with `RUST_TEST_THREADS=1`. The proof here uses native Metal and a wasm32 compile check. Browser runtime proof is a separate lane.

The full workspace gate passed 1,277 tests and the wasm32 checks. Its standard Rust run reported 16 ignored tests. The separate Metal run passed all 13 offscreen cases. The three timed PNGs matched across its two runs.

Set 'stage' to the absolute scratch directory with the verified source in its source subdirectory. Rebuild from the [Runtime seal](../../runtime-preflight/) and apply the Basic motion patch. The archive records the original executable checksums.
