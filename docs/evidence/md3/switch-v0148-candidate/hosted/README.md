# Surface v0.1.48 Switch hosted receipts

Candidate head `1346beff8a2dfcc124e0cde4d78c4d91608d46a8`, tree `4b1a812f2d384adeede0e1619dc0cfc9c1e7c960`, base `7526fac5d0546b17afe7110d923b8d490a5e6484`.
The exact 598-file source seal is `a4ec5bc9d9c0da42fe004f6f7d6877f59edc08e5503ae15c3bbefb6c73f4d086`.

Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 276 ordered Material steps through M277.
The verifier independently checked four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45499168058) passed 19,863 checks.
- [push](https://github.com/faberline/surface/actions/runs/45499148999) passed 19,863 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-switch.py --verify-package .` to check the package.
