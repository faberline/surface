# Surface v0.1.60 Segmented Button native replay

This is the v0.1.59 replay with four unchanged headless launch,
screenshot and window helpers. It repeats the desktop keyboard, roles,
selection, disabled controls, focus shape and compact keyboard reveal
checks. The new paint check reads the List segment's measured
`segment-list-view-icon` and `segment-list-view-label` descendants.

| Original PNG | Viewport | Scope |
| --- | --- | --- |
| `wide-light-rest.png` | 960 × 880 | Desktop roles, original 40 dp faces, leading 18 dp icon and 8 dp label gap, visible icon and label ink |
| `wide-light-selected-keyboard.png` | 960 × 880 | Tab entry at first enabled item, Arrow focus without activation, Space and Enter selection, disabled skip, focus ink on four edges |
| `compact-dark-rtl-200.png` | 320 × 880 | Dark RTL at 200%. Icon above the full two-line List label, centered with an 8 dp gap. Both show visible ink. Hit targets and the exterior focus outline fit. |
| `scrolled-dark-rtl-200.png` | 240 × 880 | Keyboard movement to the opposite edge of the horizontal scroll port, 48 dp minimum hit target, visible focus outline and explicit selection |

The same PNG bytes and metadata must come from Page.screenshot, the CLI
and MCP. The four original PNGs and typed checks must match across two
independent replays. The harness requests Reduce Motion and samples the
six owned Gallery process IDs. It requires 24 zero-window observations
and reaps each owned process.

This harness uses developer-tool key commands. The Chrome CDP journey
checks actual touch panning and axis lock separately. This native replay
does not measure live assistive speech.

## Check inputs

Use the controller's final 662-file source manifest and SHA-256.
The binary manifest must bind exactly `surface-material-gallery` and
`surface-devtools`, both relative to its own directory. Use a fresh
output path under `/tmp`.

```sh
lane=/tmp/surface-md3-segmented-touch-native.26fxPGhh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c harness.sha256)
"$py" -E -B "$lane/replay-segmented.py" --self-test
"$py" -E -B "$lane/replay-segmented.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" --check-only
```

`observed-working-source.sha256` is the source-only check from
preparation. Use the final controller source seal for native execution.

## Run both native headless replays

```sh
cap "$py" -E -B "$lane/replay-segmented.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

Read `native-segmented-report.json` for the measured compact icon and
label boxes and both ink samples under `replays[].compact.captures[]`.
The images for each replay are in `replay-1` and `replay-2`.
