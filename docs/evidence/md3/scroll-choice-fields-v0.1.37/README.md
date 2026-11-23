# Surface v0.1.37 text-choice Scroll Fields verification

Run from a checkout of `v0.1.37`. The base was
`1256b9b9c3f3154c84a67146a6b5b3766bd6e2d9` (v0.1.36).
Set `evidence_dir` to this folder and `CARGO_TARGET_DIR` to a build
directory in `/tmp`.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-md3-scroll-choices-replay.XXXXXXXX)
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
RUST_TEST_THREADS=1 cap sh scripts/check.sh
```

## Native headless Gallery

Native headless Metal reproduced 6 PNGs in two runs with zero visible windows. Page.screenshot, CLI and MCP returned identical PNGs.

The Gallery checks keyboard adjustment, duplicate labels, error feedback
and unique prefix completion. It checks that reorder and relabel preserve
the accepted choice. The compact window uses dark colors, RTL, and 200%
text. The replay checks pixels, focus, sizing, and each process's visible
window count.

[Native report](native/native-scroll-choice-report.json) ·
[Replay comparison](native/native-repeat.json) ·
[Focused Rust tests](native/material-focused.log) ·
[macOS AccessKit adjustable test](native/macos-accessibility.log)

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
python3 -B "$evidence_dir/native/replay.py" --workspace "$PWD" \
    --source-manifest "$evidence_dir/native/native-source.sha256" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native"
mkdir -p "$scratch_dir/gpu"
SURFACE_GPU_EVIDENCE="$scratch_dir/gpu/gpu-offscreen.png" \
    cap cargo test --locked -p surface-wgpu --test it gpu_offscreen -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
```

## Headless Chrome WebGPU

Chrome/158.0.8127.37 used Apple Metal 3. Five headless cases passed 13,711 checks. Material passed 215 steps and 13,463 checks.

M211–M216 check the real Chrome accessibility tree, prefix and alias
acceptance, catalog updates, held gesture retirement, and the compact editor.
They send synthetic composition events to check Enter and resize while a
composition draft is active.

[Browser report](web/browser-material-scroll-choice-fields.json) ·
[Browser commands](web/replay.md) ·
[Unique prefix](web/material-scroll-choice-fields-prefix.png) ·
[Ambiguous input](web/material-scroll-choice-fields-ambiguous.png) ·
[Held gesture](web/material-scroll-choice-fields-gesture-held.png) ·
[Compact fallback](web/material-scroll-choice-fields-compact-dark-rtl.png) ·
[Preedit after resize](web/material-scroll-choice-fields-resized-ime-preedit.png)

## Design references

The dated Material page export has SHA-256
`3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390`.
The token export has SHA-256
`5afcd56f8cb9fe47d822b4b14426db8b0ed27aa9d70e015d6c9389bf6d6500cd`.
The field exposes a one-based ordinal and its localized text. Looping,
independent units, popup and modal placement, system high contrast,
physical input method composition, and live speech remain open.

[Material 3 specs](https://m3.material.io/components/scroll-fields/specs) ·
[Published page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.42f95ef205743188.json) ·
[WAI spinbutton pattern](https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/) ·
[Surface #210](https://github.com/faberline/surface/issues/210)
