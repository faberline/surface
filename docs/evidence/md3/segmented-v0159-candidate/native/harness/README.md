# Baseline Segmented Button v0.1.59 native replay

The harness uses unchanged transport, headless launch, and CoreGraphics
helpers from the Common Buttons v0.1.58 replay. Only the release controller
runs this harness with the reviewed native executables.

| Original PNG | Viewport | Observations |
| --- | --- | --- |
| `wide-light-rest.png` | 960 × 880 | Two named radio groups, one named independent checkbox group, selected second and last radios, disabled choices, icon-only name and artwork, equal 40 dp faces |
| `wide-light-selected-keyboard.png` | 960 × 880 | Tab enters at first enabled choices, horizontal arrows skip disabled choices and move focus, Space and Enter select explicitly, two independent checked filters, four painted focus edges |
| `compact-dark-rtl-200.png` | 320 × 880 | Dark, 200% label text, mirrored row, first logical choice on the right, full 48 dp targets and exterior focus |
| `scrolled-dark-rtl-200.png` | 240 × 880 | Horizontal overflow, Left skips disabled choices and reveals the opposite edge, full 48 dp target and exterior focus, Space selects the revealed checkbox |

The harness launches separate headless viewports. Its keyboard calls use
the Gallery's native developer-tool interface. They test Runtime input,
authored names and roles, and painted focus. Physical keyboard, native
assistive speech and callback counts are separate checks. The harness
requests Reduce Motion. It records 24 zero-window samples over six
Gallery children, and every owned child must exit. For each capture,
the API, CLI and MCP must return identical original PNG bytes and
metadata. All four PNGs and typed results must match across two replays.

## Check the exact inputs

Use the final source seal and the native executable seal from the same
build. The native binary manifest must contain exactly
`surface-material-gallery` and `surface-devtools`. Both are relative to
the manifest directory. Use a fresh output directory under `/tmp`.

```sh
lane=/tmp/surface-md3-segmented-native.O3aJrrno
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c harness.sha256)
"$py" -E -B "$lane/replay-segmented.py" --self-test
"$py" -E -B "$lane/replay-segmented.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" --check-only
```

## Run the headless replay

```sh
cap "$py" -E -B "$lane/replay-segmented.py" \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

The result includes eight original PNGs and `native-segmented-report.json`.
The report includes separate process observations plus the typed
replays. Each saved focus proof records the authored 3 dp width, the
entire visible outline and independently measured ink on each edge.
