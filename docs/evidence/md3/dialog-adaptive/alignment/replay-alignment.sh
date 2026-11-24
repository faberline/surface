#!/bin/sh
set -eu

python_bin=${PYTHON_BIN:-python3}
evidence=$(cd "$(dirname "$0")" && pwd)
stage=$1
out=$2
mkdir "$out"
(cd "$stage/source" && shasum -c "$evidence/source.sha256") > "$out/source-before.log"
(cd "$stage/pins" && shasum -c native-binaries.sha256) > "$out/pins-before.log"
for run in 1 2; do
    "$python_bin" -B "$evidence/native-scripts/replay-dialogs.py" --gallery "$stage/pins/surface-material-gallery" \
        --cli "$stage/pins/surface-devtools" --out "$out/run-$run" > "$out/run-$run.log" 2>&1
done
diff -qr "$out/run-1" "$out/run-2" > "$out/repeat.log"
(cd "$out" && shasum -a 256 run-1/*.png run-1/native-dialog-report.json > repeat.sha256)
(cd "$stage/source" && shasum -c "$evidence/source.sha256") > "$out/source-after.log"
(cd "$stage/pins" && shasum -c native-binaries.sha256) > "$out/pins-after.log"
