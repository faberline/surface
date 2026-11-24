# Surface v0.1.43 native Search proof

Two fresh runs of the repaired stock Gallery passed. The source manifest covers 557 files and has SHA-256 `386aec95d1ad1b3fe7dfb08393eb678055eebd895cd8b1a83ae445677987fd9f`. The existing stock binary manifest has SHA-256 `9b278d6f0ff1d0e72b6f09f87bb2d3cae50f4431a65a04658044e5c0123267c7`. The verifier checked the complete source and bundle before and after capture. It reused the verified native Gallery and CLI from the original product build. Only the browser test runner changed after that build.

Each run covers the Contained and Divided variants on the 960 × 800 light left-to-right view and the 320 × 400 dark right-to-left view at 200% text size. For all four cases, Enter opens the named editor, `設計` produces four matches, and Shift+Home selects both characters backward. Down finds the first enabled result. Selecting `search-doc-review` restores focus to the opener. The headless host uses a fixed viewport for each launch.

Eight PNGs and both reports match byte for byte across repeats. The API, CLI and MCP returned identical PNG bytes for each capture. The decoded wide and compact selection captures have 752 and 2,776 changed pixels inside the editor. Twenty-four CoreGraphics window checks observed zero visible test windows.

The exact wide and compact settled frames also match the settled frames from the prior eight-PNG motion diagnostic. That earlier motion report and its checksum manifest are in `prior-motion-reference`. The diagnostic clock and intermediate motion frames remain prior evidence. The repaired stock Gallery rejected `Test.setClock` and used its standard headless Reduce Motion behavior.

`verified-native.json` contains the machine-readable receipt. Run `shasum -a 256 -c SHA256SUMS` from this folder to verify the archive. To replay, copy the scripts, `prior-motion-reference` and this README to a fresh `/tmp` folder. Create an empty `inputs` folder. Run `bash replay-native.sh STOCK_STAGE`, where `STOCK_STAGE` has the source and binary checksums above. Run `python3 -B seal-native.py` with Pillow available for the full PNG decode.
