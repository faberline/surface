# Typography roles and font assets

Surface's 30 base and emphasized types match the pinned AndroidX size, line height, spacing and weight numbers. FontTools independently read the embedded Noto Regular and Bold faces. Both full fonts map 44,810 code points. Both browser subsets map the same 5,997 code points.

[Google Material 3 typography](https://m3.material.io/styles/typography/applying-type) · [Pinned AndroidX type scale](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/TypeScaleTokens.kt) · [Pinned Noto Bold font](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/OTF/TraditionalChinese/NotoSansCJKtc-Bold.otf)

Run these checks from `spec/` after reconstructing the source bundle. Set `sources` to the reconstructed source path.

```sh
python3 -B compare.py "$sources"
python3 -B font-inventory.py "$sources"
```

Use FontTools 4.66.1 for `font-inventory.py` and to regenerate the Bold subset. The comparison measures token values. The runtime evidence separately measures the visible Regular and Bold specimens.
