# Search release source after composition cancellation repair

This 557-file staged Surface v0.1.43 source includes the browser cancellation repair from current v0.1.42 draft commit `86bf5eb86a5d070245db039b37977198ba1a417e`. The base commit has tree `93b47517d26662b1e9aa5502f730349723fb0978`. The Search patch changes the staged release version and applies the adaptive Search work. The two composition cancellation regressions stay in source. [Base commit](https://github.com/faberline/surface/tree/86bf5eb86a5d070245db039b37977198ba1a417e)

## Reconstruct

Export the base commit to a scratch copy. Set `evidence` to the absolute path of this folder and `source` to the exported base.

```sh
(cd "$evidence" && shasum -a 256 -c SHA256SUMS)
cd "$source"
git apply --whitespace=error-all "$evidence/search-after-cancel.patch"
shasum -a 256 -c "$evidence/source.sha256"
```

The commands verified all 557 reconstructed files. The manifest SHA-256 is `82d29caaad575703758710ec4e068716dee32a7c099f277e21930827d115ad23`. This receipt covers exact source reconstruction. Search still needs its release gate on this source. The earlier native and browser replays use the [pre-repair staging source](../v0143-pre-cancel-fix/).
