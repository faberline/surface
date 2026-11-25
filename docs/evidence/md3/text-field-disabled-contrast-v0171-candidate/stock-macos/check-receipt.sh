#!/bin/sh
set -eu
test "$#" -eq 2
jq -e --arg source "$(jq -er .source_sha256 "$1")" --arg tree "$(jq -er .source_tree "$1")" \
    --argjson delta "$(jq -ec .exact_vs_scoped_source_paths "$1")" \
    '.release.version == "0.1.71" and .release.source_sha256 == $source and
     .release.source_tree == $tree and .release.last_material_step == 359 and
     .cases.material.steps == 358 and .cases.ime.steps == 20 and
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
     .focused.checks == 1470 and .focused.steps == 10 and .focused.headless_samples == 162 and
     .focused.visible_windows == 0 and .focused.remaining_owned_processes == [] and
     (.focused.png_sha256 | length) == 15 and
     .focused.exact_vs_scoped_source_paths == $delta and .focused.canonical_gallery_match == true and
     .guard_and_window_checks == "passed" and .injected_chrome_flags == ["--disable-updater-scheduler"]' \
    "$2" >/dev/null
