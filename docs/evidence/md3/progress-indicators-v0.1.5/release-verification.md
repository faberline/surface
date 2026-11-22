# v0.1.5 release verification

[Surface PR #179](https://github.com/faberline/surface/pull/179) merged
from `984c0492336d0c3d46911ad1abec0d7c48e47d61` into main as
`e4caf946c3aa266c17add39bc75b5695130c6a24`. Both have the tree
`f42c8d7f5b437df9cf975301b682556773aebbe7`.
`git diff --exit-code` and both source manifests matched the merge.

| Check | Result |
| --- | --- |
| Local `cap sh scripts/check.sh` | 717 tests passed, six ignored by default. Provenance, formatting, Clippy, WASM and the existing consumer contract passed. |
| Merged source `cap sh scripts/check-gpu.sh` | Five real offscreen tests passed on Apple M1 Max Metal. |
| [Native Gallery](native/native-progress-report.json) | Four window profiles. Read-only AppKit values. Unknown progress clears the range. Page.screenshot, CLI and MCP returned the same 320 dp PNG. |
| [Slider compatibility](native/ax-slider-compatibility.json) | The final candidate preserved the v0.1.4 AppKit query, the write response, and the resulting application value. Both processes stayed alive. |
| [Chrome 157 on Metal](web/browser-material-progress.json) | 43 Material steps and 1,167 checks passed with zero page errors. |
| [PR CI](https://github.com/faberline/surface/actions/runs/45031697228) and [push CI](https://github.com/faberline/surface/actions/runs/45031694005) | Both jobs succeeded at the final PR head. |
| [Merged main CI](https://github.com/faberline/surface/actions/runs/45032376699) | Both jobs succeeded. Chrome 158 on Linux with SwiftShader ran seven cases and 1,716 checks with zero failures. |

The wave geometry uses sampled vector contours. Live screen-reader
speech and virtual focus during animation remain unverified.
