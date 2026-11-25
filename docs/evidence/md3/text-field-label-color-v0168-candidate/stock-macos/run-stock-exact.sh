#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
stage=/tmp/surface-md3-text-field-label-color-v0168.XNrxeeOp
candidate=$stage/exact-candidate
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=17aeee497edef781a3da477d8fdb7820c0eaa3494a4b840fcf41133925949df6
source_tree=0db939d3f663fdb5b9fa3dd334382937693b409e
patch_sha=05814fa4065bf7b65ce1b85023ae79623fafc0abb4870555368b71153c3463b9
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
        test "$(shasum -a 256 "$stage/text-field-label-color-v0168.patch" | cut -d' ' -f1)" = "$patch_sha"
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
            '.release.version == "0.1.68" and .release.source_sha256 == $source and
             .release.source_tree == $tree and .release.last_material_step == 350 and
             .cases.material.steps == 349 and (.multiline_text_field.pngs | length) == 12 and
             (.textbox_values.inactive_ax | length) == 6 and
             .textbox_values.inactive_ax.single_line.multiline == false and
             .focused.checks == 1798 and .focused.steps == 26 and
             (.focused.captures | length) == 34 and (.text_field_label_motion.pngs | length) == 34 and
             .focused.exact_vs_scoped_source_paths == ["docs/material-coverage.json"] and
             (.text_field_label_motion.label_color.outlined.frames | length) == 8 and
             (.text_field_label_motion.label_color.filled.frames | length) == 8 and
             .injected_chrome_flags == ["--disable-updater-scheduler"]' "$lane/stock-mac-exact-offline.json" >/dev/null
        touch "$lane/stock-mac-v0168-passed"
        ;;
    --recheck)
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
        ;;
    *)
        printf '%s\n' "Use --preflight, --prepare, --run or --recheck." >&2
        exit 2
        ;;
esac
