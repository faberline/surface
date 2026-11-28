# Surface v0.1.69 evidence

Source tree: `5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb`

Source manifest SHA-256: `3eed0f67f0440d4cc37ba48843857e45ad165188f174ebaf2dacf62fd19c426f`

Forward patch SHA-256: `d798f16db3cf693e003da19775f48762e0108916a8e71a00643e6e1b85d497dc`

Exact base commit: `26758a7313527c9aab7c2d5e677ec880866638da`

The patch reconstructs the target tree from the v0.1.68 base commit above. Check the base and patch first, apply the patch, then verify the resulting tree equals `5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb`.

- The workspace passed 1584 Rust tests and three WebAssembly checks.
- Apple M1 Max passed 18 offscreen Metal tests and wrote 21 PNGs.
- Two native replays wrote 40 matching PNGs through Page.screenshot, CLI and MCP. The runs sampled no visible windows and reaped all four Gallery processes.
- Headless Chrome WebGPU passed 352 Material steps and 25570 checks. The direct verifier checked 58 placeholder PNGs. The run sampled no visible windows and found no surviving test processes.

Browser PNGs included: 58 (5524285 bytes total for the full set; full set limit is 15 MiB). The browser report and direct verifier output preserve hashes for all 58 images.

## Reproduce

Use the pinned source manifest and forward patch to reconstruct the tested source. The original logs preserve exact test output. The guarded runs used the following commands.

```sh
cap sh scripts/check.sh
sh /tmp/surface-md3-v0169-replay.WfBRFHlH/build.sh --build /tmp/surface-md3-v0169-replay.WfBRFHlH/outputs
sh /tmp/surface-md3-v0169-replay.WfBRFHlH/native.sh --run /tmp/surface-md3-v0169-replay.WfBRFHlH/native-run
sh /tmp/surface-md3-v0169-replay.WfBRFHlH/browser.sh --run /tmp/surface-md3-v0169-replay.WfBRFHlH/browser-run
/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3 -E -B /tmp/surface-md3-v0169-replay.WfBRFHlH/verify-placeholder.py --artifacts /tmp/surface-md3-v0169-replay.WfBRFHlH/browser-run/artifacts
```

The build and replay scripts in `replay/` record the original temporary paths. Use the manifest and toolchain notes to reproduce the layout. The build and binary manifests are in `build/`.

## Portable archive readback

```sh
shasum -a 256 -c portable.tar.gz.sha256
tar -tzf portable.tar.gz
tar -xzf portable.tar.gz
cd surface-md3-v0169-evidence
shasum -a 256 -c SHA256SUMS
python3 -E -B replay/verify-placeholder.py --artifacts browser/artifacts
```
