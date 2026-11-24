# Surface v0.1.44 first hosted Dialog result

The exact first candidate `9c841f6e0654cc43ab778d6660994ad6b63fa28c` on source tree `ad19a57148ade929a60258535ad3ee61ccf88193` has a split hosted result. Both build jobs passed. Both events ran Chrome/158.0.8127.88 with SwiftShader software WebGPU.

- [PR job](https://github.com/faberline/surface/actions/runs/45460777934/job/138682422636) passed eight stock suites and 18,506 checks. The portable validator checked all 41 Dialog PNGs and their pixels.
- [Push job](https://github.com/faberline/surface/actions/runs/45460753966/job/138682344294) reported seven suites passed, one failed, and 18,245 checks. Dialog stopped in M263 before the first Full-screen transform capture.

The failed Dialog export shows the new 320 × 400 viewport. The real, enabled `dialog-demo-fab-open` has bounds `[24, 320, 56, 56]`, while the Dialog motion state retains `[24, 1720, 56, 56]` from the previous 1,800 dp viewport. The shape is resting at progress zero. The check waited 30 seconds for the measured anchor to match the real FAB and timed out. There were zero Dialog page errors. The same stock check passed on the PR checkout with `[24, 320, 56, 56]`.

Runtime rebuilds the view for resize before computing the new layout. The Gallery motion callback can read only the last computed layout. The first resized draw has no current layout for the callback, and a resting transform schedules no next draw. The old measurement can therefore remain cached until another event requests a frame.

The exact test predicate is in [the M263 scene setup](https://github.com/faberline/surface/blob/9c841f6e0654cc43ab778d6660994ad6b63fa28c/e2e/src/material_dialog_transform.rs#L20). The measurement is in [the Gallery Dialog frame callback](https://github.com/faberline/surface/blob/9c841f6e0654cc43ab778d6660994ad6b63fa28c/examples/material-gallery/src/dialog_demo.rs#L160). Runtime’s order is in [the frame renderer](https://github.com/faberline/surface/blob/9c841f6e0654cc43ab778d6660994ad6b63fa28c/crates/surface-app/src/lib.rs#L1619).

Both I11 receipts are present and verified. Both original GitHub ZIPs match their published checksums. This folder keeps representative screenshots, the raw Dialog reports, job-log excerpts and the geometry comparison. `original-artifacts.json` records the original ZIP checksums and the archived member checksums.

The rendered FAB remains in the correct place in both events. The `640 × 888` screenshots include a 44 dp Web header at 2× scale. The 112 × 112 pixel region at physical coordinates `[48, 728, 160, 840)` corresponds to the real FAB bounds. That region has identical RGBA bytes in the failing viewport and passing rest capture. The complete encoded screenshots also match byte for byte. `geometry-comparison.json` records both checksums.

## Reconstruct the evidence

Run `shasum -a 256 -c SHA256SUMS` in this folder. Download the complete original archives while GitHub keeps them.

```sh
gh api repos/faberline/surface/actions/artifacts/14954967686/zip > pr-artifact.zip
gh api repos/faberline/surface/actions/artifacts/14954202904/zip > push-artifact.zip
shasum -a 256 pr-artifact.zip push-artifact.zip
```

The PR and push ZIP checksums must match `original-artifacts.json`. The run links above open the full logs. To replay, use candidate `9c841f6e0654cc43ab778d6660994ad6b63fa28c`, the flags and dependencies in `ci.yml`, and the stock test command.

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e
```

To check the viewport pixels again, run `python3 verify-comparison.py` with Pillow installed.
