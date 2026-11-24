//! Material gallery journey: choices, slider, buttons, chips, dialogs,
//! menus, Tab cycles and mobile sizing.
//! Port of `scripts/browser-material.mjs` (M1–M23 in issue #62).

#[path = "material_app_bar.rs"]
mod material_app_bar;
#[path = "material_badge.rs"]
mod material_badge;
#[path = "material_bottom_sheet.rs"]
mod material_bottom_sheet;
#[path = "material_button_group.rs"]
mod material_button_group;
#[path = "material_card.rs"]
mod material_card;
#[path = "material_carousel.rs"]
mod material_carousel;
#[path = "material_checkbox.rs"]
mod material_checkbox;
#[path = "material_command_menu.rs"]
mod material_command_menu;
#[path = "material_date_picker.rs"]
mod material_date_picker;
#[path = "material_date_picker_gestures.rs"]
mod material_date_picker_gestures;
#[path = "material_date_range_picker.rs"]
mod material_date_range_picker;
#[path = "material_dialog.rs"]
mod material_dialog;
#[path = "material_divider.rs"]
mod material_divider;
#[path = "material_docked_date_picker.rs"]
mod material_docked_date_picker;
#[path = "material_drawer.rs"]
mod material_drawer;
#[path = "material_extended_fab.rs"]
mod material_extended_fab;
#[path = "material_fab.rs"]
mod material_fab;
#[path = "material_fab_menu.rs"]
mod material_fab_menu;
#[path = "material_flexible_navigation_bar.rs"]
mod material_flexible_navigation_bar;
#[path = "material_list_actions.rs"]
mod material_list_actions;
#[path = "material_list_corners.rs"]
mod material_list_corners;
#[path = "material_list_expansion.rs"]
mod material_list_expansion;
#[path = "material_list_media.rs"]
mod material_list_media;
#[path = "material_list_reorder.rs"]
mod material_list_reorder;
#[path = "material_list_ripple.rs"]
mod material_list_ripple;
#[path = "material_lists.rs"]
mod material_lists;
#[path = "material_lists_multi.rs"]
mod material_lists_multi;
#[path = "material_loading.rs"]
mod material_loading;
#[path = "material_progress.rs"]
mod material_progress;
#[path = "material_rail.rs"]
mod material_rail;
#[path = "material_scroll_choice_fields.rs"]
mod material_scroll_choice_fields;
#[path = "material_scroll_fields.rs"]
mod material_scroll_fields;
#[path = "material_scroll_loop_fields.rs"]
mod material_scroll_loop_fields;
#[path = "material_scroll_modal_fields.rs"]
mod material_scroll_modal_fields;
#[path = "material_scroll_pair_fields.rs"]
mod material_scroll_pair_fields;
#[path = "material_scroll_popup_fields.rs"]
mod material_scroll_popup_fields;
#[path = "material_search_overlay.rs"]
mod material_search_overlay;
#[path = "material_side_sheet.rs"]
mod material_side_sheet;
#[path = "material_snackbar.rs"]
mod material_snackbar;
#[path = "material_split_button.rs"]
mod material_split_button;
#[path = "material_switch.rs"]
mod material_switch;
#[path = "material_time_baseline.rs"]
mod material_time_baseline;
#[path = "material_time_dial.rs"]
mod material_time_dial;
#[path = "material_time_input.rs"]
mod material_time_input;
#[path = "material_time_scroll.rs"]
mod material_time_scroll;
#[path = "material_toolbar.rs"]
mod material_toolbar;
#[path = "material_tooltip.rs"]
mod material_tooltip;
#[path = "material_web_contrast.rs"]
mod material_web_contrast;
#[path = "material_web_contrast_png.rs"]
mod material_web_contrast_png;

use anyhow::Result;
use serde_json::{json, Value};

use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub const STEPS: &[&str] = &[
    "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10", "M11", "M12", "M13", "M14", "M15",
    "M16", "M17", "M18", "M19", "M20", "M21", "M22", "M24", "M25", "M26", "M27", "M28", "M29",
    "M30", "M31", "M32", "M33", "M34", "M35", "M36", "M37", "M38", "M39", "M40", "M41", "M42",
    "M43", "M44", "M45", "M46", "M47", "M48", "M49", "M50", "M51", "M52", "M53", "M54", "M55",
    "M56", "M57", "M58", "M59", "M60", "M61", "M62", "M63", "M64", "M65", "M66", "M67", "M68",
    "M69", "M70", "M71", "M72", "M73", "M74", "M75", "M76", "M77", "M78", "M79", "M80", "M81",
    "M82", "M83", "M84", "M85", "M86", "M87", "M88", "M89", "M90", "M91", "M92", "M93", "M94",
    "M95", "M96", "M97", "M98", "M99", "M100", "M101", "M102", "M103", "M104", "M105", "M106",
    "M107", "M108", "M109", "M110", "M111", "M112", "M113", "M114", "M115", "M116", "M117", "M118",
    "M119", "M120", "M121", "M122", "M123", "M124", "M125", "M126", "M127", "M128", "M129", "M130",
    "M131", "M132", "M133", "M134", "M135", "M136", "M137", "M138", "M139", "M140", "M141", "M142",
    "M143", "M144", "M145", "M146", "M147", "M148", "M149", "M150", "M151", "M152", "M153", "M154",
    "M155", "M156", "M157", "M158", "M159", "M160", "M161", "M162", "M163", "M164", "M165", "M166",
    "M167", "M168", "M169", "M170", "M171", "M172", "M173", "M174", "M175", "M176", "M177", "M178",
    "M179", "M180", "M181", "M182", "M183", "M184", "M185", "M186", "M187", "M188", "M189", "M190",
    "M191", "M192", "M193", "M194", "M195", "M196", "M197", "M198", "M199", "M200", "M201", "M202",
    "M203", "M204", "M205", "M206", "M207", "M208", "M209", "M210", "M211", "M212", "M213", "M214",
    "M215", "M216", "M217", "M218", "M219", "M220", "M221", "M222", "M223", "M224", "M225", "M226",
    "M227", "M228", "M229", "M230", "M231", "M232", "M233", "M234", "M235", "M236", "M237", "M238",
    "M239", "M240", "M241", "M242", "M243", "M244", "M245", "M246", "M247", "M248", "M249", "M250",
    "M251", "M252", "M253", "M254", "M255", "M256", "M257", "M258", "M259", "M260", "M261", "M262",
    "M263", "M264", "M265", "M266", "M267", "M268", "M269", "M270", "M271", "M272", "M273", "M274",
    "M275", "M276", "M277", "M278", "M279", "M280", "M281",
];

const STATE: &str = "JSON.parse(window.gallery.inspect())";

async fn state(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.inspect()").await
}

async fn until(tab: &Tab, condition: &str) -> Result<()> {
    tab.wait_for(&format!("(s => {condition})({STATE})")).await
}

struct Point {
    x: f64,
    y: f64,
    b: Vec<f64>,
    canvas: [f64; 4],
}

async fn point(tab: &Tab, id: &str) -> Result<Point> {
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 4",
        js(id)
    ))
    .await?;
    let b = tab
        .eval_f64s(&format!("window.gallery.control_bounds({})", js(id)))
        .await?;
    let canvas = tab.canvas_box().await?;
    Ok(Point {
        x: canvas[0] + b[0] + b[2] / 2.0,
        y: canvas[1] + b[1] + b[3] / 2.0,
        b,
        canvas,
    })
}

async fn click(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    tab.click(p.x, p.y).await
}

async fn focus(tab: &Tab, id: &str, host: &str) -> Result<()> {
    tab.wait_for(&format!(
        "{STATE}.focused === {} && document.activeElement?.id === {}",
        js(id),
        js(host)
    ))
    .await
}

pub async fn run(env: &Env, chrome: &Chrome) -> Result<()> {
    let (_server, url) = env.serve("material-gallery")?;
    let tab = chrome.page(1100, 1100, 2.0).await?;
    tab.goto(&url).await?;
    let result = journey(env, chrome, &tab).await;
    if result.is_err() {
        step(
            "M23",
            "failure dump: state, active element, DOM export, screenshot",
        );
        let dump = tab
            .eval(
                "(() => ({state: JSON.parse(window.gallery.inspect()),
                  active: document.activeElement?.id,
                  dom: JSON.parse(window.gallery.export_dom())}))()",
            )
            .await
            .unwrap_or_else(|e| json!({"dump_error": e.to_string()}));
        let _ = env.write_json("material-failure.json", &dump);
        let _ = tab.screenshot(&env.artifact("material-failure.png")).await;
    }
    result?;
    material_snackbar::run(env, chrome, &url).await?;
    material_badge::run(env, chrome, &url).await?;
    material_drawer::run(env, chrome, &url).await?;
    material_progress::run(env, chrome, &url).await?;
    material_side_sheet::run(env, chrome, &url).await?;
    material_bottom_sheet::run(env, chrome, &url).await?;
    material_app_bar::run(env, chrome, &url).await?;
    material_toolbar::run(env, chrome, &url).await?;
    material_tooltip::run(env, chrome, &url).await?;
    material_carousel::run(env, chrome, &url).await?;
    material_card::run(env, chrome, &url).await?;
    material_button_group::run(env, chrome, &url).await?;
    material_split_button::run(env, chrome, &url).await?;
    material_extended_fab::run(env, chrome, &url).await?;
    material_fab::run(env, chrome, &url).await?;
    material_fab_menu::run(env, chrome, &url).await?;
    material_date_picker::run(env, chrome, &url).await?;
    material_docked_date_picker::run(env, chrome, &url).await?;
    material_date_range_picker::run(env, chrome, &url).await?;
    material_date_picker_gestures::run(env, chrome, &url).await?;
    material_time_input::run(env, chrome, &url).await?;
    material_time_dial::run(env, chrome, &url).await?;
    material_time_dial::motion::run(env, chrome, &url).await?;
    material_time_scroll::run(env, chrome, &url).await?;
    material_time_baseline::run(env, chrome, &url).await?;
    material_lists::run(env, chrome, &url).await?;
    material_lists_multi::run(env, chrome, &url).await?;
    material_list_actions::run(env, chrome, &url).await?;
    material_list_expansion::run(env, chrome, &url).await?;
    material_list_reorder::run(env, chrome, &url).await?;
    material_list_media::run(env, chrome, &url).await?;
    material_list_corners::run(env, chrome, &url).await?;
    material_list_ripple::run(env, chrome, &url).await?;
    material_loading::run(env, chrome, &url).await?;
    material_scroll_fields::run(env, chrome, &url).await?;
    material_scroll_choice_fields::run(env, chrome, &url).await?;
    material_scroll_loop_fields::run(env, chrome, &url).await?;
    material_scroll_pair_fields::run(env, chrome, &url).await?;
    material_scroll_popup_fields::run(env, chrome, &url).await?;
    material_scroll_modal_fields::run(env, chrome, &url).await?;
    material_web_contrast::run(env, chrome, &url).await?;
    material_search_overlay::run(env, chrome, &url).await?;
    material_dialog::run(env, chrome, &url).await?;
    material_divider::run(env, chrome, &url).await?;
    material_command_menu::run(env, chrome, &url).await?;
    material_flexible_navigation_bar::run(env, chrome, &url).await?;
    material_switch::run(env, chrome, &url).await?;
    material_checkbox::run(env, chrome, &url).await
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    step("M1", "Ready");
    crate::smoke::ready(tab, "gallery").await?;

    step("M2", "radio arrows");
    click(tab, "radio-0").await?;
    tab.press("ArrowDown").await?;
    until(tab, "s.radio === 1").await?;
    tab.press("ArrowDown").await?;
    until(tab, "s.radio === 0").await?;

    step("M3", "segment manual focus, Space selects");
    click(tab, "segment-0").await?;
    tab.press("ArrowRight").await?;
    until(tab, "s.focused === 'segment-1'").await?;
    check_eq!(
        state(tab).await?["segment"],
        json!(0),
        "manual group focus does not select"
    );
    tab.press("Space").await?;
    until(tab, "s.segment === 1").await?;

    step("M4", "check and switch");
    click(tab, "check").await?;
    until(tab, "s.check === 'Checked'").await?;
    click(tab, "enabled").await?;
    until(tab, "s.enabled").await?;

    step("M5", "slider pointer and keys");
    let volume = point(tab, "volume").await?;
    tab.click(
        volume.canvas[0] + volume.b[0] + 12.0 + 0.85 * (volume.b[2] - 24.0),
        volume.y,
    )
    .await?;
    until(tab, "s.volume === 85").await?;
    tab.press("ArrowLeft").await?;
    until(tab, "s.volume === 80").await?;
    tab.press("Home").await?;
    until(tab, "s.volume === 0").await?;
    tab.press("End").await?;
    until(tab, "s.volume === 100").await?;

    step("M6", "light screenshot, theme → dark, dark screenshot");
    tab.screenshot(&env.artifact("material-selection-light.png"))
        .await?;
    click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    tab.screenshot(&env.artifact("material-selection-dark.png"))
        .await?;

    step("M7", "section 1");
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;

    step("M8", "button updates on release");
    let filled = point(tab, "filled").await?;
    tab.mouse_move(filled.x, filled.y).await?;
    tab.mouse_down().await?;
    check_eq!(
        state(tab).await?["clicks"],
        json!(0),
        "press updates on release"
    );
    tab.mouse_up().await?;
    until(tab, "s.clicks === 1").await?;

    step("M9", "disabled action does nothing");
    click(tab, "disabled-action").await?;
    check_eq!(
        state(tab).await?["clicks"],
        json!(1),
        "disabled action clicks"
    );

    step("M10", "filter chip on; input chip removed; screenshot");
    click(tab, "filter").await?;
    until(tab, "s.filter").await?;
    click(tab, "input-chip-remove").await?;
    until(tab, "!s.input_chip").await?;
    tab.screenshot(&env.artifact("material-actions.png"))
        .await?;

    step("M11", "section 2, open dialog, textarea focused");
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    click(tab, "open-dialog").await?;
    until(tab, "s.dialog").await?;
    tab.wait_for("document.activeElement?.id === 'surface-text-input'")
        .await?;

    step("M12", "insert draft; saved name unchanged");
    tab.shortcut("A").await?;
    tab.insert_text("中文框架").await?;
    until(tab, "s.draft === '中文框架'").await?;
    check_eq!(
        state(tab).await?["name"],
        json!("Surface"),
        "dialog draft stays separate"
    );

    step("M13", "Escape closes dialog and restores focus");
    tab.press("Escape").await?;
    until(tab, "!s.dialog && s.focused === 'open-dialog'").await?;
    check_eq!(
        state(tab).await?["name"],
        json!("Surface"),
        "name after Escape"
    );

    step("M14", "outside click dismisses without activating the tab");
    click(tab, "open-dialog").await?;
    until(tab, "s.dialog").await?;
    point(tab, "rename-dialog").await?;
    click(tab, "section-1").await?;
    until(tab, "!s.dialog").await?;
    check_eq!(
        state(tab).await?["section"],
        json!(2),
        "outside dismiss must not activate a background tab"
    );

    step("M15", "Tab through dialog with DOM host checks");
    click(tab, "open-dialog").await?;
    tab.wait_for("document.activeElement?.id === 'surface-text-input'")
        .await?;
    tab.shortcut("A").await?;
    tab.insert_text("中文框架").await?;
    until(tab, "s.draft === '中文框架'").await?;
    tab.press("Tab").await?;
    until(tab, "s.focused === 'dialog-cancel'").await?;
    tab.wait_for("document.activeElement?.id === 'surface'")
        .await?;
    tab.press("Tab").await?;
    until(tab, "s.focused === 'dialog-save'").await?;
    tab.press("Tab").await?;
    until(tab, "s.focused === 'dialog-name'").await?;
    tab.wait_for("document.activeElement?.id === 'surface-text-input'")
        .await?;

    step("M16", "three Tab / Shift+Tab cycles");
    for _ in 0..3 {
        for (key, id, host) in [
            ("Tab", "dialog-cancel", "surface"),
            ("Tab", "dialog-save", "surface"),
            ("Tab", "dialog-name", "surface-text-input"),
            ("Shift+Tab", "dialog-save", "surface"),
            ("Shift+Tab", "dialog-cancel", "surface"),
            ("Shift+Tab", "dialog-name", "surface-text-input"),
        ] {
            tab.press(key).await?;
            focus(tab, id, host).await?;
        }
    }

    step("M17", "dialog screenshot and DOM export");
    tab.screenshot(&env.artifact("material-dialog.png")).await?;
    let export = tab.eval_json("window.gallery.export_dom()").await?;
    check_eq!(
        export["text_inputs"]["dialog-name"]["value"],
        json!("中文框架"),
        "dialog draft value"
    );
    check_eq!(
        export["interaction"]["focused"],
        json!("dialog-name"),
        "focused"
    );
    check!(
        export["dom"]["nodes"].as_array().is_some_and(|n| n
            .iter()
            .any(|n| n["role"] == "Dialog" && n["stable_id"] == "rename-dialog")),
        "no Dialog node rename-dialog"
    );
    check_eq!(
        export["layout"]["nodes"].as_array().map(Vec::len),
        export["dom"]["nodes"].as_array().map(Vec::len),
        "layout count equals node count"
    );
    env.write_json("material-dom-export.json", &export)?;

    step("M18", "save the dialog");
    click(tab, "dialog-save").await?;
    until(tab, "s.name === '中文框架' && !s.dialog").await?;

    step("M19", "menu focus, arrows and Escape");
    click(tab, "open-menu").await?;
    until(tab, "s.focused === 'menu-rename'").await?;
    tab.press("ArrowDown").await?;
    until(tab, "s.focused === 'menu-close'").await?;
    tab.press("Escape").await?;
    until(tab, "!s.menu && s.focused === 'open-menu'").await?;

    step("M20", "480×760 dialog fits; mobile screenshot");
    material_time_input::resize(tab, 480, 716).await?;
    click(tab, "open-dialog").await?;
    until(tab, "s.dialog").await?;
    let dialog = point(tab, "rename-dialog").await?;
    check!(
        dialog.b[0] >= 0.0 && dialog.b[0] + dialog.b[2] <= 480.0,
        "dialog does not fit 480 px: {:?}",
        dialog.b
    );
    tab.screenshot(&env.artifact("material-dialog-mobile.png"))
        .await?;

    step("M21", "canvas corner click dismisses; section stays 2");
    let canvas = tab.canvas_box().await?;
    tab.click(canvas[0] + 1.0, canvas[1] + 1.0).await?;
    until(tab, "!s.dialog").await?;
    check_eq!(
        state(tab).await?["section"],
        json!(2),
        "outside dismiss must not activate the background tab"
    );

    step("M22", "no page errors; evidence JSON");
    let errors = tab.errors();
    check!(errors.is_empty(), "page errors:\n{}", errors.join("\n"));
    let evidence = json!({"browser": chrome.version, "driver": "jet-bridge CDP", "state": state(tab).await?, "errors": errors});
    env.write_json("browser-material.json", &evidence)?;
    println!("  evidence: {}", evidence["state"]);
    material_rail::journey(env, tab).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn failure_dump_step_is_not_required() {
        // M23 only runs on failure, so a passing run must not require it.
        assert!(!super::STEPS.contains(&"M23"));
    }
}
