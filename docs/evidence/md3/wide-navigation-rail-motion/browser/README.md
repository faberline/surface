# Chrome wide rail replay

The six focused cases use the same browser fixtures and product bundles
as the normal Surface runner. The generator builds a new temporary
checkout with a route to M179–M181 and A69–A71.

Restore the verified current source from the parent guide. Use Cap,
the pinned Rust and wasm-bindgen versions, and Chrome. Run these
commands with loopback and WebGPU available.

```sh
motion_evidence=/path/to/wide-navigation-rail-motion
source_repo="$PWD"
browser_work=$(mktemp -d /tmp/surface-md3-browser-motion.XXXXXXXX)
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${browser_work##*/}"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
shasum -a 256 -c "$motion_evidence/production.sha256"
shasum -a 256 -c "$motion_evidence/browser/canonical-fixtures.sha256"
cap cargo xtask web build material-gallery --dev --no-compress \
  --out "$browser_work/web/material-gallery"
cap cargo xtask web build material-gallery --dev --features devtools --no-compress \
  --out "$browser_work/web-devtools/material-gallery"
python3 "$motion_evidence/browser/prepare-wide-rail-motion-browser.py" "$source_repo" "$browser_work/focused"
(
  cd "$browser_work/focused"
  shasum -a 256 -c "$motion_evidence/browser/focused-fixtures.sha256"
  export CARGO_TARGET_DIR="$CARGO_TARGET_DIR-focused"
  cap cargo build --locked -p surface-e2e --bin surface-e2e
  "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --web-root "$browser_work/web" \
    --bridge-web "$browser_work/web-devtools/material-gallery" \
    --artifacts "$browser_work/captures" material web-accessibility
)
```

The [recorded browser artifact manifest](recorded-run-artifacts.sha256)
lists all 26 files from the run. This packet includes the measurements
and representative PNGs. The [normal](wasm-bundle.sha256) and
[DevTools](devtools-wasm-bundle.sha256) manifests record the exact
historical package bytes. Build paths affect debug WASM hashes.

The [pre-fix 40 ms PNG](pre-fix-inline-40ms.png) shows the long label
cropped. The product changes after that capture are recorded in
`../inline-containment.patch.gz`. Reverse that patch in a copy of the
restored source and verify `../production-before-containment.sha256`
to recover the product used in the earlier run. The final browser
fixture checks the row, pill, label clip and visible ink.
