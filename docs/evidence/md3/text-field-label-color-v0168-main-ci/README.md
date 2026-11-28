# Surface v0.1.68 main verification

GitHub ran both required jobs on commit
`987b5ce8bc4aa260e0cda30c2278ebb3317f879f` and source tree
`9d3ce235f8efde1459fd41862cd918367fc85207`.

The Linux workspace passed 1,550 Rust tests and three WebAssembly checks.
Twenty ignored Rust tests did not run. All eight browser and bridge suites
passed 25,653 checks. The Material suite ran 349 steps.

The verifier read 34 original Chrome WebGPU PNGs on SwiftShader. All four
M350 pixels reached RGB 206, 191, 243 at 44 ms. The package retains the
original artifact, logs, screenshots and exact source identifiers.

[Original main workflow](https://github.com/faberline/surface/actions/runs/46122360299/attempts/1) ·
[Original browser artifact](https://github.com/faberline/surface/actions/runs/46122360299/artifacts/15255516725)

## Offline readback

From this directory on Python 3.12 with Pillow, run the verifier.

```sh
python3 -E -B portable/verify-hosted.py --verify-package portable --package-sha b7d5462ec0fc60f256aaf82fe48ba7afbc6c9069e3f26f3abda0fa04621c2137
```
