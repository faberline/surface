# Surface v0.1.6 release verification

The main commit is `d72aee04962615d6280a5f68f10aa7c3f1f87cf8`.
Its source tree is byte-identical to PR head
`794eb5b52a0b2dd4d9344e9e8d220c31838bf383`. Both source manifests
passed against the main commit.

- PR and push workflow checks both passed on the PR source.
  [Pull request #180](https://github.com/faberline/surface/pull/180) ·
  [PR CI](https://github.com/faberline/surface/actions/runs/45036893962) ·
  [Branch CI](https://github.com/faberline/surface/actions/runs/45036888009)
- The macOS workspace check passed 732 tests with six ignored. The
  separate offscreen GPU command passed five tests on Apple M1 Max
  Metal. The clipboard hand check did not run.
- The exact Gallery and CLI build passed the nine native headless
  side-sheet cases and all 76 native headless DevTools checks. The
  19 matrix PNGs, one DevTools screenshot, and both JSON receipts
  matched independent runs. CoreGraphics also counted zero onscreen
  windows belonging to a live headless Gallery process.
  [Native source](native/native-source.sha256) ·
  [Side-sheet report](native/native-side-sheet-report.json) ·
  [DevTools report](native/devtools-headless-result.json)
- Both jobs passed on the exact main commit. The E2E runner passed
  8 suites and 2,725 checks. Material passed 48 steps and 2,100
  checks. Headless DevTools passed 3 steps and 76 checks with no
  X11 or Wayland display available to the Gallery.
  [Main CI](https://github.com/faberline/surface/actions/runs/45037672707) ·
  [Hosted E2E result](https://github.com/faberline/surface/actions/runs/45037672707/job/137254924753)
