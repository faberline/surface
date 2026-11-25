# Surface v0.1.70 exact macOS stock headless guard

This lane verifies the five stock suites `smoke`, `ime`, `material`,
`bridge-web` and `devtools-headless`. The full Material case must run
355 ordered steps, M1 through M356 with M23 omitted. All 20 IME steps,
the existing transport checks, and all predecessor feature decoders
remain part of the gate.

## Exact inputs

- The 689-file final source manifest is
  `bc6f3023d68c8bec8c801c7a9a2d7c88207d625697cffb34f1eb4f5fef5a6d85`.
  The source tree is `409a21f6023802f055371cdec4391976c55ffd17`.
  The patch is
  `ca4fe795f6a347be0ddd6296b7bca2537ba70b645b93cfc641228f833f31095f`.
- Actual R1 focused Chrome passed 32 steps and 5,228 checks, with
  547 observations, no visible windows and no owned survivors. It
  supplied 33 boundary PNGs, 34 label and 58 placeholder PNGs. Its
  exact source differs from the final source only in
  `docs/material-coverage.json`. Its private runner selects one scoped
  registry. The final stock Gallery must equal the four tested scoped
  Gallery assets byte-for-byte. The scoped evidence is not used as a
  substitute for full stock execution.
- The original prior stock proof is the verified 654-file portable
  .69 archive, seal
  `6e6069ce438e25821707ad230972f3a435397cdc98d054b36fddeb8ad774d4e8`.
  Its real stock run passed 25,851 checks and 352 Material steps.
  Preflight compares available local .69 raw receipts to the portable
  originals by hash. The old observer directory is a corroborating
  record, while the portable files supply the independent predecessor.
- The review file must contain the actual stock-runner digest and
  actual native-pair manifest digest, both read from the approved
  exact build. The example deliberately leaves these invalid. The
  native and canonical Web build markers must both contain the final
  source tree and a newline.

The guard accepts two Chrome processes with `--headless=new`, private
separate profiles and the tested launch-only Chrome switch
`--disable-updater-scheduler`. It verifies the detached Bridge process
and the existing offscreen native Gallery flags and checks CoreGraphics
window ownership of all observed processes. It blocks a compact
native sentinel. A guard rejection, observed owned window or surviving
owned process fails the run.

## Controller commands

The first command checks the sealed tools and runs offline only. The
stock guard reads four exact binaries and three four-file Web bundles
in place through symbolic links. Preserve the underlying controller
build outputs while the guard uses them. It copies only small source,
hash and review files. Neither the preflight nor preparation launches
a process under test. Reserve GPU time for the actual stock run.

```sh
lane=/tmp/surface-md3-field-boundary-stock-v0170.Mz6M22zL
stage=/tmp/surface-md3-field-boundary-v0170.X35mBqoS
candidate="$stage/exact-candidate"
pins=/tmp/v0170-reviewed-stock-pins.json
# Compare the digest to the independently delivered harness seal.
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
sh "$lane/run-stock-exact.sh" "$harness_sha" --preflight

# After the exact native and canonical Web builds complete,
# review both digests and fill the two placeholders in the example.
jq --arg runner "$(shasum -a 256 "$candidate/web-canonical/debug/surface-e2e" | cut -d' ' -f1)" \
   --arg native "$(shasum -a 256 "$candidate/binaries/SHA256SUMS" | cut -d' ' -f1)" \
   '.stock_runner_sha256 = $runner | .native_manifest_sha256 = $native' \
   "$lane/reviewed-pins.example.json" > "$pins"
pins_sha=$(shasum -a 256 "$pins" | cut -d' ' -f1)
sh "$lane/run-stock-exact.sh" "$harness_sha" --prepare "$pins" "$pins_sha" \
  "$candidate" "$stage/text-field-boundary-v0170.patch"

# Run locally headless only after the controller authorizes this gate.
sh "$lane/run-stock-exact.sh" "$harness_sha" --run "$pins" "$pins_sha"
sh "$lane/run-stock-exact.sh" "$harness_sha" --recheck "$pins" "$pins_sha"
```

## Results and pixel coverage

- `run/verified-boundary-stock.json` proves actual adapter, process
  observations, stock suites and the full guarded raw artifact set.
- `stock-mac-exact-offline.json` rechecks the raw files independently,
  including I11, existing scalar RGB and pixel expectations, IME,
  all previous Material component reports, and PNG checks.
- `text_field_boundary_motion` adds 33 original PNGs and 2,478 decoded
  probes. Each width comes from independent physical coverage. The
  full samples also match independent timed Oklab color. At 14/44 ms,
  Covered quarters are Filled 10/16 and Outlined 11/20. One blend
  convention fits all fractional samples.
- `text_field_label_motion` keeps 34 originals, exact previous
  glyph/AX/layout checks, and the intended new M347 boundary state.
  The early shallow cut at 0.25/0.75/1.25 dp has 0/0/1 covered
  quarters. The uncut control has 4/4/3. Both were independently
  observed in the real R1 images before being required in stock.
  The 58 paired placeholder originals keep the same decoder bytes.
- `stock-mac-v0170-passed` is written only after an accepted full
  run. This setup neither starts stock nor claims it passed.
