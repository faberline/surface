# Native Slider refinements replay

This draft checks two Surface Material Gallery routes. It uses the four
unchanged transport, layout, process, and window helpers from the v0.1.56
Range replay. The final runner requires an approved v0.1.57 source seal and
the approved native Gallery and DevTools executables.

The Vertical Centered selectors come from the controller's prepared
`vertical_centered_slider_demo.rs`. The Inset selectors come from the
Gallery preflight `slider_inset_demo.rs`. These selectors and the final
source manifest must match before execution.

## Matrix

Each recurrence uses one headless process per route and profile. The
wide scene is light LTR at 960 × 1040. The compact scene is dark RTL at
320 × 880 and 200% text. Each headless process uses Reduce Motion.

| Original PNG | Required evidence |
| --- | --- |
| `centered-wide-light-rest.png` | Negative, zero, and positive spans, a signed discrete lattice, and a disabled specimen |
| `centered-wide-light-signed-focus.png` | Native keyboard Home/End and directional keys, exact −37.25 SetValue, visible signed text and ring |
| `centered-compact-dark-rtl-200.png` | Signed discrete tie snaps to existing zero, +34 snaps to +25, real focus reveal and large clear label |
| `inset-wide-light-rest.png` | 24/24/32 dp artwork on both axes, both track colors, hidden short-rail icon, disabled ink |
| `inset-wide-light-relocated.png` | One normal speaker glyph in the inactive track at 5% with token padding from the preserved stop |
| `inset-wide-light-minimum.png` | Home selects 0%. Only the glyph changes in the same icon box, with a different decoded pixel mask |
| `inset-compact-dark-rtl-200-horizontal.png` | Mirrored active art, original hit geometry and full large focused label |
| `inset-compact-dark-rtl-200-vertical.png` | Tab reveals the upright icon and vertical target. Large label sits on the RTL side with 4 dp of clearance |

Native `Action.key`, `Action.setValue`, and `Action.activate` drive both
routes. The record includes Tab and Shift+Tab order, disabled value
rejection, Back focus restoration, stable rail geometry while a value
changes, and the localized value text. The test also checks that the
vertical inset glyph relocates before it switches at the minimum. Artwork
must avoid stops, ticks, handles, rings, and value badges.

Each of the eight screenshots must agree byte for byte across
`Page.screenshot`, CLI, and MCP. The PNGs and typed observations must
also agree across two independent recurrences. The runner checks the
exact source and executable bytes before and after. It requires 32
CoreGraphics samples with zero visible owned windows, and it waits
for all eight owned processes to exit.

The SDK snapshot describes the authored native role, orientation,
range and value text. This headless draft does not exercise physical
input or assistive speech.

## Run after exact pins arrive

Set `source_root`, `source_manifest`, `source_sha`, `binary_manifest`, and
`binary_sha` from the final approved build. Use a fresh private `out` under
/tmp. The real command needs local access to loopback, the headless Metal
renderer, and CoreGraphics.

```sh
shasum -a 256 -c harness.sha256
python3 -E -B replay-slider-refinements.py --self-test

python3 -E -B replay-slider-refinements.py \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" \
  --check-only

cap python3 -E -B replay-slider-refinements.py \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" \
  --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

The result contains both sets of original PNGs in `replay-1` and `replay-2`,
`native-slider-refinements-report.json`, and `SHA256SUMS`.

Draft verification ran five positive and four negative synthetic controls.
It also rejected the previously sealed v0.1.56 inputs. The draft has no
actual Gallery, CLI, browser, or GPU replay yet.
