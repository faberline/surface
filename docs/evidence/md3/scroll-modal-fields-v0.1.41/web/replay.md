# Surface 0.1.41 headless browser replay

Run from the Surface repository root on macOS with a system Chrome and the Rust and wasm-bindgen prerequisites from `README.md`, plus a WebGPU-capable adapter. Read the exact Chrome version from the verification receipt. `cap` and two build jobs limit local resource use. The stock runner uses the four compiled debug binaries and three optimized Web bundles. It runs five suites without opening a window.

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

out=$(mktemp -d /tmp/surface-md3-scroll-modal-fields-replay.XXXXXXXX)
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

The recorded build used Rust 1.96.1 and the macOS Command Line Tools
SDK. The following variables were set before the build commands above.
Omit the Cargo cache settings on a fresh machine.

```sh
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export CARGO_HOME=/tmp/surface-md3-cargo
export CARGO_NET_OFFLINE=true
```

The recorded run used byte-for-byte copies of these binaries and Web bundles. `browser-binary-sha256.txt` uses paths relative to the bundle root, and `browser-source.sha256` uses repository-relative paths. Hashes were verified before and after the recorded run.

M235–M240 verify the named modal quantity and unit editor. They confirm both
Tab cycles and wide focused access, a single atomic Save for accepted and
unsubmitted text, per-field errors, Cancel and scrim isolation, synthetic IME
priority, and a dark RTL 320 dp viewport with 200% text. The compact checks
measure both pinned 48 dp actions, actual scroll and painted focus rings.
They also prove an invalid Save reveals its live summary while the Save
button retains focus. The modal hides at 217 dp and restores both drafts
at 218 dp. The shortest-editor capture measures 640×524 at DPR 2.
M229–M234 keep the earlier modeless popup covered independently. M228 keeps
the full outer focus stroke check for the short inline pair.

The detailed report is `browser-material-scroll-modal-fields.json`. The complete replay also runs the finite, looping, and inline-pair Scroll Field suites, M205–M234, and verifies each report. `verified-browser.json` records the adapter, suite counts, measurements, page errors, and process exit. The `material-scroll-modal-fields-*.png` files are screenshots from headless Chrome.
