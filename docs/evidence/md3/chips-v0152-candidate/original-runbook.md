# Material 3 Chips v0.1.52 local candidate proof

This is a local evidence packer and verifier. It reads saved test outputs. It starts no test or visible app and writes no source or Git state.

The candidate contains 624 source files. Apply `chips-v0152.patch` from the 616-file Tabs v0.1.51 checkout with tree `187930ac394178c821649c76b0f29c2f9c751b9e`. Verify the result with `shasum -a 256 -c source.sha256`. The resulting Chips tree is `48d560ab127cc1c692a5ba0eba6fb5a8949234a8`. The packer replays the patch in a private temporary copy and independently recomputes both Git tree IDs from file content and modes.

## Check and seal

Use Python 3.12 with Pillow. There is no need to run a Cargo or Chrome command to use this verifier.

```sh
python3 -B chips-proof.py --check
python3 -B chips-proof.py --pack /tmp/chips-v0152-candidate-proof
python3 -B /tmp/chips-v0152-candidate-proof/chips-proof.py --check \
  --package /tmp/chips-v0152-candidate-proof
```

The first two commands use the exact staged candidate, base, and stock observer paths stored in `chips-proof.py`. Use `--stage`, `--base`, `--stock`, and `--run` to point to different copies of the same sealed inputs. The stock observer must be the final guarded macOS run with all five explicit headless cases. Its manifest must match `source.sha256`. Incomplete, preflight, or red evidence exits with a failure and produces no sealed candidate directory. `--package` checks the completed archive without requiring the product checkout or test binaries.

## Exact gates and original files

- `rust/` holds the exact workspace log, before and after source checksum lists, the final real offscreen Metal log, and the native AccessKit projection log. The proof requires the successful workspace and exact Metal markers. The AccessKit test checks two independent Chip actions under separate grid cells. It exercises the native projection in a headless test.
- `gpu/` holds the complete PNG set from the final real Metal run.
- `native/` holds both offscreen recurrences, their two images each, reports, executable pins and replay scripts. The recurrences compare Page.screenshot, CLI and MCP bytes. They use a light 960 × 800 canvas and a dark RTL 320 × 760 canvas at 200 percent text. Each run recorded six samples with zero visible windows. The binaries cover the exact production files. The final source seal also includes subsequent E2E and documentation changes that did not affect those executables.
- `stock-macos/` holds the original five-suite archive, including the Material typed Chips report and four Chips PNGs. The verifier reuses its original window and input receipts, retained browser regressions, and exact before/after pins. It independently decodes the Chips paint and matches sample colors and carved regions to the original report.

The `SHA256SUMS` file covers the complete local package, including the full stock browser artifacts. For a narrower publication, keep the full original archive and publish its manifest alongside selected report, screenshots and logs. The scoped v0.1.51-stamped Chips development run is deliberately outside this exact candidate proof. Hosted Linux CI, live screen-reader speech, system high contrast, and optional pressed/drag reorder acceptance are separate.
