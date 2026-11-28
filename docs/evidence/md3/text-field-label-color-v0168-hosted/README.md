# Surface v0.1.68 Linux review evidence

The original PR and branch workflows each passed 1,550 Rust tests and
three WebAssembly checks. Twenty ignored Rust tests did not run in either
workflow. Each workflow passed eight browser and bridge suites, with
25,653 checks and 349 Material steps.

The reviewed source tree is `9d3ce235f8efde1459fd41862cd918367fc85207`.
The original push checked out `26758a7313527c9aab7c2d5e677ec880866638da`.
The original PR checked out `0e77157bb75a8547c68ccee11633db9eee145291`.
Both checkouts have the same source tree.

The portable verifier decoded 34 PNGs from each original Chrome WebGPU
artifact. All four M350 samples reached RGB 206, 191, 243 at 44 ms on
SwiftShader. The package keeps both original ZIPs, job logs, exact source
and job identifiers, and the extracted PNGs.

[Original PR workflow](https://github.com/faberline/surface/actions/runs/46118336963) ·
[Original branch workflow](https://github.com/faberline/surface/actions/runs/46118164503)

## Offline readback

From this directory on Python 3.12 with Pillow, run the verifier against
the portable package.

```sh
python3 -E -B portable/verify-hosted.py --verify-package portable --package-sha a48d7c66607c74fee34f76714e04da27ba937eb3fde3643edb1b50c67ee30c81
```
