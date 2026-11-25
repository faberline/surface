# Surface v0.1.62 Filled Text Field candidate proof

Exact source manifest `78ae815e1c6407c91c65b13219e07b2c6f2298eb5218530670fad26f70a3dabe` pins 670 files. Its Git tree is `7266ce0588d5bc2d83a4151ca2e13c780153e279`.
The 15-path patch applies to the v0.1.61 tree `7c7881f78c3dc44129f52688a079b6874c9a4991`. This archive records local evidence.

- All five stock macOS suites passed 23,891 checks. The
  Material suite ran 331 steps through M332. The IME suite ran all 20 steps.
- Chrome used Apple Metal. The guard observed zero visible owned windows and
  zero owned survivors. Canvas evidence includes the six original Filled PNGs
  and five inherited Outlined PNGs from the same exact stock run.
- The separate focused run passed 235 checks on the four canonical Gallery Web
  files also used by stock. Its six PNGs and guard receipt are separate from stock. The two trusted Enter events, including the first during synthetic
  CDP preedit, produce zero and one submit. The receipt records whether an
  explicit synthetic text commit was needed.
- The native Gallery replay saved four PNGs twice. Page.screenshot, CLI and
  MCP matched. All 16 observations found zero visible windows.
- The workspace passed 1,531 tests and
  three Wasm checks. Apple M1 Max Metal ran 18 offscreen tests, including 16
  primary tests and two supplemental raster-cache and text-selection tests.

## Offline recheck

From the archive, use Python 3.12 or newer with Pillow. Provide the reviewed
SHA-256 of `SHA256SUMS`. This command reads proof only and starts no app.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-filled-field.py --verify-package . --package-sha "$manifest_sha"
```

`source.sha256` and `base-source.sha256` verify the exact source bytes.
Decompress `filled-v0162.patch.gz` for the reviewed patch, and decompress the
Rust and runner logs to obtain their original bytes. The copied stock harness,
focused runner builder and native replay script give the exact test recipes.
The original local stock run retains all raw artifacts. This compact archive
preserves its signed-by-hash receipts, editor PNGs and raw inherited controls.
