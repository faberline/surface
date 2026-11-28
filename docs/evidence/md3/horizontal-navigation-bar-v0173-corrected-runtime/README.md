# Surface v0.1.73 navigation evidence

The source tree `791d8b2dd3f9b44f3c2a967f1418b671920a2899` has 692 files and uses base tree `8673365398b4c7b51f5e7409abe280d79b79d852`. The source manifest SHA-256 is `513d366accbb3ef9cd39120dc0a45bd98d033e0216c56613079011d0bf8e94dd`. The forward patch SHA-256 is `2d5f8b8561161332c9d792b4d0f7f8eed89478a5d24190174d5a2743117f746d`.

The workspace passed 1600 Rust tests and three WebAssembly checks. It ignored 20 Rust tests. Metal logs record 18 passing tests. This package includes 21 sealed Metal screenshots. The completed Gallery build records 2 native executable hashes and 3 Wasm builds.

The native replay verified 4 original PNGs in two matching replays, 20 zero-window samples and 4 reaped Gallery processes. Headless Chrome WebGPU passed 364 Material steps and 30249 checks. Three Browser PNGs capture a light layout at 839 dp and dark RTL at 720 dp. The interaction report records the 839, 600, 599 and 600 dp sequence and its focus and selection checks.

## Reproduce

Run these commands from the exact source tree. The archived replay helpers keep their original hashes.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
cap sh replay/native.sh --run /tmp/surface-md3-v0173-native-replay
cap sh replay/browser.sh --run /tmp/surface-md3-v0173-browser-replay
python3 -B portable-readback.py
```

The replay scripts retain the exact local paths and check source/build pins. Portable readback checks the package inventory, native and browser geometry, screenshot pixels and the recorded window observations. The offline comparison uses the JSON representation of the browser receipt. The native replay uses the Gallery’s light and dark themes. The build logs and manifests record the executable hashes.
