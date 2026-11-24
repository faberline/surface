# Dialog release candidate and the clipped Date Picker launcher

This is the original macOS stock headless regression from the rejected source fingerprint `be98d80e2cc14f3ded2abc8758e4d7b82c7e0dce32359859183bcc65a39a224e`. Stock Material finished M111 and timed out during Date Picker setup before M112. The other four stock suites passed.

The Date Picker button was wholly below the 1100 × 880 dp Gallery Canvas. It starts at y924 dp with height 48 dp. The pointer helper accepts an offscreen layout rectangle and computes its uncut center. It dispatched at (550, 948) in the Canvas, or (550, 992) after the 44 px host header. The Canvas ended at page y924. The Gallery stayed in section 2, the launcher had no projected button, and the browser reported no page errors.

The source added two 64 dp Gallery card rows ahead of the existing routes. Move the new buttons together into one wrapping row after the existing routes. This preserves legacy launcher placement. The stock Dialog setup first uses a 1800 dp Canvas. It opens the fixture before changing to smaller dialog sizes.

- [Exact original failure snapshot](material-date-picker-failure.json)
- [Exact original failure screenshot](material-date-picker-failure.png)
- [Extracted clipping measurements](clipping.json)
- [Stock log](runner.log) with private workspace path replaced
- [Rejected source fingerprints](rejected-source.sha256), [stock artifact fingerprints](rejected-binaries.sha256) and [original receipt fingerprints](original-evidence-manifest-lines.txt)

Run `shasum -a 256 -c SHA256SUMS` from this directory.
