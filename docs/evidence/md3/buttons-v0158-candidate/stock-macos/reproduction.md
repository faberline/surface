# Surface v0.1.58 Common Buttons stock macOS observer

This lane pins the exact 656-file Common Buttons source manifest `b03623198811629a68a2dfe5620d5705c55ea7744157ff2ac732d174833eedbf` and Git tree `8ba487496a2816f3bff3363328ba0dc40a059c30`. The controller owns the exact builds and stock launch. The preparation and offline checks here create no app or browser processes.

The observer runs exactly `smoke ime material bridge-web devtools-headless` in that order. It requires all 318 ordered Material checkpoints through M319 (M23 omitted), 20 IME steps with 77 checks, actual Apple Metal, and passing Bridge/DevTools receipts. The source and 16 binary/Web inputs are checked before and after. Its pre-exec guard requires `--headless=new` and private profiles for both Chrome launches, and `--headless --width 720 --height 640` for the native Gallery. CoreGraphics observations check the verified process family. A guarded compact launch, a visible window, an extra process, or incomplete proof fails the run.

The `common_buttons` result retains the original `material-buttons.json` and all seven exact PNG names with original and decoded pixel SHA-256 values. M316 checks five size layouts, vector colors, all five command appearances and physical focus clearance. M317 reconstructs the four checked toggle faces and label counts. M318 recounts the intermediate press shape and Reduce Motion endpoint, and checks the rejected outside release, one accepted mouse action, canceled touch and one accepted touch. M319 proves the ordered compact Tab journey, the initial clipped and revealed layout, both RTL icon positions, 200 percent text, actual dark focus and label pixels, and return focus. The verifier preserves all earlier receipts and pixel checks, including Standard/Forms/Range/Refinements and the two I11 cancellation regressions.

## Pin the controller's exact bundle

Wait for both exact build markers. Keep the root stage read-only during replay.

```sh
lane=/tmp/surface-md3-common-buttons-stock-v0158.u15I1f59
stage=/tmp/surface-md3-buttons-v0158.l0TJqeGW
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=b03623198811629a68a2dfe5620d5705c55ea7744157ff2ac732d174833eedbf

(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/prepare-pins.py" \
  --source-manifest "$stage/source.sha256" --source-sha256 "$source_sha" \
  --native-source "$stage/source" --web-source "$stage/source" \
  --native-binaries "$stage/native-v0158-binaries" --native-ready "$stage/native-final-built" \
  --web-target "$stage/web-final" --web-ready "$stage/stock-final-built" \
  --wide-canvas-height 800 --rtl-canvas-height 1020
```

The final two flags preserve the inherited Chips dimensions. The exact Buttons PNGs use 1920×2408 px for wide and 640×1848 px for compact.

## Verify and observe

The first command is an input-only readback. The second command starts the controller-authorized, guarded stock run.

```sh
runner_sha=$(cat "$lane/approved-runner.sha256")
"$py" -E -B "$lane/observe-common-buttons.py" --verify-inputs-only \
  --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner_sha"
"$py" -E -B "$lane/observe-common-buttons.py" --out "$lane/run" \
  --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner_sha" --timeout 1200
"$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
```

The last command replays both the new proof and the inherited evidence without launching. It reads the source and bundle seal, guard ledger, `runner.log`, `invocation.json`, window receipt, raw artifacts and `verified-common-buttons-stock.json`.

## Offline calibration and controls

The exact .57 stock comparator remains the previous release's proof. It rechecked 22,606 checks and 314 Material steps. The original Common Buttons development-3 focused comparator measured 398 checks, seven PNGs, 58 Chrome samples, zero windows and zero surviving owned processes. Its actual contour counts are `[466,504,623,623]`, and its compact dark PNG independently contains 5,280 focus pixels and 12,643 text pixels. The earlier focused JSON predates the checked geometry and compact palette fields. Calibration uses a separately verified dark palette from the earlier stock and falls outside exact stock acceptance. The first two focused failures remain independent raw diagnostics.

```sh
"$py" -E -B "$lane/check-controls.py"
"$py" -E -B "$lane/observe-common-buttons.py" --self-test
```

`offline-controls.json` records rejection of the development source, previous full runner log, focused scope, two real failures, and modified copies with wrong motion, touch, Tab order, text ink or missing PNG. `genuine-focused-pixels.json` is the original focused pixel calibration. The exact Common Buttons stock result remains pending until the controller runs all five suites.
