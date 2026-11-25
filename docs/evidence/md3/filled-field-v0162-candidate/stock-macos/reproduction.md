# Surface v0.1.62 Filled Text Field stock macOS observer

This observer pins all 670 staged source files and the native Gallery,
native DevTools, stock browser runner, Jet Bridge and three Web bundles.
The image and source check uses the exact staged source SHA-256
`78ae815e1c6407c91c65b13219e07b2c6f2298eb5218530670fad26f70a3dabe`
and tree `7266ce0588d5bc2d83a4151ca2e13c780153e279`.

Stock passes only for the five ordered cases `smoke ime material bridge-web
devtools-headless`, including 331 ordered Material steps through M332 and
all 20 IME steps. The six existing feature validators match the sealed
v0.1.61 R2 files byte for byte. Chrome and Bridge Chrome must use
`--headless=new` with distinct owned profiles. The native Gallery must
use `--headless` and have no display. The compact executable guard rejects
every launch. The observer checks that neither a visible owned window nor
an owned survivor exists and that the stock adapter is actual Apple Metal.

The Filled report reopens and decodes all six original Chrome PNGs.
It measures both bottom indicator stripes, both top corners, the square
bottom, hover and disabled color blending, caption and lower value ink,
and 96 dp dark RTL 200% geometry. It also verifies the actual Chrome AX
description and invalid link, assertive deliveries, read-only and disabled
behavior, and Back focus. The CDP composition proof records two trusted
Enter events and the real submit counts. It records whether synthetic
`insertText` completed the composition before the second Enter.

The controller's separate 235-check focused run and six artifacts passed
the independent decoder. `offline-controls.json` records the scoped result
and the exact predecessor seal. The controls reject 16 deliberate defects,
including a changed interior PNG pixel, stale prior source, missing M324,
M328, M332 or I11, premature submit and a lost AX description. A synthetic
log only tests the required parser structure. No proof here claims live
IME, screen-reader speech or an actual stock result before the stock run.

```sh
lane=/tmp/surface-md3-filled-field-stock-v0162.X1DV91RH
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/observe-filled-field.py" --self-test
"$py" -E -B "$lane/check-controls.py"
sh "$lane/run-stock-exact.sh" "$harness_sha" --prepare
sh "$lane/run-stock-exact.sh" "$harness_sha" --run
sh "$lane/run-stock-exact.sh" "$harness_sha" --recheck
```

Pinning waits for `native-final-built` and the explicit uncompressed
`stock-canonical-built` markers. The Web assets come from `web-canonical`. `--prepare` starts no
app and compares the observer's independent input checks. Only the
controller runs `--run`. Both the run and recheck require matching stock
inputs. The run stays at `run/`, including its original log and observer
receipt. The live verdict is `run/verified-filled-field-stock.json` with
the `filled_text_field` key. The offline replay is
`stock-mac-exact-offline.json` at the lane root. Its matching step and
source pins create the `stock-mac-v0162-passed` marker.
