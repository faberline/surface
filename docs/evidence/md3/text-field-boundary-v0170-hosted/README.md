# Surface v0.1.70 original review verification

The original branch and pull request workflows passed in attempt 1.
They tested source tree `87f50f8095b38a4ba37c83f71aaf1119dda945dc`.
The branch checked out `a9777c4cac71ac9f122729c4b1e7ed2b1ca6c06b`.
The pull request checked out `131b57a93c131ee9d24be94c2390a1fc594d9b7f`, with the same tree.

- The branch passed 1,562 Rust tests and 29,083 browser and bridge checks.
- The pull request passed 1,562 Rust tests and 29,083 browser and bridge checks.
- Both runs passed three WebAssembly checks and eight browser and bridge suites.
  Each Material run passed 355 steps.
- The offline reader decoded 33 original SwiftShader border PNGs from each run.
  It checked border geometry, motion, colors and matching ZIP entries.

[Pull request](https://github.com/faberline/surface/pull/246)

[Branch run](https://github.com/faberline/surface/actions/runs/46132708724/attempts/1) · [Original branch ZIP](https://github.com/faberline/surface/actions/runs/46132708724/artifacts/15258864397)

[Pull request run](https://github.com/faberline/surface/actions/runs/46132730417/attempts/1) · [Original pull request ZIP](https://github.com/faberline/surface/actions/runs/46132730417/artifacts/15259696485)

With Python 3.12 and Pillow, run this command from the archive folder.

```sh
python3 -E -B portable/hosted.py --verify-package portable \
  --package-sha fb135ef04416c15d50e28d12d9abb4dbb55ad172e7ee5611bea18498048e4109
```
