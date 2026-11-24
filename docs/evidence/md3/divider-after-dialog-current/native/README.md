# Divider retarget and stock native replay

The Divider browser test records the actual WebGPU adapter fallback status and requires a boolean. This matches Search and Dialog. The existing pixel and accessibility checks remain in M264 and M265. This is the only edit from the earlier 583-file Divider retarget. The workspace is still v0.1.44.

| Input or output | SHA-256 |
| --- | --- |
| Corrected Dialog base, 579-file manifest | `5d4e389d32159f245a732689a1dbb7f157abb4b3fcf56287d7e6c82dc59ef145` |
| Previous Divider retarget, 583-file manifest | `38a589e74410c9ec4bec4d518f447058da99f1d5f797ca24b8aacd94932682bc` |
| Corrected Divider source, 583-file manifest | `73486892d094b921162adbbd69ecfe67ed982c35549be68775f86f1927635f5c` |
| Full Divider patch on Dialog base | `06f5ff4f075ee578aeba8693e95402b1d5e2159eb63346654563ca4536d4aeba` |
| Adapter assertion-only patch | `8ab8bc451a5dc990940ce8bb9942c95754694f4f26bf408a30b2105445d6d5c7` |

An independent copy of the exact Dialog source applied the full patch and reproduced all 583 hashes. The two native binaries were built and pinned from that exact source.

## Native results

Both stock headless Metal runs passed. The wide light scene is 960 × 640. The compact dark RTL scene is 320 × 640 with 200% text. Each scene draws all four one-dp dividers, verifies the two inset layouts and returns focus to the launcher. The screenshot checker measures actual line ink, including the absence of that color on neighboring pixels.

The Page.screenshot API, CLI and MCP returned identical PNG bytes and metadata. The PNGs and JSON report match across runs. Both captured PNGs also match the earlier Divider preflight. All 12 visible-window samples were zero. The raw reports and captures are in `native-final/`.

## Reproduce

The build uses the macOS Command Line Tools SDK, a Metal device for the headless replay, and an offline Cargo cache. Use a fresh target and an exact source directory that matches `source.sha256`.

```sh
batch=/tmp/surface-md3-divider-retarget-native.AZ0EotL0
cd "$batch/source"
shasum -a 256 -c ../source.sha256
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export CARGO_HOME=/tmp/surface-md3-cargo CARGO_NET_OFFLINE=true
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-divider-retarget-native.AZ0EotL0
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
cap cargo fmt --all --check
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
cd "$batch"
PYTHON_BIN=/opt/homebrew/bin/python3.12 sh replay-native.sh "$batch/native-replay-new"
```

The replay script uses the verified binaries in `pins/` and verifies the source and binary manifests before and after use. For a fresh build, copy the two binaries into a new batch's `pins/` and record them with `shasum -a 256 surface-material-gallery surface-devtools > native-binaries.sha256` from that directory. The real Chrome run uses the same source seal and has a separate receipt.
