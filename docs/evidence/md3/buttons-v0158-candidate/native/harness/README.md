# Common Buttons v0.1.58 native calibration

The harness uses the unchanged native transport and window helpers from the exact v0.1.57 replay. It opens the Common Buttons route with semantic Tab and Enter at each fixed headless viewport. Tab reaches the same late section of the home route in the prior native replay. A browser opening the route by pointer needs the taller home viewport.

## Scenes

| Original PNG | What the replay checks |
| --- | --- |
| `wide-light-rest.png` | Five painted heights, all five command appearances, toggle rests, disabled fill, localized labels, leading and trailing artwork |
| `wide-light-selected-focus.png` | Space selects the round-authored filled toggle and paints its square face and focus outline |
| `wide-light-square-toggle.png` | Enter selects the square-authored tonal toggle and paints its round face and focus outline |
| `compact-dark-rtl-200-sizes.png` | The first compact row reverses while 200% text and the fixed-size glyphs paint clearly |
| `compact-dark-rtl-200-wrapped-focus.png` | Tab reveals the complete wrapped label, right-side leading glyph, and visible focus outline at 320 dp |

The wide viewport is 960 × 1160. The compact viewport is 320 × 880, dark and right-to-left. The harness checks hit and face bounds separately and requires the minimum hit extent of 48 dp on both axes. The focus outline extends 5 dp beyond the face. The labels and glyphs belong to the same named action. The replay records the original status and checked values after each keyboard action. Forward and reverse Tab skip both disabled actions. A direct semantic activation attempt on each disabled action must return `disabled` and leave the frame unchanged. Back restores the original opener.

The harness requests Reduce Motion. Each original capture must have identical PNG bytes and metadata from `Page.screenshot`, the CLI and MCP. The five PNGs and typed results must also match across independent repeats. The harness records four zero-window observations per owned child, across four children. Native observations describe programmatic actions, authored accessibility state and offscreen pixels. Physical input, real assistive speech and live motion have separate verification lanes.

## Check the staged inputs

Use Python 3.12. Set the source root and both manifest paths and digests from the exact controller-approved build. Choose a new output directory under `/tmp`.

```sh
lane=/tmp/surface-md3-common-buttons-native.juKPEyE6
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c harness.sha256)
"$py" -E -B "$lane/replay-common-buttons.py" --self-test
"$py" -E -B "$lane/replay-common-buttons.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" --check-source-only
"$py" -E -B "$lane/replay-common-buttons.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" --check-only
```

To execute the reviewed headless replay, use the same pinned inputs.

```sh
cap "$py" -E -B "$lane/replay-common-buttons.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

The output has both sets of original PNGs in `replay-1` and `replay-2`, and the typed report in `native-common-buttons-report.json`. `owned_process_observations` records the four captured process IDs, exit codes and zero-window arrays outside the identical replay data. Each screenshot records its exact decoded color samples, text ink measures and focus ring measurements.
