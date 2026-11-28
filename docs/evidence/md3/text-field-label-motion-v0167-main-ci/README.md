# Surface v0.1.67 exact main verification

GitHub tested the merged main commit `6fb40370ca824b8ed44e6e17099e6156f8d15a9b`
with source tree `c218faaf7527ec2872fb1ec1097c87667f3bf1f2`.
[Main push workflow](https://github.com/faberline/surface/actions/runs/46113171688/attempts/1)

The workspace passed 1,545 Rust tests and three WebAssembly checks.
The Rust run left 20 tests ignored. The Chrome and bridge run passed all
eight suites and 25,350 checks. It ran 349 Material steps. The original
artifact checksums and 15 label motion PNGs match the verified receipt.
Linux Chrome used SwiftShader. The [macOS evidence](https://github.com/faberline/surface/tree/d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4/docs/evidence/md3/text-field-label-motion-v0167-candidate)
records the separate Apple Metal and native runs.

Run the [packaged verifier](proof/README.md) with Python 3.12 and Pillow.

```sh
python3 -E -B proof/verify-main.py --verify proof \
  --sha 429effb53c718f211b15adb081cc5604df4eafef48d25c3037eae309e6d7c05b
```
