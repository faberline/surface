# Surface v0.1.5 progress indicator verification

The candidate starts from Surface v0.1.4,
`c42c65307281fdeac8e7a281acd7213e80cbf3e2`. The manifests pin
the tested source files. Run their checks from the candidate checkout.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
```

## macOS Metal

The native Gallery ran on macOS 27.2.1 and Apple M1 Max Metal.
It checked flat and wavy linear and circular indicators at 0%, 25%,
100% and unknown progress. The replay checked actual fill, track, gap
and end-marker pixels. It verified the four shapes in light and dark
960 × 800 dp windows and in 320 × 680 and 260 × 720 dp right-to-left
windows with 200% text. AppKit scrolling revealed the narrow samples.

The AppKit probe confirmed localized names, read-only numeric values,
and removal of the range when progress becomes unknown. Tab passes
the indicators. Under Reduce Motion all indicator pixels stayed
unchanged. The 320 dp quarter-progress PNG matched byte for byte
through Page.screenshot, CLI and MCP.

[Native report](native/native-progress-report.json) ·
[Light 25%](native/light-desktop-quarter.png) ·
[320 dp dark RTL](native/dark-rtl-200-compact-quarter.png) ·
[260 dp after AppKit scroll](native/light-rtl-200-narrow-quarter.png)

The profile launcher calls the same Gallery and native host with a
different logical window size. Copy it to
`examples/material-gallery/src/bin/progress_gallery_profile.rs` in a
temporary checkout of the candidate. Choose a separate Cargo target
directory below `/tmp/cargo-target`.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-progress-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery --features devtools \
    --bin surface-material-gallery --bin progress_gallery_profile
cap cargo build --locked -p surface-devtools --bin surface-devtools
xcrun swiftc -module-cache-path "$scratch_dir/swift-cache" \
    "$evidence_dir/native/read-progress-ax.swift" -o "$scratch_dir/read-progress-ax"
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-progress-native.py" \
    --source "$source_digest" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
    --profile-launcher "$CARGO_TARGET_DIR/debug/progress_gallery_profile" \
    --ax-reader "$scratch_dir/read-progress-ax" --out "$scratch_dir/native-results"
```

The AppKit reader needs Accessibility permission for the local caller.
It reads the public AX API and sends a page scroll action to the
short window. The replay used semantic actions and did not listen
to a screen reader. Virtual focus during live animation remains open.

## Chrome WebGPU

Chrome 157.0.8103.79 on Apple Metal 3 passed 43 Material steps and
1,167 checks with zero page errors. M41–M44 use Chrome DevTools input,
full accessibility trees and the painted output. They check value
updates and retirement of the range, noninteractive indicators, all
four painted variants, live and static frames, 320 dp dark RTL at
200% text, 204 dp linear width at 260 dp, and real wheel scrolling
in a 320 × 480 dp view.

[Web report](web/browser-material-progress.json) ·
[Dark RTL at 320 dp](web/material-progress-dark-rtl-320-200.png) ·
[Short view after scroll](web/material-progress-dark-rtl-320-480-scrolled.png)

```sh
cap cargo xtask web build material-gallery --release --no-compress
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```
