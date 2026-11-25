# Surface v0.1.64 Character Counter candidate proof

Exact source manifest `fe7349b293de3078ca44fd6b1fa1885a5fa156ba559503c795cd9d6daab5e4a3` pins 678 files. Its Git tree is `eeef2dc049409f7df3a67773d8e72e76b298faf6`.
The 13-path patch applies to the v0.1.63 tree `3e2f280ed159c28f5b939813c4d4fd88d26803ff`. This archive records local evidence.

- All five stock macOS suites passed 24,442 checks. The
  Material suite ran 339 steps through M340. The IME suite ran all 20 steps.
- Chrome used Apple Metal. The guard observed zero visible owned windows and
  zero owned survivors. The exact run supplied seven Counter PNGs and the
  inherited Adornments, Filled and Outlined originals.
- The separate focused run passed 308 checks on the four canonical Gallery Web
  files also used by stock. Its seven PNGs and observer record have their own
  scope. The focused receipt keeps the passive count, live error deliveries,
  trusted keyboard events and the synthetic CDP cancel and commit steps.
- The native Gallery saved seven PNGs in each of two replays. Page.screenshot,
  CLI and MCP matched. All 24 samples found zero visible windows, and all four
  Gallery processes exited. The replay uses Runtime DOM and DevTools.
- The separate R1 calibration found zero pixels near the unblended foreground
  in the wide ratio. R2 finds 51 strong glyph pixels in the same original PNG.
  Wrong hue and wrong opacity remain rejection controls.
- The workspace passed 1,543 tests and
  three Wasm checks. Apple M1 Max Metal ran 18 offscreen tests, including 16
  primary tests and two supplemental raster-cache and text-selection tests.

System IME and screen-reader speech require separate evidence.

## Offline recheck

From the archive, use Python 3.12 or newer with Pillow. Provide the reviewed
SHA-256 of `SHA256SUMS`. This command reads proof only and starts no app.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-counter.py --verify-package . --package-sha "$manifest_sha"
```

`source.sha256` and `base-source.sha256` verify the exact source bytes.
Decompress `counter-v0164.patch.gz` for the reviewed patch, and decompress the
Rust and runner logs to obtain their original bytes. The copied stock harness,
focused runner builder and native replay script give the exact test recipes.
The original stock run retains all raw artifacts. This archive preserves
its guarded inputs, original observer and runner bytes, editor PNGs and
inherited IME controls.
