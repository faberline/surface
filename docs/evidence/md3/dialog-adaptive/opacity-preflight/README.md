# Isolated opacity before Dialog motion integration

The renderer accepts balanced `BeginOpacityGroup { opacity }` and `EndOpacityGroup` commands. It draws a partly opaque group into a transparent texture. It then blends the complete group into the target once. This keeps overlapping text, shapes and images at the correct opacity. The same temporary texture serves sequential groups. The renderer validates the order and alpha of each group. Full opacity uses the ordinary paint path.

The exact workspace gate passed 1,270 tests, provenance, formatting, strict Clippy and WebAssembly checks. Twelve offscreen GPU tests passed on Apple M1 Max Metal. The group test compares actual overlap, clip and alignment pixels in two target formats at device scales 1 and 2. It verifies the zero and full-opacity endpoints, sequential layers and invalid commands. The two Metal replays produced the same representative PNG bytes. [Workspace log](workspace-final.log) · [Metal log](gpu-final.log) · [GPU output](gpu/gpu-opacity-group.png)

This is a renderer checkpoint. Dialog transition policy, focus handback and exact Chrome WebGPU motion still need their own proof.

## Reproduce

Rebuild the [565-file aligned Dialog source](../alignment/README.md#reproduce-the-native-replay) in a scratch directory. Verify [the incoming seal](incoming-source.sha256), apply [the patch](isolated-opacity.patch) using `git apply --whitespace=error-all`, and verify [all 569 output inputs](source.sha256). The independent rebuild matched the full manifest.

Use a fresh Cargo target and macOS with an available Metal adapter. The workspace run set `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1`.

```sh
cap sh scripts/check.sh
SURFACE_GPU_EVIDENCE=offscreen.png sh scripts/check-gpu.sh
```

Verify the archive with `shasum -a 256 -c SHA256SUMS`.
