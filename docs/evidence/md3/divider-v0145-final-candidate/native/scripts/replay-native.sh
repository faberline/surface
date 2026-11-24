#!/bin/sh
set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
python=${SURFACE_QA_PYTHON:-python3}
export PYTHONDONTWRITEBYTECODE=1

"$python" -B "$root/scripts/pin-stock.py" "$@" --pin > "$root/pin-before.json"
for repeat in 1 2; do
    "$python" -B "$root/native-scripts/replay-divider.py" \
        --gallery "$root/pins/surface-material-gallery" --cli "$root/pins/surface-devtools" \
        --out "$root/replay-$repeat" > "$root/replay-$repeat.log" 2>&1
done
diff -qr "$root/replay-1" "$root/replay-2" > "$root/repeat.log"
"$python" -B "$root/scripts/pin-stock.py" "$@" > "$root/pin-after.json"
cmp "$root/pin-before.json" "$root/pin-after.json"
"$python" -B "$root/scripts/verify-native.py" --assert-live-clean > "$root/verified-native.json"
