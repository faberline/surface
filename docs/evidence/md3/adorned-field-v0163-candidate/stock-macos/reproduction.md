# Surface v0.1.63 Adorned Text Field stock macOS observer

The verifier pins 675 paths in the candidate source. The manifest
SHA-256 is `2d671c1f3c4b450a21ab728a22ba0f8126af4790291a1b718d9005ab4f29eec2`.
The candidate tree is `3e2f280ed159c28f5b939813c4d4fd88d26803ff`.

Stock requires five ordered cases: `smoke ime material bridge-web devtools-headless`.
The Material case must reach 335 ordered steps through M336. The IME case
must reach all 20 steps, including I11. All seven predecessor pixel
verifiers are byte for byte copies of the v0.1.62 originals.

Both Chrome processes must use `--headless=new` and distinct private
profiles. The guard also observes Bridge Chrome after it detaches.
Native Gallery must use `--headless`. The observer rejects an owned
visible window, an owned process left after cleanup, and any macOS
WebGPU adapter other than Apple Metal.

The new verifier opens the six original screenshots. It checks the
Outlined edge and notch, the Filled indicator stripes and face, the
affixes and preedit regions, and the dark RTL layout. At 112 dp, it
checks the relocated 48 dp action and zero leftover ink in the gutter
beside the caption. It checks the ordered accessibility descriptions
and named action, the retained UTF-8 selection, trusted action events,
release-only click, locks, and the two-stage composition commit.

The real focused development run passed 243 checks and yielded the six
original PNGs. Its CoreGraphics receipt recorded 42 Chrome samples and
zero visible windows or survivors. The verifier accepted those inputs
and rejected 22 deliberate faults in source, log, runtime state or pixels.
The appended log control checks only the parser. The focused run does
not supply a stock verdict. Composition uses CDP input and records its
cancel and commit fallbacks. It does not measure a live system input
method or screen-reader speech.

```sh
lane=/tmp/surface-md3-adorned-field-stock-v0163.4j9v3w3j
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
harness_sha=$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/observe-adorned-field.py" --self-test
"$py" -E -B "$lane/check-controls.py"
sh "$lane/run-stock-exact.sh" "$harness_sha" --prepare
```

`--prepare` requires `native-final-built` and `stock-canonical-built`.
It copies and verifies the build inputs without starting an app.
Only the controller launches stock with the next command. The last
command checks the saved run without opening an app.

```sh
sh "$lane/run-stock-exact.sh" "$harness_sha" --run
sh "$lane/run-stock-exact.sh" "$harness_sha" --recheck
```

The live output stays in `run/`. Its verdict is
`run/verified-adorned-field-stock.json` with the `text_field_adornments`
key. The required offline verdict is `stock-mac-exact-offline.json`.
The marker `stock-mac-v0163-passed` means both passed.
