# Surface v0.1.73 main verification

The original main workflow checked commit `160df74d7c26d2fd5ee25aff4133e423f492b505` and tree `791d8b2dd3f9b44f3c2a967f1418b671920a2899`.
It passed 1,575 Rust tests, three Wasm checks and 30,907 browser and bridge checks.
The Material suite ran 364 ordered steps and 30,249 checks.

The original GitHub artifact ZIP matched its download digest and byte count.
The portable package preserves the receipts, original logs and three representative WebGPU PNGs.

Re-run the portable readback with Python 3.12 and Pillow.

```sh
python3 -E -B portable/hosted-main.py --verify portable --config-sha eda804e2efd74a51dba757279a1d46a9a546446ac5283a93b1a64f34aa9c97b7 --manifest-sha 54128e170a5bc3391c05d42faaf2c3cf2fe5dbf1aec07522b4ca45f575944351
```

[Original GitHub workflow](https://github.com/faberline/surface/actions/runs/46160413309)
