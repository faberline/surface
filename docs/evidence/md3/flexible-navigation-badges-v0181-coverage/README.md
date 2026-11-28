# Flexible navigation badge coverage update

Candidate `fdce7102dc6b88011c2e806f6ea076f0769b2c37` has the covered tree `7003aaa98b356de153c57cdcfc195cc0f7ef476d`.
Its only change from the tested runtime source `b9dc4ad7a5c6cdddd71196a669354f17984af7aa` is the coverage ledger.
The Badge and navigation bar rows retain their partial status.
The remaining family and foundation rows are unchanged.

Restore the tested source bundle, then use `git apply --index coverage.patch`.
Check `git write-tree` against the tree above.
Run `shasum -a 256 -c source.sha256` from the reconstructed checkout.
The archived replay passed all 714 source entries.

[Exact tested runtime and source bundle](https://github.com/faberline/surface/tree/69e9886676be828acd93d64b1c4252227c4d5d82/docs/evidence/md3/flexible-navigation-badges-v0181-runtime)
