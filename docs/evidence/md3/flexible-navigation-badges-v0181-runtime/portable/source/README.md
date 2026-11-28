# Badge candidate source

The self-contained bundle holds candidate `b9dc4ad7a5c6cdddd71196a669354f17984af7aa` and tree `ea61008a90b12a3f753398b9e5c1ff58f5813b3d`. Its 714-file manifest is `source.sha256`. The forward patch also reproduces this tree on the verified Typography candidate `47d194868af375cd4c47e26a425cff2968a57a2a`.

Run these commands from this directory.

```sh
lane=$(mktemp -d /tmp/surface-badge-source.XXXXXXXX)
manifest="$PWD/source.sha256"
git clone --branch codex/md3-flexible-navigation-badges-v0181 badges-source.bundle "$lane/surface"
git -C "$lane/surface" rev-parse HEAD 'HEAD^{tree}'
(cd "$lane/surface" && shasum -a 256 -c "$manifest")
```

Bundle SHA-256 is `565c1ffab9c8725c84e7c3aa0f5c76dce72425d12a64e8251e92680a998075fa`. Forward patch SHA-256 is `08199999d9f12d1c5ee9b35c9bbdbd6f2e5155e0b418e8f258f1939e02343ccb`. Source manifest SHA-256 is `8e4f44ab8774aebbdf629bea859101887d42c8904cd6abb32502ff7ffc0b230c`.

[Typography base record](https://github.com/faberline/surface/tree/929fcda00bc5d0291e201697b7c23a6a6c1c0061/docs/evidence/md3/typography-bold-v0180-coverage)
