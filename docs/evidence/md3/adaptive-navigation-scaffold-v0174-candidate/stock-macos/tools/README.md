# Surface v0.1.74 macOS stock headless gate

This lane is prepared for the exact 696-file candidate. The source manifest is
7aedeb9161baadca46aa6fdd83848b7e9c5498bccfac0e9a0e21e2782ec3f0bd.
The tree is f7a92f77ecc9b37c91d81ace4877cd8038044d5c.
The original patch changes 13 paths. The exact build and R4 differ in the
cumulative coverage document and adaptive Navigation E2E test only.

The guard requires five suites, 368 ordered Material steps through M369, and
20 IME steps. It keeps the q73 guard and pixel decoders. The adaptive
Navigation decoder reads four real PNGs. It verifies current page and focus,
the 599/600 and 839/840 transitions, backward textarea selection, and the
short RTL rail reveal at 200% text. The recorded row moves by the rail's
100 dp scroll change while the page's scroll position stays at zero. Only
the scoped R4 receipt has a fixed count of 200 checks.

## Offline preflight

Use the reviewed harness digest from the handoff.

```sh
lane=/tmp/surface-md3-adaptive-nav-stock-v0174.j8xout6u
harness=REVIEWED_HARNESS_SHA256
q74=/tmp/surface-md3-adaptive-nav-v0174.b8nBXDdY
candidate="$q74/exact-candidate"
patch="$candidate/adaptive-navigation-v0174.patch"
sh "$lane/run-stock-exact.sh" "$harness" --preflight
```

Preflight replays the original q73 stock verifier and independently decodes all
four actual R4 originals. It treats the q74 canonical build as pending and does
not run a browser.

## Canonical build intake

Wait for both exact build markers. Each marker contains the verified source
tree and a newline. The expected files are native Gallery/DevTools in
`exact-candidate/binaries` with `SHA256SUMS`, and the stock E2E/Jet binaries
plus three four-file Web packs in `exact-candidate/web-canonical`.
The Web path includes `test-binaries.sha256`.

```sh
sh "$lane/run-stock-exact.sh" "$harness" --describe-built "$candidate" "$patch" > "$lane/authority-pins.json"
pin=$(shasum -a 256 "$lane/authority-pins.json" | cut -d' ' -f1)
sh "$lane/run-stock-exact.sh" "$harness" --prepare "$lane/authority-pins.json" "$pin" "$candidate" "$patch"
```

Review the described pins against the exact builder receipts before calling
prepare. Describe and prepare rehash the source and all 16 canonical files.
They check release profiles and artifact bytes. The regular Web Gallery must
match the verified R4 Gallery byte for byte. An unexpected difference fails
before launch.

## Controller-only execution

```sh
sh "$lane/run-stock-exact.sh" "$harness" --run "$lane/authority-pins.json" "$pin"
sh "$lane/run-stock-exact.sh" "$harness" --recheck "$lane/authority-pins.json" "$pin"
```

The browser guard requires private Chrome profiles and the exact headless
flags. It requires the native Gallery's headless arguments and captures
owned-window observations. The compact sentinel denies execution.
The only extra Chrome switch remains the recorded process-local
`--disable-updater-scheduler`.

The genuine run produces `run/verified-adaptive-navigation-stock.json`,
`stock-mac-exact-offline.json`, and `stock-mac-v0174-passed`.
