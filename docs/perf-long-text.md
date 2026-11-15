# Long, growing CJK text: measurement

This records part 1 of issue #107: how much a frame costs when the log holds
thousands of CJK characters and grows several times per second. It reports
numbers only. It does not pick a fix.

## Run it

```sh
sh scripts/bench-long-text.sh
```

It builds `crates/surface-wgpu/examples/long_text_bench.rs` in release mode and
runs it. It needs a real GPU adapter. With no adapter, or a software adapter,
it prints `did not run` and exits 2. It has no threshold. It is not part of
`scripts/check.sh` or CI.

## Method

- Text: 5,000 Traditional Chinese characters from a fixed pool and a fixed
  seed, with a full-width comma or stop about every 12th character. No file, no
  download. The bundled Noto Sans CJK TC covers all of it (0 missing glyphs).
- Width: 800 px root, 20 px padding, so text wraps at 760 px. Font 16 px, line
  height 24 px. Scale 1.
- Growth: 8 characters per append, 625 appends. At 20 appends per second that is
  31 seconds of simulated time and 160 characters per second. Each append is
  one frame. Frames are not paced, so the numbers are cost per frame, not a
  frame rate.
- Path per frame, as a real app does it: build the `MicroDom`,
  `LayoutEngine::compute` (one engine kept across frames, one `TextSystem`
  shared with the renderer), `DisplayList::compile`, `Renderer::render` into an
  offscreen `Rgba8Unorm` texture, then wait for the GPU.
- Phases are timed separately: build tree, layout (measure), compile display
  list, render CPU return (shaping, glyph prepare, encode, submit), render
  including the GPU wait. Total frame is build + layout + compile + render with
  the GPU wait.
- Scenarios:
  - chat: 20 message nodes of 250 characters. An append grows the last one. A
    full message opens a new one.
  - one node: all text in one node.
  - tall viewport: 800 x 3600, so every node is visible and painted.
  - 600 px window: 800 x 600 with the root clipped, showing the top of the log.
    Compile and the renderer skip nodes outside the clip.
  - growing: empty to 5,000 characters. static: the full 5,000 characters,
    rendered again each frame with no change.
- A discarded warm-up pass runs first, so no scenario pays the one-time font
  and glyph-atlas cold start. One layout call per scenario is still slower than
  the rest because the layout engine is new. That is the 7 to 10 ms layout
  maximum in every static row.
- Columns: median, p95 and max over all 625 frames; median of the first 10% of
  frames (0 to 500 characters); median of the last 10% (4,500 to 5,000
  characters); and the ratio of the last to the first.

## Results

Machine: Apple M1 Max, 10 CPU cores, macOS (Darwin 25.5.0). Adapter:
`Apple M1 Max` (`IntegratedGpu`), backend Metal. This is a real GPU, not a
software adapter. Release build, 3 runs on 2026-11-15.

**The machine was loaded.** Other build sessions ran at the same time (load
average 25 to 34 on 10 cores). CPU phases are steadier than the GPU wait, and
the GPU wait and the maxima are noisy. Treat the spread between runs as the
error bar. A quiet machine will give lower and tighter numbers. Re-run it on a
quiet machine before you decide.

Values are ms. "range" is the lowest and highest of the three run values.

| scenario | layout median (range) | layout last 10% | render CPU median | render incl. GPU wait median | total frame median | total frame p95 | total frame last 10% |
|---|---|---|---|---|---|---|---|
| chat, tall, growing | 0.26 to 0.65 | 0.27 to 0.68 | 1.7 to 3.5 | 3.1 to 8.4 | 3.4 to 9.7 | 6.5 to 26.9 | 4.9 to 10.4 |
| chat, tall, static | 0.012 to 0.016 | 0.013 to 0.016 | 3.3 to 4.1 | 4.9 to 7.0 | 5.0 to 7.2 | 4.7 to 22.9 | 4.8 to 8.2 |
| chat, 600 px, growing | 0.26 to 0.41 | 0.27 to 0.56 | 0.76 to 0.94 | 2.1 to 4.1 | 2.3 to 4.9 | 3.1 to 15.0 | 2.3 to 8.4 |
| chat, 600 px, static | 0.012 to 0.019 | 0.012 to 0.019 | 0.84 to 1.19 | 2.1 to 3.3 | 2.2 to 3.3 | 2.4 to 13.2 | 2.1 to 2.8 |
| one node, tall, growing | 4.0 to 6.1 | 10.5 to 11.3 | 1.7 to 2.2 | 3.0 to 5.6 | 7.1 to 13.5 | 23.6 to 32.4 | 16.1 to 21.0 |
| one node, tall, static | 0.005 to 0.013 | 0.004 to 0.013 | 3.0 to 6.0 | 4.3 to 11.8 | 4.3 to 11.9 | 5.4 to 28.8 | 4.3 to 12.2 |

Growth from an empty log to 5,000 characters (first 10% to last 10%, median of
the three runs):

| scenario | layout | render CPU | total frame |
|---|---|---|---|
| chat, tall, growing | 0.3 to 0.3 ms (flat) | 0.4 to 3.2 ms | 5.4 to 8.9 ms (GPU wait is noisy) |
| one node, tall, growing | 0.5 to 11.1 ms (about 20x) | 0.4 to 3.8 ms | 2.4 to 18.8 ms |

Build tree and compile display list stay under 0.1 ms in every scenario (median
0.01 to 0.03 ms and 0.002 to 0.004 ms). They do not matter here.

The full per-phase tables, with p95, max and the first and last 10%, are the raw
output of each run. One run is in the pull request.

## What the numbers show

- Layout is cached per node. Taffy reruns the measure function only for a node
  whose text or style changed. In the chat shape an append re-measures one
  250-character node: about 0.3 ms at any log length. Unchanged frames cost
  about 0.01 ms.
- In the one-node shape every append re-measures the whole text. Layout grows
  with length: about 0.5 ms for the first 500 characters and about 11 ms at
  5,000 (about 20x).
- Render does not cache. `Renderer::render` builds a new glyphon `Buffer` and
  reshapes every text op on every frame. That cost depends on the total visible
  text, not on what changed. Static frames cost the same as growing frames of
  the same length: about 3 to 4 ms CPU for 5,000 visible characters in the tall
  viewport, about 0.8 ms when 600 px of the log is visible.
- Clipping to the window cut render CPU by about 4x in this test, because ops
  outside the clip are not compiled or shaped. The window here shows the top
  of the log, not the tail. A real chat that shows the tail would cut the same
  way.
- At 20 appends per second there are 50 ms between appends. At 60 Hz the frame
  budget is 16.7 ms. On this loaded machine the chat shape fits in that budget
  at the median in all three runs. Its p95 went over 16.7 ms in one of the three runs. The one-node shape reaches about 16 to 21 ms
  at full length, so it does not fit in the 16.7 ms budget at 5,000 characters.

### What this suggests for the three options in the issue

This is input for the owner's decision. It does not choose.

- **Cache shaped text.** The target is the render phase: 3 to 4 ms of CPU at
  5,000 visible characters, for every frame, changed or not. It would also
  remove the reshaping that layout does for the one-node shape, if the cache
  serves measure as well.
- **Reshape only changed nodes.** Layout already does this for the chat shape,
  so the remaining gain is in render, and in the one-node shape. If an app
  splits its log into message nodes, layout is already cheap.
- **Wait for virtualization** (`docs/migration.md` step 21). A visible window of
  about 600 px cost about a quarter of the tall viewport in render CPU. That is
  the size of the saving for a log that is mostly off screen. It does not help
  when the whole log is visible, and it does not change the one-node layout
  cost unless text is also split.

## Limits

- Real input, scrolling, and the platform window and swap chain are not
  involved. The target is an offscreen texture. A real host adds present time.
- One font size, one font, Traditional Chinese with a little punctuation. No
  Latin runs, no emoji, no bidirectional text, no mixed styles.
- Text is pseudo-random characters, not real prose. Line breaks happen at any
  character because there are no spaces, as for real CJK text.
- The scroll position is fixed. The 600 px window shows the top of the log.
- Frames run back to back and are not paced at 50 ms. A real app leaves idle
  time between frames, so GPU clocks may differ from this run.
- The machine was loaded by other builds during all three runs. The GPU wait
  and the maxima are the least reliable. Other adapters, including Intel and
  discrete GPUs, were not measured.
- The first layout call of each scenario is slower (7 to 10 ms) because the
  engine is new. A real app pays this once.
