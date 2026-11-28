# Surface v0.1.76 corrected choice menu replay

This bundle binds the combined source to tree
`37e4454bc7d3f63f7111c8264d4e59e451c8ef9e` and its 703-file manifest
`a9aa38ee7ab64746471491ccfd6cd7df0132e8f30254d536a6fdd3fc95e10eb8`.
Preflight recomputes both the base R3 and corrected trees. It verifies
the exact seven-path delta and all three correction patches.

## Build

Use the same macOS and Rust toolchain as the combined workspace check.
Run the build after that check releases the shared Cargo target. The
script uses `cap` and two Cargo jobs.

```sh
lane=/tmp/surface-md3-choice-menu-v0176-corrected-replay.Q1k95t8V
sh "$lane/build-pinned.sh" --check-source
sh "$lane/build-pinned.sh" --build "$lane/outputs"
sh "$lane/build-pinned.sh" --verify "$lane/outputs"
```

The builder records three original build logs, the toolchain and fresh
pins for the native Gallery, Surface CLI, browser runner and Web Gallery.
Its SHA-256 manifests bind the actual build output to this corrected tree.

## Replay with no visible windows

```sh
sh "$lane/run-guarded.sh" native --check-inputs "$lane/outputs"
sh "$lane/run-guarded.sh" browser --check-inputs "$lane/outputs"
sh "$lane/run-guarded.sh" native --run "$lane/outputs" "$lane/guarded-native"
sh "$lane/run-guarded.sh" browser --run "$lane/outputs" "$lane/guarded-stock-material"
sh "$lane/run-guarded.sh" native --verify "$lane/outputs" "$lane/guarded-native"
sh "$lane/run-guarded.sh" browser --verify "$lane/outputs" "$lane/guarded-stock-material"
```

Run native and browser in sequence. The native runner requires three
profiles, two repetitions, six reaped Gallery processes and eight
original PNGs. Each capture must match through Page.screenshot, the
Surface CLI and MCP. The stock browser requires one guarded Chrome
launch, 374 ordered steps through M375, three reports, nine original
PNGs, zero visible owned windows and no surviving processes.

M373 uses an actual browser mouse click on the disabled owner row. The
report must show an open menu, focus on that row, zero choice changes,
and the same checked values as before the click. The other choice menu
steps retain their WebGPU, semantics, keyboard, pointer, touch, and
compact dark RTL checks.

The exact corrected source check passed. Five offline controls rejected
the old R3 sources, build receipt and producer manifests. The M373
receipt parser passed its offline controls. Runtime evidence requires a
fresh build and both terminal replay receipts. Keep the old R3 runtime
results and the corrected run in their respective evidence records.
Archive the corrected pins, toolchain, replay tools, logs and original
menu reports and PNGs. Keep build caches, browser profiles, executables
and active DevTools sessions in scratch.
