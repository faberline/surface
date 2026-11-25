# Surface v0.1.74 native adaptive navigation replay

This harness revision keeps the original R2 product and executables.
The first compact replay stopped before taking a PNG. Its artwork check
looked for a contour on the outer icon group. The vertical bar and
collapsed rail use a passive 24 dp clip with an unnamed contour child.
The medium bar puts its contour on the icon group itself. The revised
check follows those shapes and still verifies the real contour bounds,
ink and rendered pixels. Compact paint remains unverified until replay.

When a scene fails after Gallery starts, the runner saves the actual
`DOM.snapshot` with computed bounds, clips and focus in
`repeat-N/<profile>-failure.json`. It records the failing stage and
previous window samples. The original exception still fails the run.
The app launch guard then reaps the owned process. The diagnostic does
not store the connection session.

The runner uses three fixed viewport sizes. Each starts in a fresh
headless Gallery process. It repeats the three profiles independently.

| Profile | Native viewport | Checks |
| --- | --- | --- |
| Compact light | 320 × 620 | Bottom bar, 56 × 32 dp vertical pill, and keyboard Tab |
| Medium light | 600 × 620 | Bottom bar, 112 dp hit target, and icon at the start of a horizontal pill |
| Short dark RTL, 200% | 840 × 300 | 96 dp rail at the right edge, enlarged labels and focus reveal |

Each profile checks one named Runtime navigation landmark, one
current page and a disabled destination. Tab or Down moves focus
without selecting. Enter or Space then selects the saved page.
The next move puts focus on another destination while Saved remains
current. Direct activation of the disabled destination must fail.
The short rail starts with Settings outside the rail clip. Down
reveals the entire target without moving the page layout or editor.
Back restores the launcher focus.

The replay opens the demo by stable ID. The section's launcher can
be clipped in the 300 dp home view. The keyboard test starts after
opening the demo. Native tests use separate fixed-width app instances.
They do not test live resizing, Web accessibility, or VoiceOver.

The reporter checks six original PNGs. Each must match in
Page.screenshot, the CLI and MCP. The verifier decodes the background,
pill and corner colors. It also checks selected icon and label ink
and a visible focus stroke on the separate focused destination.
A completed run must report 30 CoreGraphics observations, no visible
windows and six reaped children.

## Reviewed inputs

- Surface source tree `98b75a23d173b72567fb60d925d2c301e1a12ea5`.
- 696-source-file manifest SHA-256
  `8e3accebefb67a07449011ff637f76e8a717f629bd9e595e526cadac3833e160`.
- Two-executable manifest SHA-256
  `bf2732532f164a4b8a050d48da85e9f7291a303e30a4eeacf7eed0b94b7efa57`.
- The four unmodified helper files came from the horizontal bar
  replay. Its harness manifest SHA-256 is
  `4fc8a3572605cb7dbc38e50c7361cefb4ed2e5cc3648d090c482cdd9f0638bea`.

## Offline preparation

From this folder, use Python 3.11 or newer. Pillow is needed only for
the synthetic controls.

```sh
q=/tmp/surface-md3-adaptive-nav-v0174.b8nBXDdY
source_sha=8e3accebefb67a07449011ff637f76e8a717f629bd9e595e526cadac3833e160
binary_sha=bf2732532f164a4b8a050d48da85e9f7291a303e30a4eeacf7eed0b94b7efa57
shasum -a 256 -c SHA256SUMS
python3 -E -B verify-offline-controls.py
python3 -E -B replay-adaptive-nav.py --check-inputs \
  --source "$q/development-r2/source" --source-manifest "$q/development-r2/source.sha256" \
  --source-sha "$source_sha" --binary-manifest "$q/native-development/SHA256SUMS" \
  --binary-sha "$binary_sha"
```

These checks do not run the Gallery or capture a desktop. The two
executables and mounted source must also match their approved
manifests after the actual replay.

## Controller-only replay and readback

After dispatch, use `--run --out "$output"` in place of
`--check-inputs`. Choose a path that does not exist under a fresh
directory in `/tmp`. Run under `cap` with headless Metal and
CoreGraphics access.

The runner emits `native-adaptive-nav-report.json` and
`SHA256SUMS` next to `repeat-1` and `repeat-2`. Each repeat has
the following exact PNG names.

- `compact-320-light-vertical-bar.png`
- `medium-light-horizontal-bar.png`
- `wide-dark-rtl-200-short-rail.png`

Read the sealed output offline.

```sh
evidence_sha=$(shasum -a 256 "$output/SHA256SUMS" | cut -d' ' -f1)
python3 -E -B replay-adaptive-nav.py --readback "$output" \
  --source-sha "$source_sha" --binary-sha "$binary_sha" --evidence-sha "$evidence_sha"
```
