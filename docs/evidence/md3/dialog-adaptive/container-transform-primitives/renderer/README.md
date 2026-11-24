# Isolated paint transform

Put a `BeginTransformGroup { transform: PaintTransform { source, destination, clip, radius, opacity } }` command before the group's paint and reuse `EndOpacityGroup` to close it. All rectangles and the radius use logical display-list coordinates. Place any following paint after the end marker. Groups can follow each other and cannot nest.

The renderer takes the group's visible source crop, maps it to the destination, and masks it to the independent rounded clip. It scales the geometry to device pixels. Sampling outside the render target is transparent. It interpolates premultiplied colors, so partial input paint and the group alpha blend once. The rounded edge coverage is one device pixel wide. The radius is capped at half the clip's shorter side.

The compositor keeps one reusable intermediate texture per renderer. The prior opacity shader still handles ordinary opacity groups and full-target identity transforms. Zero and full opacity keep their direct paths when the geometry is neutral. A mapped or cropped group still uses the compositor when opaque. The renderer rejects invalid or unrepresentable geometry and unbalanced or nested commands.

This patch supplies the renderer and its Core API. Dialog layout, focus, and animation state are in other layers. Native Metal tests exercise the shader. Browser runtime and its Dialog integration are separate checks.

## Exact source and proof

- Incoming Basic Motion manifest SHA-256 `6e9faca3b07a7057f652c18967a9109a23c7ad4f0124859789dc4696e1f7f044` with 572 files.
- Patch SHA-256 `1e448f105d6d1d30fb04740c753e64b8dc322eeb9a492ae3c7a75d67ee9c350c`. The patch changes nine paths.
- Output manifest SHA-256 `2273a9fad52bfbe7167d5e206ae87fe62791ac892861581553ca60718d0a68ef` with 574 files.

Apply the patch only to a new copy of the verified input. The raw test logs and the source fingerprints live next to this file.

```sh
batch=/tmp/surface-md3-dialog-transform.pgRGJiFN
cd "$batch/source"
shasum -a 256 -c ../source.sha256
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-dialog-transform.pgRGJiFN
export CARGO_HOME=/tmp/surface-md3-cargo CARGO_NET_OFFLINE=true
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
cap cargo test --locked -p surface --test opacity_group --test text_alignment
sh scripts/check.sh
SURFACE_GPU_EVIDENCE="$batch/gpu/gpu-offscreen.png" sh scripts/check-gpu.sh
```

The new Metal test checks scaled positions, source-edge crop, horizontal bilinear interpolation, a partial-alpha layer, the independent clip, the rounded edge, and neighboring paint order at 1× and 2×. It checks full-target identity against direct paint and old opacity groups in the sRGB target. It also checks rejection of malformed ranges and invalid logical or scaled geometry.

The visual checks are `gpu-sealed/gpu-transform-rounded.png` and `gpu-sealed/gpu-transform-crop.png`. Both captures match the first run byte for byte. The exact RGBA samples are in `pixel-samples.json`. The four earlier opacity and Basic Dialog captures also match their prior exact baseline byte for byte. See `opacity-before-after.log`.

The workspace run passed 1,278 tests and reported 18 ignored tests. The separate Metal run passed all 15 offscreen cases twice. Its second run used the pinned binary in `gpu-sealed/` with `RUST_TEST_THREADS=1` and the arguments `gpu_offscreen --ignored --nocapture`.
