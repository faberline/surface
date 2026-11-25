# Text field label motion browser proof

This private E2E patch adds M347–M350 on the v0.1.66 source seal
`d4b0c5b7a1ae5daf1f7a9084e765b30b24f8cae6ecbda881efb9468daf730783`.
The two changed paths are the Material registry and one new test file.
The runner uses the exact v0.1.67 development Gallery Web pack, sealed by
`5ee0d1175d5c590d39b1c4d4438e241457e391a4abc7711fcf9b73334802ca43`.

The scoped runner also executes M325–M346 from the original tests.
The previous field scenes already set CSS Reduce Motion for their
static endpoint checks. Their inputs, geometry, colors, and checks
remain unchanged. The new tests turn that preference off when measuring
motion, then turn it on again to measure the snap.

## Observation plan

| Step | Chrome flow and Canvas evidence |
| --- | --- |
| M347 | At 960 dp light LTR, start from neutral Outlined. Trust Tab into the real textarea. Compare real label pixels at rest, 0, 14, 44, and 800 ms. At 14 ms sample both the open upper and retained lower portions of the top stroke from ink-free notch padding. At 44 ms measure a wider gap. Check the endpoint gap, untouched side and bottom stroke, placeholder paint, stable editor geometry, and a single named AX textbox with support text. |
| M348 | At the same wide scene, trust Shift+Tab out and hold the clock at 844 ms. Trust Tab back and compare the same painted label position. Switch CSS Reduce Motion at that time and compare the actual endpoint's label and notch without advancing the clock. |
| M349 | At 960 dp light LTR, separately observe the Filled label from the start through its intermediate and settled positions. During the overlap, use CDP preedit to verify marked glyphs and no placeholder. Cancel, then verify the settled placeholder, untouched model and one real editing host. |
| M350 | At 320 dp dark RTL with 200% text, measure the short label's right anchor and upward movement. Tab through the read-only field to see the complete long label above its editor. Check actual label ink and a closed outline with no phantom notch. |

The four cases write `material-text-field-label-motion.json` and 15
original PNGs. Wide images use a 960 × 820 dp Canvas. Compact images use
a 320 × 960 dp Canvas. Both use DPR 2 and the 44 dp page header. The
capture code verifies paint in real WebGPU pixels. Runtime exports
provide the measured editor, label endpoint, and helper rectangles.
The legacy endpoints were inspected offline in existing .65 images
to place the glyph and stroke samples. The new intermediate proof
requires the real run.

## Run after the prior stock result

The controller owns the v0.1.66 stock run. Wait for the controller's
confirmation before starting Chrome. The wrapper also requires the
successful v0.1.66 stock marker and a fresh output directory.

```sh
lane=/tmp/surface-md3-label-motion-e2e.PywRddEC
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
cap "$py" -E -B "$lane/run-focused.py" \
  --runner "$lane/pins/surface-e2e" \
  --runner-sha256 1db87fe350d4051190d1212d75a41eebe98098181def36ab24308502352ced3b \
  --web-root "$lane/web" --out "$lane/development-r1"
```

The guard checks Chrome's `--headless=new` and private profile. It samples
onscreen window owners and records cleanup. The first evidence is a
scoped run. A full v0.1.67 stock run is separate.
