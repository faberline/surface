# Surface v0.1.76 evidence

Source tree `b61630995f470772785605a944e3515a45b6d553` has 703 files. Source manifest SHA-256: `84bb4b9da77641b89a990c703bf59789f7f14657cfe97b60592cb3bbd9cfdb02`. Base tree: `a47ceb4426cd8bc6eb02f1792748fdced9524793`. Forward patch SHA-256: `46494d3c7a2d0c18aeabde438ba186e70bebf88fb1120f0f077d1a2e45b83e0c`.

Workspace records 1,617 passing tests and 20 ignored tests. Metal records 18 passing tests with 21 sealed screenshots. The build records three Web builds and two native executable hashes.

The detached AppKit selector run completed. Its exact Cargo lock, test crate, log and completion marker are included.

The native run verifies eight original PNGs in two repeats using Page.screenshot, CLI and MCP. It records 46 zero-window samples and six reaped processes. The Browser run verified 374 Material steps and 30,965 checks. It includes three typed reports and nine original screenshots for M373–M375, with the run inputs and window observations.

## Reproduce

The build command writes to a fresh output path. Native and Browser replay commands use the verified build in the exact replay lane and write fresh evidence paths.

```sh
cd /tmp/surface-md3-v0176-forward.pjxmpm8l/strict-replay
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap sh /tmp/surface-md3-v0176-current-replay.qSPAab9k/build.sh --build /tmp/surface-md3-v0176-build-repro
cap sh /tmp/surface-md3-v0176-current-replay.qSPAab9k/appkit.sh /tmp/surface-md3-v0176-appkit-rerun
cap sh /tmp/surface-md3-v0176-current-replay.qSPAab9k/native.sh --run /tmp/surface-md3-v0176-native-rerun
cap sh /tmp/surface-md3-v0176-current-replay.qSPAab9k/browser.sh --run /tmp/surface-md3-v0176-browser-rerun
python3 -B portable-readback.py
```

Portable readback verifies source and package hashes, the AppKit marker and lock, native geometry/pixels, and Browser reports, images, input pins and window proof.
