# Surface v0.1.51 Tabs hosted receipts

Candidate head `e8163ad77a063adbbcc6df4fae99eb9cb9b1c406`, tree `187930ac394178c821649c76b0f29c2f9c751b9e`, base `fceb6a45b10eca20d94fb04e0f77ec949407d3ac`.
The exact 616-file source seal is `f928486f4fd04824a503424b0ebf53caa5b2784ba363dae928c7fca17c202cd0`.

Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 288 ordered Material steps through M289.
The verifier independently checked four Tabs, four Radio, four Checkbox, four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Radio, Checkbox, Switch, Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45515434800) passed 21,041 checks.
- [push](https://github.com/faberline/surface/actions/runs/45515420949) passed 21,041 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-tabs.py --verify-package . --originals ..` to check the package and original receipts.
