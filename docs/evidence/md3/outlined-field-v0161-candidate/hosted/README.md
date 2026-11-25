# Surface v0.1.61 Outlined Text Field R2 hosted receipts

Candidate head `c6d7d06861d3f6221d06c1c6dd682692fa1984a5`, tree `7c7881f78c3dc44129f52688a079b6874c9a4991`, base `98c62d31d1646878b1a5393f84aa896e52aa2985`.
The exact 668-file source seal is `3c79ae5ee15bf3b9b786d37452f1e0e5929ac0d31276ca7f6506897462dd9cc8`.

Accepted hosted evidence is the original pull request (attempt 1) and the full same-source push retry (attempt 2).
Original push attempt 1 failed the earlier Tabs M287 in-between-frame check. Its artifact identity and
the separate preserved failure manifest are recorded in hosted-verified.json. It is excluded from acceptance.

Both accepted stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed
SwiftShader adapter. Each workflow ran 327 ordered Material steps through M328.
The verifier independently checked five Outlined Text Field PNGs, seven Segmented Button PNGs, including both touch directions, and seven Common Buttons, seven Slider Refinements, four Range Slider, four Slider Forms, four Standard Slider, four Icon Buttons, four Chips, four Tabs, four Radio, four Checkbox, four Switch, four Navigation, three Menu and four
Divider PNGs, both I11 cancellation paths and strict Divider stripes.
The Radio, Checkbox, Switch, Navigation and Menu validators accept the enclosing stock step count. Their
image and behavior checks match the included original validators.

- [pull_request](https://github.com/faberline/surface/actions/runs/45569894616) passed 24,033 checks.
- [push](https://github.com/faberline/surface/actions/runs/45569881486) passed 24,033 checks.

The original GitHub ZIP identities and member SHA-256 values are in each
`verified.json`. The complete original ZIPs and logs remain in the local
capture directory. This package contains selected unchanged artifact members.

Run `shasum -a 256 -c SHA256SUMS` and
`python3 -B verify-hosted-outlined-field.py --verify-package . --originals ..` to check the package and original receipts.
