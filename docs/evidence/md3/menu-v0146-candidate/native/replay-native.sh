#!/bin/sh
set -eu

batch=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
out=$1
python=${PYTHON_BIN:-python3}
export PYTHONDONTWRITEBYTECODE=1
mkdir "$out"
(cd "$batch/source" && shasum -a 256 -c ../source.sha256) > "$out/source-before.log"
(cd "$batch/pins" && shasum -a 256 -c native-binaries.sha256) > "$out/pins-before.log"
for run in 1 2; do
    "$python" "$batch/native-scripts/replay-menu.py" \
        --gallery "$batch/pins/surface-material-gallery" \
        --cli "$batch/pins/surface-devtools" \
        --out "$out/replay-$run" > "$out/replay-$run.log" 2>&1
done
diff -qr "$out/replay-1" "$out/replay-2" > "$out/repeat.log"
(cd "$out" && shasum -a 256 replay-1/*.png replay-1/native-menu-report.json > repeat.sha256)
(cd "$batch/source" && shasum -a 256 -c ../source.sha256) > "$out/source-after.log"
(cd "$batch/pins" && shasum -a 256 -c native-binaries.sha256) > "$out/pins-after.log"
"$python" "$batch/verify-native.py" "$out" > "$out/verify.log"
