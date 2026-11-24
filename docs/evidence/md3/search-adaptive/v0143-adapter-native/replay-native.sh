#!/bin/bash
set -euo pipefail

root=$(cd "$(dirname "$0")" && pwd)
stage=$(cd "$1" && pwd)
export PYTHONDONTWRITEBYTECODE=1
expected_source=386aec95d1ad1b3fe7dfb08393eb678055eebd895cd8b1a83ae445677987fd9f
expected_bundle=9b278d6f0ff1d0e72b6f09f87bb2d3cae50f4431a65a04658044e5c0123267c7

manifest_matches() {
    [[ "$(shasum -a 256 "$1" | cut -d ' ' -f 1)" == "$2" ]]
}

verify_stage() {
    local phase=$1
    manifest_matches "$stage/incoming-source.sha256" "$expected_source"
    manifest_matches "$stage/browser-binary-sha256.txt" "$expected_bundle"
    local source
    source=$(python3 -B - "$stage/stock-stage.json" <<'PY'
import json, sys
from pathlib import Path
print(json.loads(Path(sys.argv[1]).read_text())["source"])
PY
)
    test "$(wc -l < "$stage/incoming-source.sha256")" -eq 557
    (cd "$source" && shasum -a 256 -c "$stage/incoming-source.sha256") > "$root/inputs/source-$phase.log"
    (cd "$stage" && shasum -a 256 -c sealed-inputs.sha256) > "$root/inputs/stage-$phase.log"
    (cd "$stage/bundle" && shasum -a 256 -c "$stage/browser-binary-sha256.txt") > "$root/inputs/bundle-$phase.log"
}

verify_stage before
for manifest in incoming-source.sha256 browser-source.sha256 browser-binary-sha256.txt sealed-inputs.sha256 stock-stage.json; do
    cp "$stage/$manifest" "$root/inputs/$manifest"
done
bundle="$stage/bundle/debug"
for n in 1 2; do
    repeat="$root/repeat-$n"
    mkdir "$repeat"
    python3 -B "$root/native-scripts/replay-search.py" --gallery "$bundle/surface-material-gallery" \
        --cli "$bundle/surface-devtools" --out "$repeat/interactive" | tee "$repeat/interactive.log"
    python3 -B "$root/native-scripts/replay-stock-settled.py" --gallery "$bundle/surface-material-gallery" \
        --cli "$bundle/surface-devtools" --out "$repeat/settled" | tee "$repeat/settled.log"
done
verify_stage after
diff -qr "$root/repeat-1" "$root/repeat-2" > "$root/inputs/repeat-diff.log"
