# v0.1.71 Gallery fixed-palette native preview

Harness R3 checks captions with an antialias coverage model. The real
R2 Outlined capture contained 176 GrayText caption pixels that fit
the ink-to-backdrop blend, including 47 at 85% coverage or higher.
The caption had zero solid pixels. The value had 152 solid GrayText
pixels and keeps the exact check. The unmodified frame is stored in
`r2-wide-light-outlined-idle.png`. The original R2 run and helper seal
stay at `/tmp/surface-md3-field-disabled-contrast-native-r2.tNGcz5yJ`.

This replay measures disabled and read-only Filled and Outlined fields.
It starts the production Gallery in headless mode. The Gallery receives a
fixed emulation palette from [W3C CSS Color Adjustment Level 1 §5.2](https://www.w3.org/TR/css-color-adjust-1/#forced-color-palettes).
Light uses white Canvas, black CanvasText, and GrayText `#600000`.
Dark uses black Canvas, white CanvasText, and GrayText `#3ff23f`.

| Profile | Viewport | Scope |
|---|---:|---|
| Wide light | 960 × 900 | LTR, 100% text |
| Wide dark | 960 × 900 | LTR, 100% text |
| Compact dark | 320 × 900 | RTL, 200% text |

Each form captures the same locked fields before and after a real
Runtime Tab reaches the read-only field. The replay verifies both names
and values. It verifies that attempted fills fail without any change
of focus or value, that Tab skips the disabled field, and Back returns
focus to the route launcher. It checks the actual clipped layout before
each shot. The checker uses those bounds to find GrayText in the
disabled label and value, and CanvasText in the read-only label and
value. Each value requires solid foreground pixels. A caption pixel
must follow the expected backdrop-to-ink blend across all RGB channels
with a rounding margin of one byte and enough strong pixels. Filled
disabled captions use the measured neutral face backdrop. The checker
also verifies the read-only bottom stroke for
the 1 dp and 3 dp focus states. It records exact color pixel counts,
extents, stroke samples and geometry. Compact body glyphs must be at
the right logical edge.

Two replays should agree on all 24 PNGs. In each capture the direct
Page.screenshot API, CLI and MCP must return the same bytes and metadata.
The v0.1.70 helpers are reused at seal
`34291d67bd5e47502553bcbf0171b424b4bee2f9be0fa7e5f2873b7c235d7575`.
A private argv guard requires the exact headless request before
executing the pinned Gallery binary with the specified palette.
The observer takes 78 samples from the six owned Gallery processes
and requires zero visible windows and all six processes reaped.
The launcher and image transports use only the pinned native binary
pair. The native helper sets Reduce Motion.

This is fixed-palette rendering and Runtime semantics. macOS system
forced colors, screen-reader speech, and browser CSS resolution have
separate evidence scopes. The corrected caption check already reads
the original partial R2 frame. A full v0.1.71 run still follows.

## R1 development inputs and authorized launch

The following uses the R1 source and binary receipts. Pass equivalent
reviewed pins to replay a later exact release snapshot. Start the
`--run` command only after the controller hands over the headless
runtime slot. `--verify-inputs-only` can run beforehand.

~~~sh
lane=/tmp/surface-md3-field-disabled-contrast-native-r3.SXfqVJZd
stage=/tmp/surface-md3-field-contrast-v0171.xE0ruhQh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=888bf446dba55df082f0f8c8c9181e5a76fbad76c63e3cc002c9f3d211d07ed8
binary_sha=2932f41e240c5d8eb0898b1d40b2c41d6f1f278c301d80186df33c04ffc9be56
args=(--source-root "$stage/development-snapshot-r1/source" \
  --source-manifest "$stage/development-snapshot-r1/source.sha256" \
  --source-manifest-sha "$source_sha" \
  --binary-manifest "$stage/native-development/SHA256SUMS" \
  --binary-manifest-sha "$binary_sha")
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/replay-disabled-contrast.py" --verify-inputs-only "${args[@]}"
cap "$py" -E -B "$lane/replay-disabled-contrast.py" --run "${args[@]}" --out "$lane/development-r1-harness-r3"
~~~

The output path must be fresh. If the run stops, keep its original
captures and log. A completed output contains the original PNGs,
`native-text-field-disabled-contrast-report.json`, and a file seal
`SHA256SUMS`. Record the result printed by `--run`.

## Offline readback

Review the replay manifest digest before passing it to the verifier.
Use the same source and binary digests as the run. The checker
decodes and rechecks all original PNGs and the transport, focus and
window receipts. It needs no app or source checkout.

~~~sh
"$py" -E -B "$lane/replay-disabled-contrast.py" \
  --verify-stored "$lane/development-r1-harness-r3" \
  --source-manifest-sha "$source_sha" --binary-manifest-sha "$binary_sha" \
  --evidence-manifest-sha "$reviewed_replay_sha"
~~~

`offline-controls.py` compares the verified v0.1.70 predecessor to its
stored receipt, checks the exact 690-path R1 input seal, checks guard
arguments with a shell-only fixture, and probes the pixel checks with
synthetic valid and damaged data. The partial R2 frame is tested with
both the original solid-only predicate and the new caption predicate.
The checks also reject a nearby wrong red and a grayscale fallback
with the same original glyph coverage. These controls are setup
evidence. The R2 image remains a partial run.
