# Surface v0.1.4 navigation drawer verification

The candidate starts at Surface v0.1.3, commit
`e7fa1d407c97a0d992ccce42994b7a08e1d6d987`. The source manifests match the
tested files. Run the checks from the Surface checkout:

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
```

## Native macOS

The Gallery ran on macOS 27.2.1 and Apple M1 Max Metal. The replay captured
light and dark at 960 × 800 dp, and dark with right-to-left placement and
200% text at 320 × 480 and 320 × 240 dp. It checked shared layout reflow,
arrow keys, selection, disabled controls, dialog focus and dismissal,
wrapping and scroll reveal. AppKit reported `AXARIACurrent=page` and the
disabled option had no press action. AppKit `AXPress` selected Inbox and
restored focus to the opener. The compact native PNG matched byte for
byte across the wire API, CLI, and MCP.

[Native report](native/native-drawer-report.json) ·
[Light standard](native/light-desktop-standard.png) ·
[Dark modal](native/dark-desktop-modal.png) ·
[320 dp short window](native/dark-rtl-200-short-modal.png)

To replay, copy `native/drawer_gallery_profile.rs` to
`examples/material-gallery/src/bin/` in a temporary checkout of the
release. The helper changes the logical window size. It runs the same
Gallery and native host. Set `CARGO_TARGET_DIR` to an isolated path under
`/tmp/cargo-target` and build with the `devtools` feature.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-drawer-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery --features devtools \
    --bin surface-material-gallery --bin drawer_gallery_profile
cap cargo build --locked -p surface-devtools --bin surface-devtools
xcrun swiftc -module-cache-path "$scratch_dir/swift-cache" \
    "$evidence_dir/native/read-drawer-ax.swift" -o "$scratch_dir/read-drawer-ax"
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-drawer-native.py" \
    --source "$source_digest" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
    --profile-launcher "$CARGO_TARGET_DIR/debug/drawer_gallery_profile" \
    --ax-reader "$scratch_dir/read-drawer-ax" \
    --out "$scratch_dir/native-results"
```

The AppKit reader needs macOS Accessibility permission for the caller.
The native replay uses semantic keys and actions. Physical touch and live
VoiceOver speech are outside this receipt.

## Chrome WebGPU

Chrome 157.0.8103.79 ran all 39 Material steps and 893 checks with zero
page errors. Drawer steps M36–M40 exercised shared layout, focus and the
Chrome accessibility tree, scrim hit testing, 320 dp right-to-left
placement with 200% text, mouse and touch swipes, cancellation, resize,
and Reduce Motion. Pixel samples checked the held panel and scrim with
the pointer still down. Touch input was sent through Chrome DevTools.
The lists scroll with a wheel, keyboard focus and accessibility actions.
Vertical finger pans and Android predictive back remain open.

[Browser report](web/browser-material-navigation-drawer.json) ·
[RTL standard](web/material-navigation-drawer-standard-rtl-320-200.png) ·
[RTL held swipe](web/material-navigation-drawer-touch-rtl-opening-held.png)

```sh
cap cargo xtask web build material-gallery --release --no-compress
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```
