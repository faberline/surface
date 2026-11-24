# Full-screen Dialog motion pre-release proof

The exact 576-file product source has manifest SHA-256 `bb0e450ddb5c7a6ebaf7d592cf6c3f54e97f8db51ba118e5dc74d743ac9eba6a`. The [native package](native) reconstructs it from the [Basic motion checkpoint](https://github.com/faberline/surface/tree/66ff69ddb9b4b1ee531f69c5aa2a10a0a3b4db56/docs/evidence/md3/dialog-adaptive/basic-motion) and verifies each patch. The focused [Chrome package](web) applies only the stock Dialog tests to the same product files.

- Two native Metal replays produced 14 PNGs each. All PNGs match between replays and across Page.screenshot, CLI and MCP. All 44 window samples found no visible window. The viewports include short dark RTL at 200% text and a wide Basic fallback. [Native PNGs and receipts](native)
- Headless Chrome passed M256–M263 with 834 checks, 41 decoded PNGs, zero page errors and no surviving browser processes. The same journey covers real keys, wheel and clicks, accessibility tree, motion reversal, Reduce Motion and synthetic preedit across the breakpoint. [Browser PNGs and receipts](web)
- The integrated workspace passed 1,286 Rust tests in 174 reported groups, with 19 ignored in the normal workspace run. Formatting, dependency provenance, strict Clippy and WebAssembly compile checks passed. The separate Metal run executed all 16 filtered offscreen cases. Its transform test checks real shell, clip, color and geometry with a manual frame clock. [Workspace record](metal/workspace.log) and [Metal record](metal/gpu.log)

Reproduce the Rust records from the exact source with `cap sh scripts/check.sh` and run the GPU cases with `SURFACE_GPU_EVIDENCE=/path/to/offscreen.png sh scripts/check-gpu.sh`. Use two Cargo jobs and a private target. The release candidate and full stock browser checks have their own gates.

Run `shasum -a 256 -c SHA256SUMS` here to verify the archive.
