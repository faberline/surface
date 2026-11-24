#!/bin/sh
set -eu

batch=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
python=${PYTHON_BIN:-python3}
export PYTHONPATH="$batch/native-scripts" PYTHONDONTWRITEBYTECODE=1
(cd "$batch/pins" && shasum -a 256 -c native-binaries.sha256) > "$batch/pins-before-replays.log"
"$python" "$batch/check-dialog-route-reach.py" \
    --gallery "$batch/pins/surface-material-gallery" --cli "$batch/pins/surface-devtools" \
    --out "$batch/route-reach" > "$batch/route-reach.log" 2>&1
"$python" "$batch/check-native-fab-bounds.py" \
    --gallery "$batch/pins/surface-material-gallery" --cli "$batch/pins/surface-devtools" \
    --out "$batch/fab-bounds" > "$batch/fab-bounds.log" 2>&1
PYTHON_BIN="$python" sh "$batch/replay-native.sh" "$batch/run"
(cd "$batch/pins" && shasum -a 256 -c native-binaries.sha256) > "$batch/pins-after-replays.log"
cmp "$batch/pins-before-replays.log" "$batch/pins-after-replays.log"
