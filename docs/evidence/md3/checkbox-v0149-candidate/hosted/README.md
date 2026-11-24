# Surface v0.1.49 Checkbox hosted receipts

Candidate head `8e5c7220c750fa709c6a9b91216345e04947a028`, tree `238edf883656d675e05b02ee7df280e35fbd988b`, base `dc85a3714fc11c1708155c48f23cfa6c637b3498`.
The exact 602-file source seal is `3a9aeda30a8ca3520afd04d9d4db8c26f3a900dc5aae20fad14e4d366d2544fe`.

Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 280 ordered Material steps through M281.
The verifier independently checked four Checkbox, four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Switch, Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45504989995) passed 20,204 checks.
- [push](https://github.com/faberline/surface/actions/runs/45504974898) passed 20,204 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-checkbox.py --verify-package . --originals ..` to check the package and original receipts.
