# Surface v0.1.75 corrected validation evidence

This package binds the disabled-menu click correction to tree
`8046e25a630284411e1ebf9f9d16d1004b02651b`. It preserves the two-path patch,
the exact source and build pins, and the original test logs. The red and
green logs cover a disabled row, a transparent group gap and an outside
click. The full workspace record checks 1,607 passes and three Wasm
completions. The offscreen record checks 18 real Metal tests and 21
original PNGs.

The native receipt requires the same eight PNGs through Page.screenshot,
the Surface CLI and MCP. It records six reaped Gallery processes and 46
samples with zero visible windows. The stock Chrome receipt requires
all 371 ordered Material steps, a single private headless Chrome launch,
zero visible owned windows and no survivors. It preserves and independently
checks the two menu reports and six original PNGs, including actual Apple
Metal WebGPU adapter details, image dimensions, pixel samples and geometry.

## Collect or read back

Use Python 3.12 with Pillow. All commands below run offline and start no
app. The collector expects the actual corrected build and guarded runs
under `/tmp/surface-md3-disabled-menu-corrected-replay.ZIfGJCvi`. Optional
`--build`, `--native` and `--browser` flags let the controller supply the
exact physical directories.

```sh
lane=/tmp/surface-md3-disabled-menu-corrected-proof-r2.xUTYrxQM
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
"$py" -E -B "$lane/collect.py" --preflight
"$py" -E -B "$lane/collect.py" --collect "$lane/portable"
"$py" -E -B "$lane/portable/collect.py" --verify "$lane/portable" --sha REVIEWED_SHA
```

Collection requires both successful terminal receipts. It rejects the
published original native and Chrome runs and leaves the archive path
empty if either corrected run is pending. The command prints the package
manifest SHA-256. Use that value for readback. The reader resolves the
physical paths before checking native evidence. For direct native
`run-guarded.sh --verify` use the physical directory from `pwd -P`.

## Reproduce

The attached `replay/README.md` gives the exact workspace, build and
guarded replay commands. Run the primary and supplemental Metal checks
from the pinned corrected workspace with two Cargo jobs. Use a fresh
scratch directory for the images.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v
export CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-disabled-menu-click.qfrcXZ1r-source
gpu_scratch=$(mktemp -d /tmp/surface-md3-corrected-metal.XXXXXXXX)
export SURFACE_GPU_EVIDENCE="$gpu_scratch/gpu-offscreen.png"
cd /tmp/surface-md3-disabled-menu-click.qfrcXZ1r/source
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The portable package keeps original reports, guard ledger, selected
screenshots, compressed original logs and the small offline readers. The
large artifacts, build caches, private sessions and browser profiles stay
in scratch. This is evidence of the corrected local runtime. The earlier
original-source and original-runtime evidence stay in their own records.
