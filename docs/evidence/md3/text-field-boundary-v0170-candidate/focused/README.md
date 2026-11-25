# Surface 0.1.70 R1 focused headless verification

The 689 original source files are pinned to tree
`a3e9e2962861a71825fc7c12ee3580561d813806`, manifest
`f0b9bf4f43c9ec853fd2ec327028dabfe8215846ef0beec6222f3ac7fecb684c`.
The original source retains all 355 ordered Material steps. The private
runner changes only the registry to select M325–M356. Its manifest is
`27113e50b159dbefc3901f41a3858b7ae901b3a40b1aa12141e2fda21046797d`.
All four original Gallery files match manifest
`cd449385e300bf6a78226070f0a8941534520ab461abf3ddaa764b9e5f47304a`.
Only a symbolic link to the immutable asset folder is in this lane.
The Surface test server already resolves this folder before serving it.

The observer is the original verified v0.1.69 R2 script, byte-for-byte.
The launch adds only the previously used per-process
`--disable-updater-scheduler` flag. The observer allows one primary
Chrome headless launch and requires no visible or surviving owned
process. Both pin manifests are narrowly enforced before and after
execution. The full stock registry is available in the controller source.

## Controller actions

Source and Gallery preparation have been completed, without Cargo or
Chrome. The controller owns both next actions. Coordinate the handoff
with any existing GPU runs before starting Chrome.

```sh
lane=/tmp/surface-md3-field-boundary-focused-v0170.rWtO97J5
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c launcher.sha256 && shasum -a 256 -c verifier.sha256)
sh "$lane/build-focused.sh" --build-scoped
"$py" -E -B "$lane/run-focused.py" --verify-inputs-only
cap "$py" -E -B "$lane/run-focused.py" --run --out "$lane/development-r1"
"$py" -E -B "$lane/verify-boundary.py" --run "$lane/development-r1" > "$lane/development-r1-verified.json"
```

Build target is
`/tmp/cargo-target/surface-surface-md3-field-boundary-focused-v0170.rWtO97J5-source`.
The executable is pinned by reference. Preserve the target until the
runner proof is verified. For a revised source, use a fresh lane. Pass
the new adjacent source and manifest with approved tree/digest to
`prepare-focused.py prepare`. Pass the corresponding Web inputs and
producer source manifest to `pin-gallery`. A prior Web bundle is
compatible only with a byte-proven E2E-only source delta.

## Independent checks

The new receipt expects 33 images. It reads 10 focus, 16 palette and
7 compact images. The previous 34 label and 58 placeholder originals
remain required, in addition to prior M325–M346 captures. The verifier
reads each original PNG, derives frame metadata from the phase list
and compares reported points, coverage and RGB to the decoded file.
It requires both full-coverage color and quarter-pixel geometry.
At 14/44 ms, Filled has 10/16 quarters and Outlined 11/20. It checks
same-time reversal, the new width after 20 ms, focus and palette
endpoints, the dark physical-right notch and the closed fallback.
One blending convention must explain all fractional pixels on the
observed adapter. Full ink and background samples remain exact.

The migrated M347–M350 verifier recomputes the Outlined ink for each
elapsed time. It re-samples the reported early shallow cut at 0.25,
0.75 and 1.25 dp, preserving older label, notch, CJK and AX checks.
It still reads the original 0.25/2.5 dp coordinates. The separate
placeholder pixel verifier remains byte-identical to v0.1.69 R2.

`verify-boundary.py --artifacts PATH` verifies only the new raw
boundary group. `--run PATH` additionally checks exact source and
binary provenance, all 32 ordered steps and the clean guard receipt.
`--self-test` uses original .69 files and synthetic controls only.
The saved result proves prior label and hint PNGs still read,
a real .69 3 dp first focus has ink where .70 requires background,
and a synthetic low-alpha outer pixel cannot pass as fully covered
ink. It supplies no fresh .70 runtime result.
