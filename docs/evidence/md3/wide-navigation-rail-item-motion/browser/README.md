# Chrome wide rail item replay

The focused runner uses the canonical browser fixtures and product
bundles. It runs four Material cases and four accessibility cases.
The generator copies the restored source under `/tmp` and selects the
eight cases in the copy.

Use a Chrome installation with WebGPU, Cap and the Rust and
wasm-bindgen versions pinned in the restored Surface source. Run
from that checkout with loopback and a GPU available.

```sh
item_motion_evidence=/path/to/wide-navigation-rail-item-motion
source_repo="$PWD"
browser_work=$(mktemp -d /tmp/surface-md3-item-browser.XXXXXXXX)
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${browser_work##*/}"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
shasum -a 256 -c "$item_motion_evidence/production.sha256"
shasum -a 256 -c "$item_motion_evidence/browser/canonical-fixtures.sha256"
cap cargo xtask web build material-gallery --dev --no-compress \
  --out "$browser_work/web/material-gallery"
cap cargo xtask web build material-gallery --dev --features devtools --no-compress \
  --out "$browser_work/web-devtools/material-gallery"
python3 "$item_motion_evidence/browser/prepare-rail-item-motion-browser.py" "$source_repo" "$browser_work/focused"
(
  cd "$browser_work/focused"
  shasum -a 256 -c "$item_motion_evidence/browser/focused-fixtures.sha256"
  export CARGO_TARGET_DIR="$CARGO_TARGET_DIR-focused"
  cap cargo build --locked -p surface-e2e --bin surface-e2e
  "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --web-root "$browser_work/web" \
    --bridge-web "$browser_work/web-devtools/material-gallery" \
    --artifacts "$browser_work/captures" material web-accessibility
)
```

Chrome 157 on Apple Metal WebGPU passed 1,374 checks. The four
early-width frames show 105, 114, 123 and 128 dp. At 200% text, the
label stays whole in both directions. Its alpha rounds to zero
at 59, 60 and 61 ms. The icon center moves about 0.03 dp horizontally
and 1.48 dp vertically in that interval. At 119 dp the browser
exposes one real button for Inbox. Enter selects it once.
[Browser receipt](browser-rail-item-receipt.json)
· [Exact run](verified.log)

The manifests record the [normal](wasm-bundle.sha256) and
[DevTools](devtools-wasm-bundle.sha256) historical package bytes.
Build paths can change debug WASM hashes. The
[recorded output manifest](recorded-run-artifacts.sha256) lists
all run artifacts. This packet includes the measurements and
representative frames.
