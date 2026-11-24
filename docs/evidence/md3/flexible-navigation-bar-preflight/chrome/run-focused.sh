#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd -P)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
run=$(mktemp -d "$root/run.XXXXXXXX")
mkdir "$run/browser-temp"
export TMPDIR="$run/browser-temp"
unset SURFACE_CHROME_ARGS
printf '%s\n' "$run"
(cd "$root" && shasum -a 256 -c runtime.sha256) > "$run/runtime-before.log"
(cd "$root/focused-source" && shasum -a 256 -c "$root/focused-source.sha256") > "$run/source-before.log"
set +e
cap "$root/pins/surface-e2e" --chrome '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
    --chrome-arg --headless=new --web-root "$root/web" --artifacts "$run" material 2>&1 | tee "$run/runner.log"
codes=("${PIPESTATUS[@]}")
set -e
printf 'runner=%s tee=%s\n' "${codes[@]}" > "$run/exit.txt"
(cd "$root" && shasum -a 256 -c runtime.sha256) > "$run/runtime-after.log"
(cd "$root/focused-source" && shasum -a 256 -c "$root/focused-source.sha256") > "$run/source-after.log"
if (( codes[1] != 0 )); then exit "${codes[1]}"; fi
exit "${codes[0]}"
