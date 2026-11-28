# Recovered Surface v0.1.75 source

The recovered source matches the earlier candidate tree exactly. The native
and browser proof still needs a fresh run. The menu adds the Standard and
Vibrant vertical layouts, named groups, keyboard focus and bounded scroll
in a short window.

| Input | Exact value |
| --- | --- |
| v0.1.74 base tree | `f7a92f77ecc9b37c91d81ace4877cd8038044d5c` |
| Base manifest SHA-256 | `7aedeb9161baadca46aa6fdd83848b7e9c5498bccfac0e9a0e21e2782ec3f0bd` |
| Candidate tree | `1a8f21971d18d2782882fc8e8e57424bdb9cf301` |
| Candidate manifest SHA-256, 698 source files | `4ab84a94bbc6923a313a66e6be5f68d43d03834e18d8f962bc8f95413b439b87` |
| Patch SHA-256 | `4783224d6fcc7fbcc952471d21e75d64a4083f019109c4e9500dd15d4de0104c` |

From a fresh checkout of the [exact v0.1.74 base](https://github.com/faberline/surface/tree/86c12c6decab921ed16087c31ea8a52f9f49141f/docs/evidence/md3/adaptive-navigation-scaffold-v0174-candidate),
set `proof` to this archive directory, then run:

```sh
test "$(git write-tree)" = f7a92f77ecc9b37c91d81ace4877cd8038044d5c
shasum -a 256 -c "$proof/base-source.sha256"
patch=$(mktemp /tmp/surface-vertical-menu-patch.XXXXXXXX)
gzip -d -c "$proof/vertical-menu-v0175.patch.gz" > "$patch"
git apply --index --check --whitespace=error-all "$patch"
git apply --index --whitespace=error-all "$patch"
test "$(git write-tree)" = "$(cat "$proof/source.tree")"
shasum -a 256 -c "$proof/source.sha256"
```

Use the repository's full check script, offscreen GPU tests, and
headless Gallery runs to verify the reconstructed build. Browser steps
M267–M269 and M370–M372 cover the Baseline and both vertical palettes.
