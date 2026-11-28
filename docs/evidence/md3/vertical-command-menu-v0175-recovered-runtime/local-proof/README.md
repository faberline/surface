# Surface v0.1.75 recovered validation evidence

The collector verifies the exact regenerated source tree and its 17-path patch. It reads the original workspace log and checks three Wasm commands. It checks the producer pins and the actual ignored tests on Metal. It decodes the original GPU PNGs.

The browser verifier requires the unmodified Material registry. It reads all 371 ordered Material steps through M372, with M23 retired. It checks both Menu reports and decodes the six original Menu PNGs. It also checks the observed Chrome launch and process closure. The native verifier reads the original eight PNGs, the equality of the three screenshot transports, and the six guarded Gallery exits.

The recovered GitHub source archive identifies the source used by this rerun. The new tests establish the runtime results. This evidence covers the macOS workspace, Metal, Material browser suite, and headless native Menu replay. Hosted Linux and the other browser suites are separate.

[Exact source recovery on GitHub](https://github.com/faberline/surface/tree/4b7a41753fc00b996b3a1b70885e272876cb77ca/docs/evidence/md3/vertical-command-menu-v0175-recovered-source)

## Check or collect

Use Python 3.12 and Pillow. These commands start no app and do no build.

    py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
    lane=$(pwd -P)
    "$py" -E -B "$lane/package.py" --preflight
    "$py" -E -B "$lane/package.py" --collect "$lane/portable"

The collector expects the exact stage at `/tmp/surface-md3-menu-v0175-recovered.uSvgC4AK`. It reads `guarded-stock-material` and `guarded-native` there. Pass `--browser` or `--native` only if the real replay output has a different directory. It refuses to write output while a required gate is pending.

The archive keeps reports, guard records, byte-identical PNGs, source and build manifests, compressed original logs, and the small offline readers. The app binaries and browser profiles remain in the original lanes.

From the portable proof directory, rerun the offline check using the SHA-256
printed by the collector.

    python3 -E -B package.py --verify . --sha REVIEWED_SHA

This rereads the archive and verifies the actual receipts. It needs neither the original source nor the original binaries.
