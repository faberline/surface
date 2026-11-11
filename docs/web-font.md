# Browser CJK font

## Choice

The browser (wasm32) build bundles a subset of Noto Sans CJK TC. Native
builds keep the full font. The subset is made at build time by
`scripts/subset-font.py` and committed as
`crates/surface-wgpu/assets/NotoSansCJKtc-Subset.otf`.

`TextSystem::default()` picks the face with a `cfg` on the target:

- `wasm32`: `NotoSansCJKtc-Subset.otf`
- other targets: `NotoSansCJKtc-Regular.otf`

The subset face is loaded without a copy (`TextSystem::with_cjk_font`), so
the font is no longer copied to the heap twice.

Lazy loading and Unicode-range splitting are not used. The CJK font is about
88% of the WASM binary and is already compressed CFF data, so brotli and gzip
save little on it. Only a smaller font changes the download much.

## Sizes

| File | Bytes |
| --- | ---: |
| `NotoSansCJKtc-Regular.otf` (full) | 16,435,884 |
| `NotoSansCJKtc-Subset.otf` (browser) | 3,615,900 |

Measured with `cargo build --release --target wasm32-unknown-unknown -p surface-showcase --lib`
(raw `.wasm`, no `wasm-opt`, same toolchain for both builds):

| Build | Raw `.wasm` bytes |
| --- | ---: |
| before (HEAD, full font copied) | 21,405,505 |
| after (subset, no copy) | 8,586,142 |

The `.wasm` shrinks by 12,819,363 bytes. The xtask web build (`cargo xtask
web build`) uses `wasm-opt -Oz` and reports different absolute sizes, so these
numbers are not comparable with it.

## Character set

The subset keeps:

- printable ASCII (`U+0020` to `U+007E`);
- the Big5 characters with lead bytes `0xA1` to `0xC6` (the common-use
  Traditional Chinese set, plus the Big5 symbol block);
- the characters in `crates/surface-wgpu/assets/subset-extra.txt`
  (`简体汉语✓−×`, used by the tests and the browser IME check).

Bopomofo `ㄅ` to `ㄩ` is in the set. The subset has 6,000 characters.

## Regenerate

```sh
python3 -m pip install fonttools==4.68.0
python3 scripts/subset-font.py
python3 -I scripts/check-provenance.py
```

The script must print `6000 characters, 3615900 bytes`. When the output
changes, update the `sha256` and `bytes` of `NotoSansCJKtc-Subset.otf` in
`docs/provenance.json` in the same change.

## Limit

Characters outside the set draw as missing glyphs, but only in the browser
build. For example `龘` (U+9F98) is in the full font and not in the subset.
Native builds still draw it. Add a character to `subset-extra.txt` and
regenerate when a browser page needs it.

## Checks

- `subset_font_keeps_the_chinese_text_the_tests_use` and
  `subset_font_is_a_real_subset` in `crates/surface-wgpu/tests/it.rs`.
- `chinese_font_and_shaped_editing_geometry_have_real_glyphs` still passes.
- `cargo check --target wasm32-unknown-unknown` for the showcase and the
  material gallery passes.

The browser IME test (`scripts/browser-ime.mjs`) was not run. It needs Chrome
and a built package, so a person or browser CI must run it. A successful wasm
check is not browser proof.
