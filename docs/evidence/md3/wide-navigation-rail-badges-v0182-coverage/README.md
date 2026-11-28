# Wide navigation rail Badge coverage

This record updates the Badge and navigation rail rows in the coverage ledger.
Both rows remain partial.
The covered candidate is `0882d02c61ca50232604950bc48d4cf280122e21`, with tree `6f6ac718805d6a0526d07459d8e3de9c05008ea0`.
The runtime evidence was collected from `7a05c24179b9e2ddf582ddc811cb7cc88a965bde`.

Restore the source bundle from the runtime record.
Apply `coverage.patch` with `git apply --index`.
Check the tree with `git write-tree`.
Run `shasum -a 256 -c source.sha256` in the restored checkout.
The recorded replay passed all 716 entries.

[Exact runtime receipt and source bundle](https://github.com/faberline/surface/tree/5a601f57c703fdf780702c4dd06b6c080ff1f0fa/docs/evidence/md3/wide-navigation-rail-badges-v0182-runtime)
