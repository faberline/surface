# Surface v0.1.59 Baseline Segmented Button stock macOS observer

The observer binds 660 source files to manifest SHA-256
`9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8`
and Git tree `757179e260a331b4488c6be1b49410263097d675`. It keeps the five stock
cases `smoke ime material bridge-web devtools-headless`, all 322 ordered
Material steps through M323, and both I11 cases. It preserves the prior
Common Buttons and slider checks. The observer requires actual Apple
Metal, zero page errors, zero visible owned windows and no survivors.

The new `segmented` receipt independently decodes five original WebGPU
Canvas PNGs. It checks equal 40 dp faces and seven 1×38 dp seam pixels,
localized radio and checkbox states, pressed feedback from the hit
padding, and both dark RTL focus rings at 320 and 240 dp. It checks
measured scrolling without narrowing the touch target, deliberate
activation counts, and Back focus restoration. The earlier 304-check
focused development run has separate scope.

## Pin exact build outputs

The controller runs these commands after the exact build markers exist.
Preparation and the `--verify-inputs-only` step start no app.

```sh
lane=/tmp/surface-md3-segmented-stock-v0159.tCMt0tSs
stage=/tmp/surface-md3-segmented-v0159.yzqEnSpq
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8

(cd "$lane" && shasum -a 256 -c harness.sha256)
"$py" -E -B "$lane/prepare-pins.py" \
  --source-manifest "$stage/source.sha256" --source-sha256 "$source_sha" \
  --native-source "$stage/source" --web-source "$stage/source" \
  --native-binaries "$stage/native-v0159-binaries" --native-ready "$stage/native-final-built" \
  --web-target "$stage/web-final" --web-ready "$stage/stock-final-built" \
  --wide-canvas-height 800 --rtl-canvas-height 1020

runner_sha=$(cat "$lane/approved-runner.sha256")
"$py" -E -B "$lane/observe-segmented.py" --verify-inputs-only \
  --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner_sha"
```

The final dimension options preserve the inherited Chips capture sizes.
The new Canvas captures are 1920×1848 wide, 640×1848 compact, and
480×1848 scrolled.

## Guarded run and offline verification

Only the controller runs the first command below. The second command
starts no app. It rechecks all source and executable bytes, the guard
ledger, the original report and PNGs, and inherited receipts.

```sh
"$py" -E -B "$lane/observe-segmented.py" --out "$lane/run" \
  --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner_sha" --timeout 1200
"$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
```
