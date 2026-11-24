#!/bin/sh
# Run from the diagnostic root while a matched stock build is still underway.
set -eu

reference=$1
out=$2
python=${PYTHON_BIN:-python3}
mkdir "$out"
(cd source && shasum -c ../source.sha256) > "$out/source-before.log"
(cd pins && shasum -c native-binaries.sha256) > "$out/pins-before.log"
for run in 1 2; do
    "$python" native-scripts/replay-search-motion.py --gallery pins/surface-material-gallery \
        --cli pins/surface-devtools --out "$out/motion-$run" --stock "$reference" > "$out/motion-$run.log" 2>&1
done
diff -qr "$out/motion-1" "$out/motion-2" > "$out/repeat.log"
(cd "$out" && shasum -a 256 motion-1/*.png motion-1/native-search-motion-report.json > repeat.sha256)
(cd source && shasum -c ../source.sha256) > "$out/source-after.log"
(cd pins && shasum -c native-binaries.sha256) > "$out/pins-after.log"
