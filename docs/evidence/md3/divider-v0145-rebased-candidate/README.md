# Surface v0.1.45 Divider candidate

The 583-file source manifest SHA-256 is 86f886cc10ab1d1b82e62364caf738824d2ce5295e3fb82533082f155763277e.
Start from Dialog tree 50c3a5646a033b04280f7050748ca92c42154ad5. Verify
dialog-base.sha256, apply divider-v0145.patch with git apply --whitespace=error-all,
and verify source.sha256. An independent copy reproduced all 583 files.

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

The design-site layout uses 16 dp insets and outline variant. The Android
reference adds the 8 dp Heavy weight.
[Google layout specs](https://m3.material.io/components/divider/specs) ·
[Google Heavy style](https://github.com/material-components/material-components-android/blob/6061a69915d04f11230dbf6f417bc27d9c3056ad/docs/components/Divider.md#adding-heavy-divider)

Run shasum -a 256 -c SHA256SUMS in this directory to verify the archive.
