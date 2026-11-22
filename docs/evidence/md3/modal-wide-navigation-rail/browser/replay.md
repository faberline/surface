# Modal Wide Navigation Rail browser replay

Run these commands from the verified Surface checkout. The build needs `cap`, Rust, the repository's pinned `wasm-bindgen`, and Chrome. The Chrome run opens a local DevTools connection and serves the WASM packages over loopback.

The recorded build used Rust 1.96.1, `wasm-bindgen` 0.2.129 and the shared Cargo cache at `/tmp/surface-md3-cargo`. The browser run used Chrome 157.0.8103.79 with hardware WebGPU on Apple Metal 3.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
modal_proof=$(mktemp -d /tmp/surface-md3-modal-browser-replay.XXXXXXXX)
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-modal-browser-${modal_proof##*.}"

cap cargo xtask web build material-gallery --dev --no-compress \
    --out "$modal_proof/web/material-gallery"
cap cargo xtask web build material-gallery --dev --features devtools --no-compress \
    --out "$modal_proof/web-devtools/material-gallery"
cap cargo build --locked -p surface-e2e --bin surface-e2e

(
    cd "$modal_proof/web/material-gallery"
    shasum -a 256 index.html *.js *.wasm surface-web.json
) > "$modal_proof/web.sha256"
(
    cd "$modal_proof/web-devtools/material-gallery"
    shasum -a 256 index.html *.js *.wasm surface-web.json
) > "$modal_proof/web-devtools.sha256"
shasum -a 256 "$CARGO_TARGET_DIR/debug/surface-e2e" > "$modal_proof/runner.sha256"

"$CARGO_TARGET_DIR/debug/surface-e2e" --web-root "$modal_proof/web" \
    --bridge-web "$modal_proof/web-devtools/material-gallery" \
    --artifacts "$modal_proof/material" material
"$CARGO_TARGET_DIR/debug/surface-e2e" --web-root "$modal_proof/web" \
    --bridge-web "$modal_proof/web-devtools/material-gallery" \
    --artifacts "$modal_proof/accessibility" web-accessibility
```

The standard Gallery package serves the primary test page. The Material suite also opens a page with the devtools package after M174. That page runs the existing FAB paint checks. Its WASM build exposes `export_fab_toggle_paint`.

The focused replay uses the same product package and modal case. The temporary `prepare-modal-wide-rail-browser.py` script copies the checkout and routes the two normal suite entry points directly to M180 and A70. It retains the source of each case. Build that copy with the same `cap cargo build --locked -p surface-e2e --bin surface-e2e` command and a distinct external `CARGO_TARGET_DIR`. Use the same `--web-root` and `--bridge-web` flags to run `material web-accessibility` against the focused runner.

## Replay the Material tail

The full run reached M180 with the earlier layout wait.
`modal-before-layout-wait.patch.gz` restores the exact earlier
test source. Use a separate checkout for that historical run.

To run the final 12 cases directly, use the current source and
`prepare-modal-wide-rail-tail.py`. Pass the verified checkout and the
path for a new scratch copy. Verify the three paths with
`tail-runner.sha256` from that copy. Build the same `surface-e2e` binary
with a separate external target and run only `material`. Pass both
bundle paths as in the standard replay. The route uses the original case
functions and records the browser page error check.
