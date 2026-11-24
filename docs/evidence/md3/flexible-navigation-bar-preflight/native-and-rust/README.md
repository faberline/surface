# Flexible Navigation bar product preflight

This archive verifies the compact, vertical Material 3 Flexible Navigation bar on the exact **v0.1.46 Menu-based source**. It records a component preflight. The v0.1.47 release has its own source, stock browser and hosted checks.

The additive component supports three to five destinations. It keeps one caller-controlled current page. The Gallery adds a route at the end of `section-3`. It shows the selected page and a stationary bar while the contents scroll.

## Results

- **24 targeted Rust tests passed.** Seven cover the Flexible bar, six cover badges, eight cover the original bar, one covers the consumer contract, and two cover the Gallery. Scoped native and WASM checks passed. Strict Clippy passed for both the component and Gallery. [Library tests](logs/library/targeted-final.log) · [Consumer contract](logs/library/consumer-contract-final.log) · [Gallery tests](logs/gallery/final-targeted-gallery.log) · [Library Clippy](logs/library/clippy-final.log) · [Gallery Clippy](logs/gallery/final-clippy.log)
- **Two native offscreen runs produced two PNGs each.** The same bytes and metadata arrived through the screenshot API, DevTools CLI and MCP. Both run directories matched byte for byte. The six window samples per run showed zero windows. [First report](native/replay-1/native-gallery-report.json) · [Second report](native/replay-2/native-gallery-report.json) · [Independent verification](verified-archive.json)
- **The two profiles exposed real layout and ink.** The light 960 × 800 dp viewport centers a 480 × 64 dp bar at the bottom. The dark 320 × 400 dp viewport uses RTL and 200% text. Its full-width bar grows to 112 dp. The `最新消息` label wraps into two full lines. [Light PNG](native/replay-1/wide-light-focused.png) · [Dark compact PNG](native/replay-1/compact-dark-rtl-200.png)
- **The compact preview keeps the selected page visible.** In the earlier native run, the selected page title sat at y288, the exact bottom of the content clip. The pinned header moved the title to y16. The final capture measured 614 light ink pixels within it. [Earlier PNG](native/pre-header/compact-dark-rtl-200.png) · [Earlier layout](native/pre-header/native-gallery-report.json) · [Correction](patches/pin-gallery-header.patch)

The native runs use offscreen Metal and the native runtime key path. They exercise Tab through enabled destinations, Enter and Space, the disabled destination, the three, four and five item settings, and Back focus return. This is evidence about measured runtime state and native paint. [Design decisions and primary links](design-audit.md)

## Exact inputs

| Input | Paths | SHA-256 of its manifest or patch |
| --- | --- | --- |
| [Menu v0.1.46 source](inputs/menu-v0.1.46.sha256) | 590 | `92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5` |
| [Final library patch](patches/flexible-navigation-bar.patch) | 3 changed | `4334f93d3cf9b49891fa32e16b9664d7c0a0425a1a6276f850c0a1002a38a0fa` |
| [Library test source](inputs/library-source.sha256) | 592 | `2c1dcc32cef020e321d20b550873e77f9441e4bc0c9ca936d75e4aaa0703e44a` |
| [Final Gallery patch](patches/flexible-nav-gallery.patch) | 2 changed | `05486af913ff3041a39f642442f58ed289d5d87b8f90387cd40a88a14ad9a317` |
| [Combined Gallery test source](inputs/combined-source.sha256) | 593 | `bf0fde8a8b91e22426a97306a93e2dc1420b094f41d6bc0cafc04e79906d479d` |
| [Combined source before the header correction](inputs/pre-header-source.sha256) | 593 | `450accbc1475456c0ab507b1e2782e3b2dc72c59bc3a0dc52c6aac910dbddc5e` |
| [Original binaries used by both corrected replays](inputs/native-binaries.sha256) | 2 | `a906cdaa724c3026f196c3336b41447dbdcc0f4869ee3fdaea9d072d88be3d5a` |

The package contains the source patches and verification data. Executables are stored separately. The original log digests are recorded alongside portable copies, which replace machine-specific build paths. [Original log digests](logs/original-digests.json)

## Verify or replay

Run these commands from this package. To verify source reconstruction, set `menu_source` to the sealed Menu v0.1.46 export. The verifier uses a temporary copy of that source. It decodes the five archived PNG files and checks the logs without running the application.

```sh
shasum -a 256 -c SHA256SUMS
python3 -B scripts/verify-archive.py --menu-source "$menu_source"
```

To build and replay on macOS, use the Command Line Tools SDK, an offline Cargo cache with the Rust WASM target, `cap`, and Python. Native screenshot transport needs private loopback. The replay only starts the Gallery in `--headless` mode.

```sh
set -eu
archive=$(pwd -P)
replay=$(mktemp -d /tmp/surface-nav-gallery-preflight.XXXXXXXX)
cp -R "$menu_source" "$replay/source"
cd "$replay/source"
shasum -a 256 -c "$archive/inputs/menu-v0.1.46.sha256"
patch -p1 < "$archive/patches/flexible-navigation-bar.patch"
shasum -a 256 -c "$archive/inputs/library-source.sha256"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export CARGO_HOME=/tmp/surface-md3-cargo CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${replay##*/}" CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUST_TEST_THREADS=1
cap cargo fmt --all --check
cap cargo test --locked -p surface-material --test flexible_navigation_bar --test navigation_bar --test navigation_badge
cap cargo test --locked -p ui-runtime --test consumer_contract
cap cargo check --locked -p surface-material --all-targets
cap cargo check --locked -p surface-material --lib --target wasm32-unknown-unknown
cap cargo clippy --locked -p surface-material --all-targets -- -D warnings
patch -p1 < "$archive/patches/flexible-nav-gallery.patch"
shasum -a 256 -c "$archive/inputs/combined-source.sha256"
cap cargo test --locked -p surface-material-gallery --features devtools --lib flexible_navigation_bar_demo
cap cargo check --locked -p surface-material-gallery --all-features
cap cargo check --locked -p surface-material-gallery --all-features --target wasm32-unknown-unknown
cap cargo clippy --locked -p surface-material-gallery --all-features --all-targets -- -D warnings
cap cargo build --locked -p surface-material-gallery -p surface-devtools --features surface-material-gallery/devtools --bins
for repeat in 1 2; do
  python3 -B "$archive/scripts/replay-flexible-nav.py" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$replay/native-$repeat"
done
diff -qr "$replay/native-1" "$replay/native-2"
```

The verifier reconstructs the earlier capture source by reversing the pinned header correction and applying the archived test-import delta. The test import affects the exact source digest only. The earlier title clip is visible in the preserved compact screenshot.
