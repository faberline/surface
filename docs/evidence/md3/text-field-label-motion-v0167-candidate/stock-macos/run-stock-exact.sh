#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
stage=/tmp/surface-md3-text-field-label-motion-v0167.Y7f9HqIi
candidate=$stage/exact-candidate
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271
source_tree=c218faaf7527ec2872fb1ec1097c87667f3bf1f2
patch_sha=d3b38aac44fad4cb071fcf2734e1edc935f741af2e6e5d710da92b2b8b40c6ba
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
        test "$(shasum -a 256 "$stage/text-field-label-motion-v0167.patch" | cut -d' ' -f1)" = "$patch_sha"
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
            '.release.version == "0.1.67" and .release.source_sha256 == $source and
             .release.source_tree == $tree and .release.last_material_step == 350 and
             .cases.material.steps == 349 and (.multiline_text_field.pngs | length) == 12 and
             (.textbox_values.inactive_ax | length) == 6 and
             .textbox_values.inactive_ax.single_line.multiline == false and
             .focused.checks == 1495 and .focused.steps == 26 and
             (.focused.captures | length) == 15 and (.text_field_label_motion.pngs | length) == 15 and
             .injected_chrome_flags == ["--disable-updater-scheduler"]' "$lane/stock-mac-exact-offline.json" >/dev/null
        touch "$lane/stock-mac-v0167-passed"
        ;;
    --recheck)
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
        ;;
    *)
        printf '%s\n' "Use --preflight, --prepare, --run or --recheck." >&2
        exit 2
        ;;
esac
