# Material Snackbar status and actions checkpoint

The Snackbar exposes its message as a live status. Undo and Close each have a separate named action. The app owns the duration, placement, and response to each action.

## Restore the exact product source

Use the published Directional Drawer recovery instructions. Verify the resulting tree at the pinned revision, then run the Snackbar recovery below. It reverses the 11-path Directional Drawer delta. It checks all file bytes, symlink targets and executable bits. The original 10-path Snackbar forward delta is included for use with the Classic Navigation Rail predecessor.

- [Pinned Directional Drawer recovery](https://github.com/faberline/surface/tree/2725afd021230fd73df20582098a2587d2149a20/docs/evidence/md3/directional-drawer)
- Source tree SHA-256 `0ca01ae3d9951e3de61375a865c78dfe412710e140df9ad539fbdb80e4762f6a` for 658 source files.
- Original forward delta SHA-256 `7af04c54b11d63b2bf9331ccd8871670308c14079597b90ceb83fdb39573a3ed` before compression.

```sh
shasum -a 256 -c evidence.sha256
sh restore-source.sh /path/to/verified-directional-drawer-source
```

## Retained results

- Rust workspace verification recorded 1,374 passed, zero failed and 25 ignored. The focused records cover nine Material Snackbar tests and ten Gallery tests.
- Chrome WebGPU passed 156 focused checks and 24,515 full checks. The focused checks cover separate actions, status, focus retention, and rejection of retired actions.
- The native Apple M1 Max Metal run recorded 40 CLI PNGs and 40 AppKit reads. The 320 × 640 dark 200% PNG is byte-identical through CLI and MCP. AppKit received one polite message-only announcement request on each of the 16 shows. Four Undo and eight Close AXPress operations dispatched once per action.

The native fixture used a private test overlay. Chrome tested the accessible tree and focus. The native run recorded announcement requests. Live screen-reader speech was outside the verified scope. The packet contains selected source deltas, summarized receipts and representative PNGs. The historical working sessions, binaries and broad generated output are outside the packet. This recovery run checked the original source and artifact digests.
