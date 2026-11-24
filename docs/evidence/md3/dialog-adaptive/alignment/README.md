# Dialog alignment on every visible line

The incoming repaired Dialog source has 563 files and manifest SHA-256 `616038a54fd44090e671899ae9d1457c96a12601cac15a91d96df016045c14d5`. The [alignment patch](dialog-text-alignment.patch) SHA-256 is `4c7bd1241624d30eecfbbd37352d7dbaf65862a794e26ce5829ca0068ece98fc`. The [565-file result manifest](source.sha256) SHA-256 is `d038f7b00b6aab24933bd3d047b536d4d707e54c3b8285a628ed037c20fb0229`. The incoming source still matches its seal.

The optional `Node::text_alignment(TextAlignment::{Left, Center, Right})` selects a physical position inside each existing text box. WGPU gives the original paragraph to Glyphon and the pinned cosmic-text shaper. The shaper aligns each resulting line within the original wrap width. Text and editable controls retain their existing style, selection, and IME paths. No new layout or platform state is needed.

`ExtendedPaintOp::AlignedText` keeps the underlying read-only run and its alignment together. Draw order and owner attribution stay intact when the list moves through `into_ops()`. The public `TextStyle` and legacy `PaintOp` shapes stay compatible. The legacy `Paint.snapshot` and legacy display list show the original text box. Use extended paint or screenshots to see the new line positions. Readers of older extended lists remain compatible with frames that do not opt in.

Basic uses logical start for headings without an icon and for its supporting text. The heading centers when the caller supplies an icon. Full-screen titles use logical start inside the measured header slot. The caller controls alignment of its own body views. The compact discard example still scrolls its supporting text.

## Proof

- 32 focused Rust tests passed, including Node serde, extended attribution and clip, forced selection paint, Runtime editing, paint export, Basic and Full-screen Dialog. Format, strict Clippy, provenance, native build and the Gallery WASM check passed.
- The 10-test headless GPU gate ran on Apple M1 Max Metal. The alignment test checked Left, Center and Right for three unequal visual CJK lines from a soft wrap and a hard newline at 1× and 2×. [Renderer sample](gpu/gpu-text-alignment.png).
- Two stock Gallery replays yielded eight byte-identical PNGs and identical reports, with matching Page.screenshot, CLI and MCP. All 24 window checks returned zero. The compact RTL 200% Full-screen title has two unequal lines whose rightmost painted pixel is x249 for both. Basic and the three visible Alert supporting lines align at logical start, with extra space for full-width punctuation. The old and new editor rectangles and both complete editor pixel crops match. The Gallery replay also verifies Alert acknowledgement, Continue, Save, Discard, and restored focus. [Compact edit](exact-native-checked/run-1/compact-dark-rtl-200-edit.png), [compact Alert](exact-native-checked/run-1/compact-dark-rtl-200-alert.png), [native report](exact-native-checked/run-1/native-dialog-report.json), [editor parity](editor-pixel-parity.json).

Browser rendering, live IME hardware and VoiceOver remain separate checks. The scoped tests use synthetic Runtime composition.

## Reproduce the native replay

Start from the [reconstructed Search after cancellation repair](https://github.com/faberline/surface/tree/ae893d0b6b075c87640d9b8cf1bbf6b3bc717e75/docs/evidence/md3/search-adaptive/v0143-cancel-fixed-source). Apply the [Dialog component patch](../preflight/source/basic-full-ax.patch) and [Dialog Gallery patch](../preflight/source/dialog-gallery.patch). Verify the [incoming manifest](incoming-source.sha256), then apply the alignment patch and check the [result manifest](source.sha256) with `shasum -a 256 -c`. The complete sequence independently reproduced all 565 inputs. Follow the local Rust build settings and build the Gallery and CLI with `cap cargo build --locked -p surface-material-gallery -p surface-devtools --features surface-material-gallery/devtools`. Use a fresh target, copy the two executables to `pins`, and record them in `pins/native-binaries.sha256` from that directory.

Run `PYTHON_BIN=python3 sh replay-alignment.sh "$stage" "$stage/replay"` from this evidence directory. Use an absolute scratch directory with the verified checkout in `source` and the rebuilt binaries in `pins`. The output directory must be new. The runner verifies source and binary seals before and after testing. Use `python3 -B native-scripts/compare-editor-pixels.py BEFORE AFTER` with the two replay result directories to reproduce the editor parity check.

Verify the archive with `shasum -a 256 -c SHA256SUMS`.
