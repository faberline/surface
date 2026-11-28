# Surface v0.1.68 M350 label color diagnosis

The corrected test failed after Tab moved focus to the long right-to-left label's editor. The label was still at its idle color in the screenshot. Its glyph positions were unchanged.

- The fresh idle image is byte-identical to the previously passing idle image. The fresh middle and failure PNGs are also identical.
- The label occupies logical rectangle `[40,728,240,96]` at scale 2. The canvas starts 44 CSS pixels below the window top. The outlined label crop has exactly the same pixels in fresh idle and fresh middle.
- The same 12,125 solid glyph pixels are idle `[202,196,207]` in fresh middle and intermediate `[206,191,243]` in historical middle. All four probes match this pattern. The current editor border is already focused purple `[207,189,254]`.
- Across the previous read-only label, the 2,289 pixels that were idle in historical middle still carry the incoming focus color `[206,191,243]` in fresh middle. This is consistent with the final focus change reaching the renderer at the advanced clock, leaving the new transition at its first pose.

In `compact`, the test waits for the new focus and advances the held clock by 44 ms without waiting for a paint at the held baseline. The neighboring `enter` helper waits two animation frames before it advances the clock. Use the same frame barrier after the final Tab. Preserve the expected RGB, the four probe positions, and the geometry checks. The screenshot evidence identifies a timing difference. A fresh headless replay must confirm that the barrier resolves it.

The [previous passing source and Chrome screenshots](https://github.com/faberline/surface/tree/14b110f1c269c1127389be68b3e9c7cb40e69b5f/docs/evidence/md3/text-field-label-color-v0168-candidate/stock-macos/run/artifacts) provide the historical originals. The six PNGs here are unmodified.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B verify.py
```

The verifier uses Pillow 12.3. It does not launch an application.
