# Surface v0.1.64 Character Counter stock macOS observer

The exact source has 678 paths. Its manifest SHA-256 is
`fe7349b293de3078ca44fd6b1fa1885a5fa156ba559503c795cd9d6daab5e4a3`.
Its tree is `eeef2dc049409f7df3a67773d8e72e76b298faf6`.

The observer runs the five stock cases in order.
`smoke ime material bridge-web devtools-headless`
It requires all 339 Material steps through M340 and all 20 IME steps,
including I11. The eight prior image verifiers are byte-for-byte
copies of the sealed v0.1.63 tools.

Chrome and Bridge Chrome must use `--headless=new` and separate private
profiles. The observer samples Bridge Chrome after detach. Native
Gallery must use `--headless`. The observer rejects a visible owned
window or leftover owned process and requires Apple Metal for WebGPU.

The counter verifier decodes seven original PNGs. It measures the 4 dp
face-to-help gap, the 16 dp insets and parallel gap, and the 112 dp
stacked examples with a separate 48 dp action. It checks actual glyph
ink for ordinary, overflow, manual error and disabled states. It also
checks the passive count description, over-limit submit, two distinct
live validation messages, trusted Escape and Enter events, and the
separate synthetic composition commit. It does not claim live system
IME or screen-reader speech.

The earlier focused run supplied 308 checks, seven PNGs, 50 Chrome
window samples and zero visible windows or survivors. The decoder
accepted those artifacts and rejected 25 deliberate faults. The
synthetic log control checks parser behavior only. The stock verdict
comes from a separate run with the exact stock binaries.

```sh
lane=/tmp/surface-md3-counter-stock-v0164.ZBB8Yftn
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
seal=$(shasum -a 256 "$lane/harness.sha256" | awk '{print $1}')
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/observe-counter.py" --self-test
"$py" -E -B "$lane/check-controls.py"
sh "$lane/run-stock-exact.sh" "$seal" --prepare
```

`--prepare` requires both exact build markers. It verifies pinned inputs
without launching an app. The controller starts the run and reads it
back with these commands.

```sh
sh "$lane/run-stock-exact.sh" "$seal" --run
sh "$lane/run-stock-exact.sh" "$seal" --recheck
```

The live verdict is `run/verified-counter-stock.json`. The offline
verdict is `stock-mac-exact-offline.json`. Both contain
`text_field_counter` with measured geometry,
image checksums, and the synthetic commit and cancel flags. The
`stock-mac-v0164-passed` marker means both checks passed.
