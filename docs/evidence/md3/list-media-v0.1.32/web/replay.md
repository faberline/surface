# Surface 0.1.32 headless browser replay

Run from the Surface repository root on macOS with a system Chrome and the Rust and wasm-bindgen prerequisites from `README.md`, plus a WebGPU-capable adapter. The test browser was Chrome 157.0.8103.79. `cap` and two build jobs limit local resource use. The stock runner uses the four compiled debug binaries and three optimized Web bundles. It runs five suites without opening a window.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-progress
cap cargo fmt -p surface-e2e --check
cap cargo clippy --locked -p surface-e2e --bin surface-e2e -- -D warnings
cap cargo test --locked -p surface-e2e --bin surface-e2e
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build showcase --release --no-compress
cap cargo xtask web build material-gallery --release --no-compress
cap cargo xtask web build material-gallery --release --features devtools \
  --out "$CARGO_TARGET_DIR/web/devtools/material-gallery" --no-compress
cap cargo build --locked -p surface-material-gallery --features devtools --bin surface-material-gallery
cap cargo build --locked -p surface-devtools --bin surface-devtools
cap cargo build --locked -p jet-bridge --bin jet-bridge

out=$(mktemp -d /tmp/surface-md3-list-media-replay.XXXXXXXX)
unset SURFACE_CHROME_ARGS
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
  --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  --chrome-arg --headless=new \
  --web-root "$CARGO_TARGET_DIR/web/release" \
  --bridge-web "$CARGO_TARGET_DIR/web/devtools/material-gallery" \
  --jet-bridge "$CARGO_TARGET_DIR/debug/jet-bridge" \
  --devtools-bin "$CARGO_TARGET_DIR/debug/surface-devtools" \
  --native-bin "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
  --artifacts "$out" \
  smoke ime material bridge-web devtools-headless
```

The recorded run used byte-for-byte copies of these binaries and Web bundles. `browser-binary-sha256.txt` uses paths relative to the bundle root, and `browser-source.sha256` uses repository-relative paths. Hashes were verified before and after the recorded run.

M187–M191 verify image size profiles and actual local artwork, rounded center-crop and alpha pixels, safe named detail and checkbox routes, and compact RTL with 200% text.

The detailed report is `browser-material-list-media.json`. The `verified-browser.json` report records the adapter, suite counts, measurements, page errors, and process exit. The `material-list-media-*.png` files are actual screenshots from headless Chrome.
