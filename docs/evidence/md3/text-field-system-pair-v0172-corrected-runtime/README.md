# Surface v0.1.72 system-pair evidence

The target tree `8673365398b4c7b51f5e7409abe280d79b79d852` has 691 files.

The source manifest SHA-256 is `a59fd437cbd2f3cea304708ff2e95752fd70e8f1ec1fe2004a1289903d6a6f49`.

The base tree is `e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc`.

The forward patch SHA-256 is `cb7bba3fb3d7fa6614501a69ee578b9325010f9c96955057d845eae58dbb358c`.

The source patch has 19 sections. The archive preserves the exact patch, changed-file list, target manifest and both tree records.

## Evidence

The workspace log records 1,595 passing Rust tests, 20 ignored Rust tests and three WebAssembly checks. The GPU logs record 18 passing tests and the archive includes the 21 sealed GPU PNGs.

The native run contains 16 original PNGs and passed two-run pixel readback. It uses the Gallery’s fixed `field-pair` preview palette. Verification of the live macOS appearance setting remains open. The archive keeps the native report and exact SHA seal. The native binary hashes are in `build/native/SHA256SUMS`.

Headless Chrome WebGPU passed 361 Material steps and 29626 checks. It records the q70 boundary and q71 contrast images by hash. The light and dark cases use the Chrome DevTools Protocol to emulate system colors. This archive stores only q72's 14 `system-pair` PNGs and typed report. The original full run was verified before packaging.

## Reproduce the test runs

Use the target source and the saved build toolchain and paths.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
sh replay/build.sh --check-source
cap sh replay/native.sh --run /tmp/surface-v0172-native-replay
cap sh replay/browser.sh --run /tmp/surface-v0172-browser-replay
```

## Portable readback

After extraction, run `python3 -B portable-readback.py`. It checks the package hash list, the native saved PNGs, and q72's 14 original Browser images and report. The readback runs offline with Python and Pillow.

To check the compressed archive and its extracted files:

```sh
shasum -a 256 -c surface-md3-v0172-evidence.tar.gz.sha256
tar -tzf surface-md3-v0172-evidence.tar.gz
tar -xzf surface-md3-v0172-evidence.tar.gz
cd surface-md3-v0172-evidence
shasum -a 256 -c SHA256SUMS
python3 -B portable-readback.py
```
