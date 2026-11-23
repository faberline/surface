//! End-to-end Material 3 Time Dial on the real browser runtime.

#[path = "material_time_dial_capture.rs"]
mod capture_loss;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const DIAL: &str = "time-dial";
const FACE: &str = "time-dial-dial-face";
const OPEN: &str = "time-dial-demo-open";
const HOUR: &str = "time-dial-hour-selector";
const MINUTE: &str = "time-dial-minute-selector";
const USE_INPUT: &str = "time-dial-use-input";
const USE_DIAL: &str = "time-dial-use-dial";

type Rect = [f64; 4];
type Position = (f64, f64);

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.time_dial)")).await?;
    tab.settle().await
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("time-dial-demo-{name}")).await
}

async fn enter_scene(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1244).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1200).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "time-dial-demo-show").await?;
    ready(tab, "d.visible && d.draft === null && d.commits === 0").await
}

async fn open(tab: &Tab) -> Result<()> {
    activate(tab, OPEN).await?;
    ready(tab, "d.draft?.mode === 'Dial' && d.draft.stage === 'Hour'").await?;
    focused(tab, HOUR).await
}

async fn closed(tab: &Tab, saved: &str, commits: u32) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.draft === null && d.saved === {} && d.commits === {commits} && d.outside_actions === 0",
            js(saved),
        ),
    )
    .await?;
    runtime_focus(tab, OPEN).await
}

async fn face(tab: &Tab, viewport: Rect) -> Result<Rect> {
    let dialog = bounds(tab, DIAL).await?;
    let clock = bounds(tab, FACE).await?;
    check!(
        contains(viewport, dialog)
            && contains(dialog, clock)
            && near(clock[2], 256.)
            && near(clock[3], 256.),
        "the complete 256 dp clock face fits: {viewport:?} {dialog:?} {clock:?}",
    );
    Ok(clock)
}

async fn tick(tab: &Tab, value: f64, count: f64, radius: f64) -> Result<Position> {
    let clock = point(tab, FACE).await?;
    let angle = value / count * std::f64::consts::TAU;
    Ok((
        clock.x + radius * angle.sin(),
        clock.y - radius * angle.cos(),
    ))
}

async fn mouse(tab: &Tab, event: &str, (x, y): Position, held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({
            "type":event,"x":x,"y":y,"button":"left","buttons":u8::from(held),"clickCount":1,
        }),
    )
    .await?;
    tab.settle().await
}

async fn touch(tab: &Tab, event: &str, at: Option<Position>) -> Result<()> {
    let touches = at.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":event,"touchPoints":touches}),
    )
    .await?;
    tab.settle().await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.screenshot(&env.artifact(&format!("material-time-dial-{name}.png")))
        .await
}

async fn portrait(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M139",
        "portrait face previews a held diagonal and chooses only on release",
    );
    resize(tab, 390, 720).await?;
    open(tab).await?;
    ready(
        tab,
        "d.orientation === 'Vertical' && d.draft.candidate === '09:07'",
    )
    .await?;
    let shape = face(tab, [0., 0., 390., 720.]).await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "dialog", "選擇時間")?;
    check_eq!(
        tab.eval(&format!("{} === null", projected("time-dial-demo-outside")))
            .await?,
        json!(true),
        "background lies outside the modal accessibility scope"
    );
    capture(env, tab, "portrait-light-hour").await?;

    let start = tick(tab, 3., 12., 100.).await?;
    let end = tick(tab, 6., 12., 100.).await?;
    mouse(tab, "mousePressed", start, true).await?;
    mouse(tab, "mouseMoved", end, true).await?;
    ready(
        tab,
        "d.draft.preview?.value === 6 && d.draft.stage === 'Hour' &&
        d.draft.raw_hour === '9' && d.draft.candidate === '09:07' && d.commits === 0",
    )
    .await?;
    capture(env, tab, "portrait-light-hour-held").await?;
    mouse(tab, "mouseReleased", end, false).await?;
    ready(
        tab,
        "d.draft.preview === null && d.draft.stage === 'Minute' &&
        d.draft.raw_hour === '6' && d.draft.raw_minute === '07' && d.commits === 0",
    )
    .await?;
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;
    Ok(json!({"face":shape,"preview":6,"staged":"06:07","saved":"09:07"}))
}

async fn checked_value(tab: &Tab, id: &str, expected: &str) -> Result<String> {
    let control = projected(id);
    let label = tab
        .eval_str(&format!("{control}?.getAttribute('aria-label')"))
        .await?;
    check_eq!(label, expected, format!("localized complete name of {id}"));
    let tree = ax_tree(tab).await?;
    let item = ax(&tree, "radio", &label)?;
    check_eq!(
        ax_property(item, "checked"),
        Some(&json!("true")),
        "Chrome Accessibility tree marks the current dial value"
    );
    Ok(label)
}

async fn landscape(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M140",
        "landscape 24-hour dark RTL touch reads the inner ring and every minute",
    );
    preset(tab, "theme").await?;
    preset(tab, "rtl").await?;
    preset(tab, "format").await?;
    resize(tab, 844, 390).await?;
    open(tab).await?;
    ready(
        tab,
        "d.orientation === 'Horizontal' && d.rtl && d.clock === 'TwentyFourHour'",
    )
    .await?;
    let shape = face(tab, [0., 0., 844., 390.]).await?;
    check_eq!(
        tab.eval(&format!("{} === null", projected("time-dial-period")))
            .await?,
        json!(true),
        "the 24-hour dial omits period"
    );
    checked_value(tab, "time-dial-dial-hour-9", "小時 09，共 24 個選項").await?;
    capture(env, tab, "landscape-dark-rtl-rings").await?;
    let outer = tick(tab, 0., 12., 100.).await?;
    touch(tab, "touchStart", Some(outer)).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "d.draft.stage === 'Minute' && d.draft.raw_hour === '00' && d.commits === 0",
    )
    .await?;
    activate(tab, HOUR).await?;
    ready(tab, "d.draft.stage === 'Hour'").await?;
    let inner = tick(tab, 0., 12., 64.).await?;
    touch(tab, "touchStart", Some(inner)).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "d.draft.stage === 'Minute' && d.draft.raw_hour === '12' && d.commits === 0",
    )
    .await?;
    let minute = tick(tab, 59., 60., 100.).await?;
    touch(tab, "touchStart", Some(minute)).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "d.draft.candidate === '12:59' && d.saved === '09:07' && d.commits === 0",
    )
    .await?;
    checked_value(tab, "time-dial-dial-minute-59", "分鐘 59，共 60 個選項").await?;
    let tree = ax_tree(tab).await?;
    let adjacent = ax(&tree, "radio", "分鐘 00，共 60 個選項")?;
    check_eq!(
        ax_property(adjacent, "checked"),
        Some(&json!("false")),
        "the nearby zero choice stays in Chrome accessibility"
    );
    check_eq!(
        tab.eval(
            "(() => {
                const nodes = JSON.parse(window.gallery.export_dom()).dom.nodes;
                const byId = id => nodes.find(n => n.stable_id === id);
                const zero = byId('time-dial-dial-minute-0');
                const selected = byId('time-dial-dial-minute-59');
                return [zero?.first_child ?? null, nodes[selected.first_child]?.text];
              })()",
        )
        .await?,
        json!([null, "59"]),
        "the selected 59 paints without an overlapping zero glyph"
    );
    capture(env, tab, "landscape-dark-rtl-minute").await?;
    focus_projected(tab, "time-dial-dial-minute-59").await?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.draft.candidate === '12:00' && d.commits === 0").await?;
    checked_value(tab, "time-dial-dial-minute-0", "分鐘 00，共 60 個選項").await?;
    tab.press("ArrowLeft").await?;
    ready(tab, "d.draft.candidate === '12:59' && d.commits === 0").await?;
    activate(tab, "time-dial-confirm").await?;
    closed(tab, "12:59", 1).await?;
    Ok(
        json!({"face":shape,"hour_rings":{"outer":0,"inner":12},"minute":59,
            "keyboard_wrap":[59,0,59],"overlapping_zero_ink_removed":true,"saved":"12:59"}),
    )
}

async fn axes_and_keys(tab: &Tab) -> Result<Value> {
    step(
        "M141",
        "projected keyboard permits each minute and the selected value is named",
    );
    resize(tab, 960, 820).await?;
    open(tab).await?;
    let hour = checked_value(tab, "time-dial-dial-hour-9", "小時 9，共 12 個選項").await?;
    focus_projected(tab, "time-dial-dial-hour-9").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.draft.stage === 'Minute' && d.draft.candidate === '09:07'",
    )
    .await?;
    runtime_focus(tab, MINUTE).await?;
    let minute = checked_value(tab, "time-dial-dial-minute-7", "分鐘 07，共 60 個選項").await?;
    focus_projected(tab, "time-dial-dial-minute-7").await?;
    tab.press("ArrowRight").await?;
    ready(
        tab,
        "d.draft.raw_minute === '08' && d.draft.stage === 'Minute' && d.commits === 0",
    )
    .await?;
    let next = checked_value(tab, "time-dial-dial-minute-8", "分鐘 08，共 60 個選項").await?;
    tab.press("ArrowLeft").await?;
    ready(tab, "d.draft.raw_minute === '07' && d.commits === 0").await?;
    activate(tab, "time-dial-pm").await?;
    ready(
        tab,
        "d.draft.period === 'Pm' && d.draft.candidate === '21:07' && d.saved === '09:07'",
    )
    .await?;
    checked_value(tab, "time-dial-pm", "下午").await?;
    activate(tab, "time-dial-cancel").await?;
    closed(tab, "09:07", 0).await?;
    Ok(json!({"hour":hour,"minute":minute,"next":next,"steps":[7,8,7]}))
}

async fn edit(tab: &Tab, id: &str, raw: &str) -> Result<()> {
    click(tab, id).await?;
    focused(tab, id).await?;
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    tab.settle().await
}

async fn switch_and_compose(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M143",
        "Input and Dial share invalid drafts and Enter during composition only accepts text",
    );
    resize(tab, 960, 820).await?;
    open(tab).await?;
    activate(tab, USE_INPUT).await?;
    ready(
        tab,
        "d.draft.mode === 'Input' && d.draft.candidate === '09:07'",
    )
    .await?;
    let minute = "time-dial-minute-input";
    edit(tab, minute, "60").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.draft.minute_error === 'OutOfRange' && d.draft.candidate === null",
    )
    .await?;
    activate(tab, USE_DIAL).await?;
    ready(
        tab,
        "d.draft.mode === 'Dial' && d.draft.raw_minute === '60' && d.draft.candidate === null",
    )
    .await?;
    let end = tick(tab, 11., 12., 100.).await?;
    tab.click(end.0, end.1).await?;
    ready(
        tab,
        "d.draft.stage === 'Minute' && d.draft.raw_hour === '11' && d.draft.raw_minute === '60'",
    )
    .await?;
    activate(tab, "time-dial-confirm").await?;
    ready(
        tab,
        "d.draft.minute_error === 'OutOfRange' && d.commits === 0 && d.saved === '09:07'",
    )
    .await?;
    activate(tab, USE_INPUT).await?;
    ready(
        tab,
        "d.draft.mode === 'Input' && d.draft.raw_minute === '60' && d.draft.raw_hour === '11'",
    )
    .await?;
    click(tab, minute).await?;
    focused(tab, minute).await?;
    tab.shortcut("a").await?;
    tab.ime_compose("５９", 2, 2).await?;
    let composition = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(minute)
    );
    tab.wait_for(&composition).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.draft.mode === 'Input' && d.draft.raw_minute === '60' && d.commits === 0",
    )
    .await?;
    tab.insert_text("５９").await?;
    ready(tab, "d.draft.candidate === '11:59' && d.commits === 0").await?;
    capture(env, tab, "input-shared-draft").await?;
    activate(tab, "time-dial-confirm").await?;
    closed(tab, "11:59", 1).await?;
    open(tab).await?;
    let outside = tab.canvas_box().await?;
    let panel = bounds(tab, DIAL).await?;
    check!(
        panel[0] > 12. && panel[1] > 12.,
        "canvas has an exposed modal scrim"
    );
    tab.click(outside[0] + 12., outside[1] + 12.).await?;
    closed(tab, "11:59", 1).await?;
    Ok(
        json!({"unresolved":"60","hour_repaired":"11","saved":"11:59","commits":1,"scrim_discards":true}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M144",
        "short dark RTL at 200% uses the editor and can explicitly return to the full dial",
    );
    preset(tab, "theme").await?;
    preset(tab, "rtl").await?;
    preset(tab, "large-text").await?;
    resize(tab, 1120, 880).await?;
    open(tab).await?;
    let end = tick(tab, 4., 12., 100.).await?;
    tab.click(end.0, end.1).await?;
    ready(
        tab,
        "d.draft.stage === 'Minute' && d.draft.raw_hour === '4'",
    )
    .await?;
    resize(tab, 320, 300).await?;
    ready(
        tab,
        "d.rtl && d.text_scale === 2 && d.orientation === null &&
        d.draft.mode === 'Input' && !d.draft.fits && d.draft.raw_hour === '4' &&
        d.draft.raw_minute === '07' && d.commits === 0",
    )
    .await?;
    check_eq!(
        tab.eval(&format!(
            "window.gallery.control_bounds({}).length",
            js(FACE)
        ))
        .await?,
        json!(0),
        "compact fallback removes the dial face from layout"
    );
    let panel = bounds(tab, DIAL).await?;
    let ok = bounds(tab, "time-dial-confirm").await?;
    let cancel = bounds(tab, "time-dial-cancel").await?;
    check!(
        contains([0., 0., 320., 300.], panel) && contains(panel, ok) && contains(panel, cancel),
        "short fallback keeps both action targets inside the viewport",
    );
    capture(env, tab, "compact-dark-rtl-input").await?;
    click(tab, "time-dial-hour-input").await?;
    focused(tab, "time-dial-hour-input").await?;
    tab.press("Tab").await?;
    focused(tab, "time-dial-minute-input").await?;
    tab.press("Tab").await?;
    runtime_focus(tab, "time-dial-am").await?;
    tab.settle().await?;
    let body = bounds(tab, "time-dial-body").await?;
    let period = [
        bounds(tab, "time-dial-am").await?,
        bounds(tab, "time-dial-pm").await?,
    ];
    check!(
        period
            .iter()
            .all(|target| contains(body, *target) && target[2] >= 47.5 && target[3] >= 47.5),
        "Tab reveals both complete period targets in the compact fallback: {period:?}"
    );
    resize(tab, 1120, 880).await?;
    ready(
        tab,
        "d.draft.mode === 'Input' && d.draft.fits && d.draft.candidate === '04:07'",
    )
    .await?;
    activate(tab, USE_DIAL).await?;
    ready(
        tab,
        "d.draft.mode === 'Dial' && d.draft.candidate === '04:07' && d.commits === 0",
    )
    .await?;
    face(tab, [0., 0., 1120., 880.]).await?;
    activate(tab, "time-dial-cancel").await?;
    closed(tab, "09:07", 0).await?;
    Ok(
        json!({"compact_panel":panel,"actions":[ok,cancel],"period":period,
            "retained":"04:07","explicit_return":true}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    enter_scene(tab, url).await?;
    let portrait = portrait(env, tab).await?;
    enter_scene(tab, url).await?;
    let landscape = landscape(env, tab).await?;
    enter_scene(tab, url).await?;
    let keyboard = axes_and_keys(tab).await?;
    enter_scene(tab, url).await?;
    let lost_capture = capture_loss::run(env, tab).await?;
    enter_scene(tab, url).await?;
    let switch = switch_and_compose(env, tab).await?;
    enter_scene(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "time dial page errors: {errors:?}");
    env.write_json(
        "browser-material-time-dial.json",
        &json!({"browser":chrome.version,"portrait":portrait,"landscape":landscape,"keyboard":keyboard,
            "lost_capture":lost_capture,"switch":switch,"compact":compact,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1244, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-time-dial-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-time-dial-failure.png"))
            .await;
    }
    result.context("Material 3 time dial")
}
