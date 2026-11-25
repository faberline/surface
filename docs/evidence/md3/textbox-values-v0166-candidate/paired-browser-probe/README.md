# Inactive multiline textbox values in Chrome

The paired replay uses the Surface Material Gallery and reads
`Accessibility.getFullAXTree`. It compares the pinned pre-fix v0.1.65
development pack with the corrected v0.1.66 R2 development pack. Both
runs use Chrome 158.0.8127.37 with Apple Metal 3. The guard recorded nine
window samples per run, zero visible windows and zero survivors.

The same journey reads six inactive values across four named fields.
It focuses the Grow field through the Web accessibility proxy, replaces
the value in the actual textarea, and leaves the editor with a trusted
Tab event. It checks that the restored proxy reads the exact committed
value, including two leading spaces, two spaces before a line break,
a combining accent and a terminal newline. It reveals the locked fields
with a trusted wheel event and checks their role and state. The read-only
field transfers to the real textarea and Tab skips the disabled field.

The original pack exposes no inactive value for any of the six samples.
The corrected pack exposes all six values exactly. Both preserve the
names, help and multiline states. Every sampled state has one named
textbox per field. The corrected proxy has no additional readable
static-text descendant.

The proof pins the HTML, JS, Wasm and release manifest before and after
the run. It records the hashes of the HTML, JS and Wasm served to Chrome.
The evidence covers the Chromium accessibility projection, keyboard
handoff and committed editor value. macOS VoiceOver speech and the full
stock suite are separate.

## Read the result again without a browser

```sh
lane=/tmp/surface-md3-textbox-values-proof.n472jbhl
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/compare.py"
```

## Replay with the same private headless guard

Use a new output directory inside the lane for each replay.

```sh
cap "$py" -E -B "$lane/run-proof.py" \
  --web /tmp/surface-md3-multiline-e2e.imcVWpWb/web-integrated/material-gallery \
  --manifest "$lane/v0165-web.sha256" \
  --manifest-sha256 befc055db590dc0781e17c0af2687189e469aadfcd3bd4b7ea853baf8bff0490 \
  --expect missing --out "$lane/replay-v0165"

cap "$py" -E -B "$lane/run-proof.py" \
  --web /tmp/surface-md3-textbox-values-v0166.l4950l9e/web-development-r2 \
  --manifest "$lane/v0166-r2-web.sha256" \
  --manifest-sha256 d37470c46d9b81fed27eab062b137633039bcbbf0712afee3d7047dd9b86a797 \
  --expect present --out "$lane/replay-v0166"
```

The first negative calibration is preserved at `v0165-negative`. It
timed out before commit because the first driver used a bare command
chord. Both accepted runs use the explicit macOS select-all command
already used by the stock Surface test runner.
