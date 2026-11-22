# Native standalone wide navigation rail swipe

These captures use Metal on macOS 27.2.1 and Apple M1 Max. The production source manifest has SHA-256 `94d55d3426a0b9c2e790a728876e1a688b9b03e58920c733984455bc65308579`. The sizing example starts the Gallery with public window options. The first build produces stock Gallery and CLI binaries. A second scratch build adds two test-only source changes to route scripted Winit window events in its own process.

## Replay

Use the restored Surface checkpoint and an active macOS desktop with Metal. Installations of Cap, the Rust toolchain, and the Xcode command-line tools are required. The AppKit collector uses existing accessibility access.

```sh
shasum -a 256 -c evidence.sha256
sh build-and-run.sh /path/to/restored-surface /tmp/new-wide-rail-swipe-captures
```

The scripted Gallery accepts only its private loopback session while the fixture is active. The screenshot host, CLI and MCP paths are the production implementations. Replay credentials and binaries stay in scratch directories.

## Results

| Native window | Input source | Full panel | Visible panel at 34% dismissal travel |
| --- | --- | --- | --- |
| 704 × 512 dp, light LTR | Synthetic Winit mouse | 288 dp | 190.08 dp |
| 704 × 512 dp, dark RTL, 200% text | Synthetic Winit mouse | 360 dp | 237.60 dp |
| 320 × 240 dp, dark RTL, 200% text | Synthetic Winit Touch | 320 dp | 211.20 dp |

The held scrim alpha changed from 82 to 54 in all three profiles. Pixel measurements found 190, 237 and 211 visible panel pixels. AppKit confirmed the named active modal, the focused Inbox and the current Home page during the hold. The background rejected activation.

Pointer cancellation restored the opening anchor and ignored the stale release. A slow release at 28% travel returned to that anchor. A slow release at 64% travel kept focus inside during the sampled closing motion, then restored focus to the opener. The selected page remained Home. This native run verified the distance thresholds. The velocity threshold is outside its measured scope.

The compact wheel event changed the destination scroll offset from 136 to 46 dp while the header stayed fixed. A vertical synthetic contact did not dismiss or select. The settled compact Inbox text displayed both 200% lines. Across the three profiles, 12 held or settled captures matched byte for byte through the raw protocol, CLI and MCP.

The optional stock process-targeted mouse probe found existing posting access. Its process identity guard stopped the attempt before any event was posted. The completed Winit routes are synthetic. Physical mouse and touch were outside this run.

[Prior Drawer swipe evidence](https://github.com/faberline/surface/tree/3d817301c8905043f06ca7e0533833608f99ecc5/docs/evidence/md3/drawer-swipe)
