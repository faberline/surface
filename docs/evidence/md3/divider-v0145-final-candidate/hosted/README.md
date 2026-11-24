# Surface v0.1.45 hosted Divider verification

[PR #220](https://github.com/faberline/surface/pull/220) tested head `c24162d07ef23de0a646bbf7a923ecd16cd88244`
with source tree `120b536f534016b946eb29e716e1fa64cd902b50` and `main` base `9e2e89d77e584f66888cdfb20c0bb1257dc55d9c`.

All 583 files in the source manifest match the GitHub source tree. The candidate follows the
released Dialog tree. The correction changes only the Divider browser test. Its endpoint
probes use the line interior to avoid the two-pixel corner fringe on SwiftShader. The test
still checks exact ink, full thickness, and the immediately adjacent pixels.
[Original corner failures and pixel evidence](https://github.com/faberline/surface/tree/12d72e278ff4b6af1cb2c69941263fee645edf69/docs/evidence/md3/divider-v0145-rebased-candidate/hosted-corner-failure).

Both events passed the build and behavior job and the browser job. The build job checked
formatting, lint, workspace tests, WebAssembly and the consumer contract. The browser job
ran all eight stock suites on Linux in the Xvfb virtual display. Chrome reported SwiftShader,
a software WebGPU adapter. Native Linux window tests ran inside the virtual display.

| Event | Workflow | Build job and attempt | Browser job and attempt | Checkout | All checks | Material checks | IME checks |
| --- | --- | --- | --- | --- | ---: | ---: | ---: |
| `pull_request` | [45479269629](https://github.com/faberline/surface/actions/runs/45479269629) | [138747698145](https://github.com/faberline/surface/actions/runs/45479269629/job/138747698145) (1) | [138747697824](https://github.com/faberline/surface/actions/runs/45479269629/job/138747697824) (1) | `72dd5a4624b1128eddd453a9a1f3a77046e4a803` | 18922 | 18264 | 77 |
| `push` | [45479253160](https://github.com/faberline/surface/actions/runs/45479253160) | [138747643082](https://github.com/faberline/surface/actions/runs/45479253160/job/138747643082) (1) | [138747642502](https://github.com/faberline/surface/actions/runs/45479253160/job/138747642502) (1) | `c24162d07ef23de0a646bbf7a923ecd16cd88244` | 18922 | 18264 | 77 |

For each event, both jobs checked out the same commit. The PR job used GitHub’s merge
checkout of the named base and head. Both events used the exact candidate tree.

The Material suite ran 265 steps through M266 and omits the M23 failure-path check.
Divider M264–M266 checked the full-width, inset, middle and vertical separators, their
one- and eight-dp thickness, RTL placement at 200% text size, and keyboard focus.
Each job produced four Divider PNGs. The portable validator checked the physical line
pixels, including the row or column immediately outside each line. The archive also retains
all 41 Dialog PNGs per job and repeats the prior Dialog validator on the exact current runs.
The original GitHub ZIPs match their posted checksums. Each archived raw report and PNG
matches its original ZIP member byte for byte. The verifier fully decoded every PNG.

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
gh api repos/faberline/surface/actions/artifacts/14963177046/zip > pull_request-artifact.zip
gh api repos/faberline/surface/actions/jobs/138747698145/logs > pull_request-build-and-behavior.log
gh api repos/faberline/surface/actions/jobs/138747697824/logs > pull_request-browser-e2e.log
gh api repos/faberline/surface/actions/artifacts/14963597769/zip > push-artifact.zip
gh api repos/faberline/surface/actions/jobs/138747643082/logs > push-build-and-behavior.log
gh api repos/faberline/surface/actions/jobs/138747642502/logs > push-browser-e2e.log
```
