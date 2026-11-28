# q73 original hosted evidence

Preparation only. q72 must land and release first. q73 PR and run IDs are not known yet. This tool does not create or change a PR or workflow.

It binds q73 product `c2571a3d818ddaf0cb7874998d1c39f0ffc7b1f2`, base `6aa6052b6a830db2b16b481744ac451f45f007df`, and tree `791d8b2dd3f9b44f3c2a967f1418b671920a2899`. If the actual product source changes, review and update these pins before use. A reviewed config identifies exactly one PR, event, branch, attempt-1 run, both job IDs and checkout. Push checkout is the product head; PR checkout is the actual two-parent synthetic merge. Both jobs must pass before any download.

The collector downloads original small logs and the original artifact ZIP. It checks the GitHub receipt's size and SHA-256 against the actual ZIP, extracts three Horizontal PNGs and two typed reports, then removes the temporary ZIP. The portable subset contains exact original API receipts, logs, selected files, independent pixel reader and a seal. It does not contain the full ZIP. Offline readback checks the package and selected images, but cannot independently rehash the omitted ZIP. Retain the independently reviewed config and package SHA values outside the package.

Use Python 3.12 and Pillow. After the actual run is green, copy `config.example.json`, fill exact reviewed values and run:

```sh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
kit=/tmp/surface-md3-v0173-hosted-scaffold.u70ssRO8
config_sha=REVIEWED_CONFIG_SHA256
"$py" -E -B "$kit/hosted.py" --collect /tmp/NEW_Q73_PACKAGE --config "$kit/config.json" --config-sha "$config_sha"
"$py" -E -B /tmp/NEW_Q73_PACKAGE/hosted.py --verify /tmp/NEW_Q73_PACKAGE --config-sha "$config_sha" --manifest-sha REVIEWED_PACKAGE_SHA256
```

The reader checks 364 ordered Material steps, 20 ordered IME steps, eight suites, required Rust and WebAssembly steps, SwiftShader adapter report, and direct Pillow geometry and pixels from all three q73 PNGs. Earlier local fixed-source proof is [here](https://github.com/faberline/surface/tree/fa9c096b28747b91d597120e3194decfc31da4b4/docs/evidence/md3/horizontal-navigation-bar-v0173-corrected-runtime). It is not hosted CI evidence.
