# Material 3 badges on the wide navigation rail

Surface candidate `7a05c24179b9e2ddf582ddc811cb7cc88a965bde` uses tree
`1466ad312818bcf7c80902536c7068d19b47ef0e` and a 716-file source manifest.
The additive builders support dot, localized label, count and hidden badge states.
Collapsed rails reserve space for scaled badge text. The optional scroll viewport lets
keyboard focus reveal a target in a short rail. Expanded badges align before the visible label.

Workspace verification passed 1,632 Rust tests and three Wasm checks.
Eighteen separate offscreen Metal tests produced 21 original PNGs.
Two native headless replays produced ten original screenshots. Page.screenshot, the CLI,
and MCP returned identical bytes at each capture. Native window checks recorded
18 zero-window samples and all eight Gallery processes exited.

Stock headless Chrome passed 387 Material steps and 32,479 checks.
It recorded 5,026 guarded Chrome samples with zero owned visible windows.
Its 320 × 720 dp dark RTL case verified keyboard scrolling without page movement.
The Chrome accessibility tree retained the complete localized count until activation.

The large-text native screenshot at 320 dp shows 144 sampled mailbox ink pixels.
The earlier 320 dp render showed 20. The first Browser attempt reached M386 and stopped
at the clipped Gallery launcher. The next focused run exposed a pointer-focus assertion
that expected the Chrome DOM element. The final test verifies Surface pointer focus and
keeps the separate browser accessibility focus assertion for keyboard movement.

Read back the package offline with Python 3.12 and Pillow.

```sh
python3 -E -B portable/readback.py --manifest-sha 34ae81d40854565742e800529b5209b160e4f55be747a2061cfe37616e315025
```

Restore the verified source bundle using its `codex/md3-wide-rail-badges-v0182` branch.
Use the repository's Rust build rules and the original inputs in the native and Browser configs.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo build --locked -p surface-material-gallery --features devtools
cap cargo xtask web e2e --artifacts "$evidence" material
```

[Material design reference](material-reference.md) · [Portable test package](portable/README.md)
