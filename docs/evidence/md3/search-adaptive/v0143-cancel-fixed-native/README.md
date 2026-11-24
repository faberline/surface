# Surface v0.1.43 native Search proof

Two fresh runs of the repaired stock Gallery passed. The source manifest covers 557 files and has SHA-256 `82d29caaad575703758710ec4e068716dee32a7c099f277e21930827d115ad23`. The existing stock binary manifest has SHA-256 `fb08e7575e9a0b50ad6931cfa814b5917736a4c531fb444f5642bc5054cd22e1`. The verifier checked the complete source and bundle before and after capture. It reused the sealed binaries without rebuilding them.

Each run covers the Contained and Divided variants on the 960 × 800 light left-to-right view and the 320 × 400 dark right-to-left view at 200% text size. For all four cases, Enter opens the named editor, `設計` produces four matches, and Shift+Home selects both characters backward. Down finds the first enabled result. Selecting `search-doc-review` restores focus to the opener. The headless host uses a fixed viewport for each launch.

Eight PNGs and both reports match byte for byte across repeats. The API, CLI and MCP returned identical PNG bytes for each capture. The decoded wide and compact selection captures have 752 and 2,776 changed pixels inside the editor. Twenty-four CoreGraphics window checks observed zero visible test windows.

The exact wide and compact settled frames also match the settled frames from the prior eight-PNG motion diagnostic. That earlier motion report and its checksum manifest are in `prior-motion-reference`. The diagnostic clock and intermediate motion frames remain prior evidence. The repaired stock Gallery rejected `Test.setClock` and used its standard headless Reduce Motion behavior.

`verified-native.json` contains the machine-readable receipt. Run `shasum -a 256 -c SHA256SUMS` from this folder to verify the archive. To replay, copy the scripts, `prior-motion-reference` and this README to a fresh `/tmp` folder. Create an empty `inputs` folder. Run `bash replay-native.sh STOCK_STAGE`, where `STOCK_STAGE` has the source and binary checksums above. Run `python3 -B seal-native.py` with Pillow available for the full PNG decode.

The shared [source and stock build recipe](../v0143-cancel-fixed-mac-stock/) reconstructs the existing candidate. A new stock build can use its own verified binary manifest. Set `SURFACE_EXPECTED_STOCK_BUNDLE_SHA256` to that manifest's SHA-256 for both scripts. Local paths were replaced in the published stock metadata. The original seal records the executed inputs.
