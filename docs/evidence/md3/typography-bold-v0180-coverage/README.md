# Typography coverage update

Candidate `47d194868af375cd4c47e26a425cff2968a57a2a` records the verified Bold 700 foundation in `docs/material-coverage.json`. The candidate tree is `08d94b8d63b42e7a34dab1f7ee16354ce58e2dda`. It changes only the coverage record from tested source `50c84895b631114df5773769ff69b01d00a7859b`.

One of 37 component families is complete in this candidate. The typography foundation remains open for Medium 500, rendered letter spacing, separate font roles, per-locale scaling and a full Gallery scale.

[Exact tested source and runtime evidence](https://github.com/faberline/surface/tree/9e0c4344c7271639ace9a482ff2806eadfa9e28d/docs/evidence/md3/typography-bold-v0180-runtime)

Reconstruct the tested source from its self-contained bundle. Apply `typography-coverage-only.patch` with `git apply --index`. Verify the resulting tree with `git write-tree` and all 711 files with `shasum -a 256 -c source-coverage.sha256` from that source checkout.
