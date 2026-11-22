# v0.1.4 release verification

Surface main accepted [PR #178](https://github.com/faberline/surface/pull/178).
The PR head was `6d5bc9a3afd66c6895bc946f5e2a112173bf6b8d`.
The merge commit is `c42c65307281fdeac8e7a281acd7213e80cbf3e2`.
Both commits have tree `9ebab560ed88e6957fcd16d5c4ce8d036131fac3`.
`git diff --exit-code` confirmed the trees match. The native and browser
source manifests in this folder match the merged source.

| Check | Result |
| --- | --- |
| Local `cap sh scripts/check.sh` | 706 tests passed, six ignored by default. Provenance, formatting, Clippy, WASM and the unchanged consumer contract passed. |
| Local `cap sh scripts/check-gpu.sh` | All five previously ignored offscreen GPU cases passed on Apple M1 Max Metal. |
| Native Gallery `Page.screenshot` replay | Four profiles passed. AppKit verified current page and restored focus after AXPress. API, CLI and MCP PNG bytes matched. |
| Local Chrome 157 on Metal 3 | Material ran 39 steps and 893 checks with no page errors. |
| [PR CI](https://github.com/faberline/surface/actions/runs/45027222284) and [branch push CI](https://github.com/faberline/surface/actions/runs/45027215218) | `build-and-behavior` and `browser-e2e` succeeded at the PR head. |
| [Merged main CI](https://github.com/faberline/surface/actions/runs/45027814182) | Both jobs succeeded. The full browser suite ran seven cases and 1442 checks with zero failures. Material ran 39 steps and 893 checks in Chrome 158 with SwiftShader. |

The new API keeps both legacy drawer builders. The evidence covers
desktop and Chrome WebGPU. Vertical touch panning and predictive back
remain open. Physical touch and live VoiceOver speech remain unverified.
