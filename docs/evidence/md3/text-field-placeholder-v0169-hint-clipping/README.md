# Surface v0.1.69 placeholder clipping

The original forward tree is `7c0a0990bb5aaa980b2e81d4aa9f7b7d1790dc40`.
Its disabled single-line hint loses part of the second word. A second test
shows the RTL field clips the right edge. The original logs record both
failures and the corrections. The final field run passed all 16 tests.

The same-ID regression checks what remains visible before disable, while
disabled and on the first enabled frame. It covers both appearances and
both field directions. The Material coverage row records the test.

Use the exact original source in a temporary checkout. Set `proof` to this
folder. Run Cargo through `cap` with two jobs and a unique target under
`/tmp/cargo-target`.

```sh
test "$(git write-tree)" = 7c0a0990bb5aaa980b2e81d4aa9f7b7d1790dc40
git apply --index --whitespace=error-all "$proof/hint-clipping.patch"
test "$(git write-tree)" = "$(cat "$proof/source.tree")"
shasum -a 256 -c "$proof/source.sha256"
cap cargo fmt --all --check
cap cargo test --locked -p surface-material --test outlined_text_field
```

This record covers targeted Rust checks. The corrected full workspace,
native renderer and browser results are still pending.
[Original placeholder candidate](https://github.com/faberline/surface/tree/2ed23ef70435a246547e29fa55caa89557558d07/docs/evidence/md3/text-field-placeholder-v0169-candidate)
