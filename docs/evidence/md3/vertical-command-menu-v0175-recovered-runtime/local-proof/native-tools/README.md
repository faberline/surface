# Surface v0.1.75 vertical command menu native replay

This headless macOS replay enters the Gallery through the vertical menu
launcher. It uses Native DevTools keyboard input. It checks the Runtime
semantic names, ordering and ownership of the menu, groups and items.

- Standard at 960 × 1120 uses three named groups with 2 dp gaps and no thumb.
  Keyboard navigation reaches the disabled command. Enter and Space leave it
  open and produce no result. One Enter on the final command shows its result
  and returns focus to the trigger.
- Vibrant at 960 × 1120 uses one continuous group. Tab closes it and moves
  focus to the next page control.
- Vibrant at 320 × 480 uses dark colors, RTL and 200% text. The menu opens
  above the trigger. Its three groups touch. End moves the 4 dp trailing
  thumb and fully reveals the final command inside the scroll viewport.
  Escape returns focus and reopening restores the first command's focus.

Back returns focus to the original launcher in all profiles. The replay
decodes menu, label, icon, focus and scrollbar pixels. It matches the original
PNG bytes through Page.screenshot, the CLI and MCP. Two repetitions use six
fresh Gallery processes. The proof requires eight original PNGs and 46
zero-window observations. It verifies Runtime semantics and offscreen paint.

## Use exact controller pins

The controller supplies the absolute sealed source path, source manifest,
native manifest and both SHA-256 values. The native manifest must list exactly
`surface-material-gallery` and `surface-devtools`. From this directory, run
`shasum -a 256 -c SHA256SUMS`. Check the pins without opening an application:

```sh
python3 -B replay-vertical-menu.py --check-inputs \
    --source "$source" --source-manifest "$source_manifest" --source-sha "$source_sha" \
    --binary-manifest "$binary_manifest" --binary-sha "$binary_sha"
```

The controller can replace `--check-inputs` with `--run --out "$fresh_output"`.
Use a new private directory under `/tmp`. The launcher checks the actual
arguments against the exact headless command. The process guard removes display
environment variables. The replay checks the source and binaries before and
after the run. A failed run keeps the original snapshot and any PNGs inside
its output directory.

The evidence includes `native-vertical-menu-report.json`, a manifest, and an
original copy of each capture for both repetitions:

- `standard-light-grouped.png`
- `vibrant-light-single.png`
- `vibrant-dark-rtl-200-top.png`
- `vibrant-dark-rtl-200-end.png`

Read a completed package without launching an application:

```sh
python3 -B replay-vertical-menu.py --readback "$fresh_output" \
    --source-sha "$source_sha" --binary-sha "$binary_sha" --evidence-sha "$evidence_sha"
```

`verify-offline-controls.py` tests rejection of altered geometry and PNGs. It
uses Pillow for synthetic controls only.
