# Surface v0.1.71 disabled text-field contrast evidence

The exact 690-file tree `f2b631b0d72830f76b84e71b114018a4410dc813` applies the 21-path patch to the
v0.1.70 tree `409a21f6023802f055371cdec4391976c55ffd17`. The scoped browser runner selects ten Material
steps. Its product snapshot and the exact candidate differ only in the
coverage document. Its four Web assets match the exact stock Gallery.

- Exact Rust passed 1,587 tests, provenance, lint and three Wasm checks.
  Actual Apple Metal passed 18 tests and produced 21 original PNGs.
- The exact native replay produced 24 original PNGs across two repeats.
  Direct Page.screenshot, CLI and MCP returned identical bytes.
  The 78 observations showed no visible windows. All six Gallery
  processes exited. The development R3 replay is stored separately.
  Its 24 images match the exact images byte-for-byte.
- Scoped Chrome passed 1,470 checks in ten steps. It records 15 contrast
  images and 162 headless observations with zero windows or survivors.
- Full stock passed 29,384 checks in five
  suites. It includes 358 ordered Material steps through M359, 20 IME
  steps, 15 contrast PNGs, retained decoders and zero windows or survivors.

The native Gallery used the fixed light and dark palettes in
[W3C CSS Color Adjustment Level 1 §5.2](https://www.w3.org/TR/css-color-adjust-1/#forced-color-palettes). It verifies native
paint and the Runtime state in that preview. The browser verifies CSS
GrayText via headless Chrome media emulation and actual Canvas pixels.
Neither result measures macOS setting changes or live assistive speech.

Use Python 3.12 with Pillow and the reviewed archive manifest digest.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-disabled-contrast.py --verify-package . --package-sha "$reviewed_sha"
```

This reads the sealed files locally. App and browser binaries stay outside
this packet. Raw receipts keep temporary process paths and original pixels.
