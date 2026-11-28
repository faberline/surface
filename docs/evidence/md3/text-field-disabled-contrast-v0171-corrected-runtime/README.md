# Surface v0.1.71 disabled-field contrast evidence

The source tree `e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc` has 690 files.

The source manifest SHA-256 is `7ff2289093941bab73f8f718492d800b8a9c2bd33c3a03aae0f2396b6d03a65e`.

The base tree is `87f50f8095b38a4ba37c83f71aaf1119dda945dc`.

The forward patch SHA-256 is `755a7166838c38895069c440e79ffe1e5df6baed17b8996f9d014d4c2710f6f2`.

The patch reconstructs the target tree from the exact q70 base tree above. The archive keeps the patch, target source manifest, both tree records, and the manifest checks captured before and after the workspace run.

## Captured evidence

- The workspace passed 1,589 Rust tests and three WebAssembly checks. It ignored 20 Rust tests. Apple Metal passed 18 offscreen tests.
- Two native replays used the Gallery’s fixed light and dark palettes. They produced 24 matching PNGs through Page.screenshot, CLI and MCP, with 78 zero-window samples and six reaped processes. Verification of the live macOS appearance setting remains open.
- Headless Chrome WebGPU passed 358 Material steps and 29103 checks. Its offline verifier checks the 33 text-field boundary images and the 15 disabled-contrast images, their reports, and sampled pixels.
- The build manifest records the exact native and Web toolchain inputs and binary hashes.

## Reproduce the test runs

Use the target source, the saved toolchain and the recorded replay paths.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
sh replay/build.sh --check-source
cap sh replay/native.sh --run /tmp/surface-v0171-native-replay
cap sh replay/browser.sh --run /tmp/surface-v0171-browser-replay
```

## Portable readback

After extraction, run the verifier from the package folder.

```sh
python3 -B blind-readback.py
```

The verifier reads the package inventory, native receipt and both browser image sets offline. The Browser verifier needs Python Pillow.

To verify the compressed archive before extraction:

```sh
shasum -a 256 -c surface-md3-v0171-evidence.tar.gz.sha256
tar -tzf surface-md3-v0171-evidence.tar.gz
tar -xzf surface-md3-v0171-evidence.tar.gz
cd surface-md3-v0171-evidence
shasum -a 256 -c SHA256SUMS
python3 -B blind-readback.py
```
