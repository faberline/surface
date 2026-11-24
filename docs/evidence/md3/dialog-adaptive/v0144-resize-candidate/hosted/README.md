# Surface v0.1.44 hosted Dialog verification

[PR #219](https://github.com/faberline/surface/pull/219) tested head `9b8e02b8668eba8da49c1fdcdf1832dee05865fb`
with source tree `50c3a5646a033b04280f7050748ca92c42154ad5` and `main` base `fec35d355025fa7476fac223baa60628471fd8fb`.

The corrected candidate schedules a new frame to measure its Full-screen opener after
resize. The test still checks the real Gallery geometry and motion. The source manifest
covers the 579 files in this candidate.

Both events passed the build and behavior job and the browser job. The build job checked
formatting, lint, workspace tests, WebAssembly and the consumer contract. The browser job
ran all eight stock suites on Linux in the Xvfb virtual display. Chrome reported SwiftShader,
a software WebGPU adapter. Native Linux window tests ran inside the virtual display.

| Event | Workflow | Build job | Browser job | Checkout | All checks | Material checks | IME checks |
| --- | --- | --- | --- | --- | ---: | ---: | ---: |
| `pull_request` | [45467288705](https://github.com/faberline/surface/actions/runs/45467288705) | [138705483145](https://github.com/faberline/surface/actions/runs/45467288705/job/138705483145) | [138705482421](https://github.com/faberline/surface/actions/runs/45467288705/job/138705482421) | `37f8a934ee09f47b0cb8a05af1cb0904b0960119` | 18506 | 17848 | 77 |
| `push` | [45467274408](https://github.com/faberline/surface/actions/runs/45467274408) | [138705434067](https://github.com/faberline/surface/actions/runs/45467274408/job/138705434067) | [138705433683](https://github.com/faberline/surface/actions/runs/45467274408/job/138705433683) | `9b8e02b8668eba8da49c1fdcdf1832dee05865fb` | 18506 | 17848 | 77 |

For each event, both jobs checked out the same commit. The PR job used GitHub’s merge
checkout of the named base and head. Both events used the exact candidate tree.

The Material suite ran 262 steps through M263 and omits the M23 failure-path check. Dialog
M256–M263 covers named Basic and Alert dialogs, the scroll body below pinned controls,
modal keyboard scope and focus return, the Full-screen layout change, editing and motion.
Each stock job produced 41 Dialog PNGs. The original GitHub artifact ZIPs match GitHub’s
checksums. The reports and PNGs in this folder match their ZIP members byte for byte.
The verifier decoded each image and rechecked the pinned text, 200% RTL lines, scrim,
and Full-screen motion and scroll-clearance pixels directly.

Both I11 reports confirm that trusted Select All survives a resize after cancellation. They
also confirm that a backward range recovers on cancel and trusted Right plus insertion yields
`台文X` before and after repaint. The Showcase and Material reports had zero page errors.
The accessibility checks use Chrome’s accessibility tree. The input-method steps use browser
composition events.

## Reproduce

Check out the exact commit in the table and use the dependencies and Chrome flags in `ci.yml`.

```sh
xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e
```

To verify the copied files and independently replay the receipt and pixel checks, use Python
3.12 with Pillow 12.3, then run:

```sh
shasum -a 256 -c SHA256SUMS
python3 -B verification/package-evidence.py --verify .
```

Download the original ZIPs and logs while GitHub keeps them:

```sh
gh api repos/faberline/surface/actions/artifacts/14957628805/zip > pull_request-artifact.zip
gh api repos/faberline/surface/actions/jobs/138705483145/logs > pull_request-build-and-behavior.log
gh api repos/faberline/surface/actions/jobs/138705482421/logs > pull_request-browser-e2e.log
gh api repos/faberline/surface/actions/artifacts/14958750268/zip > push-artifact.zip
gh api repos/faberline/surface/actions/jobs/138705434067/logs > push-build-and-behavior.log
gh api repos/faberline/surface/actions/jobs/138705433683/logs > push-browser-e2e.log
```
