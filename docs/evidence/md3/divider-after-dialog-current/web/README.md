# Divider headless WebGPU verification

The focused runner runs the two stock steps M264 and M265. It uses a private selector. Every Divider assertion comes from the sealed stock source. Chrome 158.0.8127.37 ran on Apple Metal 3. Both steps passed in 2.0 seconds with 125 checks. The run left no page errors or Chrome processes.

The wide light profile uses a 960 × 640 dp Canvas. The compact profile uses a 320 × 640 dp Canvas in dark colors, right-to-left layout and 200% text. Each of the four line layouts paints 1 dp thick. The browser uses 2× device scale. The independent check decodes both PNGs and finds exactly two solid pixels across each line. Neighboring pixels have no divider ink. The verifier checked 10,880 solid wide pixels and 3,200 solid compact pixels. Both PNG files match the earlier pre-retarget run byte for byte. The single leading inset mirrors in the compact view.

Chrome exposes the four section headings. It omits the decorative line nodes from the accessibility tree and Tab order. Back restores focus to the Divider launcher. The stock runner and the independent receipt provide these results.

- [Raw browser receipt](run/browser-material-divider.json) and [independent image checks](run/verified-divider.json).
- [Wide screenshot](run/material-divider-wide-light-ltr.png) and [compact screenshot](run/material-divider-compact-dark-rtl-200.png).

## Exact inputs

The Gallery Wasm was built from the original 583-file retarget manifest SHA-256 `38a589e74410c9ec4bec4d518f447058da99f1d5f797ca24b8aacd94932682bc`. The corrected stock test manifest SHA-256 is `73486892d094b921162adbbd69ecfe67ed982c35549be68775f86f1927635f5c`. The only changed input is the Divider test's adapter reporting assertion. This cannot change the Gallery build. The stock test now accepts the fallback flag as a Boolean. This proof uses the actual Apple Metal adapter, and the Linux stock run is a separate gate.

- [Gallery source manifest](source.sha256), [stock test source manifest](runner-stock.sha256) and [single E2E difference](confirmed-e2e-only-gallery-reuse.diff).
- [Focused source manifest](focused-source.sha256), [focused selector patch](focused-selector.patch) and [runtime artifact manifest](runtime.sha256).

## Reproduce

Use a fresh temporary directory. Set `gallery_source` to the original source export and `stock_source` to the corrected source export. The commands verify both. Use a Mac with Command Line Tools, `cap`, Rust, a populated offline Cargo cache, `wasm-bindgen`, `wasm-opt`, Chrome and Pillow 12. The check compares captured images with the earlier Divider run. Set `prior_run` to that archive's browser folder.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
evidence=$(pwd -P)
batch=$(mktemp -d /tmp/surface-divider-focused.XXXXXXXX)
batch=$(cd "$batch" && pwd -P)
for file in build-gallery.sh prepare-focused.py build-focused.sh run-focused.sh verify-divider.py; do
  cp "$evidence/$file" "$batch/"
done
bash "$batch/build-gallery.sh" "$gallery_source" "$evidence/source.sha256" \
  38a589e74410c9ec4bec4d518f447058da99f1d5f797ca24b8aacd94932682bc
python3 -B "$batch/prepare-focused.py" "$stock_source" "$evidence/runner-stock.sha256" \
  73486892d094b921162adbbd69ecfe67ed982c35549be68775f86f1927635f5c
bash "$batch/build-focused.sh"
bash "$batch/run-focused.sh"
python3 -B "$batch/verify-divider.py" "$batch"/run.* "$prior_run"
```

The build uses a private Cargo target. The runner opens Chrome with `--headless=new`. The raw JSON and PNGs in this archive are byte-identical to the verified run. The logs replace private work paths.
