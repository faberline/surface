# Showcase IME cancel and redraw

Headless Chrome 158.0.8127.37 ran both sides on the same copied stock test runner. I11 pauses the pending redraw after synthetic composition starts. It then cancels composition, sends a trusted Select All, and releases an actual Canvas resize from 1,000 to 1,001 dp before ordinary text input. It restores the original browser scheduler and viewport before I12.

The [old stock replay](stock-red/browser-ime-cancel-redraw.json) shows Chrome select the whole string at offsets `[0, 2]`. The Runtime still has byte offsets `[3, 3]`. The next redraw collapses Chrome's selection to `[1, 1]`. The inserted string lands between the two original characters.

The [repaired stock replay](stock-green/browser-ime-cancel-redraw.json) keeps Chrome offsets `[0, 2]` and Runtime byte offsets `[0, 6]` before and after redraw. The inserted string fully replaces the previous value. All 20 IME steps pass with 63 checks. The original unmodified stock IME suite separately passed 20 steps and 57 checks.

The same controlled probe [passed in v0.1.41](comparison/v041-controlled.json) and reproduced the lost selection in [v0.1.42](comparison/v042-controlled.json) and [v0.1.43](comparison/v043-controlled.json). Natural runs also captured the regression in [v0.1.42](comparison/v042-natural.json) and [v0.1.43](comparison/v043-natural.json) without pausing or resizing.

The [backward selection case](comparison/fixed-backward.json) exercises a different cancellation path. Chrome briefly reports `台` on cancellation while the Runtime retains `台文` at byte anchor 6 and cursor 3. A real redraw restores `台文` and Chrome's backward range `[1, 2]`.

If repaint remains paused, [Select All only selects the abbreviated browser text](comparison/fixed-backward-select-all-prepaint.json). This still succeeds as a whole-field replacement. The discriminating ArrowRight and X input sequence loses the original character before repaint. It stores `台X` when the expected result is `台文X`. This separate edge reproduces in [v0.1.41](comparison/v041-backward-right-prepaint.json), [v0.1.42](comparison/v042-backward-right-prepaint.json), and [the matched-value-only repair](comparison/fixed-backward-right-prepaint.json).

The [widened repair](widened-ime-cancel-mirror.patch) restores the same active editor's canonical value and directed selection as soon as a cancel is accepted. With the [two-case stock I11 test](stock-ime-i11-both.patch), the [previous repair fails](stock-selected-red/browser-ime-cancel-selected-redraw.json) at the selected-cancel subcase. The [widened repair passes](stock-selected-green/browser-ime-cancel-selected-redraw.json). It restores `台文` and the backward browser range `[1, 2]` before paint. Trusted Right moves to `[2, 2]`, and inserting X accepts `台文X` before repaint and retains it later. All I1–I20 pass with 77 checks.

## Reproduce

Use an isolated source export of `ab8615c8495d99a652f4fac38d4db37f08c82c7f` and a fresh target under `/tmp/cargo-target`. Apply [the two-case I11 test patch](stock-ime-i11-both.patch) and build the stock test binary and Showcase release Web bundle. Run only the `ime` suite with Chrome `--headless=new`. Keep the same test runner. Build the Web bundle with [the matched-value repair](ime-cancel-mirror.patch) in a fresh export for the selected-case red. Build with [the widened repair](widened-ime-cancel-mirror.patch) in a fresh export for green in both cases.

```bash
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-ime-cancel-repro
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build showcase --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" --chrome-arg --headless=new \
  --web-root "$CARGO_TARGET_DIR/web/release" --artifacts "$CARGO_TARGET_DIR/ime-evidence" ime
```

The source and binary manifests identify both Web bundles and the shared runner. [Verified values and patch digests](verified-i11.json) are included. Browser input events alone do not verify a live operating-system input method or screen-reader speech.
