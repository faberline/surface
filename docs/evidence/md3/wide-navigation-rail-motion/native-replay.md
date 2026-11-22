# Native Wide Navigation Rail motion

All seven native profiles use the exact Surface containment build. The source manifest is `motion-production-containment.sha256`, SHA-256 `e07e72345d9dfd150bea7876f14033e10b42342d2959e347f5d4c7b8469b142c`. The host was macOS 27.2.1 with Apple M1 Max Metal and Rust 1.96.1.

## Replay

Use the restored Surface motion checkpoint and an active macOS desktop with Metal. Cap and the Xcode command-line tools must be available. The separate sizing launcher uses the production Gallery and selects the initial window size through public options. Input uses public DevTools actions and keys. The AppKit helper only reads the matching native window. The two reduced-motion profiles use the documented `SURFACE_REDUCED_MOTION=1` setting.

```sh
shasum -a 256 -c evidence.sha256
bash build-and-run.sh /path/to/restored-surface /tmp/new-native-motion-captures
python3 -B audit-inline-containment.py captures/inline
```

The runner builds a source copy and opens private localhost DevTools sessions. The final command audits the archived frames, whose sample times vary on a new run. Session descriptors and build binaries are outside the packet.

## Results

| Native profile | Content viewport | Intermediate widths recorded |
| --- | --- | --- |
| Light LTR inline | 704 × 512 dp | Open 276 dp. Close 174 and 110 dp. |
| Light LTR standalone modal | 960 × 800 dp | Open 146 and 260 dp. Close 254 and 32 dp. |
| Dark RTL standalone modal | 960 × 800 dp | Open 218 and 348 dp. Close 272 and 16 dp. |
| Dark RTL retained modal | 960 × 800 dp | Open 320 dp. Close 278 dp. |
| Dark RTL 200% standalone modal | 320 × 240 dp | Open 176, 250, 290 and 308 dp. Close 246, 26 and 8 dp. |

The inline width ranged from 96 to 288 dp. The adjacent page reflowed by the same amount and kept its selected Inbox. At 174 dp, the archived closing frame displays all four lines of the wrapped Inbox label. The last ink band ends at y245. The rail background reaches y254 in that 0.5× capture. The final single-glyph ink band has 39 pixels in both the 174 dp closing frame and the 276 dp opening frame. The separate 110 dp frame shows squeezed text near the collapsed endpoint and reaches the bottom of the Gallery viewport.

The wide modals show changing scrim brightness in both directions. The retained background rail remains at 96 dp. AppKit confirmed the modal name, navigation landmark, initial Close focus and current Home destination. The arrow moved focus past disabled Archive. Escape restored the opener immediately. The retiring 320 dp Inbox kept y128 and was noninteractive. Both lines of its 200% label contained ink across 26 rows.

The two reduced-motion profiles showed their endpoints in the first capture after the action. All seven settled 1× PNGs matched byte for byte across the raw protocol, CLI and MCP. Intermediate captures are 0.5×. Timing fields describe client receipt latency. This replay uses keyboard and semantic actions.
