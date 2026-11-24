# Surface v0.1.42 hosted verification

- Candidate head `86bf5eb86a5d070245db039b37977198ba1a417e`
- Candidate tree `93b47517d26662b1e9aa5502f730349723fb0978`
- Main base `5a90a6fd12717207c934f2b0f27b37aff4e0a2c2`
- Source comparison [PR #217](https://github.com/faberline/surface/pull/217)

Each run passed both `build-and-behavior` and `browser-e2e`. The browser job ran the stock eight-case suite under Linux with Xvfb, which provides a virtual display. Headless Chrome used SwiftShader, a software WebGPU adapter. The stock workflow also ran the suites that use native Linux windows inside that virtual display.

| Event | Workflow run | Browser job | Actual workflow checkout | Eight-case checks | Material checks | IME checks |
| --- | --- | --- | --- | ---: | ---: | ---: |
| `pull_request` | [45435977006](https://github.com/faberline/surface/actions/runs/45435977006) | [138593771394](https://github.com/faberline/surface/actions/runs/45435977006/job/138593771394) | `22f28f685b0813eeed66caa3ed5d38a30fbcb5cd` | 16500 | 15842 | 77 |
| `push` | [45435959502](https://github.com/faberline/surface/actions/runs/45435959502) | [138593705026](https://github.com/faberline/surface/actions/runs/45435959502/job/138593705026) | `86bf5eb86a5d070245db039b37977198ba1a417e` | 16500 | 15842 | 77 |

The PR checkout is GitHub’s merge of the named base and candidate head. The build job and browser job checked out the same commit for each event. Both checkout commits have the candidate tree above.

I1 through I20 ran in each job with 77 checks. At I11, trusted Select All survives a Canvas resize after cancellation. In a second case with repaint held, cancellation restores a backward range. Trusted Right and insertion produce `台文X` before repaint. Repaint preserves it. The raw browser reports for both sequences are included.

I20 records the browser’s UTF-16 ranges and Surface’s UTF-8 byte positions for the same authored CR/LF and Unicode text. The Material case ran 246 steps through M247. The suite omits M23.

M246 records two trusted ArrowRight key holds through a rendered palette change, one with normal event delivery and one with the textarea’s `select` event delayed until key release. It also records a backward range. M247 uses Chrome’s resolved forced-color pair and a synthetic composition. The decoded stock PNGs match the reported underline colors for the selected middle and both unselected sides.

These records cover browser preference emulation, synthetic composition, browser input and stock rendering in the hosted environment. Live system settings, a physical input method and screen-reader speech need separate checks.

## Reproduce

From the exact checkout shown above, use the [candidate CI workflow](https://github.com/faberline/surface/blob/86bf5eb86a5d070245db039b37977198ba1a417e/.github/workflows/ci.yml) for dependencies and Chrome flags. Run the same command with those settings:

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e
```

To download the original CI records while GitHub keeps them, run the commands below. The published logs contain the checkout, invocation, test steps and case verdicts from the corresponding full log.

```sh
gh run download 45435977006 --repo faberline/surface --name surface-e2e-artifacts --dir raw-pull_request
gh api repos/faberline/surface/actions/jobs/138593771394/logs > raw-pull_request.log
gh run download 45435959502 --repo faberline/surface --name surface-e2e-artifacts --dir raw-push
gh api repos/faberline/surface/actions/jobs/138593705026/logs > raw-push.log
```

Run `shasum -a 256 -c SHA256SUMS` from this evidence folder to verify the published files.
