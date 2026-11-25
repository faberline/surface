# Surface v0.1.73 exact macOS stock headless gate

This lane is prepared offline. It pins 692 exact source files, two build
markers, four executables and twelve canonical Web files. It checks the three
published Web descriptors against their payload hashes and byte counts.
All four release Gallery files equal the independently verified R3 scoped
Gallery. The source diff from scoped to stock is the cumulative coverage doc
and one inherited Field-pair test file. The actual runner remains stock.

The guard can launch only the named five suites in sequence. It requires
`--headless=new` and private profiles for both Chrome processes, and
`--headless` for the native Gallery. CoreGraphics reads owned windows only.
A process-local Chrome flag, `--disable-updater-scheduler`, remains explicit
and recorded. The Compact sentinel denies execution. The full runner must
complete 364 ordered Material steps through M365 and all 20 IME steps, with
the existing independent raw report and pixel validators still in the gate.

The new Navigation decoder checks four Compact and three Horizontal original
PNGs. It verifies actual Chrome/WebGPU metadata, accessible names and
current page, destination order and sizes, keyboard and cancelled touch,
599/600 handoff, selected/focus paint separation, and dark RTL wheel captures.

## Controller commands

```sh
lane=/tmp/surface-md3-horizontal-nav-stock-v0173.5buwihck
pin=00c57cab4d7ef87483ef9df1a89dfac32d7f79a7d2605394b8829dafa3e4281a
harness=REVIEWED_HARNESS_SHA256
sh "$lane/run-stock-exact.sh" "$harness" --preflight
```

The controller must use the reviewed harness hash in the handoff. The lane
already has `inputs-rechecked.json` matched with `inputs-before.json`.

```sh
sh "$lane/run-stock-exact.sh" "$harness" --run "$lane/authority-pins.json" "$pin"
sh "$lane/run-stock-exact.sh" "$harness" --recheck "$lane/authority-pins.json" "$pin"
```

Only a real run can create `stock-mac-v0173-passed`. Its original verdict is
`run/verified-horizontal-navigation-stock.json` and independent receipt is
`stock-mac-exact-offline.json`. Existing q72 and R3 originals are checked
read-only. Neither receipt qualifies this full stock gate on its own.
