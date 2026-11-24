# Isolated container transform checks

[Basic Dialog motion](https://github.com/faberline/surface/tree/66ff69ddb9b4b1ee531f69c5aa2a10a0a3b4db56/docs/evidence/md3/dialog-adaptive/basic-motion) supplies the 572-file input. Verify the input hash file, apply the renderer patch, verify the renderer source, then apply the Runtime patch and verify the Runtime source. Use `git apply --whitespace=error-all` in a fresh source copy. The manifests record the separate source stages.

- [Renderer](renderer) records 1,278 Rust tests plus formatting, Clippy, dependency provenance, and WASM compile checks. It records 15 headless Metal cases twice and representative transformed and cropped PNGs. The previous opacity and dialog samples were byte-identical to the earlier baseline.
- [Runtime](runtime) records 465 Core and Runtime tests, including 17 focused modal and ripple checks. The transform applies to the scoped panel and leaves its backdrop separate. A retiring panel gives up input and accessibility immediately.

These two packages qualify the shared paint and Runtime layers. The completed Full-screen Dialog and Chrome WebGPU journey have their own checks. Logs replace temporary source paths with placeholders. Source manifest, patch and PNG bytes are preserved.

Run `shasum -a 256 -c SHA256SUMS` in this folder to verify the archive.
