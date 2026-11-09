#!/usr/bin/env python3
"""Make the browser subset of Noto Sans CJK TC.

Needs fontTools 4.68.0 (`python3 -m pip install fonttools==4.68.0`).
Keeps printable ASCII, the Big5 symbol block and the common-use Chinese
characters (Big5 lead bytes 0xA1 to 0xC6), and every character in
crates/surface-wgpu/assets/subset-extra.txt.
"""
from pathlib import Path
import sys

from fontTools import subset

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "crates/surface-wgpu/assets"
SOURCE = ASSETS / "NotoSansCJKtc-Regular.otf"
EXTRA = ASSETS / "subset-extra.txt"
OUTPUT = ASSETS / "NotoSansCJKtc-Subset.otf"


def big5_common():
    chars = set()
    for lead in range(0xA1, 0xC7):
        for tail in list(range(0x40, 0x7F)) + list(range(0xA1, 0xFF)):
            try:
                chars.add(bytes([lead, tail]).decode("big5"))
            except UnicodeDecodeError:
                pass
    return chars


def main():
    chars = big5_common()
    chars |= {chr(code) for code in range(0x20, 0x7F)}
    chars |= set(EXTRA.read_text(encoding="utf-8").strip())
    subset.main([
        str(SOURCE),
        "--text=" + "".join(sorted(chars)),
        "--layout-features=*",
        f"--output-file={OUTPUT}",
    ])
    print(f"{len(chars)} characters, {OUTPUT.stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
