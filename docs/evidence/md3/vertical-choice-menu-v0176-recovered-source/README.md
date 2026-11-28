# Recovered Surface v0.1.76 source

The recovered source matches the earlier checkable-menu candidate. It adds
caller-controlled single and multiple choice groups, a leading checkmark, and
checked state in Web and macOS accessibility. Browser and native runtime
verification remain pending.

| Input | Exact value |
| --- | --- |
| v0.1.75 base tree | `1a8f21971d18d2782882fc8e8e57424bdb9cf301` |
| Base manifest SHA-256 | `4ab84a94bbc6923a313a66e6be5f68d43d03834e18d8f962bc8f95413b439b87` |
| Candidate tree | `b3e62f07a03b08f1741de8695fe845d3b771f11d` |
| Candidate manifest SHA-256, 702 source files | `b1b3dea6427bb8bdd5debdd2715d3956e8ac4549a03a70880aa7fe12aabc700f` |
| Uncompressed patch SHA-256 | `406b83d02a28da0116cd93e446bc42692a8469def3a67bae57ce1551b0e92cc1` |

From a fresh checkout of the [exact v0.1.75 base](https://github.com/faberline/surface/tree/4b7a41753fc00b996b3a1b70885e272876cb77ca/docs/evidence/md3/vertical-command-menu-v0175-recovered-source),
set `proof` to this archive directory, then run:

```sh
test "$(git write-tree)" = 1a8f21971d18d2782882fc8e8e57424bdb9cf301
shasum -a 256 -c "$proof/base-source.sha256"
patch=$(mktemp /tmp/surface-vertical-choice-patch.XXXXXXXX)
gzip -d -c "$proof/vertical-choice-menu-v0176.patch.gz" > "$patch"
git apply --index --check --whitespace=error-all "$patch"
git apply --index --whitespace=error-all "$patch"
test "$(git write-tree)" = "$(cat "$proof/source.tree")"
shasum -a 256 -c "$proof/source.sha256"
```

Run the workspace check and the headless Gallery tests. Browser steps
M373–M375 exercise the checkable menus. Retain the Baseline and
vertical command steps as regression coverage.
