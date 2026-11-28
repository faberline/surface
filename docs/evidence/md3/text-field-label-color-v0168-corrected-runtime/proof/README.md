# Surface v0.1.68 label color runtime evidence

This evidence pairs corrected product tree `72ae6c3eb56e9554532f428b3308aadd8358a242` (R2) with browser test tree `9d3ce235f8efde1459fd41862cd918367fc85207` (R3). The manifests cover 685 files. R3 adds a paint wait to one file, `e2e/src/material_text_field_label_motion.rs`. The other 684 file digests are identical. R3 uses the freshly built runner with the same R2 Web, native and Jet assets.

R2 passed 1,575 workspace tests, three Wasm checks and 18 Apple M1 Max Metal tests. The Metal results include 21 PNGs. Two headless native runs produced 14 original PNGs matching the reviewed endpoints. The guard observed zero visible windows across 24 samples. All four native Gallery processes exited. The [contrast defect proof](https://github.com/faberline/surface/tree/7541bb6474e536945643b84badd84c604a4859fd/docs/evidence/md3/text-field-label-color-v0168-enable-contrast) records the separate first-frame Rust test.

R2's browser suite stopped at M350. Four other suites passed. The wrapped label held 12,125 idle-colored glyph pixels at the expected midpoint. The historical midpoint has the intermediate color at those exact coordinates. The prior read-only label carried 2,289 intermediate-color pixels in the R2 frame. This indicates that the last focus change did not paint at the held clock before sampling. The two lossless crops in `r2-browser-failure/pixels/` show the current and historical label region. The raw PNGs, log, guard receipts and fixture source preserve the exact failure. [Historical candidate proof](https://github.com/faberline/surface/tree/14b110f1c269c1127389be68b3e9c7cb40e69b5f/docs/evidence/md3/text-field-label-color-v0168-candidate)

## Collect on the original host

Use Python 3.12 and Pillow 12.3 or later. Collection reads the existing results. It starts no app or build and writes the archive only if the real R3 receipt passes.

```sh
python3 -E -B collect.py --preflight
python3 -E -B collect.py --collect /tmp/surface-v0168-r3-runtime
```

Use a fresh archive path. The reader requires all five successful stock suites, 349 Material steps, 20 IME steps and 34 original label PNGs. M350 must show RGB `[206,191,243]` at 44 ms in all four probe positions and in the actual original screenshot. The guard must record zero visible windows and surviving processes. Historical whole-image equality is a separate reported count. Live pixel and typed assertions decide the M350 result.

## Read back or reproduce

```sh
python3 -E -B /path/to/archive/collect.py --verify /path/to/archive --sha "$reviewed_manifest_sha256"
```

Use the SHA-256 printed by collection. The readback checks the sealed files, pixel oracles and before/after input pins. It uses the physical path for the native reader on macOS.

Start with the historical exact v0.1.68 candidate. Apply `inputs/fix-only.patch` and `inputs/regression-only.patch` in a fresh scratch checkout. Check the resulting tree and files against `source.sha256` and the R2 ID above. In a new copy apply `r3/settle-compact-focus.patch` and check `r3/source.sha256` and the R3 tree. The [R2 replay recipe](inputs/replay-tools/README.md) records the product commands. The [R3 guard recipe](r3-tools/README.md) records the exact headless stock replay. Set the source and target paths for the new copy and verify the new build pins before rerunning.

The evidence covers local headless behavior. Hosted Linux, live VoiceOver speech and system input composition need separate evidence.
