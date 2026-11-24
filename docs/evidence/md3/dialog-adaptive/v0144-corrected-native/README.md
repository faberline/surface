# Surface v0.1.44 native Dialog route verification

The corrected exact source has 579 files. Its SHA-256 manifest is `5d4e389d32159f245a732689a1dbb7f157abb4b3fcf56287d7e6c82dc59ef145`. The new Gallery and CLI binary manifest is `6f5095e0d96bb99181d00667491dd04fa7b00c9ce740a5c478f564b39b6ce284`. The executables remain separate.

At 960 × 1800 dp, both Dialog entry buttons are fully visible in one row at y1732–1780. Date Picker starts at y796. Docked Date starts at y868. Date Range starts at y940. The first Dialog test and the Full-screen transform test both launch before shrinking their viewports.

At 320 × 400 dp, both buttons initially start at y1972. Tab through the Gallery reveals them at y324–372. Enter opens each correct preview. Back puts focus on its exact entry button. The two screenshots match through the raw screenshot API, CLI and MCP. All nine visible-window samples returned zero.

The standard headless replay passed twice. Each replay has 14 PNGs with identical bytes and metadata through all three transports. The two replay directories match byte for byte. All 14 PNGs also match both the pre-release archive and the earlier v0.1.44 native capture. Their behavior reports match except for the binary fingerprints. Each run returned zero for all 22 window samples. No test process remained after both lanes.

- [Route access measurements](route-reach/route-reach.json) and [compact screenshot](route-reach/compact-revealed-1.png).
- [Independent native report](verified-native.json) and [comparison with earlier v0.1.44](verified-native-v0144-rejected-comparison.json).
- [Raw native run 1](run/replay-1/native-full-screen-dialog-report.json) and [raw native run 2](run/replay-2/native-full-screen-dialog-report.json).

The route probe uses the published DevTools key action in the native runtime. It exercises the real focus and scroll rules. Browser pointer input, Chrome accessibility and motion timing use their own checks.

## Verify and replay

Run from this package. Set `exact_source` to the corrected source export. Set `prior_archive` to the previous native archive for an independent pixel comparison. The build needs macOS, Command Line Tools, Rust, `cap`, an offline Cargo cache, and Python.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
(cd run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
archive=$(pwd -P)
batch=$(mktemp -d /tmp/surface-dialog-route-native.XXXXXXXX)
cp "$archive/build-pinned.sh" "$archive/replay-native.sh" "$archive/check-dialog-route-reach.py" "$archive/verify-native-replay.py" "$batch/"
cp -R "$archive/native-scripts" "$batch/"
sh "$batch/build-pinned.sh" "$exact_source" "$archive/source.sha256" \
    5d4e389d32159f245a732689a1dbb7f157abb4b3fcf56287d7e6c82dc59ef145
PYTHONPATH="$batch/native-scripts" python3 -B "$batch/check-dialog-route-reach.py" \
    --gallery "$batch/pins/surface-material-gallery" --cli "$batch/pins/surface-devtools" \
    --out "$batch/route-reach"
PYTHON_BIN=python3 sh "$batch/replay-native.sh" "$batch/run"
python3 -B "$batch/verify-native-replay.py" "$batch" "$batch/run" \
    "$prior_archive/native/replay-1" "$prior_archive/inputs/final/source.sha256" \
    "$batch/verified-native.json"
```

The build script uses a private Cargo target. Set `SURFACE_QA_CARGO_HOME` if the offline cache is outside its default location.
