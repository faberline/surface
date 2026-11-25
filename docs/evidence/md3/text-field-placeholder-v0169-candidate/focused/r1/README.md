# Surface v0.1.69 scoped headless placeholder replay

The root development source has 688 sealed paths rooted at
`/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY/development-snapshot-r1/source`.
Its tree is `9084d355d3132fd983576ef4f9191a5a36acddb8`.
The source manifest hashes to
`15e771de1fc3e14fddc8810e3d3c6388b823b477b124eb3f6d43171320f224e7`.
The scoped source manifest hashes to
`8201f790c1afb37a4405e777ba87903a67c7b0221dd0d73e8ca593d3bfdb7917`.
The only scoped difference is `e2e/src/material.rs`. It selects the
29 original steps M325–M353. All six earlier test modules through
M350 retain their exact v0.1.68 bytes, including the M347–M350
label color proof.

The lane already contains the private scoped source and its manifests.
Gallery assets come from the controller’s independent exact build.
The source and executable are rechecked before and after the test.

## Offline and Gallery preparation

Verify the harness and source without opening an app.

```sh
lane=/tmp/surface-md3-field-placeholder-focused-v0169.612HMDhp
q=/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/prepare-focused.py" recheck-source
"$py" -E -B "$lane/verify-placeholder.py" --self-test
```

After the controller approves the root build and supplies the four-file
Web manifest SHA, verify and copy those four files.

```sh
"$py" -E -B "$lane/prepare-focused.py" pin-gallery \
  --source-manifest "$q/development-snapshot-r1/source.sha256" \
  --source-sha256 15e771de1fc3e14fddc8810e3d3c6388b823b477b124eb3f6d43171320f224e7 \
  --web-manifest "$q/web-development.sha256" --web-sha256 "$WEB_SHA"
"$py" -E -B "$lane/prepare-focused.py" recheck
```

## Controller build and actual run

The private Rust build uses the local cap and two jobs. Build only
after the controller authorizes it. The guarded Chrome process is
headless with a fresh private profile and the launch-only updater
option. It requires exact v0.1.68 stock to be complete and still
needs the controller to confirm the current GPU handoff. The guard
samples only its owned processes, records windows and cleanup, and
rejects any owned survivor.

```sh
sh "$lane/build-focused.sh" --build-scoped > "$lane/build-focused-r1.log" 2>&1
"$py" -E -B "$lane/run-focused.py" --verify-inputs-only
cap "$py" -E -B "$lane/run-focused.py" --run --out "$lane/development-r1"
"$py" -E -B "$lane/verify-placeholder.py" --run "$lane/development-r1" \
  > "$lane/development-r1-verified.json"
```

The offline verifier compares all 29 ordered ledger entries, actual
Chrome and Apple Metal, exit state, source, binary and Web pins,
and window and process receipts. It decodes the prior 34 label PNGs
and the 58 new paired placeholder PNGs. It independently recomputes
whole-crop negative masks, positive pixel opacity and logical
alignment, the two spring rates, and the continuing velocity.
The typed report distinguishes native browser CDP marked text and
the deliberately synthetic, untrusted empty DOM Begin. Both go
through the real editing bridge. The evidence scope stays one
focused Material journey. Full stock and native IME are separate.

Existing M347–M350 keep the 0 ms mask, the settled exact palette and
marked-text checks. Their 44 ms and preedit neutral-token counts
measure full-strength glyphs. The new paired controls compare all
RGB bytes for hidden phases, including translucent glyph ink.
