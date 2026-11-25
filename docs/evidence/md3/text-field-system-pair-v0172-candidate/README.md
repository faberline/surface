# Surface v0.1.72 system Field pair evidence

The 691-file candidate tree `9333c1f8dba9a7d15aba2ca47dc92c997abf98af` applies the
19-path patch to v0.1.71 tree `f2b631b0d72830f76b84e71b114018a4410dc813`.
Only the Material coverage document differs from scoped R8.
The exact stock Gallery uses the same four Web assets as scoped R8.

- Exact workspace passed 1,593 Rust tests and three Wasm checks.
  Apple Metal passed 18 tests and produced 21 original PNGs.
- Exact native produced 16 original PNGs across two replays.
  Page.screenshot, CLI and MCP bytes match. All 44 observations showed
  zero visible windows. Four Gallery processes exited. The separate
  R4 development replay has 16 identical PNGs.
- Scoped R8 browser completed 1,993 checks across 13 Material steps.
  Its 233 headless observations found zero visible windows or survivors.
  It includes 14 Field pair and 15 retained disabled-contrast PNGs.
- Full stock completed 29,907 checks in five suites.
  It includes 361 Material steps through M362, 20 IME steps, 14 Field pair
  PNGs, prior feature decoders and zero visible windows or survivors.

The real equal contrast pairs come from system CSS in headless Chrome
media emulation. The browser also tests a distinct Field and FieldText
pair through a fixture-only probe. Native uses the fixed synthetic-palette
preview, with no change to system preferences. These runs observe
rendering and input. Live assistive speech and hosted CI are outside this
local packet. Rejected R4, R5 and R7 image evidence and the R6
compiler-only failure are in `rejected/`.

Use Python 3.12 with Pillow. Supply the reviewed SHA-256 of `SHA256SUMS`.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-field-pair.py --verify-package . --package-sha "$reviewed_sha"
```

The offline verifier reads the original images and source/launch receipts.
The packet does not include app binaries or browser profiles.
