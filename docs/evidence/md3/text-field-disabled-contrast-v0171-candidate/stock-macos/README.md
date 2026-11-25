# Surface v0.1.71 exact macOS stock headless guard

The guard requires the five stock suites `smoke`, `ime`, `material`,
`bridge-web` and `devtools-headless`. Material must complete 358 ordered
steps through M359 with M23 omitted. The gate retains all 20 IME steps,
the transport checks, and the predecessor pixel and semantics decoders.
The Browser processes require `--headless=new` and private profiles for
primary and detached Bridge Chrome. The native Gallery requires its
existing offscreen arguments. The guard fails on an owned visible window,
a rejected launch, or an owned process still present after shutdown.

The exact final source has 690 files. Its source manifest, tree and patch
are pinned in `reviewed-pins.example.json`. The final and R2 runtime
source differ by `docs/material-coverage.json` only. R2 kept the R1 Web
build. The recorded R1-to-R2 differences are two test modules only.
The exact stock Gallery must match the four Web assets from the measured
R2 journey byte-for-byte.

The .70 portable predecessor uses seal
`5768509d54c91d83054cf12afff6a3eee8070c333dfa29a5324ee67077fc5a48`.
It was published as evidence commit `f2fdd247f8365fe50d63157e04dc1065f4baf9f3`.
Its guarded run completed 28,706 checks and 355 Material steps. The
.70 offline replay established how to bind canonical Web files when the
stock bundle links to them. The .71 guard validates all stock bundle
origins and opens the original regular Gallery files for the strict asset
comparison. It refuses altered link targets, filenames or hashes.

## Offline preflight

The offline preflight validates exact source and delta, the sealed .70
archive, original predecessor screenshots, the actual .71 R2 receipt,
and the independent screenshot decoder on all 15 R2 images. It checks
1,470 scoped checks, ten Material steps, 162 samples, zero windows and
no survivors. It also tests rejection paths and verifies the real .70
bundle links without launching a process under test. This proves the
harness behavior and the scoped run. Full .71 stock remains a separate gate.

```sh
lane=/tmp/surface-md3-field-contrast-stock-v0171.PykYn5L5
stage=/tmp/surface-md3-field-contrast-v0171.xE0ruhQh
candidate="$stage/exact-candidate"
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
# Compare harness_sha to the reviewed seal before proceeding.
sh "$lane/run-stock-exact.sh" "$harness_sha" --preflight
```

## Exact stock run

After the controller finishes the native and canonical Web builds, review
both build digests and fill the two placeholders in the example. Both
build markers must contain the exact final tree and a newline.
The stock lane links the approved build outputs in place. Keep those
outputs available for the run and its offline recheck. The `--prepare`
command checks input bytes and writes receipts. It launches no suite.

```sh
pins="$stage/reviewed-v0171-stock-pins.json"
jq --arg runner "$(shasum -a 256 "$candidate/web-canonical/debug/surface-e2e" | cut -d' ' -f1)" \
   --arg native "$(shasum -a 256 "$candidate/binaries/SHA256SUMS" | cut -d' ' -f1)" \
   '.stock_runner_sha256 = $runner | .native_manifest_sha256 = $native' \
   "$lane/reviewed-pins.example.json" > "$pins"
pins_sha=$(shasum -a 256 "$pins" | cut -d' ' -f1)
sh "$lane/run-stock-exact.sh" "$harness_sha" --prepare "$pins" "$pins_sha" \
  "$candidate" "$stage/text-field-disabled-contrast-v0171.patch"

# Run locally only after controller authorization and GPU coordination.
sh "$lane/run-stock-exact.sh" "$harness_sha" --run "$pins" "$pins_sha"
sh "$lane/run-stock-exact.sh" "$harness_sha" --recheck "$pins" "$pins_sha"
```

## Evidence output

`run/verified-disabled-contrast-stock.json` records the guarded runtime
result, actual Apple Metal and Chrome metadata, and the independent
stock artifact checks. `stock-mac-exact-offline.json` independently
rechecks the raw suite logs, guard ledger, predecessor reports, assets
and window receipt. `stock-mac-v0171-passed` exists only after both gates
and the final receipt predicate pass.

`text_field_disabled_contrast` covers five field scenes in normal,
forced and restored states. The decoder reads original Canvas pixels,
bounds and measured ink counts. It verifies exact opaque CSS GrayText
on disabled parts, independent ordinary read-only ink, stable lock
semantics, an unchanged focused draft and directed selection, and normal
ink restoration. Normal and restored captures can have identical bytes.
The new proof covers headless Chrome media emulation and Canvas paint.
It does not exercise live assistive speech or host preference changes.
