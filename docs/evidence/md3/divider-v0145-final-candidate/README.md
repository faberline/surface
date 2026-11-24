# Surface v0.1.45 Divider candidate

The 583-file source manifest SHA-256 is a2750ba3be68e0b6bc5b6f0183d4d21acf7e90fe9c4a65e623ecacbd192dd0aa.
Start from Dialog tree 50c3a5646a033b04280f7050748ca92c42154ad5. Verify
dialog-base.sha256, apply divider-v0145.patch with git apply --whitespace=error-all,
then apply endpoint-sampling.patch and verify source.sha256. An independent copy reproduced all 583 files.

- The complete Rust run passed 1,289 tests plus formatting, strict Clippy,
  provenance and WebAssembly checks. The separate Metal run passed 16 cases.
  [Workspace log](workspace.log) · [GPU log](gpu.log)
- Two native headless runs each produced four matching PNGs through
  Page.screenshot, CLI and MCP. All 16 samples found no visible windows.
  [Native evidence and reproduction](native)
- Stock headless Chrome passed five cases and 18,545 checks. M264–M266
  cover light, dark, RTL and 200% text with 1 dp and 8 dp lines at DPR 2.
  The verifier decodes all four images, checks the complete line depth and
  adjacent pixels, and verifies focus, AX omission and return to Regular.
  [Chrome screenshots and results](mac)

The original [Linux corner receipts](https://github.com/faberline/surface/tree/12d72e278ff4b6af1cb2c69941263fee645edf69/docs/evidence/md3/divider-v0145-rebased-candidate/hosted-corner-failure)
show the same ink and thickness with three antialiased corner pixels per
line. The final Rust test moves its endpoint probes without changing
its color or thickness assertions.

The design-site layout uses 16 dp insets and outline variant. The Android
reference adds the 8 dp Heavy weight.
[Google layout specs](https://m3.material.io/components/divider/specs) ·
[Google Heavy style](https://github.com/material-components/material-components-android/blob/6061a69915d04f11230dbf6f417bc27d9c3056ad/docs/components/Divider.md#adding-heavy-divider)

Run shasum -a 256 -c SHA256SUMS in this directory to verify the archive.
