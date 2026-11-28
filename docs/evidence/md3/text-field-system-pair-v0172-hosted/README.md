# Surface v0.1.72 original review checks

The original push and pull request workflows tested source tree `8673365398b4c7b51f5e7409abe280d79b79d852`. The push ran commit `e99b6c060304ad3de74237d9cdc5e152cb8237d8`. The pull request ran the original merge checkout `c946335980ccf66d11ac3cb56a9087f9062df3fa`.

Each run passed 1,570 Rust tests and eight browser and bridge suites. The Material Gallery passed 361 ordered steps with 29,626 checks. The full browser and bridge total was 30,284 checks.

The offline reader checked each complete GitHub ZIP and decoded 40 original PNGs in each run. They cover 14 Field and FieldText views, 11 text selection views and 15 disabled-label contrast views. The selection check compared 12,876 original pixels per run.

[Pull request](https://github.com/faberline/surface/pull/248) · [Original branch workflow](https://github.com/faberline/surface/actions/runs/46149096116/attempts/1) · [Original pull request workflow](https://github.com/faberline/surface/actions/runs/46149103037/attempts/1)

[Branch artifact](https://github.com/faberline/surface/actions/runs/46149096116/artifacts/15265514778) · [Pull request artifact](https://github.com/faberline/surface/actions/runs/46149103037/artifacts/15265233888)

Use Python 3.12 with Pillow. Run the offline check from this directory.

```sh
python3 -E -B portable/hosted.py --verify-package portable --package-sha 2ed94b2d165ff9ae0813ffcf5d997381ae19ff159eaf818003090dd02fc1741d
```
