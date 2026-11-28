# Surface v0.1.76 native choice-menu replay

This replay uses the Gallery's windowless renderer and Native DevTools keyboard input. It reads the menu, named groups, checked values and focus from the Runtime DOM. It distinguishes radio items from independent check items by the authored menu-radio marker.

- Standard light at 960 × 1120 checks both radio groups. Each accepted choice closes the menu and restores the opener. The other group keeps its value. Enter and Space on a checked single choice or a disabled choice have no effect. Space and Enter change independent checks while the menu stays open. Tab moves to the next page control. Escape returns to the opener.
- Vibrant light at 960 × 1120 checks the same named groups, a checked multiple choice, its blank unchecked neighbor and the two changed values after reopening.
- Vibrant dark at 320 × 480 uses RTL and 200% text. It checks the leading mark on the right, wrapped text, the clipped viewport and the moving thumb. End reveals the last disabled row. The long enabled choice remains usable after the reveal.

The tool reads actual selected and unchecked face pixels, the check mark, the blank mark slot, the label, focus stroke, and the thumb if present. It records separate contrast ratios for the two face colors and for the check mark against its face. It requires the latter to be at least 3:1. Back restores the original launcher.

The two repetitions use six fresh Gallery processes and eight original PNG files. Each PNG has matching bytes and metadata across Page.screenshot, the DevTools CLI and MCP. The process proof requires 46 zero-window samples and all six processes reaped.

This replay measures Runtime DOM, keyboard behavior and offscreen pixels. Platform accessibility projection, spoken output, physical pointer input and callback counts belong to other tests.

## Check the prepared tool

Run the first two commands offline. None opens an app. Pillow is only needed for the synthetic control.

```sh
shasum -a 256 -c SHA256SUMS
python3 -B verify-offline-controls.py
```

Use the authorized build's exact source and executable manifests. The tool checks every source hash and both executable hashes.

```sh
python3 -B replay-vertical-choice-menu.py --check-inputs \
  --source "$source" --source-manifest "$source_manifest" --source-sha "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-sha "$binary_sha"
```

The controller's development producer passed this offline preflight. Source seal is `4fc216a07b9a54fd2bfa0b6ee2a81fb6dbdd46c3968da602afb9dfa9dd0f1970` across 701 paths. Binary manifest seal is `215c453415adec3bc843a13f5ce8f78cf074385c4c391ad7e3aa0e42e44f76c4`. The actual app replay is pending.

## Authorized replay

On macOS, replace `--check-inputs` with `--run --out "$fresh_output"` only when the controller authorizes the headless run. Supply a new private directory under `/tmp`. The launcher requires the exact `--headless --width W --height H` command. The process helper clears display environment variables. Checks repeat after the run. A failure preserves any original captures and an actual DOM snapshot.

The output has `native-vertical-choice-menu-report.json`, a byte manifest, and two directories with the following four original captures:

- `standard-light-initial.png`
- `vibrant-light-checks.png`
- `vibrant-dark-rtl-200-initial.png`
- `vibrant-dark-rtl-200-checked.png`

Read the completed package offline:

```sh
python3 -B replay-vertical-choice-menu.py --readback "$fresh_output" \
  --source-sha "$source_sha" --binary-sha "$binary_sha" --evidence-sha "$evidence_sha"
```

The source version defaults to `0.1.76`. Supply `--source-version 0.1.75` only when checking a reviewed precursor with that exact version. Use the same version in readback.
