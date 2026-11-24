//! Command menu interactions and viewport behavior with trusted headless input.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_tree, bounds, contains, focus_projected, focused, projected, resize, Raster,
};
use super::{state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "menu-demo-show";
const BACK: &str = "menu-demo-back";
const OPENER: &str = "menu-demo-opener";
const PANEL: &str = "menu-demo-panel";
const FIRST: &str = "menu-demo-inbox";
const DISABLED: &str = "menu-demo-archive";
const FIND: &str = "menu-demo-find";
const LAST: &str = "menu-demo-details";
const OUTSIDE: &str = "menu-demo-list";

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2040).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, SHOW).await?;
    focused(tab, BACK).await?;
    resize(tab, 960, 800).await
}

async fn open(tab: &Tab) -> Result<()> {
    focus_projected(tab, OPENER).await?;
    tab.press("Enter").await?;
    until(tab, "s.menu_demo.open").await?;
    focused(tab, FIRST).await
}

async fn canvas_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {id:?}")).await?;
    // A trusted canvas press hands keyboard input back to the canvas. Its
    // accessibility projection remains available for browser navigation.
    tab.wait_for(&format!(
        "document.activeElement === document.querySelector('canvas') && {} !== null",
        projected(id)
    ))
    .await
}

async fn point(tab: &Tab, id: &str, x_fraction: f64) -> Result<(f64, f64)> {
    let [x, y, width, height] = bounds(tab, id).await?;
    let [left, top, _, _] = tab.canvas_box().await?;
    Ok((left + x + width * x_fraction, top + y + height / 2.))
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    open(tab).await?;
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-expanded')",
            projected(OPENER)
        ))
        .await?,
        "true",
        "opener remains expanded while its menu owns focus"
    );
    let tree = ax_tree(tab).await?;
    ax(&tree, "menu", "檔案動作")?;
    ax(&tree, "menuitem", "移到收件匣")?;
    check!(
        tree.iter()
            .any(|node| node["ignored"] == false && node["role"]["value"] == "separator"),
        "menu exposes the grouping separator"
    );
    tab.press("ArrowDown").await?;
    focused(tab, DISABLED).await?;
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-disabled')",
            projected(DISABLED)
        ))
        .await?,
        "true",
        "the focused item still reports disabled"
    );
    for input in ["Enter", "Space"] {
        tab.press(input).await?;
        until(tab, "s.menu_demo.open && s.menu_demo.last === null").await?;
        focused(tab, DISABLED).await?;
    }
    let (x, y) = point(tab, DISABLED, 0.5).await?;
    tab.click(x, y).await?;
    until(tab, "s.menu_demo.open && s.menu_demo.last === null").await?;
    canvas_focus(tab, DISABLED).await?;
    tab.press("ArrowDown").await?;
    canvas_focus(tab, FIND).await?;
    focus_projected(tab, FIND).await?;
    focused(tab, FIND).await?;
    tab.press("End").await?;
    focused(tab, LAST).await?;
    tab.press("Home").await?;
    focused(tab, FIRST).await?;

    let panel = bounds(tab, PANEL).await?;
    let trigger = bounds(tab, OPENER).await?;
    check!(
        panel[1] >= trigger[1] + trigger[3],
        "the wide menu opens below its trigger"
    );
    check_eq!(panel[2], 240., "the preferred menu width is applied");
    let capture = env.artifact("material-command-menu-wide.png");
    tab.screenshot(&capture).await?;
    let raster = Raster::read(tab, capture).await?;
    let center = raster.sample(panel[0] + 3., panel[1] + panel[3] / 2.)?;
    let corner = raster.sample(panel[0], panel[1])?;
    check!(center != corner, "the rounded panel clips its top corner");

    tab.press("Enter").await?;
    until(
        tab,
        "!s.menu_demo.open && s.menu_demo.last === '移到收件匣'",
    )
    .await?;
    focused(tab, OPENER).await?;
    Ok(
        json!({"trigger":trigger,"panel":panel,"center_rgb":center,"corner_rgb":corner,"disabled_focus":true}),
    )
}

async fn dismissal(tab: &Tab) -> Result<Value> {
    open(tab).await?;
    tab.press("Tab").await?;
    until(tab, "!s.menu_demo.open").await?;
    focused(tab, OUTSIDE).await?;
    open(tab).await?;
    tab.press("Shift+Tab").await?;
    until(tab, "!s.menu_demo.open").await?;
    focused(tab, "menu-demo-bottom").await?;
    open(tab).await?;
    let [x, y, width, height] = bounds(tab, PANEL).await?;
    let target = bounds(tab, OUTSIDE).await?;
    let fraction = if target[0] + target[2] * 0.5 >= x + width {
        0.5
    } else {
        0.98
    };
    let [left, top, _, _] = tab.canvas_box().await?;
    let (click_x, click_y) = point(tab, OUTSIDE, fraction).await?;
    check!(
        click_x < left + x
            || click_x >= left + x + width
            || click_y < top + y
            || click_y >= top + y + height,
        "outside point clears the menu panel"
    );
    tab.click(click_x, click_y).await?;
    until(
        tab,
        "!s.menu_demo.open && s.menu_demo.outside_actions === 0",
    )
    .await?;
    canvas_focus(tab, OPENER).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-expanded') === 'false'",
        projected(OPENER)
    ))
    .await
    .context("the opener exposes its collapsed state after pointer dismissal")?;
    Ok(
        json!({"tab_forward":OUTSIDE,"tab_backward":"menu-demo-bottom","outside_action_suppressed":true}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    for control in [
        "menu-demo-theme",
        "menu-demo-rtl",
        "menu-demo-large-text",
        "menu-demo-bottom",
    ] {
        activate(tab, control).await?;
    }
    resize(tab, 320, 480).await?;
    open(tab).await?;
    let panel = bounds(tab, PANEL).await?;
    let trigger = bounds(tab, OPENER).await?;
    check!(
        contains([8., 8., 304., 464.], panel),
        "the compact RTL panel is inside the safe viewport"
    );
    check!(
        panel[1] + panel[3] <= trigger[1],
        "the bottom trigger opens above at 320 dp"
    );
    let rename = bounds(tab, "menu-demo-rename").await?;
    check!(rename[3] > 48., "200 percent text wraps in a taller row");
    tab.press("End").await?;
    focused(tab, LAST).await?;
    let last = bounds(tab, LAST).await?;
    check!(
        contains(panel, last),
        "keyboard scroll reveals the final command"
    );
    tab.screenshot(&env.artifact("material-command-menu-compact.png"))
        .await?;
    tab.press("Escape").await?;
    until(tab, "!s.menu_demo.open").await?;
    focused(tab, OPENER).await?;
    let scroll_id = "menu-demo-preview-scroll";
    let status_id = "menu-demo-last";
    let scroller = bounds(tab, scroll_id).await?;
    let status_before = bounds(tab, status_id).await?;
    check!(
        scroller[1] + scroller[3] + 8. <= trigger[1],
        "the content clip respects the fixed trigger clearance"
    );
    check!(
        !contains(scroller, status_before),
        "the 200 percent command status needs scrolling in the short viewport"
    );
    let (x, y) = point(tab, scroll_id, 0.5).await?;
    tab.mouse_move(x, y).await?;
    tab.wheel(0., 800.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({status_id:?})[1] < {}",
        status_before[1]
    ))
    .await?;
    tab.settle().await?;
    let status_after = bounds(tab, status_id).await?;
    check!(
        contains(scroller, status_after),
        "the wheel reveals the whole status above the launcher: {status_after:?}"
    );
    check_eq!(
        bounds(tab, OPENER).await?,
        trigger,
        "the launcher keeps its bounds while the page scrolls"
    );
    tab.screenshot(&env.artifact("material-command-menu-compact-footer.png"))
        .await?;
    Ok(
        json!({"trigger":trigger,"panel":panel,"last":last,"wrapped_height":rename[3],
          "scroller":scroller,"status_before":status_before,"status_after":status_after,
          "state":state(tab).await?["menu_demo"]}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2084, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step("M267", "named command menu keeps a disabled command focusable and rejects its activation");
        route(&tab, url).await.context("M267 route")?;
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter();if(!a)return null;const {isFallbackAdapter,vendor,architecture}=a.info;return {isFallbackAdapter,vendor,architecture};})()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(),
            "WebGPU adapter reports fallback status: {}", report["adapter"]);
        report["keyboard"] = keyboard(env, &tab).await.context("M267")?;
        step("M268", "Tab leaves the menu in both directions and outside clicks dismiss without activating the page");
        report["dismissal"] = dismissal(&tab).await.context("M268")?;
        step("M269", "short dark RTL menu flips above, wraps 200 percent text and keeps the bottom launcher clear of scrolling content");
        report["compact"] = compact(env, &tab).await.context("M269")?;
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }.await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-command-menu-failure.png"))
            .await;
    }
    env.write_json("browser-material-command-menu.json", &report)?;
    result
}
