# Surface v0.1.52 Chips hosted receipts

Candidate head `a01bbaa71bce622603239dee9abcd98951306827`, tree `48d560ab127cc1c692a5ba0eba6fb5a8949234a8`, base `de11edf089a273c794875d54597a08e4ff1d9c0f`.
The exact 624-file source seal is `8d2548019eca22046b20992b27f48f4404cb0f5af84f26d6d864d6de498509b1`.

Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 292 ordered Material steps through M293.
The verifier independently checked four Chips, four Tabs, four Radio, four Checkbox, four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Radio, Checkbox, Switch, Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45520724951) passed 21,326 checks.
- [push](https://github.com/faberline/surface/actions/runs/45520711073) passed 21,326 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-chips.py --verify-package . --originals ..` to check the package and original receipts.
