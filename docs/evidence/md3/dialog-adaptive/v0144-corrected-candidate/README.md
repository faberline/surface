# Surface v0.1.44 corrected Dialog release candidate

The exact 579-file manifest SHA-256 is
`5d4e389d32159f245a732689a1dbb7f157abb4b3fcf56287d7e6c82dc59ef145`.
Start from the Search release commit `fec35d355025fa7476fac223baa60628471fd8fb` with tree
`35f17666034d0f39367dc3b139e06a927fca0455`. Verify `search-base.sha256`,
apply the portable `dialog-v0144.patch` using `git apply --whitespace=error-all`,
and verify `source.sha256`. An independent copy reproduced all 579 files.

- `cap sh scripts/check.sh` passed 1,286 Rust checks and the dependency provenance,
  formatting, strict Clippy and WASM checks. [Workspace log](workspace.log)
- `sh scripts/check-gpu.sh` ran all 16 relevant offscreen Metal cases. The new
  case samples real shell paint, rounded clipping and scaled content. [Metal log](gpu.log)
- Two native replays produced 14 identical PNGs each through Page.screenshot, CLI
  and MCP. All 44 visible-window samples were zero. The new launchers are fully
  visible at 960 × 1800 and can be reached by Tab at 320 × 400. [Native proof](https://github.com/faberline/surface/tree/530df1824800661dc796b05fccc1e81128b0ae3f/docs/evidence/md3/dialog-adaptive/v0144-corrected-native)
- The complete headless Mac Chrome run passed five cases and 18129 checks.
  It ran Material M256–M263 and independently decoded 41 Dialog PNGs.
  It reached Date Picker through the original 1100 × 880 route. All 140748
  Chrome and 14 native window samples found zero visible windows.
  [Browser receipts and PNGs](mac)

The corrected launcher row addresses the [initial Date Picker clipping result](https://github.com/faberline/surface/tree/0a77dd9f2cc7f51368717583515d76fbe55946d1/docs/evidence/md3/dialog-adaptive/v0144-gallery-regression).
The standalone [focused Dialog evidence](https://github.com/faberline/surface/tree/6350b8d8e09e488ef6bc3b1a3145ece3905263d7/docs/evidence/md3/dialog-adaptive/full-screen-motion)
records intermediate motion and the original clearance fixes. The optional
[hosted receipt validator](hosted-validator) verifies future exact Linux results
with their observed SwiftShader adapter. Its archive only establishes Metal
pixels and SwiftShader metadata so far.

Run `shasum -a 256 -c SHA256SUMS` in this folder to verify the archive.
