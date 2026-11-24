#!/bin/bash
set -euo pipefail

scratch=$(cd "$(dirname "$0")" && pwd)
stage=$1
run=$stage/${2:-run}
case "$stage" in "$scratch"/stock.*) ;; *) exit 2 ;; esac
case "$run" in "$stage"/run*) ;; *) exit 2 ;; esac
test -s "$stage/sealed-inputs.sha256"
mkdir "$run"
mkdir "$run/browser-temp"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PYTHONDONTWRITEBYTECODE=1 TMPDIR="$run/browser-temp"
unset SURFACE_CHROME_ARGS

bundle="$stage/bundle"
check_stage() {
    local phase=$1
    (cd "$stage" && shasum -a 256 -c "$stage/sealed-inputs.sha256") > "$run/stage-$phase.log"
    (cd "$scratch" && shasum -a 256 -c "$stage/harness.sha256") >> "$run/stage-$phase.log"
    python3 -B "$scratch/stock-inputs.py" verify "$stage" "$run/$phase-run-sha256.txt"
    (cd "$bundle" && shasum -a 256 -c "$stage/browser-binary-sha256.txt") > "$run/bundle-$phase.log"
}
check_stage before

set +e
cap "$bundle/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --chrome-arg --headless=new \
    --web-root "$bundle/web/release" --bridge-web "$bundle/web/devtools/material-gallery" \
    --jet-bridge "$bundle/debug/jet-bridge" --devtools-bin "$bundle/debug/surface-devtools" \
    --native-bin "$bundle/debug/surface-material-gallery" --artifacts "$run" \
    smoke ime material bridge-web devtools-headless 2>&1 | tee "$run/runner.log"
execution=("${PIPESTATUS[@]}")
set -e
python3 -B "$scratch/verify-stock.py" attempt "$stage" "$run" "${execution[@]}"
(
    cd "$run"
    for file in attempt.json runner.log ./browser-material*.json ./browser-ime*.json ./material-web-contrast-*.json ./material-web-contrast-*.png ./*failure*; do
        if [[ -f "$file" ]]; then
            shasum -a 256 "$file"
        fi
    done
) > "$run/attempt-evidence.sha256"
check_stage after
if (( execution[1] != 0 )); then
    exit "${execution[1]}"
fi
if (( execution[0] != 0 )); then
    exit "${execution[0]}"
fi
python3 -B "$scratch/verify-stock.py" passing "$stage" "$run"
(
    cd "$run"
    shasum -a 256 attempt.json runner.log headless-results.txt verified-browser.json \
        bridge-web-result.json devtools-headless-result.json browser-smoke.json browser-ime*.json browser-material-web-contrast*.json \
        material-web-contrast-*.json material-web-contrast-*.png browser-material-scroll-*.json \
        material-scroll-modal-fields-*.png material-scroll-popup-fields-compact-dark-rtl.png \
        material-scroll-pair-fields-short-unit-focused.png stage-{before,after}.log bundle-{before,after}.log \
        before-run-sha256.txt after-run-sha256.txt
) > "$run/evidence-sha256.txt"
