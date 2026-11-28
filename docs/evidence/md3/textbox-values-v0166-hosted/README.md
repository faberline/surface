# Surface v0.1.66 Linux verification

Both first attempts ran eight suites on Ubuntu with Chrome WebGPU through
SwiftShader and Xvfb. Each passed 25,127 checks and all 345 Material steps.
Each Chrome accessibility report contains the six expected inactive
textbox values. The reports retain text, line breaks, locked states,
and the active textarea handoff.

| Event | Run | Browser log | Build log | Artifact ID |
| --- | --- | --- | --- | --- |
| Pull request | [45607226398](https://github.com/faberline/surface/actions/runs/45607226398/attempts/1) | [139181132408](https://github.com/faberline/surface/actions/runs/45607226398/job/139181132408) | [139181131933](https://github.com/faberline/surface/actions/runs/45607226398/job/139181131933) | 15020853613 |
| Push | [45607214912](https://github.com/faberline/surface/actions/runs/45607214912/attempts/1) | [139181094044](https://github.com/faberline/surface/actions/runs/45607214912/job/139181094044) | [139181093372](https://github.com/faberline/surface/actions/runs/45607214912/job/139181093372) | 15021126743 |

The checkout commits differ. Both use source tree
`3de0c143c43d35911593cfc95d3ea5931016c308`. See the original job log hashes,
artifact hashes, and selected file hashes in [verified.json](verified.json).
Each event directory contains 12 original PNGs, the Chrome report and two
gzip-compressed original logs. The [local evidence](../textbox-values-v0166-candidate)
has the source patch, native screenshots and macOS run.

## Reproduce

Use the repository's [CI workflow](../../../../.github/workflows/ci.yml)
at commit `c846aebd38367c16c697729d06e6cfb5fa71744e`, with both jobs.
The browser job runs `xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e`
with the Chrome and SwiftShader settings in that workflow.

The archive can be checked with `shasum -a 256 -c SHA256SUMS`. To check
original inputs while GitHub retains the ZIPs, download these API responses
into a folder named for each run. Use Python 3.12 with Pillow.

| Local file | GitHub API path under `repos/faberline/surface` |
| --- | --- |
| `run.json` | `actions/runs/RUN` |
| `jobs.json` | `actions/runs/RUN/attempts/1/jobs` |
| `artifact.json` | `actions/artifacts/ARTIFACT` |
| `original.zip` | `actions/artifacts/ARTIFACT/zip` |
| `browser.log` | `actions/jobs/BROWSER_JOB/logs` |
| `build.log` | `actions/jobs/BUILD_JOB/logs` |
| `checkout.json` | `git/commits/CHECKOUT` from the matching `verified.json` entry |

```sh
python3 -E -B verify-hosted.py "$originals" ../textbox-values-v0166-candidate
```

The Chrome accessibility data measures the browser tree. Spoken output
requires a separate screen-reader test.
