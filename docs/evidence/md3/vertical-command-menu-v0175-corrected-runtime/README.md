# Surface v0.1.75 corrected vertical menu verification

The corrected source tree is `8046e25a630284411e1ebf9f9d16d1004b02651b`.
A real pointer down and up on a disabled row now leaves the separated menu
and its focus in place. The row stays inactive. A click outside the menu
still dismisses it. The recorded [red and green regression](https://github.com/faberline/surface/tree/815b38d3f04eba4479ae0954ab4a23f57439690a/docs/evidence/md3/vertical-command-menu-v0175-recovered-runtime/disabled-click)
includes the source patch and test commands.

| Check | Corrected result |
| --- | --- |
| Rust workspace | 1,607 passed, 20 ignored, three Wasm checks |
| Offscreen Metal | 18 actual tests passed with 21 original PNGs |
| Headless native | Eight original PNGs matched across Page.screenshot, CLI and MCP. Six processes exited. All 46 samples showed zero visible windows. |
| Chrome WebGPU | All 371 Material steps and 30,557 checks passed. The chosen menu steps left six original PNGs. All 4,700 samples showed zero visible windows and no test processes survived. |

Use [the replay recipe](proof/README.md) and [sealed proof](proof/SHA256SUMS).
The SHA-256 of the sealed file list is
`581ec4b33f2b90e5a4b4adfb95c78443c1371ba43050c1afdd692418101d5374`.

```sh
python3 -E -B proof/collect.py --verify proof \
  --sha 581ec4b33f2b90e5a4b4adfb95c78443c1371ba43050c1afdd692418101d5374
```

This candidate's Linux workflow runs when the release queue reaches it.
