# Adaptive navigation Badge coverage

This record updates the Badge, Navigation bar and Navigation rail rows in the coverage ledger.
All three rows remain partial.
The covered candidate is `25ebb788865e70a7a86cd9f08e2883c838dfd50f`, with tree `aeceb044df4b3721be433a5e29fa01ee3a322d64`.
The runtime evidence was collected from `758b0af489e659fb871bdf2b238340c4f1ff479c`.

Restore the source bundle from the runtime record.
Apply `coverage.patch` with `git apply --index`.
Check the tree with `git write-tree`.
Run `shasum -a 256 -c source.sha256` in the restored checkout.
The recorded replay passed all 717 entries.

[Exact runtime receipt and source bundle](https://github.com/faberline/surface/tree/393354ccf526105b6087b7ea3286c1f04b464e2d/docs/evidence/md3/adaptive-navigation-badges-v0183-runtime)
