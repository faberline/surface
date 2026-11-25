# Surface v0.1.65 Multiline Text Field candidate proof

This archive covers local verification of the exact candidate tree
2341dc506a206fe1e892c35a32d51386702958b3. The source manifest pins 681 files. The 31-path patch
applies to the v0.1.64 tree eeef2dc049409f7df3a67773d8e72e76b298faf6.

- The workspace passed 1,561 Rust tests and
  three WebAssembly checks. 18 GPU tests ran
  on Apple Metal.
- Native headless Gallery produced seven originals in each of two
  runs. Page.screenshot, CLI and MCP matched. The window observer saw
  zero visible windows in 24 checks. All four Gallery processes exited.
- The separate integrated development Chrome run passed 247 checks from
  M341 through M345. It recorded 12 original PNGs. Its four Gallery Web
  files match the exact stock files.
- Exact stock macOS passed 24,689 checks
  in five headless suites. It ran all 344 Material steps through M345
  and both retained IME I11 paths. The archive includes 12 multiline PNGs
  plus the earlier Counter, Adornments, Filled and Outlined editor images.
  Chrome used Apple Metal, and the guard found zero visible windows or
  surviving processes.

The native harness uses semantic DevTools keys. The Chrome harness uses
CDP input and verifies trusted pointer and wheel events plus a simulated
composition. Speech and physical input methods require separate tests.

## Offline recheck

Use Python 3.12 with Pillow to read the archived files.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-multiline.py --verify-package . --package-sha "$manifest_sha"
~~~

The gzip files keep their original log or patch bytes. This command
does not launch the Gallery or Chrome.
