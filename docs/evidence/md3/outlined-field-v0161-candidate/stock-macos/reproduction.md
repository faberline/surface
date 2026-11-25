# Surface v0.1.61 Outlined Text Field stock macOS observer (R2)

The observer checks all 668 source files against manifest SHA-256
`3c79ae5ee15bf3b9b786d37452f1e0e5929ac0d31276ca7f6506897462dd9cc8`
and records tree `7c7881f78c3dc44129f52688a079b6874c9a4991`.

Stock requires the five ordered cases `smoke ime material bridge-web devtools-headless`.
It requires 327 Material steps through M328, with all previous checks,
including M324 and all 20 IME steps including I11. Both Chrome processes and
the native Gallery must pass the headless pre-execution guards. The macOS
process and window reader must observe the three headless processes, including
Bridge Chrome after detach. It requires zero detected visible owned windows
and zero owned survivors. The stock browser must report Apple Metal.

The new stock receipt key is `outlined_text_field`. It checks the typed
report, all five original PNGs, measured outline widths and notches, actual
label and placeholder ink, an invalid description, read-only and disabled
handling, and compact dark RTL at 200% text scale. The assertive region must
receive one error through unrelated edits and focus changes, and receive the
same error again after clear. The receipt records the two DOM deliveries.
This is DOM and Chrome accessibility evidence.

CDP creates synthetic preedit. A trusted Enter during composition must leave
submit count at zero. The browser can end this synthetic composition without
committing it, so the test may use CDP Input.insertText before the second
trusted Enter. The receipt records whether this was necessary. The second Enter
must yield exactly one submit. This is synthetic composition evidence.

## Offline controls

The R2 source comparison finds exactly one changed file,
`crates/surface-material/tests/outlined_text_field.rs`. Product and E2E
sources are identical to R1. The observer rejects the R1 manifest as an
exact R2 input.

`offline-controls.json` calibrated the verifier using the prior R1 scoped
`development-2` 4-step, 178-check run and its five original PNGs. That run
observed one guarded Chrome with zero visible owned windows and zero survivors.
The earlier `development-1` was a 159-check preflight before assertive validation.
These R1 runs calibrate the R2 verifier. The existing v0.1.60 stock proof was independently
reread for the previous 23,478 checks. The controls reject the focused and
prior stock logs as v0.1.61 stock, a removed M324 or I11, an incorrect source,
altered events or editor state, and reused captures. A synthetic acceptance
log tests parser structure only.

## Exact run

The controller starts the five guarded cases. These commands require the
staged build markers and stock binaries. Preparation verifies and copies
the exact inputs. It starts no app.

```sh
lane=/tmp/surface-md3-outlined-field-stock-v0161-r2.pT4k0wSO
stage=/tmp/surface-md3-outlined-field-tests-r2.j9B9xWVq
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3

(cd "$lane" && shasum -a 256 -c SHA256SUMS)
sh "$stage/run-stock-exact.sh" "$lane" "$harness_sha" --prepare
sh "$stage/run-stock-exact.sh" "$lane" "$harness_sha" --run
"$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
```

The last command reads the real run. It rechecks the exact inputs, window
receipt, process guard ledger, typed reports, original PNGs and inherited
acceptance. The accepted live result is `run/verified-outlined-field-stock.json`.
To run only the offline controls without starting any app:

```sh
"$py" -E -B "$lane/observe-outlined-field.py" --self-test
"$py" -E -B "$lane/check-controls.py"
```
