#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")" && pwd -P)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
pin=$1
case "$pin" in "$root"/focused.*) ;; *) exit 2;; esac
attempt=$(mktemp -d "$pin/run.XXXXXXXX")
mkdir "$attempt/browser-temp"
export TMPDIR="$attempt/browser-temp"
unset SURFACE_CHROME_ARGS
printf '%s\n' "$attempt"
(cd "$root" && shasum -a 256 -c "$pin/runtime.sha256") > "$attempt/runtime-before.log"
(cd "$root/focused-source" && shasum -a 256 -c "$pin/runner-source.sha256") > "$attempt/source-before.log"
set +e
cap "$pin/surface-e2e" --chrome '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' \
    --chrome-arg --headless=new --web-root "$root/web-scroll-final" --artifacts "$attempt" material 2>&1 | tee "$attempt/runner.log"
codes=("${PIPESTATUS[@]}")
set -e
printf 'runner=%s tee=%s\n' "${codes[@]}" > "$attempt/exit.txt"
(cd "$root" && shasum -a 256 -c "$pin/runtime.sha256") > "$attempt/runtime-after.log"
(cd "$root/focused-source" && shasum -a 256 -c "$pin/runner-source.sha256") > "$attempt/source-after.log"
if (( codes[1] != 0 )); then exit "${codes[1]}"; fi
exit "${codes[0]}"
