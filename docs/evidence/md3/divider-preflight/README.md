# Material Divider headless preflight

The shared builder uses the Material `outline_variant` color and a 1 dp line. Its layouts cover full width, 16 dp leading inset, 16 dp on both sides, and vertical. The leading inset follows right-to-left layout. The canvas keeps the line inside the parent at small widths. Decorative lines stay out of focus and accessibility. Nearby headings identify each section. The earlier semantic separator APIs keep their existing behavior. [Google Divider specs](https://m3.material.io/components/divider/specs) · [Exact design export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/843ffaa9-e73e-45a7-bff7-f198cac0a7c8.json)

| Verification | Result |
| --- | --- |
| Material Runtime | Two tests passed. They cover layout, colors, an undersized parent, pointer input, focus and accessibility. [Test log](component-tests.log) |
| Native headless Mac | Two reruns produced the same two PNGs and reports. Every capture matched through Page.screenshot, CLI and MCP. All 12 visible-window counts were zero. [Wide light](native/run-1/wide-light-ltr.png) · [320 dp dark RTL at 200%](native/run-1/compact-dark-rtl-200.png) · [Report](native/run-1/native-divider-report.json) |
| Headless Chrome WebGPU | Focused M262 and M263 passed 125 checks. Chrome exposed the headings and skipped the decorative lines. The decoded screenshots verified actual color and thickness on both themes. The launcher regained focus. Zero page errors and zero owned Chrome processes remained. [Browser report](browser/browser-material-divider.json) · [Wide light](browser/material-divider-wide-light-ltr.png) · [320 dp dark RTL at 200%](browser/material-divider-compact-dark-rtl-200.png) |

The native and Web checks use the same 560 Gallery inputs. The browser stock test overlay adds one input. The focused runner selects the two new cases. The full stock release gate and additional platform settings remain open.

## Reproduce the source

Reconstruct the [557-file repaired Search source](https://github.com/faberline/surface/tree/ae893d0b6b075c87640d9b8cf1bbf6b3bc717e75/docs/evidence/md3/search-adaptive/v0143-cancel-fixed-source) in `"$stage/source"`. Set `evidence` to the absolute path of this evidence folder. `stage` must be an absolute scratch path. Export a fresh Cargo target under `/tmp`.

```sh
(cd "$evidence" && shasum -a 256 -c SHA256SUMS)
(
  cd "$stage/source"
  git apply --whitespace=error-all "$evidence/source/material-divider.patch"
  shasum -a 256 -c "$evidence/source/source.sha256"
)
```

The 560-file manifest SHA-256 is `0a65f2383281de4c564acd3dad8aff2d3680e5b9a39558a98ffa07ee21f9fb27`. The recorded Google export SHA-256 is `554dcc6159baba5dd39c4a410bde7859c37a4821c6827f34c1c2f9861e2b5309`.

## Replay on macOS

Build the Gallery and CLI from the verified source. The renderer needs a Metal adapter. Use Python 3 for the checks.

```sh
(
  cd "$stage/source"
  CARGO_BUILD_JOBS=2 cap cargo build --locked -p surface-material-gallery \
    --features devtools -p surface-devtools
)
for run in 1 2; do
  python3 -B "$evidence/native-scripts/replay-divider.py" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$stage/native-$run"
done
diff -qr "$stage/native-1" "$stage/native-2"
```

## Replay in headless Chrome

Build the stock Gallery bundle from the same verified source. Make a separate stock test copy and apply the browser test patch. The provided helper derives the focused runner from the stock test source. The file called `source.sha256` in the scratch root should be the 560-file manifest.

```sh
(cd "$stage/source" && CARGO_BUILD_JOBS=2 cap cargo xtask web build material-gallery --release --no-compress)
cp "$evidence/source/source.sha256" "$stage/source.sha256"
cp -Rp "$stage/source" "$stage/web-source"
(
  cd "$stage/web-source"
  git apply --whitespace=error-all "$evidence/source/material-divider-e2e.patch"
  shasum -a 256 -c "$evidence/source/web-source.sha256"
)
cp "$evidence/browser/prepare-web.py" "$stage/"
python3 -B "$stage/prepare-web.py"
(cd "$stage/web-focused" && CARGO_BUILD_JOBS=2 cap cargo build --locked -p surface-e2e --bin surface-e2e)
cap "$CARGO_TARGET_DIR/debug/surface-e2e" --chrome "$chrome" --chrome-arg --headless=new \
  --web-root "$CARGO_TARGET_DIR/web/release" --artifacts "$stage/chrome-run" material
```

Set `chrome` to a Chrome executable. The Web screenshots use device scale 2. The original binary digests are in the native and browser evidence folders.
