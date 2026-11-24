# Verify the Surface v0.1.46 Menu browser evidence

`verify_menu_report.py` reads the actual M267–M269 Chrome report and runner log. It verifies each step appeared once in order and the Material case passed. The `stock` argument checks the 268-step Material case within the stock run. Use the existing stock gate for the other suites, build/source seals, and exit status.

The focused result uses the 590-file v0.1.46 source manifest `92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5`. The `material_command_menu.rs` test has SHA-256 `5e463f0a2ead57f1981df0f3a94c12c662a71f6d632f5d6c74352befed2478a5`.

Use Python 3 with Pillow. Pass a directory containing the Menu report and three PNG captures. The runner log and macOS process observation can either be in that directory or be passed by path.

```sh
python3 -B verify_menu_report.py "$focused_artifacts" --platform macos > verified-menu.json
python3 -B verify_menu_report.py "$stock_artifacts" --platform macos --scope stock > verified-menu.json
python3 -B verify_menu_report.py "$hosted_artifacts" --platform linux --scope stock \
    --log "$raw_material_runner_log" > verified-menu.json
```

On macOS, the default observation filenames are `headless-observation.json` for the focused run and `headless-window-observation.json` for stock. Both must record Chrome process samples, zero visible windows, and no remaining observed processes. The stock receipt also includes offscreen native Gallery samples. Use `--window-receipt` for an observation stored elsewhere. With a Linux run, macOS window sampling is not required and the result leaves `window_observation` null when no receipt is supplied.

Use `--expected-metadata` to supply matching observations from the same run when available. Its JSON has a `browser` string and an `adapter` object with `isFallbackAdapter`, `vendor`, and `architecture`. The underlying Menu report must contain actual adapter metadata. A reported fallback adapter such as SwiftShader is valid. The validation result records the observed adapter without assigning a device class from the host OS.

The verifier fully decodes all three PNGs and checks the wide panel's interior and corner samples against the raw receipt. It checks light wide placement, compact 320 × 480 placement, the disclosed disabled focus and dismissal outcomes, and retained action state. For the footer it checks the 8 dp clearance, real bounds movement, full status containment, and light status ink within the reported final status rectangle. The source test also checks that the launcher keeps the same bounds after the wheel. The verifier does not treat PNG color values from one GPU as requirements for another GPU.

`--pre-footer` is only for the sealed earlier macOS focused result. It decodes the two held interim screenshots and checks those 117 earlier checks with their observed windows. It cannot validate a final footer or prove an adapter for the interim report. Final mode rejects that interim result.

The exact focused macOS run passed the strict verifier. The [receipt](exact-focused-check.json) records its three PNG hashes, measured Apple `metal-3` adapter, and 275 process-window observations. A prior real Linux SwiftShader record verifies the adapter field parsing only. The [rejection checks](controls-check.json) also show that the verifier rejects a clipped reported status, a repeated open-menu PNG, and a truncated footer PNG.
