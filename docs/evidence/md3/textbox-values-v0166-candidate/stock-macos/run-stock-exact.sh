#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
stage=/tmp/surface-md3-textbox-values-v0166.l4950l9e
candidate=$stage/exact-candidate
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=d4b0c5b7a1ae5daf1f7a9084e765b30b24f8cae6ecbda881efb9468daf730783
source_tree=3de0c143c43d35911593cfc95d3ea5931016c308
patch_sha=a5a09a61580a7ae186d0fc95477d40582bba8799b21773e387bc21c4e38760e1
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
test "$#" -eq 2
harness_sha=$1
test "$(shasum -a 256 "$lane/harness.sha256" | cut -d' ' -f1)" = "$harness_sha"
(cd "$lane" && shasum -a 256 --status -c harness.sha256)
case "$2" in
    --preflight)
        "$py" -E -B "$lane/check-controls.py"
        ;;
    --prepare)
        test -f "$candidate/native-final-built"
        test -f "$candidate/stock-canonical-built"
        test "$(cat "$stage/source-tree")" = "$source_tree"
        test "$(shasum -a 256 "$stage/textbox-values-v0166.patch" | cut -d' ' -f1)" = "$patch_sha"
        "$py" -E -B "$lane/prepare-pins.py" \
            --source-manifest "$candidate/source.sha256" --source-sha256 "$source_sha" \
            --native-source "$candidate/source" --web-source "$candidate/source" \
            --native-binaries "$candidate/binaries" --native-ready "$candidate/native-final-built" \
            --web-target "$candidate/web-canonical" --web-ready "$candidate/stock-canonical-built" \
            --wide-canvas-height 800 --rtl-canvas-height 1020
        runner=$(cat "$lane/approved-runner.sha256")
        "$py" -E -B "$lane/observe-multiline.py" --verify-inputs-only \
            --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner" > "$lane/inputs-rechecked.json"
        cmp "$lane/inputs-before.json" "$lane/inputs-rechecked.json"
        ;;
    --run)
        test -s "$lane/inputs-rechecked.json"
        test ! -e "$lane/run"
        runner=$(cat "$lane/approved-runner.sha256")
        cap "$py" -E -B "$lane/observe-multiline.py" --out "$lane/run" \
            --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner" --timeout 1200
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run" > "$lane/stock-mac-exact-offline.json"
        jq -e --arg source "$source_sha" --arg tree "$source_tree" \
            '.release.version == "0.1.66" and .release.source_sha256 == $source and
             .release.source_tree == $tree and .release.last_material_step == 346 and
             .cases.material.steps == 345 and (.multiline_text_field.pngs | length) == 12 and
             (.textbox_values.inactive_ax | length) == 6 and
             .textbox_values.inactive_ax.single_line.multiline == false and
             .focused.checks == 308 and .focused.canonical_gallery_match == true' "$lane/stock-mac-exact-offline.json" >/dev/null
        touch "$lane/stock-mac-v0166-passed"
        ;;
    --recheck)
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
        ;;
    *)
        printf '%s\n' "Use --preflight, --prepare, --run or --recheck." >&2
        exit 2
        ;;
esac
