# Surface v0.1.72 main verification

The original main workflow tested source `6aa6052b6a830db2b16b481744ac451f45f007df` and tree `8673365398b4c7b51f5e7409abe280d79b79d852`. It passed 1,570 Rust tests and three WebAssembly checks. Eight browser and bridge suites passed 30,284 checks. The Material Gallery ran 361 ordered steps.

The offline reader checked the complete original GitHub archive and decoded 40 SwiftShader PNGs. They cover Field and FieldText, text selection and disabled-label contrast.

[Source](https://github.com/faberline/surface/tree/6aa6052b6a830db2b16b481744ac451f45f007df) · [Original workflow](https://github.com/faberline/surface/actions/runs/46152748812/attempts/1) · [Original artifact](https://github.com/faberline/surface/actions/runs/46152748812/artifacts/15267454752)

Use Python 3.12 and Pillow to read back the exact archive from this directory.

```sh
python3 -E -B portable/hosted-main.py --verify-package portable \
  --binding-sha 3fbead50f81c0ccd9af50c501b12690de01e16c10f8c72d93b260d8e2ff7d346 --package-sha 805a1cc35a19ec1cf57b4457cafb1cf509a236276984eb1a0349c1c5be002cc8
```
