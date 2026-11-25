# Counter native headless replay R2

This replay reads pinned source and executable manifests before launching
and after capture. It uses the unchanged native helpers from the v0.1.63
adornments replay. Two rounds each capture seven PNGs. Page.screenshot,
the stock CLI and MCP must each return the same bytes and metadata. The two
rounds must also agree. The Gallery uses native headless mode. The observer
checks its owned process for visible windows and verifies that it exits.

The wide 960 × 1120 run tests the starting value, its soft limit, manual
validation priority, the exact limit, named Clear, and the locked examples.
The compact 320 × 1200 run tests dark RTL at 200% text size, Filled and
Outlined appearances, and full supporting text and the counter in the
112 dp stack. It uses the moved Clear action through Tab and Enter.

The replay decodes each original PNG. It checks the neutral, error and
disabled counter inks, both appearances, and the separate painted hash
affix. The supporting row starts 4 dp below the face and keeps 16 dp side
insets and an inline gap of 16 dp. The stacked row keeps a 4 dp gap before
the count. The native proof uses the Runtime DOM and DevTools key and
fill events. It does not measure live IME or spoken accessibility output.

| Profile | Original PNG prefix |
| --- | --- |
| Wide baseline | wide-outlined-seed |
| Wide soft overflow | wide-outlined-overflow |
| Wide manual validation | wide-outlined-manual-error |
| Wide Filled | wide-filled-seed |
| Compact Outlined | compact-outlined-rtl-200 |
| Compact Filled soft overflow | compact-filled-overflow-rtl-200 |
| Compact Filled fallback | compact-filled-stacked-112 |

## Read-only checks and development inputs

The original development manifest describes 677 files. The root stage
later updated the E2E registry. The private snapshot reconstructs that
one prior file from identical v0.1.63 bytes and rechecks all 677 digests.
The development binary manifest stays in the stage.

~~~sh
lane=/tmp/surface-md3-counter-native-r2.NqEd6dD0
previous=/tmp/surface-md3-counter-native.jGyZbVdk
stage=/tmp/surface-md3-text-field-counter-v0164.zL1rFusV
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
manifest="$previous/development-source.sha256"
source_root="$previous/development-source-snapshot"
source_sha=79963ff9cfcc7e802fe74489ab62247c5bf0efd6b83a6edd7942bd83d82681c9
binaries="$stage/native-development-binaries/SHA256SUMS"
binary_sha=1f6bf2107ef940ba23912ac57b349a188cfa034d94dc8ffa574269ba195dc098

(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/replay-text-field-counter.py" --self-test
"$py" -E -B "$lane/inspect-stored-counter.py"
"$py" -E -B "$lane/replay-text-field-counter.py" --verify-inputs-only \
  --source-root "$source_root" --source-manifest "$manifest" --source-manifest-sha "$source_sha" \
  --binary-manifest "$binaries" --binary-manifest-sha "$binary_sha"
~~~

These commands start no app. The pure self-test uses geometry examples and
synthetic pixels only. The preflight verifies the real source and executable
bytes. Use fresh source and executable manifests for the final v0.1.64 gate.
Pass their reviewed hashes. Keep development captures separate.

## Original regression and revised pixel check

The first wide replay PNG matches the independent wide preview byte for byte.
Its 12-point counter glyphs have no pixel within three RGB levels of the
unblended foreground. The pixels instead match the authored color blended
over the captured background. R2 fits the hue and coverage for both
possible sRGB blend policies. One policy must fit at least 90% of the
contrasting pixels, including four at 65% coverage, one at 85%, and zero
pixels more opaque than the authored ink. The reported policy is a fit.
It does not determine which space the renderer used.

The original wide and compact PNGs both pass the revised check. The
controls reject the wrong error hue or disabled opacity on both originals.
The original failed run and PNG remain in the previous lane. These read-only
checks do not count as a successful Gallery replay.

## Controller launch

The controller can run one development round with `--repeats 1`. The
final proof uses the default two rounds. Use a new output directory.

~~~sh
cap "$py" -E -B "$lane/replay-text-field-counter.py" --out "$lane/exact-1" \
  --source-root "$source_root" --source-manifest "$manifest" --source-manifest-sha "$source_sha" \
  --binary-manifest "$binaries" --binary-manifest-sha "$binary_sha"
~~~

The output is `native-text-field-counter-report.json` and `SHA256SUMS`.
The source replay and all media remain in private scratch. This preparation
does not include a Gallery run.
