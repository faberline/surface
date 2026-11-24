# Divider v0.1.45 native replay after the browser sampling change

The replays use the stock Gallery and DevTools CLI built from the exact
583-file source with SHA-256 `a2750ba3be68e0b6bc5b6f0183d4d21acf7e90fe9c4a65e623ecacbd192dd0aa`.
The seal checks run against the stock build metadata and every pinned source,
harness, and binary file. The headless replay opens no visible windows.

Each replay checks 1 dp Regular and 8 dp Heavy dividers at 960 × 800 dp in
light LTR and 320 × 800 dp in dark RTL with 200% text. It checks four
geometries, exact color, clean adjacent pixels, and focus restoration.
Every PNG and its screenshot metadata match across `Page.screenshot`, the
CLI, and MCP. The four PNGs also match the previous v0.1.45 native run from
before the browser-only change.

[Verified receipt](verified-native.json) · [Earlier comparison](earlier-comparison.json) · [Replay 1](replay-1/native-divider-report.json) · [Replay 2](replay-2/native-divider-report.json)

## Reproduce

Use macOS and Python 3.11 or later. Set `release_stage` to the exact source
stage and `stock_stage` to the sealed stock build directory. The script
copies only the two native executables and runs them with `--headless`.
It uses local loopback to check the screenshot transports.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
python3 -B scripts/verify-native.py
replay=$(mktemp -d /tmp/surface-divider-native.XXXXXXXX)
cp -R scripts native-scripts "$replay/"
sh "$replay/scripts/replay-native.sh" "$release_stage" "$stock_stage" \
    a2750ba3be68e0b6bc5b6f0183d4d21acf7e90fe9c4a65e623ecacbd192dd0aa
```

The verifier accepts `--earlier` with the earlier native replay directory to
check the byte comparison again. This archive covers native replay.
