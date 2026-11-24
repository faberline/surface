# v0.1.42 native contrast on the final composition-cancel candidate

Source commit `86bf5eb86a5d070245db039b37977198ba1a417e` has tree `93b47517d26662b1e9aa5502f730349723fb0978`. The verified stock Gallery and CLI came from the same release candidate build as the browser pins. The test probe used the same 549 files and the included two-file native overlay. [Source commit](https://github.com/faberline/surface/commit/86bf5eb86a5d070245db039b37977198ba1a417e).

Two independent native offscreen Metal replays produced eight byte-identical PNGs and one byte-identical JSON report. Page.screenshot, CLI, and MCP returned matching bytes and metadata per capture. All 28 visible-window checks returned zero. The replays checked all source and executable digests before and after use.

The macOS startup reader returned a real Normal preference on this host. The probe then applied Increased and returned to Normal through an authenticated diagnostic input. The stock app rejected that input. Stock Normal matches the probe Normal byte for byte in the light and dark 320×400 right-to-left profiles at 200% text. Increased contrast changes the actual Metal pixels and focus outline. Restoring Normal restores the exact prior pixels and preserves the invalid draft, named error, saved value and focus. The test did not change any macOS preference. Live preference notifications, browser events, and system IME are separate checks.

| Receipt | SHA-256 |
| --- | --- |
| [Stock source manifest](stock-source.sha256) | `c99ed8ff89b0cee66a84732598ac2ba652802c2e52c9122c3c5bb27f250ccd4b` |
| [Probe source manifest](probe-source.sha256) | `14ef51e588621f2702f1f86a8013a998b2d219a203d02c797d9e5ee368493367` |
| [Native inputs](native-inputs.json) | `b10c08f14c23c8a69737862f41aaec761b6b7fe4e553731b7c77cf3675fde44d` |
| [Native contrast report](native-contrast-report.json) | `1b09e7f8847303fc7822b6f6da08f1df6f0a1471dda954fa89689888982ddc95` |
| [Repeat verification](native-repeat.json) | `cead05536510832077da5b384900e1e1e5a28cc1da7af869f587bf6f21586024` |
| [Upstream stock binary receipt](stock-bundle-binaries.sha256) | `a8d443a01b4a6251684beb5c331a124cf4203cc0e4a6850f3802a369d065fb11` |

[Light Increased](images/light-increased.png) · [Dark Increased](images/dark-increased.png) · [Stock dark Normal](images/stock-dark-normal.png)

## Reproduce

Check out the linked commit and verify `stock-source.sha256` against it. Give `SOURCE` the absolute source directory. Give `STOCK_STAGE` the absolute directory containing the independently built stock Gallery and CLI in `bundle/debug`. The exact replay accepted the release gate's `incoming-source.sha256`, `built-input-sha256.txt`, and `browser-binary-sha256.txt`.

For an independent native-only rerun, build stock with `cap cargo build --locked -p surface-material-gallery -p surface-devtools --features surface-material-gallery/devtools` in a fresh Cargo target, using two jobs. Copy those two executables into a fresh stock stage. Once the source manifest matches the exact checkout before and after its build, save that manifest under both source receipt names. Write the two stock binary digests from inside `bundle` to the binary receipt. Use a populated Cargo cache and the macOS Command Line Tools.

```sh
OUT=$(mktemp -d /tmp/surface-native-contrast.XXXXXXXX)
python3 -B native-scripts/native-proof.py prepare \
  --source "$SOURCE" --source-manifest stock-source.sha256 \
  --source-sha256 c99ed8ff89b0cee66a84732598ac2ba652802c2e52c9122c3c5bb27f250ccd4b \
  --head 86bf5eb86a5d070245db039b37977198ba1a417e \
  --tree 93b47517d26662b1e9aa5502f730349723fb0978 \
  --stock-stage "$STOCK_STAGE" --out "$OUT/sealed"
python3 -B native-scripts/native-proof.py replay --sealed "$OUT/sealed" --out "$OUT/replays"
```

The probe builds in a separate target and never modifies the stock checkout or stock executable. The replay records runtime PNG bytes and binary digests. A different build location can change executable digests. The two runs of one pinned build must still produce matching PNGs and reports.
