# Surface v0.1.63 Text Field Adornments candidate proof

Exact source manifest `2d671c1f3c4b450a21ab728a22ba0f8126af4790291a1b718d9005ab4f29eec2` pins 675 files. Its Git tree is `3e2f280ed159c28f5b939813c4d4fd88d26803ff`.
The 28-path patch applies to the v0.1.62 tree `7266ce0588d5bc2d83a4151ca2e13c780153e279`. This archive records local evidence.

- All five stock macOS suites passed 24,134 checks. The
  Material suite ran 335 steps through M336. The IME suite ran all 20 steps.
- Chrome used Apple Metal. The guard observed zero visible owned windows and
  zero owned survivors. The same exact stock run supplied six Adornments PNGs,
  six inherited Filled PNGs and five inherited Outlined PNGs.
- The separate focused run passed 243 checks on the four canonical Gallery Web
  files also used by stock. Its six PNGs and guard receipt are separate from stock. The two trusted Enter events, including the first during synthetic
  CDP preedit, produce zero and one submit. The receipt records any explicit
  synthetic CDP commit and cancel. The compact one-line placeholder can clip
  with both affixes inline. These tests do not measure native IME or speech.
- The native Gallery replay saved six PNGs twice. Page.screenshot, CLI and
  MCP matched. All 22 observations found zero visible windows.
- The archived development regression image has 112 pixels in the original
  16 by 24 dp icon gutter. Both exact native and Web checks found zero.
- The workspace passed 1,540 tests and
  three Wasm checks. Apple M1 Max Metal ran 18 offscreen tests, including 16
  primary tests and two supplemental raster-cache and text-selection tests.

## Offline recheck

From the archive, use Python 3.12 or newer with Pillow. Provide the reviewed
SHA-256 of `SHA256SUMS`. This command reads proof only and starts no app.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-adorned-field.py --verify-package . --package-sha "$manifest_sha"
```

`source.sha256` and `base-source.sha256` verify the exact source bytes.
Decompress `adorned-v0163.patch.gz` for the reviewed patch, and decompress the
Rust and runner logs to obtain their original bytes. The copied stock harness,
focused runner builder and native replay script give the exact test recipes.
The original local stock run retains all raw artifacts. This compact archive
preserves its signed-by-hash receipts, editor PNGs and raw inherited controls.
