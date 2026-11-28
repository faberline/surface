# Surface v0.1.69 main verification

GitHub Actions ran the original workflow on the merged `main` commit
`9eea96a507af5e9cbf43dbd85361b8ff49f0c728`, tree
`5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb`. Both jobs passed in attempt 1.

- The Linux build job passed 1,559 Rust tests and 3 WebAssembly checks.
- The browser and bridge job passed 8 suites and 26,228 checks.
  The Material Gallery passed 352 steps and 25,570 checks.
- The offline reader decoded 58 original SwiftShader PNGs. It checked
  the typed field and hint state, measured the image pixels, and matched the
  selected files against the original GitHub artifact ZIP.

[Exact source](https://github.com/faberline/surface/tree/9eea96a507af5e9cbf43dbd85361b8ff49f0c728)

[Original workflow](https://github.com/faberline/surface/actions/runs/46129267669/attempts/1)

[Original artifact](https://github.com/faberline/surface/actions/runs/46129267669/artifacts/15257558174)

The `portable` folder has the original ZIP, compressed logs, workflow
receipts, decoded PNG inputs and offline reader. From this folder, run
Python 3.12 with Pillow installed.

```sh
python3 -E -B portable/verify-hosted.py \
  --verify-package portable --package-sha e46b90691018af8e6cb2f6d64fcb2eb1769df77023bd791df0132a6512cce42a
```
