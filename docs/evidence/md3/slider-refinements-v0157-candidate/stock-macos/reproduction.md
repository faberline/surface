# Surface v0.1.57 stock macOS observer

The observer admits the exact 652-file R1 source manifest `5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1` and records candidate tree `72cf21a76d284e3a299aedff91b953eae232db07`. It retains the .56 guard and observes five explicit cases in the original order. The Material case must finish all 314 steps through M315. Diagnostic M23 stays outside that count.

The new `slider_refinements` receipt names the original two JSON reports and all seven PNGs. It records the decoded pixel hashes and measures the vertical signed tracks and focus readouts. It also measures the inset leading glyphs, minimum and quiet contours, and compact focus readout. The full stock receipt retains the earlier Standard, Forms, Range and IME reports. It requires actual Apple Metal, zero page errors, zero visible owned windows and no surviving owned processes.

## Prepare the exact bundle

The preparation command copies the already-built executables and Web bundles into this scratch directory. Run it once, after the exact builds supply both marker files.

```sh
lane=/tmp/surface-md3-slider-refinements-stock-v0157.z4WspMGg
stage=/tmp/surface-md3-slider-refinements-v0157.RxfdP7uB
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1

(cd "$lane" && shasum -a 256 -c harness.sha256)
"$py" -E -B "$lane/prepare-pins.py" \
  --source-manifest "$stage/source-r1.sha256" --source-sha256 "$source_sha" \
  --native-source "$stage/source" --web-source "$stage/source" \
  --native-binaries "$stage/native-v0157-r1-binaries" --native-ready "$stage/native-r1-built" \
  --web-target "$stage/web-r1" --web-ready "$stage/stock-r1-built" \
  --wide-canvas-height 800 --rtl-canvas-height 1020
```

The last two arguments preserve the original Chips capture sizes. The stock observer also checks the seven new screenshots at their measured sizes.

## Verify and observe

The first command checks the executable and source inputs without launching. The second command is for the authorized controller. It runs the stock process with the existing headless guard and writes to a fresh directory.

```sh
runner_sha=$(cat "$lane/approved-runner.sha256")
"$py" -E -B "$lane/observe-slider-refinements.py" --verify-inputs-only \
  --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner_sha"
"$py" -E -B "$lane/observe-slider-refinements.py" --out "$lane/run" \
  --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner_sha" --timeout 1200
"$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
```

The last command starts no executable under test. It requires the original run, exact bundle and source pins. It checks the guards, replays the pixel measurements and verifies the inherited reports with the pinned earlier verifier. Its JSON output records the original runner and window report digests.

## Calibration and controls

The genuine earlier .56 stock receipt replays to its original digest and has 22,116 checks. The genuine .57 development browser run passed six focused steps and 490 checks. Its seven original PNGs were decoded. The development native integrity check verifies the original two replays, 16 PNGs, 32 zero-window samples and eight reaped processes. These calibrations do not satisfy the exact R1 stock gate.

`check-controls.py` verifies those original inputs and the exact .57 source. It rejects the development source pin, a preceding full stock log as .57 proof, the focused log as stock, and modified scratch copies of the screenshot or typed report. `headless-guard-self-test.txt` records the pure launch-guard predicates.
