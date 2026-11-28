# Surface v0.1.71 original review verification

The original branch and pull request workflows passed on attempt 1.
They tested source tree `e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc`.
The branch checked out `da9403992bd643c86a5bba35071f809857c90e44`.
The pull request checked out `8189db7873b199032d94bc8ecaa85e048c5c3f1a`.

- The branch passed 1,564 Rust tests and 29,761 browser and bridge checks.
- The pull request passed 1,564 Rust tests and 29,761 browser and bridge checks.
- Both passed three WebAssembly checks and eight browser and bridge suites.
  Each Material run passed 358 steps.
- The offline reader decoded 15 original SwiftShader PNGs from each run.
  It checked field geometry and disabled and read-only ink.

[Pull request](https://github.com/faberline/surface/pull/247)

[Branch run](https://github.com/faberline/surface/actions/runs/46140687061/attempts/1) · [Original branch ZIP](https://github.com/faberline/surface/actions/runs/46140687061/artifacts/15263057298)

[Pull request run](https://github.com/faberline/surface/actions/runs/46140694868/attempts/1) · [Original pull request ZIP](https://github.com/faberline/surface/actions/runs/46140694868/artifacts/15263415307)

Use CPython 3.12 and Pillow 12.3.0. Run this command from this folder.

```sh
python3 -E -B portable/hosted.py --verify-package portable \
  --package-sha c27f46dc6d967d68da2130977c4e536abfddbfec21655120ba5a203aa3256b01
```
