//! Material gallery journey: choices, slider, buttons, chips, dialogs,
//! menus, Tab cycles and mobile sizing.
//! Port of `scripts/browser-material.mjs` (M1–M23 in issue #62).

use anyhow::Result;
use serde_json::{json, Value};

use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub const STEPS: &[&str] = &[
    "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10", "M11", "M12", "M13", "M14", "M15",
    "M16", "M17", "M18", "M19", "M20", "M21", "M22",
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
    result
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
    tab.viewport(480, 760).await?;
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
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn failure_dump_step_is_not_required() {
        // M23 only runs on failure, so a passing run must not require it.
        assert!(!super::STEPS.contains(&"M23"));
    }
}
