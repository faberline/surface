use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const PICKER: &str = "date-picker-demo-modal";
const OPEN: &str = "date-picker-demo-open";
const OPEN_INPUT: &str = "date-picker-demo-input-open";
const INPUT: &str = "date-picker-demo-modal-input";
const FORMAT: &str = "年／月／日（YYYY/MM/DD）";

fn part(name: &str) -> String {
    format!("{PICKER}-{name}")
}

fn day(date: &str) -> String {
    part(&format!("day-{date}"))
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{ const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
           return v?.width === {width} && v?.height === {height}; }})()"
    ))
    .await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 1100, 880).await?;
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    click(tab, "date-picker-demo-show").await?;
    until(tab, "s.date_picker.visible").await
}

async fn picker_state(tab: &Tab, predicate: &str) -> Result<()> {
    until(tab, &format!("(d => {predicate})(s.date_picker)")).await?;
    // The app can expose a new draft before the browser publishes its
    // rebuilt accessibility tree. Wait before sending the next key.
    tab.settle().await
}

async fn runtime_focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    runtime_focused(tab, id).await?;
    let owner = if id == INPUT {
        "document.getElementById('surface-text-input')".to_owned()
    } else {
        projected(id)
    };
    tab.wait_for(&format!("document.activeElement === {owner}"))
        .await?;
    tab.settle().await
}

async fn open_with_key(tab: &Tab, opener: &str) -> Result<()> {
    let target = projected(opener);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    focused(tab, opener).await?;
    tab.press("Enter").await
}

async fn capture(env: &Env, tab: &Tab, image: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-date-picker-{image}.png")))
        .await
}

async fn ax(tab: &Tab, role: &str, label: &str) -> Result<Value> {
    let document = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    document["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
                node["ignored"] == false
                    && node["role"]["value"] == role
                    && node["name"]["value"]
                        .as_str()
                        .is_some_and(|name| name.contains(label))
            })
        })
        .cloned()
        .with_context(|| format!("Chrome AX tree lacks {role} named {label}"))
}

fn ax_property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    let match_ = node["properties"]
        .as_array()?
        .iter()
        .find(|property| property["name"] == name)?;
    Some(&match_["value"]["value"])
}

async fn calendar(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M112",
        "the calendar uses a real date grid with distinct focus, today and selection",
    );
    open_with_key(tab, OPEN).await?;
    focused(tab, &day("2024-02-28")).await?;
    let panel = point(tab, PICKER).await?;
    check!(
        (panel.b[2] - 360.).abs() <= 0.75 && (panel.b[3] - 524.).abs() <= 0.75,
        "desktop calendar uses the modal token bounds: {:?}",
        panel.b
    );
    let selected = ax(tab, "gridcell", "2024年2月28日").await?;
    let grid = ax(tab, "grid", "2024年2月").await?;
    let header = ax(tab, "columnheader", "星期日").await?;
    let unavailable = ax(tab, "gridcell", "2024年2月27日").await?;
    check_eq!(
        ax_property(&selected, "selected"),
        Some(&json!(true)),
        "day selected"
    );
    check_eq!(
        ax_property(&unavailable, "disabled"),
        Some(&json!(true)),
        "unavailable day"
    );
    let today = projected(&day("2024-02-29"));
    let today_state = tab.eval(&format!(
        "({{current:{today}?.getAttribute('aria-current'), selected:{today}?.getAttribute('aria-selected')}})"
    )).await?;
    check_eq!(
        today_state,
        json!({"current":"date","selected":"false"}),
        "today remains a separate state"
    );
    capture(env, tab, "calendar-light").await?;

    tab.press("ArrowRight").await?;
    focused(tab, &day("2024-02-29")).await?;
    picker_state(tab, "d.session.draft === '2024-02-28'").await?;
    tab.press("Enter").await?;
    picker_state(tab, "d.session.draft === '2024-02-29' && d.commits === 0").await?;
    tab.press("ArrowRight").await?;
    focused(tab, &day("2024-03-02")).await?;
    tab.press("Shift+PageDown").await?;
    focused(tab, &day("2025-03-02")).await?;
    tab.press("Escape").await?;
    focused(tab, OPEN).await?;
    picker_state(tab, "d.session === null && d.committed === '2024-02-28'").await?;
    Ok(
        json!({"bounds":panel.b,"grid":grid["role"],"weekday":header["name"],
        "today":today_state,"selected":selected["properties"],"unavailable":unavailable["properties"]}),
    )
}

async fn year_panel(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M113",
        "the modal can browse years and outside dismissal restores the opener",
    );
    click(tab, "date-picker-demo-theme").await?;
    click(tab, OPEN).await?;
    runtime_focused(tab, &day("2024-02-28")).await?;
    capture(env, tab, "calendar-dark").await?;
    click(tab, &part("month")).await?;
    picker_state(tab, "d.session.mode === 'Year'").await?;
    let year = point(tab, &part("years")).await?;
    check!(
        year.b[2] > 0. && year.b[3] > 0.,
        "year view has room to scroll"
    );
    capture(env, tab, "years-dark").await?;
    click(tab, &part("year-prev")).await?;
    picker_state(
        tab,
        "d.session.mode === 'Year' && d.session.month[0] === 1900",
    )
    .await?;
    click(tab, &part("year-1900")).await?;
    picker_state(
        tab,
        "d.session.mode === 'Calendar' && d.session.month[0] === 1900 && d.session.draft === '2024-02-28'",
    )
    .await?;
    tab.click(year.canvas[0] + 8., year.canvas[1] + 8.).await?;
    runtime_focused(tab, OPEN).await?;
    picker_state(tab, "d.session === null && d.commits === 0").await?;
    Ok(json!({"bounds":year.b, "restored":true}))
}

async fn replace(tab: &Tab, value: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(value).await?;
    picker_state(tab, &format!("d.session.raw === {}", js(value))).await?;
    Ok(())
}

async fn input_description(tab: &Tab, target: &str, message: &str) -> Result<Value> {
    let relation = format!(
        "(() => {{
           const textbox = document.getElementById('surface-text-input');
           const ref = textbox?.getAttribute('aria-describedby');
           const hint = ref && document.getElementById(ref);
           return hint === {};
        }})()",
        projected(&part(target))
    );
    tab.wait_for(&relation).await?;
    check_eq!(
        tab.eval(&relation).await?,
        json!(true),
        "active textarea describes its visible static help"
    );
    let actual = ax(tab, "textbox", "日期").await?;
    let spoken = actual["description"]["value"].as_str().unwrap_or_default();
    check!(
        spoken.contains(message),
        "Chrome resolves help containing {message}: {spoken}"
    );
    Ok(actual["description"].clone())
}

async fn input(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M114",
        "input validates on Enter and blur, and IME composition holds the first Enter",
    );
    open_with_key(tab, OPEN_INPUT).await?;
    focused(tab, INPUT).await?;
    let panel = point(tab, PICKER).await?;
    check!(
        (panel.b[2] - 328.).abs() <= 0.75 && (panel.b[3] - 512.).abs() <= 0.75,
        "desktop input uses the modal input token bounds: {:?}",
        panel.b
    );
    let hint = input_description(tab, "hint", FORMAT).await?;
    replace(tab, "2024-2-9").await?;
    tab.press("Shift+Tab").await?;
    picker_state(tab, "d.session.raw === '2024/02/09' && d.commits === 0").await?;
    tab.press("Tab").await?;
    focused(tab, INPUT).await?;

    replace(tab, "2024/02/30").await?;
    tab.press("Enter").await?;
    picker_state(tab, "d.session.error === 'InvalidDate' && d.commits === 0").await?;
    let error = input_description(tab, "error", "這個日期不存在").await?;
    capture(env, tab, "input-invalid").await?;

    replace(tab, "2024/02/").await?;
    tab.ime_compose("２９", 2, 2).await?;
    tab.wait_for("document.getElementById('surface-text-input').value.includes('２９')")
        .await?;
    tab.press("Enter").await?;
    picker_state(tab, "d.session !== null && d.commits === 0").await?;
    if state(tab).await?["date_picker"]["session"]["draft"] != "2024-02-29" {
        tab.insert_text("２９").await?;
    }
    picker_state(tab, "d.session.draft === '2024-02-29' && d.commits === 0").await?;
    tab.press("Enter").await?;
    focused(tab, OPEN_INPUT).await?;
    picker_state(
        tab,
        "d.session === null && d.committed === '2024-02-29' && d.commits === 1",
    )
    .await?;
    Ok(json!({"hint":hint,"error":error,"composition_enter_commits":false,"explicit_submit":true}))
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M115",
        "320 dp dark RTL at 200 percent text keeps date targets and the mode switch reachable",
    );
    for control in ["theme", "rtl", "large-text"] {
        click(tab, &format!("date-picker-demo-{control}")).await?;
    }
    resize(tab, 320, 520).await?;
    open_with_key(tab, OPEN).await?;
    focused(tab, &day("2024-02-28")).await?;
    let panel = point(tab, PICKER).await?;
    let cell = point(tab, &day("2024-02-28")).await?;
    let confirm = point(tab, &part("confirm")).await?;
    check!(
        (panel.b[2] - 320.).abs() <= 0.75 && (panel.b[3] - 520.).abs() <= 0.75,
        "compact picker fills the viewport: {:?}",
        panel.b
    );
    check!(
        cell.b[2] >= 47.5 && cell.b[3] >= 47.5,
        "day target keeps 48 dp"
    );
    check!(
        confirm.b[2] >= 47.5 && confirm.b[3] >= 47.5,
        "Save keeps 48 dp"
    );
    tab.press("ArrowLeft").await?;
    focused(tab, &day("2024-02-29")).await?;
    capture(env, tab, "calendar-dark-rtl-200").await?;

    let body_scroll = format!(
        "(JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {}).style.scroll_y)",
        js(part("body"))
    );
    let before = tab
        .eval(&body_scroll)
        .await?
        .as_f64()
        .context("date picker body scroll offset")?;
    check!(
        before > 0.,
        "revealing the focused date scrolls the compact body"
    );
    let wheel = point(tab, &day("2024-02-29")).await?;
    tab.mouse_move(wheel.x, wheel.y).await?;
    tab.wheel(0., -240.).await?;
    tab.wait_for(&format!("{body_scroll} < {before}")).await?;
    let after = tab.eval(&body_scroll).await?;
    picker_state(tab, "d.session.draft === '2024-02-28' && d.commits === 0").await?;

    click(tab, &part("mode")).await?;
    picker_state(
        tab,
        "d.session.mode === 'Input' && d.session.draft === '2024-02-28'",
    )
    .await?;
    capture(env, tab, "input-dark-rtl-200").await?;
    click(tab, &part("cancel")).await?;
    runtime_focused(tab, OPEN).await?;
    picker_state(tab, "d.session === null && d.commits === 0").await?;
    Ok(
        json!({"panel":panel.b,"day":cell.b,"confirm":confirm.b,"body_scroll":[json!(before),after]}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let calendar = calendar(env, tab).await?;
    let years = year_panel(env, tab).await?;
    let input = input(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "date picker page errors: {errors:?}");
    env.write_json(
        "browser-material-date-picker.json",
        &json!({
            "browser":chrome.version,"calendar":calendar,"years":years,"input":input,
            "compact":compact,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(1100, 924, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-date-picker-failure.json",
            &json!({
               "error":format!("{error:#}"),"state":state(&tab).await.ok(),
               "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
               "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
               "errors":tab.errors(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-date-picker-failure.png"))
            .await;
    }
    result
}
