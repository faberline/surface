# Surface v0.1.75 disabled menu row correction

The correction changes one Runtime hit test and adds one regression test.
The sealed corrected tree has 698 files. Its exact patch matches the two
reviewed source files. The original source archive remains independent.
The corrected verifier requires the new source hash and rejects the original
published source and build manifests.

- Corrected tree `8046e25a630284411e1ebf9f9d16d1004b02651b`
- Corrected source manifest `8a2037d8b5fb11afcb226f5b27b41656d7c3dd428b09db9c4aaf2d711cacf2d0`
- Patch `4f1fb9d57083d7f36b7eec8900e657c1db42f182c80bd7351b74444c63a69de4`

## Build and verify

Use the existing macOS Rust and Wasm toolchain. All generated files go to
`/tmp`. The scripts reuse the corrected checkout's Cargo target and use
`cap` with two Cargo jobs.

```sh
lane=/tmp/surface-md3-disabled-menu-corrected-replay.ZIfGJCvi
sh "$lane/build-pinned.sh" --check-source
sh "$lane/check-workspace.sh" "$lane/workspace"
sh "$lane/build-pinned.sh" --build "$lane/outputs"
sh "$lane/build-pinned.sh" --verify "$lane/outputs"
```

The workspace script runs the repository's `scripts/check.sh`. The receipt
expects 1,607 passing tests, 20 ignored tests and three Wasm checks. Build
and workspace commands share one Cargo target. Run them in sequence.

The builder records these three commands and seals their actual artifacts
with the corrected tree and original toolchain output.

```sh
cap cargo xtask web build material-gallery --release --no-compress --out "$build_out/web/material-gallery"
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo build --locked --release -p surface-material-gallery --features surface-material-gallery/devtools -p surface-devtools
```

## Run with no visible windows

```sh
sh "$lane/run-guarded.sh" native --check-inputs "$lane/outputs"
sh "$lane/run-guarded.sh" browser --check-inputs "$lane/outputs"
sh "$lane/run-guarded.sh" native --run "$lane/outputs" "$lane/guarded-native"
sh "$lane/run-guarded.sh" browser --run "$lane/outputs" "$lane/guarded-stock-material"
sh "$lane/run-guarded.sh" native --verify "$lane/outputs" "$lane/guarded-native"
sh "$lane/run-guarded.sh" browser --verify "$lane/outputs" "$lane/guarded-stock-material"
```

Run native and browser in sequence. The native replays require three
profiles, two repetitions, six reaped Gallery processes and eight matching
original PNGs. The browser requires one private headless launch, all 371
ordered stock Material steps and no visible owned windows or surviving
processes. It checks the two menu reports and six original PNGs. The Rust
regression covers a real dispatch at the disabled row, a transparent group
gap and an outside point. The stock browser and native scenarios retain
their original coverage.

Source verification, pure guard checks and four original-source/build
rejection controls passed offline. New workspace and runtime receipts come
from the controller's runs. Store the manifest, patch, toolchain and
successful reports in the corrected evidence record. Keep cached build
outputs, executables and browser profiles in local scratch.
