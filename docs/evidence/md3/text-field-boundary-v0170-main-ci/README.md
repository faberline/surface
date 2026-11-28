# Surface v0.1.70 main verification

GitHub Actions passed on the merged `main` commit `edbcabf5495f24df8941a75572d994bb8d7b2f73` and
source tree `87f50f8095b38a4ba37c83f71aaf1119dda945dc` in the original attempt.

- The build job passed 1,562 Rust tests and three WebAssembly checks.
- The browser and bridge job passed eight suites and 29,083 checks.
  The Material Gallery passed 355 steps.
- The offline reader checked geometry, motion and color in 33 original
  SwiftShader PNGs. The reader compared the files with the original ZIP.

[Exact source](https://github.com/faberline/surface/tree/edbcabf5495f24df8941a75572d994bb8d7b2f73)

[Original workflow](https://github.com/faberline/surface/actions/runs/46136203079/attempts/1)

[Original artifact](https://github.com/faberline/surface/actions/runs/46136203079/artifacts/15260882618)

Use Python 3.12 with Pillow. Run this command from the archive folder.

```sh
python3 -E -B portable/hosted-main.py --verify-package portable \
  --binding-sha e99c2cec4d4a24260911b5537bb34f84e172af37c9f44a81ecd504bea3b5b452 \
  --package-sha 529b12ca24fb617297a93ca37643b851f421c7c1b4553bde93e7be75fd86499e
```
