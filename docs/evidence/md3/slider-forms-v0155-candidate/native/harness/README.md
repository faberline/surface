# Native centered and vertical Slider proof

This harness uses four unchanged helpers from the Standard Slider native proof. It verifies exact Surface v0.1.55 source and the approved Gallery and CLI executables. The headless runner accepts a fresh private directory under /tmp and starts the Gallery with Reduce Motion. It observes only the owned process via CoreGraphics. It reaps each launched Gallery before the next profile.

## Plan and invocation

Use Python 3.12+. Set these shell variables from the final controller seal before using check-only or the runner.

- `source_root` is the final candidate's source directory.
- `source_manifest` and `source_sha` identify its exact SHA-256 inventory.
- `binary_manifest` and `binary_sha` identify the checked pair of native executables.
- `out` names a new private directory with an existing parent under /tmp.

```sh
shasum -a 256 -c harness.sha256
python3 -E -B replay-slider-forms.py --self-test
python3 -E -B replay-slider-forms.py \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" \
  --check-only
python3 -E -B replay-slider-forms.py \
  --source-root "$source_root" --source-manifest "$source_manifest" \
  --source-manifest-sha256 "$source_sha" \
  --binary-manifest "$binary_manifest" --binary-manifest-sha256 "$binary_sha" \
  --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

Use the last command only with the approved final pins on a Mac with the offscreen renderer available. The two fresh recurrences each run wide light 960×1040 and compact dark RTL 320×880 with 200% text. Each recurrence records five PNGs. The wide profile records all seven base specimens and the focused forms. The compact profile records both focused forms. The API, CLI and MCP must agree on the complete original PNG bytes and metadata. Each corresponding image must also match in the second recurrence. Input hashes are checked before and after. The replay writes `native-slider-forms-report.json` and a file manifest.

The screenshot checks decode authored and actual palette fills, active and inactive spans, empty neutral separation, stops, ticks, the muted disabled appearance, and the actual ring and upright label ink. The label clears the ring by 4 dp. The geometry checks cover both centered signs, neutral, endpoints, both RTL rules, fixed hit target and input rail, and the compact scroll that reveals the vertical bar. Native semantic `Action.key` and `Action.setValue` go through separate checked sequences. Trusted physical mouse and touch and real assistive speech require their own evidence.

## Exact v0.1.55 receipt

The first actual run reached the wide screenshot and confirmed the three transports. The verifier then expected an extra Centered lower endpoint tick. The sealed component draws that endpoint through its stop indicator and omits the extra tick node. This revision checks the stop and the absence of a duplicate tick. The earlier log and screenshot remain in the parent directory.

The two full recurrences passed on the sealed 640-file source with tree `18bf4098eefd03ff562e058ceb5d7f99a34447a2`. The source digest is `93fbdb8c80cfaf7698ae702a8bbc155b15411f2fe427048e1040100a672fc2d4`. The binary manifest digest is `f5bed61a33c963a6131ddf3b42094cad3d5dbb6c2b0bed302fd27c210ed928e1`. The five pairs of original PNGs and both typed recurrences match. The four Gallery children exited. All 20 window observations found zero owned visible windows.

The compact vertical example revealed the focused rail after 660 dp of scrolling. The upright label wrapped on two visible lines at 200% and cleared the complete focus ring by 4 dp. The verifier separately confirmed semantic keys, independent absolute value changes, discrete rounding, disabled rejection and focus restoration.

Read `../exact-v0155-r2/native-slider-forms-report.json` and verify the original artifacts using `../exact-v0155-r2/SHA256SUMS`. The offline geometry controls remain four positive and three negative cases. The earlier Standard Slider image control is scoped to the prior release.
