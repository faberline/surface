# Surface v0.1.75 evidence

Source tree `a47ceb4426cd8bc6eb02f1792748fdced9524793` has 698 files. Source manifest SHA-256: `3946a01298f055f264e759e574af373c202945bfde85a002ef02b948bf25274a`. Base tree: `2532449d43fdce54448a5535cb6fcd5cb5ff6a79`. Forward patch SHA-256: `ceb92751972c68c350ee5f1fcdeb43fadf2374f0ea699649d4c3fabb653b0677`.

Workspace: 1,609 passed, 20 ignored, and 3 Wasm builds. Metal: 18 passed; 21 sealed screenshots. Build records and binary hashes are included.

Native replay: 8 original PNGs across two repeats; 46 zero-window samples and 6 reaped processes. The verifier checks Page.screenshot, CLI and MCP captures.

Chrome verified 371 Material steps and 30,597 checks. The archive includes three Baseline command-menu PNGs, three Expressive vertical-menu PNGs, and both menu reports. It also includes the before/after input pins and process and window records.

## Reproduce

The build command below uses `cap` and creates a new build output directory. The native and Browser commands use the already verified build at `/tmp/surface-md3-v0175-replay.71RwhtQh/outputs`. They write new run receipts to fresh `/tmp` directories.

```sh
cd /tmp/surface-md3-v0175-current-base.rWr9qUiy/source
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap sh /tmp/surface-md3-v0175-replay.71RwhtQh/build.sh --build /tmp/surface-md3-v0175-build-repro
cap sh /tmp/surface-md3-v0175-replay.71RwhtQh/native.sh --run /tmp/surface-md3-v0175-native-rerun
cap sh /tmp/surface-md3-v0175-replay.71RwhtQh/browser.sh --run /tmp/surface-md3-v0175-browser-rerun
python3 -B portable-readback.py
```

Portable readback checks the archive seal, all eight native PNGs and geometry, Browser input pins, window evidence, and all six menu PNGs.
