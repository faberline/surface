//! WebGPU and Chrome accessibility proof for the Material tabs Gallery.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{point, state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "tabs-demo-show";
const BACK: &str = "tabs-demo-back";
const THEME: &str = "tabs-demo-theme";
const RTL: &str = "tabs-demo-rtl";
const LARGE: &str = "tabs-demo-large";
const SCROLLABLE: &str = "tabs-demo-scrollable";
const AUTOMATIC: &str = "tabs-demo-automatic";
const PRIMARY: &str = "tabs-demo-primary";
const SECONDARY: &str = "tabs-demo-secondary";
const OVERVIEW: &str = "tabs-demo-overview";
const LOCK: &str = "tabs-demo-lock";
const ACTIVITY: &str = "tabs-demo-activity";
const HISTORY: &str = "tabs-demo-history";
const UNREAD: &str = "tabs-demo-unread";
const FIXED: [(&str, &str); 6] = [
    (OVERVIEW, "總覽"),
    (LOCK, "鎖定"),
    (ACTIVITY, "最新活動"),
    ("tabs-demo-all", "全部"),
    ("tabs-demo-private", "私密"),
    (UNREAD, "尚未閱讀"),
];

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

fn middle(rect: Rect) -> f64 {
    rect[0] + rect[2] / 2.
}

fn part(id: &str, role: &str) -> String {
    format!("{id}-{role}")
}

async fn demo(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["tabs_demo"].clone())
}

async fn select_state(tab: &Tab, primary: &str, count: u32) -> Result<Value> {
    until(
        tab,
        &format!(
            "s.tabs_demo.selected[0] === {} && s.tabs_demo.requests[0] === {count}",
            json!(primary)
        ),
    )
    .await?;
    // The projected focus targets are refreshed on the presentation frame.
    tab.settle().await?;
    let model = demo(tab).await?;
    check_eq!(
        model["selected"],
        json!([primary, UNREAD]),
        "only the requested tab can change"
    );
    check_eq!(
        model["requests"],
        json!([count, 0]),
        "one request per accepted change"
    );
    Ok(model)
}

async fn panel_links(tab: &Tab, selected: [&str; 2]) -> Result<Vec<Value>> {
    let tree = ax_tree(tab).await?;
    ax(&tree, "tablist", "工作區頁籤")?;
    ax(&tree, "tablist", "訊息頁籤")?;
    let mut ids = BTreeSet::new();
    let mut links = Vec::new();
    for (index, (id, name)) in FIXED.into_iter().enumerate() {
        let active = selected.contains(&id);
        let tab_ax = ax(&tree, "tab", name)?;
        check_eq!(
            ax_property(tab_ax, "selected"),
            Some(&json!(active)),
            "Chrome AX selection"
        );
        check_eq!(
            ax_property(tab_ax, "disabled") == Some(&json!(true)),
            matches!(index, 1 | 4),
            "Chrome AX disabled state"
        );
        let source = projected(id);
        let link = tab.eval(&format!(
            "(() => {{
                const source = {source};
                const target = document.getElementById(source?.getAttribute('aria-controls'));
                return {{
                    source: source?.id, target: target?.id,
                    role: target?.getAttribute('role'),
                    label: target?.getAttribute('aria-labelledby'),
                    hidden: target?.hidden,
                    empty: target ? !target.childNodes.length : false,
                    live: target?.getAttribute('data-surface-id'),
                    matches: target ? Array.from(document.querySelectorAll('[id]')).filter(e => e.id === target.id).length : 0
                }};
            }})()"
        )).await?;
        let target = link["target"].as_str().context("real panel ID")?;
        check!(
            ids.insert(target.to_owned()),
            "a distinct target for each tab"
        );
        check_eq!(link["matches"], json!(1), "ID occurs once in the document");
        check_eq!(link["role"], "tabpanel", "projected panel role");
        check_eq!(
            link["label"],
            link["source"],
            "panel points back to its tab"
        );
        check_eq!(
            link["hidden"],
            json!(!active),
            "only the active panel is visible"
        );
        check_eq!(
            ax(&tree, "tabpanel", name).is_ok(),
            active,
            "Chrome AX sees only the active named panel"
        );
        if active {
            check_eq!(
                link["live"],
                part(id, "panel"),
                "the active target is the real content"
            );
        } else {
            check_eq!(link["empty"], true, "inactive reference is empty");
        }
        links.push(link);
    }
    Ok(links)
}

fn ink(model: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(model["palette"][name].clone())?;
    check_eq!(alpha, 255, "opaque Material palette role");
    Ok([r, g, b])
}

async fn picture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-tabs-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn indicator(tab: &Tab, list: usize) -> Result<Rect> {
    let mut rect: Rect = serde_json::from_value(demo(tab).await?["indicators"][list].clone())?;
    let strip = bounds(tab, &part([PRIMARY, SECONDARY][list], "strip")).await?;
    rect[0] += strip[0];
    rect[1] += strip[1];
    Ok(rect)
}

async fn fixed_paint(env: &Env, tab: &Tab, dark: bool) -> Result<Value> {
    let model = demo(tab).await?;
    let top = bounds(tab, PRIMARY).await?;
    let low = bounds(tab, SECONDARY).await?;
    let cells = [
        bounds(tab, OVERVIEW).await?,
        bounds(tab, LOCK).await?,
        bounds(tab, ACTIVITY).await?,
    ];
    let unread = bounds(tab, UNREAD).await?;
    let marks = [indicator(tab, 0).await?, indicator(tab, 1).await?];
    let divider = bounds(tab, &part(PRIMARY, "divider")).await?;
    let icon_a = bounds(tab, &part(ACTIVITY, "icon")).await?;
    let icon_b = bounds(tab, &part(UNREAD, "icon")).await?;
    let label_a = bounds(tab, &part(ACTIVITY, "label")).await?;
    let label_b = bounds(tab, &part(UNREAD, "label")).await?;
    let overview = bounds(tab, &part(OVERVIEW, "label")).await?;
    check!(
        cells
            .iter()
            .all(|cell| near(cell[1], top[1]) && near(cell[2], top[2] / 3.)),
        "fixed cells form one equal row"
    );
    check!(
        near(top[3], 64.) && near(low[3], 48.),
        "primary stacked and secondary inline row heights"
    );
    for icon in [icon_a, icon_b] {
        check!(
            near(icon[2], 24.) && near(icon[3], 24.),
            "24 dp vector icon"
        );
    }
    check!(
        bottom(icon_a) <= label_a[1] + 0.75,
        "primary icon is above text"
    );
    check!(
        near(label_b[0] - right(icon_b), 8.),
        "secondary inline icon gap"
    );
    check!(
        near(marks[0][2], overview[2].max(24.)) && near(marks[0][3], 3.),
        "primary mark uses content width and 3 dp height"
    );
    check!(
        near(marks[1][2], unread[2]) && near(marks[1][3], 2.),
        "secondary mark uses full cell width and 2 dp height"
    );
    check!(
        near(bottom(marks[0]), bottom(top)) && near(bottom(marks[1]), bottom(low)),
        "both marks hug the bottom"
    );
    check!(
        near(divider[2], top[2]) && near(divider[3], 1.),
        "one 1 dp strip divider"
    );
    let capture = if dark { "fixed-dark" } else { "fixed-light" };
    let raster = picture(env, tab, capture).await?;
    let primary = ink(&model, "primary")?;
    for rect in marks {
        check_eq!(
            raster.sample(middle(rect), rect[1] + 1.)?,
            primary,
            "primary-colored indicator PNG"
        );
    }
    check_eq!(
        raster.sample(middle(cells[1]), divider[1] + 0.5)?,
        ink(&model, "surface_variant")?,
        "divider PNG"
    );
    check_eq!(
        raster.sample(middle(cells[1]), cells[1][1] + 4.)?,
        ink(&model, "surface")?,
        "surface PNG"
    );
    check!(
        raster.sample(marks[0][0] + 0.25, marks[0][1] + 0.25)? != primary,
        "curved top-left corner PNG"
    );
    if !dark {
        let ring = bounds(tab, &part(OVERVIEW, "focus-ring")).await?;
        check_eq!(
            raster.sample(ring[0] + 12., ring[1] + 1.5)?,
            ink(&model, "secondary")?,
            "keyboard focus PNG"
        );
    }
    Ok(
        json!({"capture": capture, "bars": [top, low], "cells": cells, "marks": marks, "divider": divider}),
    )
}

async fn fixed(env: &Env, tab: &Tab) -> Result<Value> {
    reduced_motion(tab, true).await?;
    for id in [THEME, RTL, LARGE, SCROLLABLE, AUTOMATIC, OVERVIEW] {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    select_state(tab, OVERVIEW, 0).await?;
    let before = panel_links(tab, [OVERVIEW, UNREAD]).await?;
    let light = fixed_paint(env, tab, false).await?;
    let overview_panel = part(OVERVIEW, "panel");
    for id in [&*overview_panel, UNREAD, "tabs-demo-read-message"] {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    tab.press("Enter").await?;
    until(tab, "s.tabs_demo.opened === 1").await?;
    focused(tab, "tabs-demo-read-message").await?;
    for id in [UNREAD, &*overview_panel, OVERVIEW] {
        tab.press("Shift+Tab").await?;
        focused(tab, id).await?;
    }
    for key in ["ArrowRight", "ArrowDown"] {
        tab.press(key).await?;
        focused(tab, ACTIVITY).await?;
    }
    select_state(tab, OVERVIEW, 0).await?;
    tab.press("Enter").await?;
    select_state(tab, ACTIVITY, 1).await?;
    focused(tab, ACTIVITY).await?;
    let after = panel_links(tab, [ACTIVITY, UNREAD]).await?;
    for (old, new) in before.iter().zip(&after) {
        check_eq!(old["source"], new["source"], "tab ID survives selection");
        check_eq!(
            old["target"],
            new["target"],
            "panel ID survives lazy mounting"
        );
    }
    let locked = point(tab, LOCK).await?;
    tab.click(locked.x, locked.y).await?;
    select_state(tab, ACTIVITY, 1).await?;
    focus_projected(tab, ACTIVITY).await?;
    for (key, id) in [
        ("Home", OVERVIEW),
        ("End", ACTIVITY),
        ("ArrowRight", OVERVIEW),
    ] {
        tab.press(key).await?;
        focused(tab, id).await?;
    }
    tab.press("Space").await?;
    select_state(tab, OVERVIEW, 2).await?;
    tab.press("Space").await?;
    select_state(tab, OVERVIEW, 2).await?;
    activate(tab, THEME).await?;
    until(tab, "s.dark").await?;
    let dark = fixed_paint(env, tab, true).await?;
    Ok(json!({"light":light, "dark":dark, "before":before, "after":after}))
}

async fn automatic(env: &Env, tab: &Tab) -> Result<Value> {
    reduced_motion(tab, false).await?;
    activate(tab, AUTOMATIC).await?;
    until(tab, "s.tabs_demo.automatic").await?;
    tab.press("Tab").await?;
    focused(tab, OVERVIEW).await?;
    let before = indicator(tab, 0).await?;
    tab.eval(
        "window.tabMotionTrace = [];
         const track = () => {
            const rect = JSON.parse(window.gallery.inspect()).tabs_demo.indicators[0];
            window.tabMotionTrace.push(rect);
            if (window.tabMotionTrace.length < 50) requestAnimationFrame(track);
         };
         requestAnimationFrame(track);",
    )
    .await?;
    tab.wait_for("window.tabMotionTrace.length >= 2").await?;
    tab.press("ArrowRight").await?;
    focused(tab, ACTIVITY).await?;
    select_state(tab, ACTIVITY, 3).await?;
    tab.wait_for("window.tabMotionTrace.length === 50").await?;
    let samples: Vec<Rect> = serde_json::from_value(tab.eval("window.tabMotionTrace").await?)?;
    let after = indicator(tab, 0).await?;
    let target = bounds(tab, ACTIVITY).await?;
    check!(
        near(middle(after), middle(target)),
        "motion ends at the accepted tab"
    );
    let start_x = middle(samples[0]);
    let end_x = middle(samples[samples.len() - 1]);
    check!(end_x > start_x + 24., "selection moves the indicator");
    check!(
        samples
            .iter()
            .any(|rect| middle(*rect) > start_x + 8. && middle(*rect) < end_x - 8.),
        "motion presents an in-between frame"
    );
    check!(
        samples
            .iter()
            .all(|rect| near(bottom(*rect), bottom(samples[0])) && near(rect[3], 3.)),
        "motion keeps the 3 dp mark pinned"
    );

    reduced_motion(tab, true).await?;
    tab.press("ArrowLeft").await?;
    focused(tab, OVERVIEW).await?;
    select_state(tab, OVERVIEW, 4).await?;
    let snapped = indicator(tab, 0).await?;
    check!(
        near(middle(snapped), middle(before)),
        "Reduce Motion snaps to accepted selection"
    );
    let hit = point(tab, OVERVIEW).await?;
    let (x, y) = (hit.b[0] + 12., hit.b[1] + 8.);
    tab.mouse_move(hit.x, hit.y).await?;
    tab.mouse_down().await?;
    let photo = picture(env, tab, "dark-pressed").await?;
    let pressed = photo.sample(x, y)?;
    let model = select_state(tab, OVERVIEW, 4).await?;
    let base = ink(&model, "surface")?;
    let source = ink(&model, "primary")?;
    let target = core::array::from_fn::<_, 3, _>(|i| {
        (u16::from(base[i]) * 229 + u16::from(source[i]) * 26).div_euclid(255) as u8
    });
    check!(
        pressed
            .iter()
            .zip(target)
            .all(|(actual, expected)| actual.abs_diff(expected) <= 2),
        "selected tab presses with 10 percent Primary ink"
    );
    tab.mouse_up().await?;
    select_state(tab, OVERVIEW, 4).await?;
    Ok(
        json!({"before":before, "after":after, "samples":samples, "snapped":snapped, "pressed_ink":pressed}),
    )
}

async fn touch(tab: &Tab, kind: &str, point: Option<(f64, f64)>) -> Result<()> {
    let points = point
        .map(|(x, y)| vec![json!({"x":x,"y":y,"id":1})])
        .unwrap_or_default();
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    Ok(())
}

async fn scrolling(env: &Env, tab: &Tab) -> Result<Value> {
    activate(tab, AUTOMATIC).await?;
    activate(tab, SCROLLABLE).await?;
    resize(tab, 320, 760).await?;
    activate(tab, LARGE).await?;
    until(
        tab,
        "s.tabs_demo.scrollable && !s.tabs_demo.automatic && s.tabs_demo.text_scale === 2",
    )
    .await?;
    let scroll_id = part(PRIMARY, "scroll");
    let strip_id = part(PRIMARY, "strip");
    let viewport = bounds(tab, &scroll_id).await?;
    let before = bounds(tab, &strip_id).await?;
    let first = bounds(tab, OVERVIEW).await?;
    let last = bounds(tab, HISTORY).await?;
    check!(near(first[0] - viewport[0], 52.), "LTR start edge padding");
    check!(last[0] >= right(viewport), "viewport clips the far tab");
    check!(
        first[2] >= 90. && first[3] >= 48.,
        "scrollable minimum target"
    );
    let hit = point(tab, &scroll_id).await?;
    tab.mouse_move(hit.x, hit.y).await?;
    tab.wheel(10000., 0.).await?;
    tab.settle().await?;
    let wheel_end = bounds(tab, &strip_id).await?;
    let last = bounds(tab, HISTORY).await?;
    check!(
        wheel_end[0] < before[0] - 48.,
        "real horizontal wheel pans the strip"
    );
    check!(
        near(right(viewport) - right(last), 52.),
        "LTR end edge padding"
    );
    select_state(tab, OVERVIEW, 4).await?;
    tab.wheel(-10000., 0.).await?;
    tab.settle().await?;
    check!(
        near(bounds(tab, &strip_id).await?[0], before[0]),
        "wheel clamps at logical start"
    );

    activate(tab, RTL).await?;
    until(tab, "s.tabs_demo.rtl").await?;
    tab.settle().await?;
    let first = bounds(tab, OVERVIEW).await?;
    let rtl_start = bounds(tab, &strip_id).await?;
    check!(
        near(right(viewport) - right(first), 52.),
        "RTL start edge padding"
    );
    let start = (hit.canvas[0] + viewport[0] + viewport[2] * 0.28, hit.y);
    let end = (hit.canvas[0] + viewport[0] + viewport[2] * 0.85, hit.y);
    touch(tab, "touchStart", Some(start)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    touch(tab, "touchEnd", None).await?;
    tab.settle().await?;
    let dragged = bounds(tab, &strip_id).await?;
    check!(
        dragged[0] > rtl_start[0] + 30.,
        "trusted touch pans the RTL strip"
    );
    select_state(tab, OVERVIEW, 4).await?;

    focus_projected(tab, AUTOMATIC).await?;
    tab.press("Tab").await?;
    focused(tab, OVERVIEW).await?;
    tab.press("ArrowLeft").await?;
    focused(tab, ACTIVITY).await?;
    tab.press("End").await?;
    focused(tab, HISTORY).await?;
    let revealed = bounds(tab, HISTORY).await?;
    let label = bounds(tab, &part(HISTORY, "label")).await?;
    check!(
        contains(viewport, revealed) && contains(revealed, label),
        "focus scrolls the complete 200 percent label into view"
    );
    select_state(tab, OVERVIEW, 4).await?;
    tab.press("Enter").await?;
    let model = select_state(tab, HISTORY, 5).await?;
    focused(tab, HISTORY).await?;
    let mark = indicator(tab, 0).await?;
    let frame = picture(env, tab, "rtl-320").await?;
    check_eq!(
        frame.sample(middle(mark), mark[1] + 1.)?,
        ink(&model, "primary")?,
        "RTL mark paints on the revealed tab"
    );
    Ok(
        json!({"start":before, "wheel_end":wheel_end, "rtl_start":rtl_start, "dragged":dragged, "revealed":revealed, "label":label, "mark":mark}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(
            "M286",
            "fixed M3 tabs, linked AX panels, keyboard and both palettes",
        );
        tab.goto(url).await?;
        crate::smoke::ready(&tab, "gallery").await?;
        report["adapter"] = tab
            .eval(
                "(async () => {
                const a = await navigator.gpu?.requestAdapter();
                if (!a) return null;
                const {vendor, architecture, isFallbackAdapter} = a.info;
                return {vendor, architecture, isFallbackAdapter};
            })()",
            )
            .await?;
        check!(
            report["adapter"].is_object(),
            "Chrome has a real WebGPU adapter"
        );
        activate(&tab, SHOW).await?;
        until(&tab, "s.tabs_demo.visible && !s.tabs_demo.scrollable").await?;
        focused(&tab, BACK).await?;
        resize(&tab, 960, 800).await?;
        report["fixed"] = fixed(env, &tab).await?;
        step(
            "M287",
            "automatic activation, measured motion, Reduce Motion and press paint",
        );
        report["automatic"] = automatic(env, &tab).await?;
        step(
            "M288",
            "scrollable LTR and RTL strip, wheel, touch and 200 percent focus reveal",
        );
        report["scroll"] = scrolling(env, &tab).await?;
        step(
            "M289",
            "restore Gallery focus when closing the tabs demonstration",
        );
        activate(&tab, BACK).await?;
        until(&tab, "!s.tabs_demo.visible && s.section === 0").await?;
        focused(&tab, SHOW).await?;
        check!(tab.errors().is_empty(), "tab journey has no browser errors");
        Ok(())
    }
    .await;
    if let Err(error) = &result {
        report["error"] = json!(format!("{error:#}"));
        report["debug"] = state(&tab).await.unwrap_or(Value::Null);
        report["active"] = tab
            .eval("document.activeElement?.getAttribute('data-surface-id')")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-tabs-failure.png"))
            .await;
    }
    report["page_errors"] = json!(tab.errors());
    env.write_json("material-tabs.json", &report)?;
    result
}
