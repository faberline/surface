# Surface v0.1.68 label ink on re-enable

The recovered label color candidate uses Git tree
`0db939d3f663fdb5b9fa3dd334382937693b409e`. It passed 1,575 Rust tests
and three Wasm checks in a fresh run. The new regression failed on that
source. The field was editable again, but its first frame still used
the disabled label alpha. [Original candidate evidence](https://github.com/faberline/surface/tree/14b110f1c269c1127389be68b3e9c7cb40e69b5f/docs/evidence/md3/text-field-label-color-v0168-candidate)

The fix clears the old label color track when the editor is disabled.
The first enabled frame uses the current enabled ink. The regression
exercises Outlined and Filled and uses the runtime's fixed clock.
All 13 field tests and the format check passed after the fix.

Reconstruct the original candidate in a fresh temporary checkout and set
`proof` to this directory. Use the normal Surface build environment with
`cap`, two Cargo jobs and a unique Cargo target under `/tmp/cargo-target`.

```sh
test "$(git write-tree)" = 0db939d3f663fdb5b9fa3dd334382937693b409e
git apply --index --whitespace=error-all "$proof/regression-only.patch"
cap cargo test --locked -p surface-material --test outlined_text_field \
    label_color_retargets_on_focus_and_error_without_changing_its_pose -- --exact
```

The first test fails at the enabled label ink assertion. Apply the fix next.

```sh
git apply --index --whitespace=error-all "$proof/fix-only.patch"
test "$(git write-tree)" = "$(cat "$proof/source.tree")"
shasum -a 256 -c "$proof/source.sha256"
cap cargo fmt --all --check
cap cargo test --locked -p surface-material --test outlined_text_field
```

The corrected tree's full workspace and app replays are in the test queue.
