# Surface v0.1.68 corrected label color replay

This bundle checks corrected commit
`e0b2a032f46a397265fee70136b471fbb5bcfd57` and tree
`72ae6c3eb56e9554532f428b3308aadd8358a242`. It checks the exact 685-file
manifest `086f06d98a7c2f3728b67c9524b15ed8b380fb078d42c48bb80b1f082c24580c`
and independently regenerates both reviewed correction patches.

Use the same toolchain and run the build after the corrected workspace
check releases the shared Cargo target. The original exact kit stays
separate. The corrected preflight rejects the original build receipt.

```sh
lane=/tmp/surface-md3-label-color-v0168-corrected-replay.2X3oZ1AB
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
sh "$lane/build-pinned.sh" --check-source
sh "$lane/build-pinned.sh" --build "$lane/outputs"
sh "$lane/build-pinned.sh" --verify "$lane/outputs"
sh "$lane/run-native.sh" --check-inputs "$lane/outputs"
"$py" -E -B "$lane/stock-fresh.py" --prepare "$lane/outputs" "$lane/stock"
"$py" -E -B "$lane/stock-fresh.py" --check-inputs "$lane/stock"
sh "$lane/run-native.sh" --run "$lane/outputs" "$lane/guarded-native"
"$py" -E -B "$lane/stock-fresh.py" --run "$lane/stock"
sh "$lane/run-native.sh" --verify "$lane/outputs" "$lane/guarded-native"
"$py" -E -B "$lane/stock-fresh.py" --verify "$lane/stock"
```

Run native and stock in sequence. Native keeps the original guards
for four reaped processes, 24 zero-window samples and 14 PNGs from
Page.screenshot, the CLI and MCP. It checks the new endpoint images
against the historical PNG hashes.

The original stock observer keeps all five suites and its typed
behavior, WebGPU, shape and pixel checks, including all 349 ordered
Material steps and M347–M350. Its input gate carries separate fields
for the older focused result and the freshly built corrected source.
The fresh reader requires the 34 motion and color PNGs plus retained
reports and images to match the previous archived hashes. It retains
the terminal run and lists exact artifact differences if parity fails.
The archived absolute-path predecessor reader remains unavailable.

The targeted Rust regression exercises the first enabled label frame.
The original browser cases continue to exercise their own label,
geometry, accessibility and transition scenarios. The new exact source
preflight and offline fixture tests passed. The fresh build and actual
headless runs are required for new runtime evidence.
