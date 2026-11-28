# Surface v0.1.75 vertical menu validation

This archive records a fresh macOS run of the recovered 698-file candidate
with Git tree `1a8f21971d18d2782882fc8e8e57424bdb9cf301`.
[Exact source and recovery patch](https://github.com/faberline/surface/tree/4b7a41753fc00b996b3a1b70885e272876cb77ca/docs/evidence/md3/vertical-command-menu-v0175-recovered-source)

| Check | Result |
| --- | --- |
| Rust workspace | 1,606 passed and 20 ignored. Three WebAssembly checks passed. |
| Offscreen Apple Metal | 18 actual tests passed. 21 original PNGs were verified. |
| Headless Chrome WebGPU | All 371 Material steps passed with 30,557 checks. The archive keeps six original Baseline and Vertical menu PNGs. |
| Native screenshot replay | Two runs captured eight original PNGs. Page.screenshot, CLI and MCP returned identical PNG bytes. |
| Window observations | 4,692 Chrome samples and 46 native samples showed zero visible windows. All test processes exited. |

[Build and replay commands](reproduce/README.md) · [Typed results](local-proof/verified.json)

The [sealed receipt](local-proof/SHA256SUMS) has SHA-256
`1822a4c70d05cc2889de5a991543424974887a8102c4e85fc7e5c0c29586881b`.
See [light Standard](local-proof/native/repeat-1/standard-light-grouped.png)
and [narrow dark RTL](local-proof/native/repeat-1/vibrant-dark-rtl-200-end.png).
The browser menu checks are M267–M269 and M370–M372.
This candidate is in the release queue. Its hosted Linux gates are pending.

[Disabled row click regression and targeted fix](disabled-click/README.md)
