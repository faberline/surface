# Native Standard Slider Gallery proof

The replay uses the production Surface Gallery and DevTools binaries from the integrated v0.1.54 build. It verifies both SHA-256 hashes before and after the run. The Gallery starts with the headless flag and Reduce Motion. The observer queries on-screen windows for each owned Gallery PID and checks that all launched children exit.

From this directory, verify the package and its current binary inputs.

```sh
python=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
manifest=/tmp/surface-md3-sliders-v0154.FBTvDz1t/native-v0154-binaries/SHA256SUMS
seal=129c825ee96410ec665cf2cb5a70196f827e0df737f8d7ffb4fce91b976996c5
shasum -a 256 -c SHA256SUMS
"$python" -E -B replay-standard-sliders.py --binary-manifest "$manifest" \
  --binary-manifest-sha256 "$seal" --check-only
```

Run the replay with an available native Metal adapter. Set `out` to a fresh private path with an existing parent under `/tmp`.

```sh
"$python" -E -B replay-standard-sliders.py --binary-manifest "$manifest" \
  --binary-manifest-sha256 "$seal" --out "$out"
(cd "$out" && shasum -a 256 -c SHA256SUMS)
```

The two recurrences each render wide light at 960×1040 and compact dark RTL with 200% text at 320×880. The four original PNGs in each recurrence must match the production Page.screenshot, CLI and MCP bytes. Every corresponding image must also match across recurrences.

The typed report records all five size presets, the disabled specimen, rail insets, handle dimensions, mirrored endpoints, nonoverlapping discrete dots, decoded paint colors, local value label and focus-ring ink. It checks exact Home and End values, mirrored arrows, an unsnapped continuous 37.25, a discrete value tie, focus skipping the disabled specimen, and return to the opener. The compact trace records the scroll that reveals the XL specimen. The label clears the outline by 4 dp.

This proof uses the Gallery's semantic DevTools input and an actual offscreen renderer. Browser input and live assistive speech need separate tests.
