# Surface v0.1.43 hosted Search verification

[PR #218](https://github.com/faberline/surface/pull/218) tested head `fc233e9a592d886f56dbc1397234edeaded2331b`
with tree `35f17666034d0f39367dc3b139e06a927fca0455` and `main` base `eacb33d06269a0b0a75c3c13f4de82de05f19031`.

The first candidate stopped at a test assertion that required a non-fallback WebGPU adapter.
The updated Search test requires Chrome to report whether its adapter is a fallback.
The checkout trees differ in this one E2E test file. Search and rendering code are identical.

Both events passed the build and behavior job and the browser job. The build job checked
formatting, lint, workspace tests, WebAssembly, and the consumer contract. Each browser job
ran all eight stock cases on Linux under the Xvfb virtual display. Chrome used SwiftShader
for software WebGPU. Native Linux window tests ran inside the virtual display.
Each original GitHub artifact ZIP matched GitHub’s digest. These raw reports and PNGs match
their ZIP members byte for byte.

| Event | Workflow | Build job | Browser job | Checked-out commit | Checks | Material | IME |
| --- | --- | --- | --- | --- | ---: | ---: | ---: |
| `pull_request` | [45452814888](https://github.com/faberline/surface/actions/runs/45452814888) | [138654194363](https://github.com/faberline/surface/actions/runs/45452814888/job/138654194363) | [138654193599](https://github.com/faberline/surface/actions/runs/45452814888/job/138654193599) | `cbd608a2178ac5d1e66e80ceda52eea1552bddd2` | 17672 | 17014 | 77 |
| `push` | [45452793148](https://github.com/faberline/surface/actions/runs/45452793148) | [138654120847](https://github.com/faberline/surface/actions/runs/45452793148/job/138654120847) | [138654121291](https://github.com/faberline/surface/actions/runs/45452793148/job/138654121291) | `fc233e9a592d886f56dbc1397234edeaded2331b` | 17672 | 17014 | 77 |

For each event, both jobs checked out the same commit. The PR workflow tested GitHub’s merge
of the base and head listed above. Both events used the candidate source tree.

The Material suite ran 254 steps through M255. It excludes the M23 failure-path step.
M248–M255 covered both Search styles, the editor across the adaptive breakpoint, accessibility
and keyboard controls, input at 200% in dark right-to-left layout, the browser forced-color
palette, motion, and Reduce Motion. Each event includes all 46 original Search PNGs. The
verifier decoded each image and checked the caret, forced-color and motion pixels directly.

Both I11 reports show that Select All survives a redraw after cancellation. They also show that
cancellation restores a backward range and that trusted Right plus insertion yields `台文X`
before repaint and stays correct after repaint. The stock fixture waits for the rendered 1001 px
viewport in the Select All case. Its auxiliary frame counter counts wrapped callbacks. The
fixture restores the original requestAnimationFrame at release. I20 checks UTF-16 and UTF-8
selection ranges.

M246 preserves trusted held-arrow selection through a palette change, including a delayed
selection event. M247 verifies the selected and unselected parts of an input method preedit
underline with Chrome’s resolved forced-color pair. The Showcase and Material reports had
zero page errors. The browser input method and preference tests use browser emulation.

## Reproduce

Check out the exact commit in the table. Use the included `ci.yml` for dependencies and Chrome
flags. Run the same command in Linux:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e
```

Download the original reports and logs while GitHub keeps them:

```sh
gh run download 45452814888 --repo faberline/surface --name surface-e2e-artifacts --dir raw-pull_request
gh api repos/faberline/surface/actions/jobs/138654194363/logs > pull_request-build-and-behavior.log
gh api repos/faberline/surface/actions/jobs/138654193599/logs > pull_request-browser-e2e.log
gh run download 45452793148 --repo faberline/surface --name surface-e2e-artifacts --dir raw-push
gh api repos/faberline/surface/actions/jobs/138654120847/logs > push-build-and-behavior.log
gh api repos/faberline/surface/actions/jobs/138654121291/logs > push-browser-e2e.log
```

Run `shasum -a 256 -c SHA256SUMS` from this folder to verify the archive.

To repeat the receipt and pixel checks, use Python 3 with Pillow and run:

```sh
python3 -B verification/observe-hosted.py --pr 218 --head fc233e9a592d886f56dbc1397234edeaded2331b --tree 35f17666034d0f39367dc3b139e06a927fca0455 --base eacb33d06269a0b0a75c3c13f4de82de05f19031 --pull-run 45452814888 --browser-job 138654193599 --push-run 45452793148 --push-browser-job 138654121291
python3 -B verification/package-evidence.py 45452814888
```
