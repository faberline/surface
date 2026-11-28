# Surface v0.1.68 corrected label-color verification

Product tree `72ae6c3eb56e9554532f428b3308aadd8358a242` passed the native
and Web builds. Browser runner tree `9d3ce235f8efde1459fd41862cd918367fc85207`
changes one test file to wait for the focused frame at the locked time.
All 684 other source files match. The verified Web, Gallery, DevTools and
Jet bridge binaries use the corrected product source.

| Check | Result |
| --- | --- |
| Rust and WebAssembly | 1,575 Rust tests passed. 20 were ignored. Three Wasm checks passed. |
| Offscreen Apple Metal | 18 tests passed. The archive keeps 21 original PNGs. |
| Native headless | Two repeats and 14 original PNGs. Page.screenshot, CLI and MCP matched. Four processes exited and 24 samples showed no visible windows. |
| Chrome WebGPU | Five suites, 25,276 checks and 349 Material steps passed. The window guard found zero visible windows or remaining processes. The 34 label PNGs all match the corresponding archived images. |

The compact M350 test keeps the glyph positions. Four probes in the original
PNG read the expected 44 ms color `[206, 191, 243]`. The archive also keeps
the earlier headless failure that motivated the focus paint synchronization.
[First enabled frame regression](https://github.com/faberline/surface/tree/7541bb6474e536945643b84badd84c604a4859fd/docs/evidence/md3/text-field-label-color-v0168-enable-contrast)

[Reproduction and typed evidence](proof/README.md) · [Sealed file list](proof/SHA256SUMS)

```sh
python3 -E -B proof/collect.py --verify proof \
  --sha 2718014e69e00127870147b9f5bea66c8b3fef121a88d7681b7c8749ad72a85f
```
