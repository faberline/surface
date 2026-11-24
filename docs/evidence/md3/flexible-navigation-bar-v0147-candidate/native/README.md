# Flexible Navigation bar v0.1.47 native replay after the Divider sampling change

The replays use the stock Gallery and DevTools CLI built from the exact
594-file source with SHA-256 `1b4c1cc1acc3719c5064fd678f91cf99a98c0dc8d1c2204b8d77eb08e732ac76`.
The seal checks run against the stock build metadata and every pinned source,
harness, and binary file. Each run uses native headless rendering.

The wide light 960 × 800 dp bar occupies `[240,736,480,64]`. The narrow dark
RTL 320 × 400 dp bar at 200% text occupies `[0,288,320,112]`. The pinned
selected destination heading and the wrapped updates label remain readable.
Both replays produce the same two PNG files through `Page.screenshot`, the
CLI, and MCP. Both PNGs also match the prior exact v0.1.47 native run.

[Verified receipt](verified-native.json) · [Process check](process-cleanup.json) · [Replay 1](replay-1/native-gallery-report.json) · [Replay 2](replay-2/native-gallery-report.json)

## Reproduce

Use macOS and Python 3.11 or later. Set `release_stage` to the exact source
stage and `stock_stage` to its sealed stock build directory. The script
copies only the two native executables and runs them with `--headless`.
It uses local loopback to check the screenshot transports.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
python3 -B scripts/verify-native.py
replay=$(mktemp -d /tmp/surface-nav-native.XXXXXXXX)
cp -R scripts "$replay/"
sh "$replay/scripts/replay-native.sh" "$release_stage" "$stock_stage" \
    1b4c1cc1acc3719c5064fd678f91cf99a98c0dc8d1c2204b8d77eb08e732ac76
python3 -B "$replay/scripts/verify-native.py"
python3 -B "$replay/scripts/check-remaining.py"
```

The verifier accepts `--preflight` with an earlier replay directory for
another independent pixel and profile comparison.
