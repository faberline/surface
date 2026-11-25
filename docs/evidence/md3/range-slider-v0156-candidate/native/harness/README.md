# Native Range slider proof

The runner verifies the two-thumb Range slider in the Surface Material Gallery.
It uses the unchanged transport, layout, and window helpers from the v0.1.55
Slider Forms proof. It requires a separately approved v0.1.56 source manifest
and native binary manifest.

## Planned replay

Each of two independent recurrences starts a light 960×1040 Gallery and a dark
320×880 Gallery with right-to-left layout and 200% text. Both use Reduce
Motion. Each selected image must agree byte for byte across `Page.screenshot`,
the CLI, and MCP. The runner decodes PNG pixels and repeats the checks against
the second set of images. It also compares the typed geometry and interaction
receipts across both recurrences.

- `wide-light-rest.png` covers the four mounted range examples, all named
  native Slider targets, and their track and handle paint.
- `wide-light-discrete-upper-focus.png` shows the signed, discrete upper
  thumb, its value label, ring, endpoints, and track dots.
- `wide-light-equal-lower-focus.png` shows a collapsed active band and one
  visible handle where the two stable targets meet.
- `compact-dark-rtl-200-continuous-lower-focus.png` shows the mirrored
  continuous pair and legible enlarged focused label.
- `compact-dark-rtl-200-discrete-upper-focus.png` shows the signed upper
  label, ring, and shared discrete rail at the compact width.

The semantic replay separately checks native `Action.key`, `Action.setValue`
and `Action.activate`. Tab reaches lower then upper in both layouts. Home and
End respect the current peer value. One thumb can be adjusted without changing
the other thumb's value or focus. An absolute input at a discrete midpoint
tests the full signed lattice. Equality keeps both controls focusable.
Both disabled thumbs reject absolute changes and stay out of the Tab order.
Back restores the launcher focus.

Geometry checks use exact stable IDs, the measured 48 dp hit rectangles, and
the physical rail's 24 dp end insets. Pixel checks sample the handles, rail
colors, a gap, endpoint dots, selected label ink, and complete focus ring.
Visible label text matches the slider's localized accessible value, and
the label clears the ring by 4 dp.

The SDK snapshot exposes the authored full numeric domain and named Slider
semantics. Peer-limited numeric behavior is verified by actual native SDK
actions. Actual derived accessibility ranges, trusted Web pointer capture,
and live assistive speech require separate evidence.

## Run

Use Python 3.12 or later and the approved native candidate pins. `out` must
name a fresh directory under /tmp. The headless invocation samples only the
owned Gallery process with CoreGraphics. It requires zero visible windows and
reaps that process before continuing.

```sh
shasum -a 256 -c harness.sha256
python3 -E -B replay-range-slider.py --self-test

python3 -E -B replay-range-slider.py \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" \
  --check-only

python3 -E -B replay-range-slider.py \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" \
  --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

The native run checks both manifests before and after. It writes the original
PNGs in `replay-1` and `replay-2`, the typed
`native-range-slider-report.json`, and `SHA256SUMS`.

The offline self-test passed three positive and four negative geometry cases.
It did not execute a Gallery, CLI, or browser process.
