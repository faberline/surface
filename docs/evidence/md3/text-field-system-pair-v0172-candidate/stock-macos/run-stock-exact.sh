#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
test "$#" -ge 2
harness_sha=$1
mode=$2
shift 2
test "$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)" = "$harness_sha"
(cd "$lane" && shasum -a 256 --status -c harness.sha256)

reviewed_config() {
    test -f "$1"
    test "$(shasum -a 256 "$1" | cut -d' ' -f1)" = "$2"
}

case "$mode" in
    --preflight)
        test "$#" -eq 0
        "$py" -E -B "$lane/check-controls.py"
        ;;
    --prepare)
        test "$#" -eq 4
        reviewed_config "$1" "$2"
        candidate=$3
        "$py" -E -B "$lane/prepare-pins.py" \
            --pins "$1" --pins-sha256 "$2" --patch "$4" \
            --source-manifest "$candidate/source.sha256" \
            --native-source "$candidate/source" --web-source "$candidate/source" \
            --native-binaries "$candidate/binaries" --native-ready "$candidate/native-final-built" \
            --web-target "$candidate/web-canonical" --web-ready "$candidate/stock-canonical-built"
        runner=$(cat "$lane/approved-runner.sha256")
        "$py" -E -B "$lane/observe-multiline.py" --verify-inputs-only \
            --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner" > "$lane/inputs-rechecked.json"
        cmp "$lane/inputs-before.json" "$lane/inputs-rechecked.json"
        ;;
    --run)
        test "$#" -eq 2
        reviewed_config "$1" "$2"
        cmp "$1" "$lane/reviewed-pins.json"
        test -s "$lane/inputs-rechecked.json"
        test ! -e "$lane/run"
        test ! -e "$lane/stock-mac-exact-offline.json"
        test ! -e "$lane/stock-mac-v0172-passed"
        runner=$(cat "$lane/approved-runner.sha256")
        cap "$py" -E -B "$lane/observe-multiline.py" --out "$lane/run" \
            --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner" --timeout 1200
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run" > "$lane/stock-mac-exact-offline.json"
        sh "$lane/check-receipt.sh" "$1" "$lane/stock-mac-exact-offline.json"
        touch "$lane/stock-mac-v0172-passed"
        ;;
    --recheck)
        test "$#" -eq 2
        reviewed_config "$1" "$2"
        cmp "$1" "$lane/reviewed-pins.json"
        recheck=$(mktemp "$lane/offline-recheck.XXXXXXXX")
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run" > "$recheck"
        sh "$lane/check-receipt.sh" "$1" "$recheck"
        cmp "$lane/stock-mac-exact-offline.json" "$recheck"
        cat "$recheck"
        ;;
    *)
        printf '%s\n' "Use --preflight, --prepare, --run or --recheck." >&2
        exit 2
        ;;
esac
