#!/bin/sh
set -eu
test "$#" -eq 2
jq -e --arg source "$(jq -er .source_sha256 "$1")" --arg tree "$(jq -er .source_tree "$1")" \
    --argjson delta "$(jq -ec .exact_vs_scoped_source_paths "$1")" \
    --arg scope "$(jq -er .focused_receipt_sha256 "$1")" \
    '.release.version == "0.1.72" and .release.source_sha256 == $source and
     .release.source_tree == $tree and .release.last_material_step == 362 and
     .cases.material.steps == 361 and .cases.ime.steps == 20 and
     (.multiline_text_field.pngs | length) == 12 and
     (.textbox_values.inactive_ax | length) == 6 and
     .textbox_values.inactive_ax.single_line.multiline == false and
     (.text_field_label_motion.pngs | length) == 34 and (.text_field_placeholder.pngs | length) == 58 and
     (.text_field_boundary_motion.pngs | length) == 33 and .text_field_boundary_motion.decoded_probes == 2478 and
     .text_field_boundary_motion.coverage_quarters_at_14_and_44_ms == {"filled":[10,16],"outlined":[11,20]} and
     .text_field_label_motion.outlined.early_shallow.coverage_quarters == {"cut":[0,0,1],"uncut":[4,4,3]} and
     (.text_field_disabled_contrast.png_sha256 | length) == 15 and
     (.text_field_disabled_contrast.ink_pixels | keys) == ["adorned","counter","filled","multiline","outlined"] and
     .text_field_disabled_contrast.browser == .browser and .text_field_disabled_contrast.adapter == .adapter and
     (.text_field_system_pair.png_sha256 | length) == 14 and
     .text_field_system_pair.browser == .browser and .text_field_system_pair.adapter == .adapter and
     .text_field_system_pair.real_source == "CDP emulated system CSS with no palette override" and
     (.text_field_system_pair.distinct_source | startswith("synthetic unequal Field pair scoped to Surface optional probe; ")) and
     .focused.checks == 1993 and .focused.steps == 13 and .focused.headless_samples == 233 and
     .focused.focused_receipt_sha256 == $scope and
     .focused.step_ids == ["M241","M242","M243","M244","M245","M246","M247","M357","M358","M359","M360","M361","M362"] and
     .focused.visible_windows == 0 and .focused.remaining_owned_processes == [] and
     (.focused.png_sha256 | length) == 14 and (.focused.retained_disabled_contrast.png_sha256 | length) == 15 and
     .focused.exact_vs_scoped_source_paths == $delta and .focused.canonical_gallery_match == true and
     .guard_and_window_checks == "passed" and .injected_chrome_flags == ["--disable-updater-scheduler"]' \
    "$2" >/dev/null
