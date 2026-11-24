# Surface Search preflight

These records use source base `ab8615c8495d99a652f4fac38d4db37f08c82c7f`. The expanded view has Contained and Divided styles. It docks where the anchor and viewport fit, and uses the full safe area in a compact window. [Google Search specification](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/22081e54-b56a-47b2-b137-9ba82953f05e.json)

| Stage | What ran | Evidence |
| --- | --- | --- |
| Browser WebGPU before the caret gap | Chrome 158 headless used Apple Metal 3. Seven Search steps and 904 checks passed. They covered both layouts, RTL and 200% text, keyboard and projected accessibility, a real 599 to 600 dp resize with active composition, and normal and forced colors. | [Browser report](web/suite.json) · [Run log](web/suite.log) · [Source manifest](source/rtl-forced.sha256) |
| Single-line composition | The existing Showcase editor gave identical traces on Surface v0.1.41 and the v0.1.42 candidate. During a selected preedit, Chrome requested a line break and sent no composition end. The guard canceled that line break. Search and Showcase passed confirmation and cancellation. Showcase passed I1 through I20. | [v0.1.41 trace](ime/v041-stock.json) · [v0.1.42 trace](ime/v042-candidate-stock.json) · [Guard trace](ime/guard-search.json) · [Showcase run](ime/guard-showcase-stock.log) |
| Native right alignment | The caret-clearance source painted the Chinese hint and query on the right in RTL. Clear restored the exact empty frame. In LTR the hint moved 4 pixels. The editor kept its 48 dp hit area and the focus outline stayed inside the pill. | [Native alignment](native-alignment/native-search-alignment-report.json) · [Source manifest](source/caret-spacing.sha256) |
| Native caret visibility | The first aligned capture placed the end caret over the focus stroke. The 4 dp inset on both sides gives the RTL end caret and LTR start caret their own column of ink. Both replays found zero visible windows in 16 samples. Page.screenshot, CLI and MCP agreed on bytes and metadata. | [Before](native-before-caret-gap/caret-edge-report.json) · [After](native-caret/native-search-caret-visibility-report.json) |
| Native Search motion | On the separate motion source, two fixed-clock replays had eight matching Metal frames and identical reports. The settled frames matched the stock headless build. Opening kept the editor mounted. Closing gave focus back and kept an inert paint shell. | [Motion report](native-motion/native-search-motion-report.json) · [Motion source](source/motion.sha256) |
| Integrated stock build | The combined source produced ten byte-identical alignment and caret PNGs with Reduce Motion. All three screenshot transports agreed. All 16 window samples were zero. | [Integrated alignment](native-integrated/native-search-alignment-report.json) · [Integrated caret](native-integrated/native-search-caret-visibility-report.json) · [Combined source](source/assembled.sha256) |

Chrome DevTools Protocol simulates composition here. The fixed-clock native overlay applies only to the diagnostic copy. Real system input methods and spoken screen-reader output need separate checks.

## Rebuild the tested source

Keep the evidence folder outside the source checkout. Start from a fresh copy of the base commit for each set of patches. Use `git apply --check` before each `git apply`. Verify the indicated manifest with `shasum -a 256 -c` from the source root.

- For the native caret proof, apply `01-gallery`, `02-nav`, `03-placeholder`, `04-alignment`, and `05-caret-spacing`. Check `source/caret-spacing.sha256`.
- For the browser app behind the 904 checks, apply `01-gallery`, `02-nav`, `03-placeholder`, `04-alignment`, `ime-guard-clean`, and `search-e2e`. Check `source/rtl-forced.sha256`. Apply `search-rtl-forced-e2e` to register the two new steps in the normal Material suite. The recorded run used a focused runner with those seven steps.
- For fixed-clock motion, apply `01-gallery`, `02-nav`, `03-placeholder`, and `motion-corrected`. Check `source/motion.sha256`. Apply `motion-test-clock` only in a separate copy with its own Cargo target directory.
- The [assembled patch](source/assembled.patch) includes the motion, caret gap, IME guard and first five Search browser cases. Its 555-file manifest matches a clean reconstruction. A release still requires the full integration gates.

## Replay

Set `proof` to this evidence folder. Use fresh output directories in `/tmp` and a fresh Cargo target for each source. These commands use headless Chrome or the headless Gallery.

```sh
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-search-proof
cap cargo xtask web e2e --chrome-arg --headless=new --artifacts /tmp/surface-search-web-proof material ime

cap cargo build --locked -p surface-material-gallery -p surface-devtools --features surface-material-gallery/devtools
bins="$CARGO_TARGET_DIR/debug"
(cd "$bins" && shasum -a 256 surface-material-gallery surface-devtools > native-binaries.sha256)
python3 "$proof/scripts/replay-alignment.py" --gallery "$bins/surface-material-gallery" --cli "$bins/surface-devtools" --manifest "$bins/native-binaries.sha256" --baseline "$proof/native-baseline" --horizontal-content-inset 4 --out /tmp/surface-search-alignment-replay
python3 "$proof/scripts/replay-caret-visibility.py" --gallery "$bins/surface-material-gallery" --cli "$bins/surface-devtools" --manifest "$bins/native-binaries.sha256" --out /tmp/surface-search-caret-replay
```

For the clock replay, build the diagnostic native binaries in their own target. Run the script below with those binaries and the stock reference frames.

```sh
python3 "$proof/scripts/replay-search-motion.py" --gallery "$diagnostic/surface-material-gallery" --cli "$diagnostic/surface-devtools" --stock "$proof/native-motion-stock" --out /tmp/surface-search-motion-replay
```

From this evidence folder, run `shasum -a 256 -c SHA256SUMS` to verify the published files.
