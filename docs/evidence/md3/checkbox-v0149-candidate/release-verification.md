# Surface v0.1.49 release verification

Main commit `71c7734ab0b7d53ac5711a07db6664b5885a99ca` and candidate `8e5c7220c750fa709c6a9b91216345e04947a028` share tree `238edf883656d675e05b02ee7df280e35fbd988b`.
The exact 602-file source manifest is `3a9aeda30a8ca3520afd04d9d4db8c26f3a900dc5aae20fad14e4d366d2544fe`. It verified against the released checkout.

## Verification

- 1335 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed. 16 offscreen Metal cases ran.
- Two native headless replays each produced two matching screenshots through the API, CLI and MCP. The observer found zero visible windows.
- Five stock macOS headless cases passed 19827 checks and verified four Checkbox PNGs.
- Eight stock Linux cases passed 20204 checks on the PR and 20204 on the branch with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/7eb50103ff7b19ba37b7dd70f59aed9da3f561c7/docs/evidence/md3/checkbox-v0149-candidate)

Live screen-reader speech and platform high contrast remain open.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45504989995) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45504974898)
