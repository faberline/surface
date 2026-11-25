# Surface v0.1.69 macOS stock headless guard

This lane prepares and checks the exact full macOS run. The five suites are
`smoke`, `ime`, `material`, `bridge-web` and `devtools-headless`. The Material
registry must run 352 ordered steps from M1 through M353, excluding M23. All
20 existing IME steps and the previous feature checks stay required.

## Inputs

- The final candidate has 688 files. It uses source manifest SHA-256
  `1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55`
  and tree `b243564b27c4fb8d2856afc5ae19cd4cb2ad905a`.
- The original v0.1.68 to v0.1.69 patch has SHA-256
  `fb2ad6586be0d5d1c7957ce53d91b37119a1c53bbe3e550488e9b24325c295ba`.
- The previous exact v0.1.68 archive has 400 sealed files. It provides the
  baseline stock run with 25,276 checks, 349 Material steps and 34 label
  motion and color PNGs. Its manifest SHA-256 is
  `5e19c8384a44e5ed31e5d61d1a7f03e14234476960b2f78ba6b78ba23fc37811`.
- The actual v0.1.69 R2 focused check passed 29 steps and 2,373 checks.
  It observed zero visible windows or surviving processes over 443 samples.
  Its independent receipt binds 34 retained PNGs and 58 new paired PNGs.
  Only `docs/material-coverage.json` changes between its original source and
  the final stock source. Its selector changes only the E2E registry. The
  exact stock Gallery runtime assets must equal the scoped Gallery assets.
- The review file must give the exact built stock runner SHA-256 and the
  exact native executable pair manifest SHA-256. The JSON example purposely
  contains invalid values for those fields. Both exact build markers must
  contain the final tree followed by one line break.

The guard checks the arguments before it starts each program. Both Chrome
processes use `--headless=new` and a private profile under `/tmp`. One belongs to
the detached Bridge. The native Gallery uses `--headless`. The only extra
Chrome switch disables the updater scheduler for these launches. The guard
samples owned processes and their CoreGraphics windows. A visible owned
window, a surviving owned process, unexpected flags or a failed suite fail
verification. The window receipt describes the samples actually taken.

## Commands

Keep the exact controller stage under `/tmp`. Use the local Python 3.12.11
installation and Pillow. The current scoped and prior stock evidence paths are
pinned in `stock_config.py`. The inherited readback is
`/tmp/surface-md3-sliders-stock.lK6a9gKl/verify-existing.py`, with its exact
retained checks and dependency manifests.

```sh
lane=/tmp/surface-md3-field-placeholder-stock-v0169.RECb56Zt
stage=/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY
pins=/tmp/v0169-reviewed-stock-pins.json
# Compare the manifests to their reviewed digests first.
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
pins_sha=$(shasum -a 256 "$pins" | cut -d' ' -f1)
sh "$lane/run-stock-exact.sh" "$harness_sha" --preflight
sh "$lane/run-stock-exact.sh" "$harness_sha" --prepare "$pins" "$pins_sha" \
  "$stage/exact-candidate" "$stage/text-field-placeholder-v0169.patch"
# Act only after the controller authorizes the actual headless stock run.
sh "$lane/run-stock-exact.sh" "$harness_sha" --run "$pins" "$pins_sha"
sh "$lane/run-stock-exact.sh" "$harness_sha" --recheck "$pins" "$pins_sha"
```

`--preflight` rechecks the real v0.1.68 archive, the final v0.1.69 source and the
actual scoped R2 receipt and pixels. It checks bad-input cases offline. Its
synthetic Material log tests only how the verifier reads the step ledger.
`--prepare` checks and copies the sixteen reviewed executable and Web files.
It copies the source, native, patch and scoped proof pins and reads the sealed
inputs back. Both commands leave applications closed.

## Result files

- `run/runner.log`, `run/invocation.json`, `run/inputs-after.json`,
  `run/headless-window-observation.json`, `run/owned-shutdown.json` and the
  original guard events record the stock run.
- `run/artifacts` keeps the typed placeholder report
  `material-text-field-placeholder.json` and 58 original paired PNGs, plus all
  predecessor artifacts, including the 34 label motion and color PNGs.
- `run/verified-placeholder-stock.json` checks actual browser and adapter,
  trusted client events, no placeholder pixels in disallowed phases, paired
  color, geometry, time, reduced motion, preedit scope and independent values.
- `stock-mac-exact-offline.json` independently rechecks the originals, including
  earlier Material components and IME I11. `stock-mac-v0169-passed` appears only
  after the guarded run and both checks pass. This prep package supplies no
  full v0.1.69 stock result. The focused run and native endpoint replay remain
  distinct evidence.
