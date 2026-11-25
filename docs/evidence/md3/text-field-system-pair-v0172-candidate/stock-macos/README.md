# Surface v0.1.72 exact macOS stock headless guard

This guard keeps the five stock suites `smoke`, `ime`, `material`,
`bridge-web` and `devtools-headless`. The names and order stay fixed.
Material must complete 361 ordered steps through M362, with M23 omitted.
The gate keeps all 20 IME steps, transport checks, and retained paint
and semantics decoders. New system-pair proof has 14 original Canvas
PNGs. GrayText proof retains 15 originals and earlier outline motion
retains 33. The guard checks all original Gallery link targets before
opening their regular source assets.

The new oracle distinguishes real equal CSS pairs from the test's
isolated distinct `Field` pair. It uses original PNGs to check exact
FieldText, the native CSS Highlight alpha composite over Field, focus,
caret, notch, compact malformed-input handling, and restored paint.
The R8 scoped runner passed 1,993 checks in 13 Material steps. The
guard recorded 233 Chrome samples, zero visible windows and zero
surviving processes. The runner used the R4 Gallery. Exactly three
E2E files differ between the Gallery producer and the R8 source.
The original R4 and R5 failures remain available.

## Offline preflight

This seal binds the final 691-path v0.1.72 source, the exact patch and
the actual R8 scoped proof. Only `docs/material-coverage.json` differs
between the R8 source and the final source. Offline preflight verifies
those inputs, the published v0.1.71 proof, two original rejections and
the fail-closed controls. The separate `setup-harness.sha256` records
the earlier preparation-only seal. The offline checks do not launch
the stock suites.

```sh
lane=/tmp/surface-md3-field-pair-stock-v0172-final.qeJADEOL
stage=/tmp/surface-md3-field-pair-v0172.SEhFjvMD
candidate="$stage/exact-candidate"
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
# Compare the seal against the reviewed value.
sh "$lane/run-stock-exact.sh" "$harness_sha" --preflight
```

## Future exact approval and run

Review the two final binary hashes and complete a fresh review file
using `reviewed-pins.example.json`. Both native and Web build markers
must contain the exact final tree and a newline. Keep the four
executables and twelve canonical Web files available for the guard's
before/after comparison.

```sh
pins="$stage/reviewed-v0172-stock-pins.json"
jq --arg runner "$(shasum -a 256 "$candidate/web-canonical/debug/surface-e2e" | cut -d' ' -f1)" \
   --arg native "$(shasum -a 256 "$candidate/binaries/SHA256SUMS" | cut -d' ' -f1)" \
   '.stock_runner_sha256 = $runner | .native_manifest_sha256 = $native' \
   "$lane/reviewed-pins.example.json" > "$pins"
pin_sha=$(shasum -a 256 "$pins" | cut -d' ' -f1)
sh "$lane/run-stock-exact.sh" "$harness_sha" --prepare "$pins" "$pin_sha" \
  "$candidate" "$stage/text-field-system-pair-v0172.patch"
# Controller-authorized only. Use the safe headless test slot.
sh "$lane/run-stock-exact.sh" "$harness_sha" --run "$pins" "$pin_sha"
sh "$lane/run-stock-exact.sh" "$harness_sha" --recheck "$pins" "$pin_sha"
```

The guard requires real headless Chrome, a detached Bridge Chrome and
an offscreen Gallery. It fails on an owned window, a rejected launch,
or a surviving process. `run/verified-system-pair-stock.json` records
observed stock proof. The independent `stock-mac-exact-offline.json`
checks it against the original screenshots, raw process receipts and
retained acceptance. `stock-mac-v0172-passed` is written only after all
of those checks pass.
