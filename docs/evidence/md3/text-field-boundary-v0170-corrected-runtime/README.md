# Surface v0.1.70 text-field boundary evidence

The target source tree is `87f50f8095b38a4ba37c83f71aaf1119dda945dc`.

The source manifest SHA-256 is `06b2b09c23fb09cc0b28c56f27b1097a5abd3d962d4866b33b0b38764b4db661`.

Apply `forward-from-v0169.patch` to the base tree `5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb`.

The forward patch SHA-256 is `194d989f86b0dce2a7bc8190e792df784930ad4fcfaf53eb3277ad2ed326e3ed`.

The patch reconstructs the target source from the exact base tree above. Verify the patch hash, apply it to that base, then run `replay/source-check.py` with the archived target source manifest and compare the resulting tree with the target tree. The source manifest lists the exact target files.

## Captured results

- The focused regression failed on the old behavior, then all 19 outlined text-field tests passed. See `tests/red.log` and `tests/green.log`.
- The corrected workspace run passed 1,587 tests and three WebAssembly checks. The workspace run ignored 20 tests. All three WebAssembly release builds completed separately.
- Offscreen Metal and text-selection GPU logs show 18 passing tests. The archive includes all 21 original Metal PNGs and their source manifest.
- Two native replays passed. They include 18 PNGs, 48 zero-window samples, and four reaped processes. Page.screenshot, CLI and MCP returned identical bytes for each capture.
- Headless Chrome WebGPU passed 355 Material steps and 28425 checks. The offline verifier checked 2424 pixels in the 33 boundary PNGs. The post-run verifier initially lacked two imports. The corrected verifier read the original test log, window receipt and PNGs offline.

## Targeted regression commands

```sh
cap cargo test --locked -p surface-material --test outlined_text_field field_boundary_uses_enabled_ink_on_first_frame_after_reenable -- --exact
cap cargo test --locked -p surface-material --test outlined_text_field
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
```

The captured runtime commands are `replay/native.sh --run <new-output>` and `replay/browser.sh --run <new-output>`. The package preserves the exact replay tools, build manifests, toolchain, logs, source manifests, and original run receipts. For offline readback, `replay/replay-boundaries.py --verify-stored` verifies the saved native captures. `replay/browser.sh --verify <browser-run>` verifies the saved Browser receipts and pixels without launching Chrome or the app.

## Portable archive readback

```sh
shasum -a 256 -c surface-md3-v0170-evidence.tar.gz.sha256
tar -tzf surface-md3-v0170-evidence.tar.gz
tar -xzf surface-md3-v0170-evidence.tar.gz
cd surface-md3-v0170-evidence
shasum -a 256 -c SHA256SUMS
python3 -E -B replay/browser-evidence-readback.py browser
```
