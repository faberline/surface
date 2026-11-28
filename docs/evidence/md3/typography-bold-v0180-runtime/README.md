# Material 3 Typography Bold

This verified Surface candidate adds native and WebGPU rendering of the Noto Sans CJK TC Bold 700 face. The Gallery compares English and Traditional Chinese at two weights. It keeps both specimens visible in 960 dp light and 320 dp dark layouts.

The exact source passed 1,628 Rust tests, three WebAssembly checks, 18 actual Metal tests, and 383 headless Material browser steps with 31,830 checks. The native API, CLI and MCP returned identical PNGs for each of the four independent captures.

[Commands, limits and source pins](portable/README.md) · [Native light PNG](portable/native/originals/repeat-1/wide-light.png) · [Native dark PNG](portable/native/originals/repeat-1/compact-dark.png) · [WebGPU light PNG](portable/browser/artifacts/material-typography-font-weight-wide-light.png) · [WebGPU dark PNG](portable/browser/artifacts/material-typography-font-weight-compact-dark.png)

Run the offline evidence check from this directory with Python 3.12 and Pillow.

```sh
python3 -E -B portable/readback.py --manifest-sha b5dba637f546eedc2ed08a4016d7a761d898c41e015b488e496d6288506aada1
```
