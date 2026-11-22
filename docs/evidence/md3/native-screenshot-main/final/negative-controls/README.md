# Screenshot fault tests on the final PR runner

Surface's end-to-end runner and CLI came from the final PR source
`8c0724fb3157e171040fe4f623bda91785d4c660`. The journey ran on Apple M1 Max
Metal and macOS 27.2.1 (26B131). It used the disposable fault app built from
`36ea9b655a12f71c0620bd6500ec7aa8487d8e2b`. The lower commit differs from
the final source only in one test file. The final runner fixes Reduce Motion
for the PNG comparison and preserves PNG evidence on failure.
[Final source](final-source.txt) · [Runner changes](final-runner.patch) ·
[Fault app patch](injected-source.patch) · [Binary digests](binaries.sha256)

The control run passed all 17 steps and 192 checks with fault injection
disabled. The final runner decoded 400 × 333 px PNGs with identical bytes
from raw DevTools, CLI and MCP. The full PNGs were 960 × 800 px in both
themes. [Passing run](results/baseline.log) · [Audited results](results/receipts.json)

| Fault mode | DN15 assertion | Exit |
| --- | --- | --- |
| Swapped red and blue | Pixel (70, 332) was `[143,85,101,255]` instead of `[101,85,143,255]`. [Run](results/swapped-red-blue.log) | 1 |
| Returned the first PNG after changing theme | Pixel (1, 1) was `[253,247,255,255]` instead of `[20,18,24,255]`. [Run](results/stale-default-image.log) | 1 |
| Ignored the requested maximum side | The longer side measured 960 px when the request limited it to 400 px. [Run](results/ignored-max-side.log) | 1 |

## Replay

Use a graphical macOS session. Put a clean copy of the final source in
`$proof/repo`, where `$proof` is a fresh directory under `/tmp`. Copy
`run-controls.py` into `$proof`. Put a second clean checkout at
`$fault/repo`, where `$fault` is a separate directory under `/tmp`. Copy
`inject.py` into `$fault`. The test-only source delta permits either
product-equivalent commit for the fault checkout.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
python3 "$fault/inject.py" "$fault/repo"
cd "$fault/repo"
test "$(git diff --name-only)" = "crates/surface-platform/src/screenshot.rs"
git diff --check
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${fault##*/}"
cap cargo build --locked -p surface-material-gallery --features devtools --bin surface-material-gallery
printf '%s\n' "$CARGO_TARGET_DIR/debug/surface-material-gallery" > "$proof/fault-app.txt"
cd "$proof/repo"
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${proof##*/}"
cap cargo build --locked -p surface-devtools -p surface-e2e --bin surface-devtools --bin surface-e2e
printf '%s\n' "$CARGO_TARGET_DIR" > "$proof/target-dir.txt"
shasum -a 256 "$(cat "$proof/fault-app.txt")" "$CARGO_TARGET_DIR/debug/surface-devtools" \
  "$CARGO_TARGET_DIR/debug/surface-e2e" > "$proof/binaries.sha256"
python3 "$proof/run-controls.py"
```
