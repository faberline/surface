# Full-screen Dialog browser evidence

Headless Chrome 158.0.8127.37 ran the real Material Gallery on Apple Metal 3 through WebGPU. The focused runner ran M256–M263 from the stock Material suite. All eight steps passed with 834 checks, 41 screenshots, zero page errors and zero remaining run-owned browser processes. This is a focused result. The full stock suite is a separate release gate.

The source patch adds only the Dialog tests and their stock Material registration. The checks exercise the Basic and Alert dialogs, real wheel input, modal focus handback, draft editing and Save or Discard, 599/600 dp resize, and a 320 dp dark right-to-left profile at 200% text size. Browser input uses Chrome DevTools Protocol. The composition test sends synthetic preedit, which is the unconfirmed text from an input method. It does not exercise a live system input method.

M263 starts from the measured 56 dp floating action button. Opening and closing paint a partial rounded panel while the midpoint leaves the rounded corner and exterior pixels unchanged. Closing returns focus and removes the editor from Chrome's accessibility tree before paint ends. A physical click can reverse that exit at the same aperture and color. Reduced Motion snaps at zero elapsed time. The adaptive test keeps the same editor, draft, backward selection, and synthetic preedit across both sides of the 600 dp breakpoint.

At 320 × 240 dp, the preview scroller ends 8 dp above the fixed button. Real wheel input brings the complete saved status into that clip. The extracted button pixels remain identical while the status moves. All screenshots include the 44 dp browser host bar above the Canvas and use a fixed 2× device scale.

## Original screenshots and receipts

- [Raw browser report](run/browser-material-dialog.json), [independent PNG and geometry verification](run/verified-dialog.json), and [runner log](run/runner.log).
- [Measured button at transform start](run/material-dialog-adaptive-transform-start.png), [entry midpoint](run/material-dialog-adaptive-transform-entering.png), and [settled editor](run/material-dialog-adaptive-transform-open.png).
- [Exit midpoint](run/material-dialog-adaptive-transform-exiting.png) and [same-pose reversal](run/material-dialog-adaptive-transform-reversed.png).
- [599 dp preedit](run/material-dialog-adaptive-transform-adaptive-full-preedit.png) and [600 dp preedit](run/material-dialog-adaptive-transform-adaptive-basic-preedit.png).
- [Saved status before wheel](run/material-dialog-adaptive-transform-clearance-short-before.png) and [fully revealed status after wheel](run/material-dialog-adaptive-transform-clearance-short-after.png).

The PNGs and the passing raw JSON are byte-identical to the run. Portable copies of some text logs replace private path and loopback values. [Original text fingerprints](original-text-sha256.json) identify the originals.

## Exact inputs

| Input | SHA-256 |
| --- | --- |
| 576-file product source manifest, after both preview clearance fixes | `bb0e450ddb5c7a6ebaf7d592cf6c3f54e97f8db51ba118e5dc74d743ac9eba6a` |
| Complete stock Dialog test patch for that product source | `16d41305a165e8286758aa5cdb5bfa32384e2cd0a9cda97909f3f6a333f12186` |
| 579-file source manifest, including stock tests | `646558e2002586deca0a9d9c2ac604bec2940a597432f57e6d119f587147a755` |
| Focused runner source manifest | `74ba784f75cdb078f939ea4fc1cb7322d561e8f51b000aba595e769b38dfec14` |
| Built artifact manifest | `0c3b0a986d269c53695a2f691e0a3b58acc081c58c90b107b69758e94a054d3d` |
| Optimized Gallery Wasm | `8a2c2cd47f17088b9da52ef7b2a5f188196f8ce668230034ff8011fda411b360` |

The scoped stock E2E format, strict Clippy and all-target compile checks passed. The Gallery built with the existing Wasm dead-code warning for `should_exit`. See [build receipts](pin).

## Reproduce the focused result

Use a fresh temporary folder. Use the exact 576-file source export listed in [scroll-base.sha256](scroll-base.sha256). Set `SURFACE_PRODUCT_SOURCE` to that export and `SURFACE_EVIDENCE` to this package. The local run used the Mac command-line SDK, Chrome headless, the locked Rust toolchain and build dependencies, a separate Cargo target, and release-mode Wasm. Set `SURFACE_CARGO_HOME` to a populated offline Cargo cache. Pillow 12 is required for the Python verifier. Keep the source path fixed for this target because the Cargo task includes its compile-time path.

```sh
set -eu
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT="$DEVELOPER_DIR/SDKs/MacOSX.sdk"
export CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=2
export CARGO_HOME="$SURFACE_CARGO_HOME"
export SURFACE_WASM_OPT=/opt/homebrew/bin/wasm-opt
batch=$(mktemp -d /tmp/surface-dialog-web.XXXXXXXX)
batch=$(cd "$batch" && pwd -P)
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$batch")"
mkdir "$batch/source"
cp -R "$SURFACE_PRODUCT_SOURCE/." "$batch/source"
for file in prepare-focused.py build-scroll-focused.sh run-scroll-focused.sh verify-full-screen.py; do
  cp "$SURFACE_EVIDENCE/$file" "$batch/"
done
cd "$batch/source"
shasum -a 256 -c "$SURFACE_EVIDENCE/scroll-base.sha256"
patch -p1 < "$SURFACE_EVIDENCE/dialog-full-screen-complete-e2e.patch"
shasum -a 256 -c "$SURFACE_EVIDENCE/source.sha256"
cap cargo fmt --all --check
cap cargo clippy --locked -p surface-e2e --all-targets --all-features -- -D warnings
cap cargo check --locked -p surface-e2e --all-targets --all-features
cap cargo xtask web build material-gallery --release --no-compress --out "$batch/web-scroll-final/material-gallery"
cd "$batch"
python3 -B prepare-focused.py
pin=$(mktemp -d "$batch/focused.XXXXXXXX")
bash build-scroll-focused.sh "$pin"
bash run-scroll-focused.sh "$pin"
python3 -B verify-full-screen.py "$pin"/run.*
```

The build and run scripts seal each new set of exact artifacts. The native runner hash can depend on its build path. For the stock Material run, use the patched source without the focused selector. For Linux, use the stock headless E2E command and SwiftShader setup in the repository CI workflow.

## Earlier results

- [The two earlier native clipping failures and their captures](../native#saved-text-and-fab-clearance).
- [Initial browser M263 failure](diagnostics/browser-reduced-anchor-assumption/browser-material-dialog.json) after 656 checks. The test incorrectly required a measured resting origin for the Reduced Motion path. A physical button was present and the origin may be absent when the mode snaps. The next [first-clearance preflight](diagnostics/browser-first-clearance-preflight/runner.log) passed 773 checks before the short-preview scroll correction. Neither run is final acceptance.
- [Rejected warm build](diagnostics/rejected-stale-build.log) reused a task compiled against the old source location. Its identical old Wasm hash prevented its use as new evidence. The final build used the verified canonical scratch source path and produced a new Wasm.
