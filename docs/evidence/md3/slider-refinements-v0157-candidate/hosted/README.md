# Surface v0.1.57 Slider Refinements hosted receipts

Candidate head `644d1e5faa1c25241306423c709ef58ca4c936be`, tree `72cf21a76d284e3a299aedff91b953eae232db07`, base `87dbf51ca5d552c6ad929b6fce5ee9439bc320a0`.
The exact 652-file source seal is `5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1`.

Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 314 ordered Material steps through M315.
The verifier independently checked seven Slider Refinements, four Range Slider, four Slider Forms, four Standard Slider, four Icon Buttons, four Chips, four Tabs, four Radio, four Checkbox, four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Radio, Checkbox, Switch, Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45544464596) passed 22,983 checks.
- [push](https://github.com/faberline/surface/actions/runs/45544453586) passed 22,983 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-slider-refinements.py --verify-package . --originals ..` to check the package and original receipts.
