#!/bin/sh
# Run from the diagnostic scratch directory. Stock pins come from the combined source.
set -eu

stock=$1
out=$2
python=${PYTHON_BIN:-python3}
mkdir "$out"
(cd source && shasum -c ../source.sha256) > "$out/source-before.log"
(cd pins && shasum -c native-binaries.sha256) > "$out/pins-before.log"
(cd "$stock" && shasum -c native-binaries.sha256) > "$out/stock-pins-before.log"

if [ -n "${STOCK_FRAMES:-}" ]; then
    stock_frames=$STOCK_FRAMES
else
    stock_frames="$out/stock"
    "$python" native-scripts/replay-stock-settled.py --gallery "$stock/surface-material-gallery" \
        --cli "$stock/surface-devtools" --out "$stock_frames" > "$out/stock-replay.log" 2>&1
fi
for run in 1 2; do
    "$python" native-scripts/replay-search-motion.py --gallery pins/surface-material-gallery \
        --cli pins/surface-devtools --out "$out/motion-$run" --stock "$stock_frames" > "$out/motion-$run.log" 2>&1
done
diff -qr "$out/motion-1" "$out/motion-2" > "$out/repeat.log"
(cd "$out" && shasum -a 256 motion-1/*.png motion-1/native-search-motion-report.json > repeat.sha256)
(cd source && shasum -c ../source.sha256) > "$out/source-after.log"
(cd pins && shasum -c native-binaries.sha256) > "$out/pins-after.log"
(cd "$stock" && shasum -c native-binaries.sha256) > "$out/stock-pins-after.log"
