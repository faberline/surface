#!/bin/sh
# Run from this batch with independently pinned Gallery binaries.
set -eu

stage=$1
out=$2
python=${PYTHON_BIN:-python3}
mkdir "$out"
expected_source=46b4b82931a1f84977693a38d6e42f5c591795aa9ee27ad63e280d578a6cd802
test "$(shasum -a 256 "$stage/source.sha256" | cut -d ' ' -f 1)" = "$expected_source"
(cd "$stage/source" && shasum -c ../source.sha256) > "$out/source-before.log"
(cd "$stage/pins" && shasum -c native-binaries.sha256) > "$out/pins-before.log"
for run in 1 2; do
    "$python" native-scripts/replay-dialogs.py --gallery "$stage/pins/surface-material-gallery" \
        --cli "$stage/pins/surface-devtools" --out "$out/replay-$run" > "$out/replay-$run.log" 2>&1
done
diff -qr "$out/replay-1" "$out/replay-2" > "$out/repeat.log"
(cd "$out" && shasum -a 256 replay-1/*.png replay-1/native-dialog-report.json > repeat.sha256)
(cd "$stage/source" && shasum -c ../source.sha256) > "$out/source-after.log"
(cd "$stage/pins" && shasum -c native-binaries.sha256) > "$out/pins-after.log"
