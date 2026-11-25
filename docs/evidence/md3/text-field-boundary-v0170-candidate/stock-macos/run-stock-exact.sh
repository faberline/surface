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
        test ! -e "$lane/stock-mac-v0170-passed"
        runner=$(cat "$lane/approved-runner.sha256")
        cap "$py" -E -B "$lane/observe-multiline.py" --out "$lane/run" \
            --runner "$lane/bundle/debug/surface-e2e" --runner-sha256 "$runner" --timeout 1200
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run" > "$lane/stock-mac-exact-offline.json"
        jq -e --arg source "$(jq -er .source_sha256 "$1")" --arg tree "$(jq -er .source_tree "$1")" \
            --argjson delta "$(jq -ec .exact_vs_scoped_source_paths "$1")" \
            '.release.version == "0.1.70" and .release.source_sha256 == $source and
             .release.source_tree == $tree and .release.last_material_step == 356 and
             .cases.material.steps == 355 and .cases.ime.steps == 20 and
             (.multiline_text_field.pngs | length) == 12 and
             (.textbox_values.inactive_ax | length) == 6 and
             .textbox_values.inactive_ax.single_line.multiline == false and
             (.text_field_label_motion.pngs | length) == 34 and (.text_field_placeholder.pngs | length) == 58 and
             (.text_field_boundary_motion.pngs | length) == 33 and .text_field_boundary_motion.decoded_probes == 2478 and
             .text_field_boundary_motion.coverage_quarters_at_14_and_44_ms == {"filled":[10,16],"outlined":[11,20]} and
             .text_field_label_motion.outlined.early_shallow.coverage_quarters == {"cut":[0,0,1],"uncut":[4,4,3]} and
             .focused.checks == 5228 and .focused.steps == 32 and .focused.headless_samples == 547 and
             .focused.visible_windows == 0 and .focused.remaining_owned_processes == [] and
             (.focused.inherited_label_motion.captures | length) == 34 and
             (.focused.inherited_placeholder.pngs | length) == 58 and (.focused.pngs | length) == 33 and
             .focused.exact_vs_scoped_source_paths == $delta and .focused.canonical_gallery_match == true and
             .guard_and_window_checks == "passed" and .injected_chrome_flags == ["--disable-updater-scheduler"]' \
             "$lane/stock-mac-exact-offline.json" >/dev/null
        touch "$lane/stock-mac-v0170-passed"
        ;;
    --recheck)
        test "$#" -eq 2
        reviewed_config "$1" "$2"
        cmp "$1" "$lane/reviewed-pins.json"
        "$py" -E -B "$lane/verify-stock-receipt.py" "$lane/run"
        ;;
    *)
        printf '%s\n' "Use --preflight, --prepare, --run or --recheck." >&2
        exit 2
        ;;
esac
