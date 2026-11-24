# Retargeted Material Divider verification

The 583-file source has manifest SHA-256 `73486892d094b921162adbbd69ecfe67ed982c35549be68775f86f1927635f5c`. Start from the exact corrected Dialog candidate in [native/dialog-base.sha256](native/dialog-base.sha256). Apply [the portable Divider patch](native/material-divider-retarget.patch) and verify [the resulting manifest](native/source.sha256). A fresh copy reproduced all inputs. The patch assigns M264 and M265 to Divider and preserves Dialog M256–M263. Its browser test records the adapter's actual fallback status.

- Two windowless native runs exercise the same four Divider styles at 960 × 640 light LTR and 320 × 640 dark RTL at 200% text. The two PNGs match between replays and across Page.screenshot, CLI and MCP. Twelve window samples found no visible window. [Native proof](native)
- Headless Chrome with Apple Metal ran M264 and M265 with 125 checks. It decoded two PNGs and measured all eight stripes at two physical pixels on the 2× browser profile. Chrome accessibility keeps the headings and excludes the decorative divider nodes. [Chrome proof](web)

The pre-release layer remains at version 0.1.44. Its full release and hosted Linux browser checks follow the Dialog release. Run `shasum -a 256 -c SHA256SUMS` here to verify the archive.
