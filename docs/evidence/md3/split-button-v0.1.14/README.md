# Surface v0.1.14 split button verification

This batch starts from Surface v0.1.13 at
`abfdd046a09b9b987a47d3be23e42c2484096743`. Check out
`v0.1.14` to reproduce the release. Set `evidence_dir` to this
folder. Run the manifest checks from the source tree.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

The workspace gate passed 836 tests. The five renderer tests ran
separately and passed on Apple Metal. The manual clipboard test did
not run.

## Native headless Metal

The Gallery exercises the leading action and a trailing command menu.
The variant matrix covers five sizes, four appearances and both themes.
The compact profile uses a 320 dp viewport, dark theme, RTL and 200%
Chinese text. The bottom profile places the control by the lower edge
and checks that its menu moves above it. The PNG check counts two
lines of Chinese text and measures ink clearance from the rounded
face. Both profiles reported zero onscreen windows before and after each
run. Both replays produced 12 identical PNGs and identical JSON. The native
API, CLI and MCP returned matching PNG bytes and metadata.

[Native report](native/native-split-button-report.json) ·
[Replay comparison](native/native-replay-comparison.json) ·
[Light keyboard menu](native/light-keyboard-menu.png) ·
[Dark RTL 200%](native/dark-rtl-200-xl.png) ·
[Open menu](native/dark-rtl-200-xl-open.png) ·
[Five Metal renderer tests](native/gpu-offscreen-tests.log)

Use a Cargo target under `/tmp/cargo-target` and an environment that
can use Metal.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-split-button-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
for attempt in 1 2; do
    PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-split-button-native.py" \
        --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
        --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-$attempt"
done
diff -qr "$scratch_dir/native-1" "$scratch_dir/native-2"
```

## Headless Chrome WebGPU

M89–M94 cover the split button and the related Button Groups update.
The cases check actual paint and contour, pointer actions, focus and
keyboard exit, animated chevron direction, menu placement, and compact
large text with independently disabled actions. Chrome 157 and Apple
Metal 3 passed 5,120 checks in five headless suites. The Material
suite passed 93 steps and 4,872 checks. The six new steps ran 436
checks. The 320 dp label stayed at least 7.5 dp inside the rounded
face.

[Browser report](web/browser-material-split-button.json) ·
[Five-suite log](web/headless-results.txt) ·
[In-motion chevron](web/material-split-button-rtl-turning-chevron.png) ·
[Dark RTL 200%](web/material-split-button-compact-dark-rtl-xl.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --chrome-arg --headless=new \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

Live VoiceOver speech and physical touch need a platform session.
System high contrast and animated touch ripple remain shared gaps.

[Material 3 split button specs](https://m3.material.io/components/split-button/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.624e8f2da4007fc8.json) ·
[Menu button keyboard and semantics](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/)
