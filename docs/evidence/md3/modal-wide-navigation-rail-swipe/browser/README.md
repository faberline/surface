# Chrome standalone modal rail swipe replay

Use the exact Surface source from the parent guide. The generator
copies the source and routes five Material and five accessibility
cases through their canonical functions.

```sh
swipe_evidence=/path/to/modal-wide-navigation-rail-swipe
source_repo="$PWD"
browser_work=$(mktemp -d /tmp/surface-md3-rail-swipe-browser.XXXXXXXX)
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${browser_work##*/}"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
shasum -a 256 -c "$swipe_evidence/production.sha256"
shasum -a 256 -c "$swipe_evidence/browser/canonical-fixtures.sha256"
cap cargo xtask web build material-gallery --dev --no-compress \
  --out "$browser_work/web/material-gallery"
cap cargo xtask web build material-gallery --dev --features devtools --no-compress \
  --out "$browser_work/web-devtools/material-gallery"
python3 "$swipe_evidence/browser/prepare-wide-rail-swipe-browser.py" "$source_repo" "$browser_work/focused"
(
  cd "$browser_work/focused"
  shasum -a 256 -c "$swipe_evidence/browser/focused-fixtures.sha256"
  export CARGO_TARGET_DIR="$CARGO_TARGET_DIR-focused"
  cap cargo build --locked -p surface-e2e --bin surface-e2e
  "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --web-root "$browser_work/web" \
    --bridge-web "$browser_work/web-devtools/material-gallery" \
    --artifacts "$browser_work/captures" material web-accessibility
)
```

Chrome 157 with Apple Metal WebGPU passed 1,622 checks. The 288 dp
panels move 83.52 dp toward the logical start in both directions.
The scrim alpha changes from 82 to 58. Actual RGB samples match the
expected blend within 1 channel value. The closed-edge probe leaves
the wide rail closed. The test dismisses the independent Gallery
drawer before sampling the wide rail scrim.

In a true 320 × 240 dp drawing buffer at 200% text, wheel scrolling
reaches 112 dp and keyboard focus reveals Inbox at a 136 dp offset.
Vertical touch sends no action. The accessibility case keeps the
named modal during a drag. Real Tab cancels the drag and moves
focus to Inbox. With Reduce Motion a second gesture reaches
dismissal in one step and restores the opener.
[Full transcript](verified.log) · [Measurements](browser-wide-rail-swipe-receipt.json)

The normal and DevTools manifests record the historical package
hashes. Debug build paths can change them. The output manifest
records all PNG and JSON files from the run. Representative captures
and measurements are included here.
