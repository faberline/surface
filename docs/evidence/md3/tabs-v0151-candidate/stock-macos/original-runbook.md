# Surface v0.1.51 Tabs, guarded macOS stock

The prepared bundle uses the approved 616-file v0.1.51 source seal
`f928486f4fd04824a503424b0ebf53caa5b2784ba363dae928c7fca17c202cd0`.
Its 16-file binary manifest SHA-256 is
`6f8f71726c86cb73d1d7a86fec3ebaf6b4a4b8a8164bc05a76a63d21d394a56c`.
The verified native and Web source roots and build markers are in
`source-origins.json` and `build-markers.json`.

The observer passes `smoke ime material bridge-web devtools-headless`
explicitly. Before startup, it checks the exact runner and every source
and binary hash. It admits the primary and detached Bridge Chrome only with
separate private profiles and `--headless=new`. The Gallery needs
`--headless` and a private DevTools session. A compact guard rejects any
unexpected windowed native suite. CoreGraphics samples the private process
family during the run. Any visible owned window, surviving owned process
or guard rejection fails the run. Source and binary hashes must still
match after the processes stop.

The stock Material proof requires 288 steps through M289 with M23 omitted.
It preserves the original `material-tabs.json` and four distinct PNGs
`fixed-light`, `fixed-dark`, `dark-pressed` and `rtl-320`. The three
wide images are 1920 × 1688 pixels. The compact image is 640 × 1608 pixels.
Those dimensions also match the scoped Chrome r4 development preflight. The stock verifier
checks observed Chrome and Apple Metal metadata, the raw result, all
ordered cases and original capture bytes. A read-only second pass verifies
the earlier Radio, Checkbox, Switch, Navigation, Menu, Divider and I11
evidence from the same stock run. Its inherited verifiers take the
enclosing Material level from the caller.

## Fixtures exercised before launch

The guard predicates and sealed input recheck ran without starting any
executable. The report contract and PNG dimensions were checked against
the scoped Mac Chrome r4 development captures. That web package reported
v0.1.50. The compared UI source files match the final build. The
stock run below uses the exact v0.1.51 binary and source seals.
See `provenance/scoped-preflight-readback.json` for the version readback.
`self-test.py` uses an
actual archived v0.1.50 run plus short-lived synthetic v0.1.51 fixtures
to check acceptance, rejected launches, wrong report or source pins, and
archive corruption. The synthetic controls are separate from the release
proof. Live VoiceOver and OS input methods are outside this verification.

## Launch the prepared bundle

Use Python 3.11 or later with Pillow. Use a fresh private output directory
and grant the same local access as the previous guarded run so
Chrome can use loopback and the observer can count owned windows.

```sh
python3 -B observe-tabs.py --self-test
python3 -B observe-tabs.py --verify-inputs-only \
  --runner-sha256 "$(cat approved-runner.sha256)"
python3 -B observe-tabs.py --out "$PWD/run" \
  --runner-sha256 "$(cat approved-runner.sha256)"
```

The original runner log, guard ledger, screenshots and window receipt
remain under `run/` after completion or failure. After a successful run,
verify it without starting another process and seal a durable copy.

```sh
python3 -B verify-existing.py --run "$PWD/run"
python3 -B archive-proof.py --seal "$PWD/portable-proof"
python3 -B archive-proof.py --verify "$PWD/portable-proof"
```

The archive keeps the raw evidence, scripts and hashes. The runnable
binaries stay outside it. To prepare another approved exact build in a
fresh private directory, copy the scripts and retained validators and
use `prepare-pins.py --help`. This stage only checks and copies final
inputs. Supply both success markers and the approved source SHA-256.
