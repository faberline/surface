# Metal checks after composition cancellation fix

These Metal tests ran on the exact Surface draft head `86bf5eb86a5d070245db039b37977198ba1a417e`, tree `93b47517d26662b1e9aa5502f730349723fb0978`. The verified manifest covers all 549 source inputs, SHA-256 `c99ed8ff89b0cee66a84732598ac2ba652802c2e52c9122c3c5bb27f250ccd4b`. All tests drew offscreen using Metal on Apple M1 Max.

| Command | Result |
| --- | --- |
| `SURFACE_GPU_EVIDENCE=offscreen.png sh scripts/check-gpu.sh` | 9 passed. [Log](renderer.log) · [PNG](offscreen.png) |
| `cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture` | 1 passed. [Log](raster-cache.log) |
| `cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture` | 1 passed. [Log](text-selection.log) |

The earlier attempt under restricted local process access returned NoAdapter. The verified rerun used a Metal-accessible background process. Both runs were headless.

Run `shasum -a 256 -c SHA256SUMS` here to check the archived files.
