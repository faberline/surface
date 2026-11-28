# Surface v0.1.76 corrected choice menu verification

The corrected product tree is `37e4454bc7d3f63f7111c8264d4e59e451c8ef9e`.
The stock browser runner uses tree `b6dc18c4baf8e6b65bddfbc6f59b7b07b4a0b5b9`.
The only source difference moves the RTL roundness probes away from the
scroll thumb. The ready Web bundle uses the same product source bytes.
The archived commands verify the two exact trees and that one-file change.

| Check | Result |
| --- | --- |
| Rust and WebAssembly | 1,615 Rust tests passed, 20 ignored. Three Wasm checks passed. |
| Offscreen Apple Metal | 18 tests passed. The archive includes 21 original PNGs. |
| Native headless | Two repeats preserved eight original PNGs with identical bytes across Page.screenshot, CLI and MCP. The six processes exited and 46 samples showed zero visible windows. |
| Chrome WebGPU | 374 Material steps and 30,925 checks passed. Nine original menu PNGs are preserved. The guard sampled 4,732 times and found zero visible windows or remaining processes. |

M373 verifies one real pointer click on a disabled item. The menu stays
open with stable focus and checked values. M374 exercises keyboard, mouse
and touch on independent choices. M375 verifies selection, checked state,
measured rounded pixels, narrow dark RTL and Escape focus return.
The earlier failure capture shows why the test now uses the leading corner.

[Build and replay recipe](proof/README.md) · [Sealed file list](proof/SHA256SUMS)

```sh
python3 -E -B proof/collect.py --verify proof \
  --sha f70aeb506b23b137ff1643e26ae70106b2af16e272ca45bd2987dc80e436f927
```

[AppKit checkmark and regression evidence](https://github.com/faberline/surface/tree/c1681c13238c1929074f6e1d0382efd2a44aca39/docs/evidence/md3/vertical-choice-menu-v0176-diagnostic)

The candidate's Linux workflow runs when it reaches the release queue.
