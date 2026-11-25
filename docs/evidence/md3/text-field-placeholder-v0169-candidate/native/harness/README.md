# Native Material placeholder endpoint replay

This helper uses the existing v0.1.69 Gallery fixture. It runs with a
fresh headless Gallery for each profile. Reduce Motion is on.

- At 960 × 900 in light mode, wide Outlined and Filled fields start with
  an empty regular label. Tab focuses the editor. Shift+Tab leaves it.
  The field is compared with a separate frame with the hint removed.
- At 320 × 900 in dark RTL mode and 200 percent text, both appearances
  show the fully wrapped label above the face. The unfocused empty hint
  is visible. Filling the field replaces the hint with the value. Clear
  restores the exact idle hint. The fixture's hidden-hint setting gives
  separate empty and populated pixel controls.
- Every screenshot is checked for byte parity through Page.screenshot,
  the CLI and MCP. The comparison measures changed pixels in the actual
  editor line and rejects changes elsewhere in the face. It checks the
  resolved palette, measured text scale, and logical text anchor.
- The signed Runtime snapshot supplies the one named TextInput, its
  model and display value, UTF-8 selection, and restored focus. The
  authored hint stays out of these semantic values. This is a native
  Runtime semantic check. It does not test VoiceOver or an OS IME.
- The fixture has no disabled toggle. The dedicated Rust disabled
  endpoint and accessibility tests cover that path. Intermediate
  animation requires the separate fixed-clock browser evidence.

The two profiles run twice. The report contains 40 original PNGs, four
reaped Gallery processes, and 92 CoreGraphics window observations on
the four live process IDs. It requires zero onscreen windows. Screenshots
across each repetition must match, including their measured face and
text geometry. Session files are private, and authentication uses
local loopback. The browser and system settings are untouched.

## Input check and controller launch

Use an exact source tree and the two executable pins from the same
build. The verify-only form reads the pins and starts nothing.

~~~sh
lane=/tmp/surface-md3-placeholder-native.oHJCRmeE
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
q=/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY
source_dir="$q/development-snapshot-r1"
binary_manifest="$q/native-development-r1/binaries/SHA256SUMS"
source_sha=15e771de1fc3e14fddc8810e3d3c6388b823b477b124eb3f6d43171320f224e7
binary_sha="REPLACE_WITH_REVIEWED_BINARY_MANIFEST_DIGEST"
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/replay-placeholder.py" --verify-inputs-only \
  --source-root "$source_dir/source" --source-manifest "$source_dir/source.sha256" \
  --source-manifest-sha "$source_sha" --binary-manifest "$binary_manifest" \
  --binary-manifest-sha "$binary_sha"
~~~

The controller authorizes the actual process and graphics access. Use
a fresh evidence folder below /tmp. For a later exact release build,
replace both paths and reviewed digests.

~~~sh
out="$lane/development-r1"
"$HOME/.local/bin/cap" "$py" -E -B "$lane/replay-placeholder.py" \
  --source-root "$source_dir/source" --source-manifest "$source_dir/source.sha256" \
  --source-manifest-sha "$source_sha" --binary-manifest "$binary_manifest" \
  --binary-manifest-sha "$binary_sha" --out "$out"
manifest_sha=$(shasum -a 256 "$out/SHA256SUMS" | awk '{print $1}')
"$py" -E -B "$lane/verify-stored-placeholder.py" "$out" \
  --manifest-sha "$manifest_sha" --source-manifest-sha "$source_sha" \
  --binary-manifest-sha "$binary_sha" > "$lane/development-r1-verified.json"
~~~

Keep the report, all sealed originals, both manifest pins and the
verifier output. The launcher checks source and binary bytes again
after the second run. A failed run keeps its original partial files.
The stored verifier reads fixed originals and starts nothing.

## Preparation checks

No application or browser ran during preparation. The synthetic
controls use inert stand-ins to test input pins and simulated screenshot
records to test the readback rules. They are distinct from native
evidence. Only those controls need Pillow. The replay and stored verifier
reuse the existing pure Python PNG decoder and native transport helpers.
