# Surface v0.1.45 Divider candidate

The 583-file source manifest SHA-256 is afad7c8d68a82dd5ee23bfb20bc05de0afd36292ffa906d72b4012e0ea724da3.
Start from Dialog tree ad19a57148ade929a60258535ad3ee61ccf88193. Verify
dialog-base.sha256, apply divider-v0145.patch with git apply --whitespace=error-all,
and verify source.sha256. An independent copy reproduced all 583 files.

- The complete Rust run passed 1,288 tests plus formatting, strict Clippy,
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
