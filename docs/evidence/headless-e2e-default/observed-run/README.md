# Surface default headless E2E observation

This scratch verifier runs the approved new Surface E2E executable against
the sealed Menu v0.1.46 app bundle. The old executable from that bundle
is rejected. All app and runner hashes are checked before and after the run.

The actual E2E invocation supplies only asset, executable, and output paths.
It names no suites and supplies no Chrome options. It removes inherited
Chrome options. The pre-exec Chrome guard requires exactly `--headless=new`
from the invoked program and a private user profile. The native guard requires
the offscreen Gallery command and a private DevTools session. The compact
guard always rejects a launch. The Jet Bridge wrapper only marks its child
environment to identify its separate Chrome process.

The observer identifies the three guarded launches by PID. It also tracks
the owned process group, private Chrome profiles and recorded process
identities. It continues sampling the Bridge browser after Jet detaches.
CoreGraphics contributes only counts and PIDs of visible windows owned by
those processes. There is no desktop capture. Both Chrome processes and
the offscreen Gallery must be observed, and no processes can remain.

The verifier checks exactly these five successful cases in order: smoke,
ime, material, bridge-web and devtools-headless. It independently decodes
the Bridge, native DevTools, Menu and Divider screenshots. It also checks
the IME cancellation reports and native screenshot transport equality.

## Reproduce

Run on macOS, using Python 3.11 or newer and Pillow 12.1 or newer. Use an
approved candidate binary SHA-256 and a fresh output directory below /tmp.
Loopback and process observation need the relevant execution permissions.
Running without suite names is the behavior under test.

```sh
shasum -a 256 -c SHA256SUMS
python3 -B observe-default.py --self-test
python3 -B observe-default.py --out "$new_private_path" --runner-sha256 "$approved_sha"
```

An optional `--runner` or `--stock` selects the input locations. The pin
still has to match. The test reports unchanged raw receipts and PNGs in
`artifacts`, plus `runner.log`, `invocation.json`, `guard-ledger`,
`headless-window-observation.json` and `verified-default.json`.
The self-test runs only argument predicates. It launches no app.
