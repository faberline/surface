# q74 hosted original evidence

This package records one original attempt for q74 PR #250. It checks the reviewed source, checkout and parent, successful original jobs, original logs and SHA-256/size of the downloaded ZIP. The ZIP is removed after extracting the typed reports and four adaptive navigation PNGs. Independent Pillow readback checks compact, medium, rail and short dark RTL geometry and pixels. The full ZIP is omitted from this portable package, so offline readback checks its recorded receipt and the sealed extracted files but cannot rehash omitted ZIP bytes.

Python 3.12 and Pillow are required. The externally reviewed config and package seal must be saved separately. No historical fixture is a substitute for hosted CI.

```sh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
kit=/tmp/surface-md3-v0174-hosted-pr.Sh1T9AYj
"$py" -E -B "$kit/hosted.py" --collect /tmp/NEW_Q74_PACKAGE --config "$kit/REVIEWED_CONFIG.json" --config-sha REVIEWED_CONFIG_SHA
"$py" -E -B /tmp/NEW_Q74_PACKAGE/hosted.py --verify /tmp/NEW_Q74_PACKAGE --config-sha REVIEWED_CONFIG_SHA --manifest-sha REVIEWED_PACKAGE_SHA
```

Local runtime evidence: https://github.com/faberline/surface/tree/86c12c6decab921ed16087c31ea8a52f9f49141f/docs/evidence/md3
