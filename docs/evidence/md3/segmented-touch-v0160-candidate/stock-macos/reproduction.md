# Surface v0.1.60 Segmented Touch stock macOS observer

The observer binds 662 source files to manifest SHA-256
`4874c29831d6a87a60c1e3b98071cc522d2ab9a933deab60b6f33091bcfd25b9`
and tree `b9539a68d5993b8c29a9dc17da571b31078c6874`.

The stock gate requires the five cases `smoke ime material bridge-web devtools-headless`.
The Material case must contain all 323 steps through M324. It keeps the prior
component, IME, WebGPU, transport, process and window checks. Stock requires Apple
Metal, zero visible owned windows and no survivors. The offline verifier reads
the exact prior retained verifier pinned in `verify-stock-receipt.py`.

The Segmented Button receipt decodes all seven original Canvas PNGs. It retains
the M320–M323 geometry, paint, accessibility, selection, and Back-focus checks.
For M324, it checks both horizontal directions, parent vertical motion, full
48 dp targets, one activation from the newly exposed item, exact outline and
text ink in the two new crops, rollback on cancellation, ordered capture events
for the same touch ID, stale release rejection, and keyboard routing from the
canvas after restoration.

## Calibrated inputs

`genuine-focused-pixels.json` and `offline-controls.json` are calibrated
against the unchanged development-3 originals. This focused run contains 474
checks and seven decoded PNGs. It uses one headless Chrome. It is separate from
the five-case stock gate.

The original v0.1.59 five-case receipt was also reread through its own sealed
verifier. It retained 23,315 checks. The v0.1.60 controls reject it as v0.1.60
stock. The fifteen rejection controls also cover omitted focus, edited
geometry, changed capture ownership and event order, stale release, missing
keyboard recovery, and corrupted or reused PNGs. The synthetic 323-step parser
control tests log shape only.

## Exact pin and verification

The controller runs the staged stock script with the sealed observer. The
preparation operation copies and verifies the exact executable and source
inputs. It starts no app. The run operation starts the guarded stock cases.

```sh
lane=/tmp/surface-md3-segmented-stock-v0160.MYdYLXnX
stage=/tmp/surface-md3-segmented-touch-v0160.5syabA1L
harness_sha=49c91fe1b3b4c7dc7abfc7a3fd80c76208c8ba6841b0ab36116f87e0a9c13adf
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3

(cd "$lane" && shasum -a 256 -c harness.sha256)
sh "$stage/run-stock-exact.sh" "$lane" "$harness_sha" --prepare
sh "$stage/run-stock-exact.sh" "$lane" "$harness_sha" --run
"$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
```

The last command requires the real guarded run and starts no app. It rechecks
the exact source and executable bytes, the guard ledger, all original reports
and PNGs, and the inherited receipts. It compares the result to the live
observer output. Before those stock files exist, test the observer offline.

```sh
"$py" -E -B "$lane/observe-segmented.py" --self-test
"$py" -E -B "$lane/check-controls.py"
```
