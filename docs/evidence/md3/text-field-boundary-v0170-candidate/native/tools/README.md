# Surface v0.1.70 native text field boundary endpoints

This is a script-only lane. It uses the existing Gallery placeholder
fixture and reuses the v0.1.69 screenshot transports and native process
guard. Exact v0.1.70 source and app pins are supplied by the controller
when they are ready.

| Profile | Actual editor path | Required endpoint paint |
|---|---|---|
| Light LTR, 960 × 900 | Outlined, idle → Tab → focused → Shift+Tab → blur | 1 dp Outline, 3 dp Primary, then original idle pixels |
| Light LTR, 960 × 900 | Filled, same sequence | 1 dp OnSurfaceVariant at the bottom, 3 dp Primary at the bottom, then original idle pixels. Quiet top and sides keep the Filled face color. |
| Dark RTL 200%, 320 × 900 | Long Outlined, same sequence | 1 dp dark Outline and 3 dp dark Primary. The whole straight top stays closed, including the right label area. Blur restores the exact idle PNG. |

The replay enters the same stable field with the fixture's committed
`範例` value. Each capture checks one named editor, unchanged value and
layout, actual focus, and visible committed glyphs. The dark profile
also checks the wrapped label and right-aligned text. Recorded bounds
drive the pixel samples. Native renders at 1×. Solid pixels measure the
1 and 3 dp thickness directly. Exterior and adjacent face pixels guard
against shifted, doubled or thicker paint.

Two complete repeats must agree on all 18 original PNGs and semantic
records. The Page.screenshot API, CLI and MCP must return the same bytes
for each capture. The paired runs check 48 times for owned visible
windows and require all four Gallery processes to exit. The transport
uses the production headless route. The pinned route sets Reduce Motion.
Live motion, disable during motion, physical keyboard hardware and live
assistive speech use separate tests.

## Offline preparation

~~~sh
lane=/tmp/surface-md3-field-boundary-native.705YkFQO
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/offline-controls.py"
~~~

The controls open seven real v0.1.69 settled PNGs under seal
`9b63bc905cf8d3a02a16c97c783c030c505f3c974d1f61e67bcaa0e9ff656f95`.
They validate the 689-file v0.1.70 R1 development source seal
`f0b9bf4f43c9ec853fd2ec327028dabfe8215846ef0beec6222f3ac7fecb684c`.
They also test a synthetic compact focus stripe with four deliberate
pixel defects. That calibration does not count as a v0.1.70 run.

## Exact replay after authorization

Supply the reviewed full source manifest and the SHA-256 of each
manifest. Use the proved native Gallery and DevTools executable pair
with the native manifest. The source and binary files are checked before
and after a replay.

~~~sh
source_args=(--source-root "$candidate_source" --source-manifest "$source_manifest" \
  --source-manifest-sha "$reviewed_source_sha" --binary-manifest "$binary_manifest" \
  --binary-manifest-sha "$reviewed_binary_sha")
"$py" -E -B "$lane/replay-boundaries.py" --verify-inputs-only "${source_args[@]}"
# Start only in the controller-approved macOS headless scope.
cap "$py" -E -B "$lane/replay-boundaries.py" --run "${source_args[@]}" --out "$fresh_tmp_replay"
evidence_sha=$(shasum -a 256 "$fresh_tmp_replay/SHA256SUMS" | cut -d' ' -f1)
"$py" -E -B "$lane/replay-boundaries.py" --verify-stored "$fresh_tmp_replay" \
  --source-manifest-sha "$reviewed_source_sha" --binary-manifest-sha "$reviewed_binary_sha" \
  --evidence-manifest-sha "$evidence_sha"
~~~

The raw receipt is `native-text-field-boundary-report.json`. The
standalone readback opens every sealed original PNG and repeats the
pixel and process receipt checks without launching an app. It needs
no source checkout or binaries.
