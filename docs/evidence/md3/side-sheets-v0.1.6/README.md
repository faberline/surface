# Surface v0.1.6 side-sheet verification

This change starts from Surface v0.1.5,
`e4caf946c3aa266c17add39bc75b5695130c6a24`. The source manifests
pin the files used for each run. From the matching Surface checkout, set
`evidence_dir` to this folder and check the manifests.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
```

## Headless macOS Metal

Nine cases cover overlay, coplanar and modal side sheets, docked and
detached styles, both edges, light and dark themes, and right-to-left
layout. The test checks page reflow, scrim and panel colors, disabled
controls, focus, Escape and semantic accessibility. The 320 × 480 dp
viewport verifies 200% Chinese text, scrolling below the fixed header,
and identical PNG bytes and metadata from the raw API, CLI and MCP.

[Native report](native/native-side-sheet-report.json) ·
[Coplanar panel](native/light-desktop-coplanar-docked-end-open.png) ·
[Narrow RTL](native/dark-rtl-200-narrow-coplanar-detached-end-open.png) ·
[Narrow after scroll](native/dark-rtl-200-narrow-coplanar-detached-end-scrolled.png)

The separate `devtools-headless` suite passed 3 steps and 76 checks on
macOS. It checks the authenticated session, a semantic action, a 720 × 640
RGBA PNG, and the same scaled PNG from raw TCP, CLI and MCP.

[Headless DevTools report](native/devtools-headless-result.json) ·
[Headless DevTools screenshot](native/devtools-headless-screenshot.png)

Use a Cargo target under `/tmp/cargo-target` and a command environment
with access to Metal. Both commands run without opening a window.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-side-sheet-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-side-sheet-native.py" \
    --headless --source "$source_digest" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
cap cargo xtask web e2e devtools-headless
```

## Chrome WebGPU

Headless Chrome 157.0.8103.79 on Apple Metal 3 passed 48 Material steps
and 2,100 checks. M45–M49 cover 24 combinations of presentation, style,
logical edge and writing direction. The tests send trusted pointer,
keyboard and touch events through Chrome DevTools. They verify WebGPU
paint, modal isolation, the named dialog role, 200% Chinese text,
responsive layout, drag cancellation, dismissal and Reduce Motion.

[Browser report](web/browser-material-side-sheet.json) ·
[Dark RTL with large text](web/material-side-sheet-compact-detached-dark-rtl.png) ·
[Touch while held](web/material-side-sheet-touch-held.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

## Earlier AppKit run with a window

The AppKit reader verified the native dialog and its accessible buttons
before the headless host was added. AppKit closed the panel with
`AXPress` and returned focus to the opening button. An
`AXScrollDownByPage` action moved the content while the header stayed
fixed. The windowed source manifest records that build.

[AppKit action report](native/appkit-actions.json) ·
[Windowed source manifest](native/windowed-appkit-source.sha256)

To repeat this optional windowed check, use a temporary checkout and
add `native/side_sheet_gallery_profile.rs` under
`examples/material-gallery/src/bin/` before building the Gallery.
Build `native/read-side-sheet-ax.swift` with `xcrun swiftc` and run
the same Python script with `--ax-reader` and `--profile-launcher`
instead of `--headless`. The caller needs macOS Accessibility permission.

Physical touch and live VoiceOver speech remain unverified.
Vertical finger panning and Android predictive back remain open.
