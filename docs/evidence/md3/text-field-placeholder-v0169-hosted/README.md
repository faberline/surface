# Surface v0.1.69 Linux review evidence

Both original GitHub workflows tested source tree
`5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb`. The branch used commit
`b600ceb3b413776abe47195476e5cc91e50499aa`. The PR used GitHub's
checkout `ba21abb2fa3ba7cf42445d09d6b2809eb16de3d4`.

Each workflow passed 1,559 Rust tests and three WebAssembly checks.
Twenty ignored Rust tests did not run. All eight browser and bridge
suites passed 26,228 checks. The Material suite ran 352 steps.

The decoder verified 58 original hint PNGs from each Chrome WebGPU run
on SwiftShader. M351–M353 exercise hint timing, direct composition and
the compact right-to-left layout. The package keeps both original
artifacts, exact run and job identifiers, build logs and captured PNGs.

[Original PR workflow](https://github.com/faberline/surface/actions/runs/46125223452/attempts/1) ·
[Original branch workflow](https://github.com/faberline/surface/actions/runs/46125215068/attempts/1)

## Offline readback

From this directory on Python 3.12 with Pillow, run the verifier.

```sh
python3 -E -B portable/verify-hosted.py --verify-package portable --package-sha 7ba6fb9e6c2cc8a99cf202d6e12f67959b36937b0bc125137f442e7187b521cb
```
