# Surface v0.1.74 adaptive navigation evidence

Source tree `2532449d43fdce54448a5535cb6fcd5cb5ff6a79` has 696 files. Its source manifest SHA-256 is `53fb1fd46ab0b360186238bc3a946862c16047646e4578fc25dff9d308811f8f`. It follows base tree `791d8b2dd3f9b44f3c2a967f1418b671920a2899` with the 13-section patch `15d0da700514f4b5d1518217f81581c8588ab5b65da085a9b2c72467f1dd50ed`.

The workspace passed 1605 Rust tests and three WebAssembly checks. It ignored 20 Rust tests. The Metal logs record 18 passing tests and include 21 sealed original PNGs. The build logs record 3 WebAssembly release builds and the native executable hashes.

Two native replays at fixed widths of 320, 600 and 840 dp produced 6 matching PNGs through Page.screenshot, CLI and MCP. The verifier checked the original pixels and geometry. It observed 30 zero-window samples and 6 reaped Gallery processes.

Headless Chrome WebGPU passed 368 Material steps and 30449 checks. The four adaptive-navigation PNGs and interaction report cover live resize, continued editing, keyboard focus and rail scrolling. The proof retains the original input pins and window observations.

## Reproduce

These commands use the exact local source and saved replay helpers. The readback command works after extracting this package.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
cap sh /tmp/surface-md3-v0174-replay.jLEUA2mj/native.sh --run /tmp/surface-md3-v0174-native-replay
cap sh /tmp/surface-md3-v0174-replay.jLEUA2mj/browser.sh --run /tmp/surface-md3-v0174-browser-replay
python3 -B portable-readback.py
```

Portable readback checks the package inventory, verifies six native original images and their geometry, checks saved Browser input and window records, and verifies all four q74 Browser screenshots. The readback runs offline.
