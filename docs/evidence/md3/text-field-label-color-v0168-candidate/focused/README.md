# Surface v0.1.68 scoped headless browser replay

The full controller development source is pinned at 685 files
`e7c6c189cd48f31257bf8b7de66da02ca9201d58a566544e741651cec4f1b006`.
The independent runner copy is sealed at
`98062ecd67a97a3e7346e97c2321b55ba0ae319c80c8bfcf5a778180d134fecc`.
Its only different file is `e2e/src/material.rs`. That file selects
all 26 ordered M325–M350 steps. The five old field test modules use the
exact v0.1.67 bytes. The new M347–M350 module remains identical to the
controller’s sealed source.

The four root-built Gallery files have also been pinned under
`web/material-gallery`. Their original manifest is
`6d3e736df6dd52d1429ccc67700351e1635a8b2ee54a1732e47c0ca01a31bc7f`.
`inputs-source-and-gallery.json` records both full source and Web
provenance. `prepare-focused.py recheck` rehashes copies and origins.
The Web pack retains its original R1 build-source digest. R2 changes
only one Rust test declaration to resolve compiler error E0282. The
scoped source and target paths, selector, and four Web originals are
the same. See `repin-r2.json` and the preserved `preflight-r1` seal.

The observer reuses the exact v0.1.67 scoped R3 guard. The Chromium
updater scheduler flag applies only to this Chrome launch and appears
in the invocation and actual guarded arguments. The guard requires
a private profile, physical sampling with zero owned visible windows,
and no owned surviving process after the original bounded grace.
It does not start Chrome until the controller authorizes the command
and the exact v0.1.67 stock pass marker exists.

## Read-only preparation and verification

The source and Gallery pinning steps have already run in this lane.
The sealed negative check accepts the old v0.1.67 proof in its original
15-image scope, then rejects the same report and genuine endpoint pixels
when tested as new intermediate color.

```sh
lane=/tmp/surface-md3-label-color-focused-v0168.Ebk8jXAK
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/prepare-focused.py" recheck
"$py" -E -B "$lane/verify-label-color.py" --self-test
```

## Controller build and guarded run

The build uses a private capped two-job Cargo target, locked offline
inputs, and SDK settings. It rechecks the complete focused source
before and after the build and copies the resulting executable to
`pins` with its digest and source-manifest binding.

```sh
sh "$lane/build-focused.sh" --build-scoped > "$lane/build-focused-r2.log" 2>&1
"$py" -E -B "$lane/run-focused.py" --verify-inputs-only
```

Wait for the controller to confirm all previous GPU-sensitive work
is terminal, then run one scoped headless Chrome test. Use a fresh
output directory. Preserve an original failed output if any gate fails.

```sh
cap "$py" -E -B "$lane/run-focused.py" --run --out "$lane/development-r2"
"$py" -E -B "$lane/verify-label-color.py" --run "$lane/development-r2" \
  > "$lane/development-r2-verified.json"
```

The independent verifier reads the actual suite ledger, observer,
raw typed report, and all 34 original label PNGs. It checks the prior
spatial and border proofs, no-overlap placeholder, synthetic CDP
preedit, independent golden RGB and velocity, fixed glyph samples,
and real Chrome accessibility values and error states. It counts
all 26 steps in one scoped Material run. The full stock release run
is separate.
