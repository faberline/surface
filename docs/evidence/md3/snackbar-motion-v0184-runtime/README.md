# Material 3 Snackbar entry and exit

Surface commit `b067e40910e9f86683ef6352ec631e9c8a8436dc` uses tree `34f4de1d361c5693643059da0d6c602732ace4a8`.
The manifest covers 718 files. `SnackbarMotion` adds optional motion to the
existing controlled Snackbar and queue. A dismissed message can keep its exit paint.
Its controls stop receiving events. A focused control returns focus to the caller's trigger.
Reduce Motion settles the transition immediately.

The first motion replay produced a duplicate announcement while an incoming message moved
into view. The final Gallery keeps the short entrance inside the viewport and moves the
retiring view below the lower edge. The six focused Chrome steps pass 158 checks. They
cover status delivery, separate actions, queue, focus return, exit paint, and Reduce Motion.

The workspace passed 1,635 Rust tests, three Wasm checks, and 18 separate Metal tests.
Two fresh native headless runs produced four original PNGs at 320 dp with RTL and 200% text.
Both runs matched `Page.screenshot`, CLI, and MCP output and returned focus after Enter.
The runs recorded 12 zero-window samples and all four Gallery processes exited.
The native PNGs show settled states. The WebGPU originals show paint during exit.

Headless Chrome passed 391 ordered Material steps and 32,757 checks.
Its independent reader decoded the motion PNGs and checked the normal and dark surface colors.

Read back the archive with Python 3.12 and Pillow 12.3.0.

```sh
python3 -E -B portable/readback.py --seal bb252ffea44d3f694abd73669671e02e066554c645d60928530d6e4df3983e3c
```

Restore `portable/source/q84-source.bundle` on branch `codex/md3-snackbar-motion-v0184`.
Run the repository's Rust checks and use the native and browser tool folders to replay it.
Set the saved configs to the rebuilt checkout and build paths before running the tools.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
cap cargo build --locked -p surface-material-gallery -p surface-devtools -p surface-e2e --features surface-material-gallery/devtools
cap cargo xtask web e2e --artifacts "$evidence" material
```
