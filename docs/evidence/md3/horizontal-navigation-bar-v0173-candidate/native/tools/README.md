# Surface v0.1.73 horizontal Flexible Navigation Bar native replay

This replay opens the stock Material Gallery in macOS headless mode. It uses the
medium-width launcher at 839 × 900 in light mode and 720 × 900 in dark mode with
RTL layout and 200% text. Both profiles test three, four and five destinations.

Each profile checks the bottom anchor, separate scrolling region, fixed 112 dp
cells, intrinsic pill size, wrapped label, and actual passive contour icon.
Native DevTools keyboard events move focus without changing the page until Enter.
They skip the disabled destination and return focus to the launcher after Back.
The replay checks the Runtime navigation landmark, name, disabled state, and one
current page. This is Runtime and native renderer evidence.

Each profile runs twice in fresh Gallery processes. Each selected screenshot must
match byte for byte in Page.screenshot, the CLI, and MCP. The verifier decodes
the originals and checks the bar color, selected fill and rounded corner,
painted icon and label ink, and the adjacent cell's 3 dp keyboard focus stroke.
The last Tab preserves selection and lets the selected pill display its base
color. The replay requires 20 CoreGraphics observations with zero
visible windows and four reaped Gallery children.

## Reviewed R2 inputs

- Source tree `6f2b29fb151e0544d2d5eeb06ed4e3df07a5cd85`
- SHA-256 of the 692-file source manifest
  `13716392d80ba252314d2be056d41b7c4812e85347684cca5ee671acd6cf836c`
- SHA-256 of the two-file native executable manifest
  `be1868febce8bfcfb611e632122476949b14d8d1470e4a83e2454557ec05cc98`

Python 3.11 or newer runs the scripts. From this directory, verify the harness
with `shasum -a 256 -c SHA256SUMS`. To verify the reviewed inputs without
launching an application, run:

```sh
python3 -B replay-horizontal-nav.py --check-inputs \
  --source /tmp/surface-md3-horizontal-nav-v0173.OEjA5TvP/development-r2/source \
  --source-manifest /tmp/surface-md3-horizontal-nav-v0173.OEjA5TvP/development-r2/source.sha256 \
  --source-sha 13716392d80ba252314d2be056d41b7c4812e85347684cca5ee671acd6cf836c \
  --binary-manifest /tmp/surface-md3-horizontal-nav-v0173.OEjA5TvP/native-development/SHA256SUMS \
  --binary-sha be1868febce8bfcfb611e632122476949b14d8d1470e4a83e2454557ec05cc98
```

The controller can replace `--check-inputs` with `--run --out <fresh-directory-under-/tmp>`
to collect evidence. The launch wrapper requires the exact headless arguments.
The runner checks the source and executable hashes again after all four runs.
The output directory contains `native-horizontal-nav-report.json`, the evidence
`SHA256SUMS`, and each of these original captures under `repeat-1` and `repeat-2`:

- `wide-light-saved.png`
- `medium-dark-rtl-200-updates.png`

Review the evidence independently with:

```sh
python3 -B replay-horizontal-nav.py --readback <evidence-directory> \
  --source-sha <reviewed-source-manifest-sha256> \
  --binary-sha <reviewed-executable-manifest-sha256> \
  --evidence-sha <reviewed-evidence-manifest-sha256>
```

Run `python3 -B verify-offline-controls.py` for synthetic checks of the verifier.
Their output provides verifier calibration only. Native screenshots and process
observations come from the guarded replay.
