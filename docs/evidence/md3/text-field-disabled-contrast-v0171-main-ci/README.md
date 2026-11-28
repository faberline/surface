# Surface v0.1.71 main verification

Original main attempt passed for source `a64f60b796b7bffcd376a99aac903b5235dee2b7` and tree `e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc`.

- Rust tests passed: 1564.
- Browser suites: 8; checks: 29761.
- Material Gallery: 358 ordered steps.
- The offline reader decoded 15 original disabled-contrast PNGs and checked the original ZIP.

[Source](https://github.com/faberline/surface/tree/a64f60b796b7bffcd376a99aac903b5235dee2b7) · [Original workflow](https://github.com/faberline/surface/actions/runs/46144848619/attempts/1) · [Original artifact](https://github.com/faberline/surface/actions/runs/46144848619/artifacts/15264692666)

Use Python 3.12 and Pillow. From this directory:

```sh
python3 -E -B portable/hosted-main.py --verify-package portable --binding-sha 0024dd5cf523242d73c5ba3d6121121cf61b6d3e74c1372823b29ec45bdb74a3 --package-sha c072fe202a3efa00461856f5287f1d19e42d7d90186846a93c45186f30845f49
```
