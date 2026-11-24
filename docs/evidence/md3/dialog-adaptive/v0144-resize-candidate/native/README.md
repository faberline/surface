# Surface v0.1.44 native Dialog resize verification

The checked source has 579 files. The source manifest SHA-256 is `f69a406b6b86be6a0f24fb951a413b89312a729c5275862446912ba2039be161`. The Gallery and DevTools binaries have a manifest SHA-256 of `04880c34d3796eae737268069828396efbbaf5905f155f5a5af65c5cb498b7a2`.

The focused Rust test ran with the normal and Reduce Motion preferences. It changed one Gallery Runtime from 960 × 1800 dp to 320 × 400 dp without any user input. The first compact frame moved the FAB to `[24, 320, 56, 56]`, cleared its previous measurement, and requested another frame. The next frame measured the compact FAB and stopped requesting frames. The test also checked the new position after switching from RTL to LTR.

The native headless executable uses a fixed viewport. Two separately launched native sessions measured the actual RTL FAB at `[24, 1720, 56, 56]` and `[24, 320, 56, 56]`. Both measurements matched the full viewport clip. All four window checks returned zero. [Fixed-viewport measurements](fab-bounds/native-fab-bounds.json) · [Live resize test log](resize-runtime-test.log)

Both Dialog launchers fit in the same row at y1732–1780 in the 960 × 1800 dp Gallery. The Date Picker, Docked Date Picker and Date Range Picker entry points also stayed visible at y796, y868 and y940. At 320 × 400 dp, Tab moved both Dialog launchers from y1972 to y324. Enter opened the correct preview. Back returned focus to its own launcher. The route check made nine zero-window checks. [Route measurements](route-reach/route-reach.json) · [Compact launchers](route-reach/compact-revealed-1.png)

Two native headless runs each produced 14 decoded PNGs. Each PNG and its metadata matched across `Page.screenshot`, the CLI and MCP. Both replay directories matched byte for byte. All 14 captures also matched the prior Gallery route proof. The two behavior reports match that prior proof except for the binary checksums. Each replay made 22 zero-window checks. [Strict summary](verified-native.json) · [First replay](run/replay-1/native-full-screen-dialog-report.json) · [Second replay](run/replay-2/native-full-screen-dialog-report.json)

[Compact dark RTL discard](run/replay-1/compact-dark-rtl-200-discard.png) · [320 × 240 compact edit](run/replay-1/short-dark-rtl-200-edit.png) · [Wide Basic edit](run/replay-1/wide-light-basic-edit.png)

## Verify and replay

Run from this package on macOS with Rust, `cap`, a populated offline Cargo cache, Command Line Tools and Python. Set `exact_source` to the current source export. Set `prior_replay` and `prior_source_manifest` to the prior route proof when comparing to it.

```sh
set -eu
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
shasum -a 256 -c SHA256SUMS
(cd run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
archive=$(pwd -P)
batch=$(mktemp -d /tmp/surface-md3-dialog-resize-native.XXXXXXXX)
cp "$archive/build-pinned.sh" "$archive/replay-native.sh" "$archive/run-native-evidence.sh" \
    "$archive/replay-resize-runtime.sh" \
    "$archive/check-dialog-route-reach.py" "$archive/check-native-fab-bounds.py" \
    "$archive/verify-native-replay.py" "$batch/"
cp -R "$archive/native-scripts" "$batch/"
sh "$batch/build-pinned.sh" "$exact_source" "$archive/source.sha256" \
    f69a406b6b86be6a0f24fb951a413b89312a729c5275862446912ba2039be161
sh "$batch/replay-resize-runtime.sh" > "$batch/resize-runtime-test.log" 2>&1
PYTHON_BIN=python3 sh "$batch/run-native-evidence.sh"
python3 -B "$batch/verify-native-replay.py" "$batch" "$batch/run" \
    "$prior_replay" "$prior_source_manifest" "$batch/verified-native.json"
```

The native script starts only offscreen Gallery instances and uses a private loopback connection. It does not start Chrome. The build script uses a separate Cargo target and accepts `SURFACE_QA_CARGO_HOME` to select the offline cache.
