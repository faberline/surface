# Snackbar motion coverage

The exact runtime source is `b067e40910e9f86683ef6352ec631e9c8a8436dc`.
The coverage-only commit is `7abe194029f04ac1c112f1a6a574f97c3229783d` with tree `1d41794c6738070c1741eff8cf6385ca85089a5c`.
It includes the previously verified adaptive navigation Badge entries and this Snackbar update.
The Snackbar row remains partial while live screen-reader speech is open.

Restore the source bundle from the runtime evidence. Apply `coverage.patch` with
`git apply --index`, then check the tree with `git write-tree`. In the restored
checkout, run `shasum -a 256 -c source.sha256` using the archived manifest path.
The recorded replay checked 718 files.

[Exact runtime and source](https://github.com/faberline/surface/tree/87b86d99a0ef0a2c039a5ce81a08970ccfbf3e0d/docs/evidence/md3/snackbar-motion-v0184-runtime)
· [Prior Badge coverage](https://github.com/faberline/surface/tree/8d9cc1b7e14539093b5e51eb6deef13b18124224/docs/evidence/md3/adaptive-navigation-badges-v0183-coverage)
