# Surface v0.1.54 Standard Slider hosted receipts

Candidate head `1d5d47994d256e1b7486052e1fe7feafd51ddac3`, tree `a92d66c4998656b1749f322723aa4324028afdc4`, base `3913703a699289229088f25aac2092565b8fd2d2`.
The exact 635-file source seal is `1e1c1be43dbd652ff26dbaab2f264651c2c811915bc04e41dfbedd425ba830d8`.

Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 300 ordered Material steps through M301.
The verifier independently checked four Standard Slider, four Icon Buttons, four Chips, four Tabs, four Radio, four Checkbox, four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Radio, Checkbox, Switch, Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45530199487) passed 21,836 checks.
- [push](https://github.com/faberline/surface/actions/runs/45530187217) passed 21,836 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-sliders.py --verify-package . --originals ..` to check the package and original receipts.
