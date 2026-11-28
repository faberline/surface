# Surface v0.1.68 R3 exact stock replay

R3 keeps the original stock suites and uses the verified R2 Web, native
and Jet Bridge artifacts. Its test patch lets the actual focus change
paint at the locked time before the 44 ms sample. Preflight verifies the
685-file R2 and R3 source snapshots, the pinned patch and the original
runner build log. Only the browser test file differs.

```sh
lane=/tmp/surface-md3-label-color-v0168-r3-stock.DhTi2bwy
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
"$py" -E -B "$lane/stock-r3.py" --check-inputs "$lane/stock"
"$py" -E -B "$lane/stock-r3.py" --run "$lane/stock"
"$py" -E -B "$lane/stock-r3.py" --verify "$lane/stock"
```

The stock stage is prepared. For a repeat use `--prepare` with a new
private directory. The original observer checks all five macOS suites,
349 ordered Material steps with M23 retired, real Apple Metal WebGPU,
and the original typed behavior and pixel oracles. It requires one
primary headless Chrome, one headless Bridge Chrome and one headless
Gallery, with zero visible owned windows and zero surviving processes.

The M350 receipt checks four fixed glyph probes in the original compact
PNG and requires the typed and actual ink `[206, 191, 243]` at 44 ms. A
separate report compares the 34 historical image hashes. The comparison
keeps differences visible while the live test and pixel oracles decide
the result.

The observer writes `stock/run/verified-label-color-stock.json` and the
independent reader writes `stock/verified-r3-stock.json`. The latter
includes exact R2 product and R3 test-runner pins, the M350 receipt and
the historical comparison. Keep the invocation, guard ledger, window
observation, original reports and PNGs with the measured result.

Exact offline preflight passed. The fixture accepted the target probes,
rejected a 45 ms result and rejected the actual earlier R2 idle pixels.
The controller runs the live stock suites.
