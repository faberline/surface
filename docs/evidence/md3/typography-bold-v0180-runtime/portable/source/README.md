# Typography candidate source

The self-contained Git bundle holds source commit `50c84895b631114df5773769ff69b01d00a7859b` and tree `db5f002fb376dc377571b04dac9ce36636e85ee6`. It precedes the separate coverage record.

Run these commands from the evidence directory.

```sh
lane=$(mktemp -d /tmp/surface-typography-replay.XXXXXXXX)
shasum -a 256 source/typography-source.bundle
git clone --branch codex/md3-real-bold-font-v0180 source/typography-source.bundle "$lane/surface"
git -C "$lane/surface" rev-parse HEAD 'HEAD^{tree}'
manifest="$PWD/source/source.sha256"
(cd "$lane/surface" && shasum -a 256 -c "$manifest")
```

The bundle SHA-256 is `fa4ff57943c455e4f3085b958829d65101a8a017569fdcb6b0fced3d8164b360`. The 711-file source manifest SHA-256 is `6bfb0523e2a040810a6f0e1e6f87918d7cc107d232e5126826d533fed37ed548`.
