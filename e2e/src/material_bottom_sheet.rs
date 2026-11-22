//! Bottom sheet interactions through Chromium's trusted input and WebGPU paint.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

const PANEL: &str = "bottom-sheet-demo-panel";
const HANDLE: &str = "bottom-sheet-demo-panel-handle";
const CLOSE: &str = "bottom-sheet-demo-panel-close";
const TITLE: &str = "bottom-sheet-demo-panel-title";
const HEADER: &str = "bottom-sheet-demo-panel-header";
const CONTENT: &str = "bottom-sheet-demo-panel-content";
const LAYER: &str = "bottom-sheet-demo-panel-layer";
const OPEN: &str = "bottom-sheet-demo-open";
const MAIN: &str = "bottom-sheet-demo-main-action";
const ACTION: &str = "bottom-sheet-demo-action";
const DISABLED: &str = "bottom-sheet-demo-disabled";
const LAST: &str = "bottom-sheet-demo-last";
const DESCRIPTION: &str = "bottom-sheet-demo-description";

async fn sheet(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["bottom_sheet"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.bottom_sheet)")).await?;
    tab.settle().await
}

async fn anchor(tab: &Tab, anchor: &str) -> Result<Value> {
    let height = match anchor {
        "closed" => "0",
        "peek" => "d.peek_height",
        "expanded" => "d.expanded_height",
        other => bail!("invalid bottom sheet anchor {other}"),
    };
    ready(
        tab,
        &format!(
            "d.state === {} && !d.dragging && !d.settling && Math.abs(d.visible_height - {height}) <= 0.75",
            js(anchor)
        ),
    )
    .await?;
    sheet(tab).await
}

async fn select(tab: &Tab, id: &str, condition: &str) -> Result<()> {
    click(tab, id).await?;
    ready(tab, condition).await
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn model_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    model_focus(tab, id).await?;
    tab.wait_for(&format!("document.activeElement === {}", projected(id)))
        .await
}

async fn focus(tab: &Tab, id: &str) -> Result<()> {
    let element = projected(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    focused(tab, id).await
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return v?.width === {width} && v?.height === {height}
        }})()"
    ))
    .await?;
    tab.settle().await
}

async fn bounds(tab: &Tab, id: &str) -> Result<[f64; 4]> {
    point(tab, id)
        .await?
        .b
        .as_slice()
        .try_into()
        .context("four control bounds")
}

async fn panel_bounds(tab: &Tab) -> Result<[f64; 4]> {
    let [x, y, width, height] = bounds(tab, PANEL).await?;
    let [_, _, viewport_width, viewport_height] = tab.canvas_box().await?;
    let displayed = sheet(tab).await?["visible_height"]
        .as_f64()
        .context("bottom sheet height")?;
    check!(
        x >= -0.75 && y >= -0.75 && x + width <= viewport_width + 0.75
            && (y + height - viewport_height).abs() <= 0.75
            && (height - displayed).abs() <= 0.75,
        "panel fits the viewport and follows its bottom anchor: [{x},{y},{width},{height}], visible {displayed}"
    );
    Ok([x, y, width, height])
}

fn total(value: &Value, field: &str) -> Result<u64> {
    value[field].as_u64().with_context(|| format!("no {field}"))
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-bottom-sheet-{name}.png")))
        .await
}

async fn standard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M50",
        "standard bottom sheet peeks and expands while the page stays live",
    );
    let initial = anchor(tab, "closed").await?;
    check_eq!(
        initial["presentation"],
        json!("standard"),
        "default presentation"
    );
    let main = point(tab, MAIN).await?;
    let outside = [2., 2., 10., 10.];
    let bare = painted_region(env, tab, "standard-before", outside).await?;
    focus(tab, OPEN).await?;
    tab.press("Enter").await?;
    anchor(tab, "peek").await?;
    focused(tab, OPEN).await?;
    let peek = panel_bounds(tab).await?;
    tab.wait_for(&format!(
        "{} !== null && {}?.getAttribute('aria-expanded') === 'false' && !document.querySelector('#surface-accessibility [aria-modal=true]')",
        projected(MAIN), projected(HANDLE)
    ))
    .await?;
    let export = dom(tab).await?;
    let layer = find_node(&export, LAYER)?;
    check_eq!(
        layer["style"]["background"][3],
        json!(0),
        "standard layer alpha"
    );
    tab.click(main.x, main.y).await?;
    ready(
        tab,
        &format!(
            "d.state === 'peek' && d.main_actions === {}",
            total(&initial, "main_actions")? + 1
        ),
    )
    .await?;
    let visible = painted_region(env, tab, "standard-peek", outside).await?;
    check!(
        visible == bare,
        "standard sheet leaves the background undimmed"
    );

    focus(tab, HANDLE).await?;
    tab.press("Enter").await?;
    anchor(tab, "expanded").await?;
    let expanded = panel_bounds(tab).await?;
    check!(
        expanded[1] < peek[1] && expanded[3] > peek[3],
        "expanded sheet rises above its peek position"
    );
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-expanded') === 'true'",
        projected(HANDLE)
    ))
    .await?;
    focused(tab, HANDLE).await?;
    click(tab, HANDLE).await?;
    anchor(tab, "peek").await?;
    check_eq!(
        panel_bounds(tab).await?,
        peek,
        "handle click returns to peek"
    );
    focus(tab, CLOSE).await?;
    tab.press("Enter").await?;
    let closed = anchor(tab, "closed").await?;
    focused(tab, OPEN).await?;
    check_eq!(
        total(&closed, "dismissals")?,
        total(&initial, "dismissals")? + 1,
        "close dismisses once"
    );
    Ok(json!({"peek":peek,"expanded":expanded,"closed":closed}))
}

async fn painted_region(env: &Env, tab: &Tab, name: &str, rect: [f64; 4]) -> Result<Vec<u8>> {
    capture(env, tab, name).await?;
    let input = File::open(env.artifact(&format!("material-bottom-sheet-{name}.png")))?;
    let mut decoder = png::Decoder::new(BufReader::new(input));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
    let frame = reader.next_frame(&mut pixels)?;
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => bail!("unexpected Chrome PNG color type {other:?}"),
    };
    let window_width = tab
        .eval("innerWidth")
        .await?
        .as_f64()
        .context("window width")?;
    let scale = f64::from(frame.width) / window_width;
    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    let [x, y, width, height] = rect;
    let left = ((canvas_x + x + 2.) * scale).ceil() as usize;
    let top = ((canvas_y + y + 2.) * scale).ceil() as usize;
    let right = ((canvas_x + x + width - 2.) * scale).floor() as usize;
    let bottom = ((canvas_y + y + height - 2.) * scale).floor() as usize;
    check!(
        left < right
            && top < bottom
            && right <= frame.width as usize
            && bottom <= frame.height as usize,
        "crop is inside the WebGPU screenshot"
    );
    let mut region = Vec::new();
    for row in top..bottom {
        let offset = row * frame.line_size;
        for pixel in
            pixels[offset + left * channels..offset + right * channels].chunks_exact(channels)
        {
            region.extend_from_slice(&pixel[..3]);
        }
    }
    Ok(region)
}

async fn compact_scroll(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M53",
        "320 dp dark mode wraps 200% text and clips the real scrolled content",
    );
    select(tab, "bottom-sheet-demo-scale", "d.text_scale === 2").await?;
    select(tab, "bottom-sheet-demo-skip-peek", "d.skip_peek").await?;
    click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    resize(tab, 320, 580).await?;
    click(tab, OPEN).await?;
    anchor(tab, "expanded").await?;
    let panel = panel_bounds(tab).await?;
    let header = bounds(tab, HEADER).await?;
    let title = bounds(tab, TITLE).await?;
    let description = bounds(tab, DESCRIPTION).await?;
    let last_before = bounds(tab, LAST).await?;
    let content = point(tab, CONTENT).await?;
    let clip = bounds(tab, CONTENT).await?;
    let export = dom(tab).await?;
    let text = find_node(&export, DESCRIPTION)?;
    let line = text["style"]["line_height"]
        .as_f64()
        .context("description line height")?
        * 2.;
    check!(
        description[3] >= 2. * line - 0.75
            && description[0] >= clip[0] - 0.75
            && description[0] + description[2] <= clip[0] + clip[2] + 0.75,
        "long CJK text wraps inside the scaled panel"
    );
    check!(
        title[0] >= clip[0]
            && title[0] + title[2] <= clip[0] + clip[2] + 0.75
            && title[1] >= clip[1]
            && title[1] + title[3] <= clip[1] + clip[3] + 0.75,
        "scaled title is visible inside the content viewport"
    );
    check!(
        last_before[1] + last_before[3] > clip[1] + clip[3],
        "last control starts outside the scroll viewport"
    );
    tab.mouse_move(content.x, content.y).await?;
    tab.settle().await?;
    // Wheel scroll moves rows through the header's coordinates. Compare the
    // actual pixels there before and after scroll to check that paint clips.
    let before = painted_region(env, tab, "compact-dark-expanded", header).await?;
    let surface: [u8; 4] =
        serde_json::from_value(find_node(&export, PANEL)?["style"]["background"].clone())?;
    check!(
        surface[..3].iter().all(|c| *c < 96)
            && before.chunks_exact(3).any(|pixel| pixel == &surface[..3]),
        "WebGPU paints the dark surface in the compact header"
    );
    tab.wheel(0., 2600.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({})[1] < {}",
        js(LAST),
        last_before[1] - 24.
    ))
    .await?;
    tab.settle().await?;
    let last = bounds(tab, LAST).await?;
    let scrolled_description = bounds(tab, DESCRIPTION).await?;
    check_eq!(
        bounds(tab, HEADER).await?,
        header,
        "scroll keeps the header fixed"
    );
    check!(
        scrolled_description[1] < clip[1] - 16.
            && last[1] >= clip[1] - 0.75
            && last[1] + last[3] <= clip[1] + clip[3] + 0.75,
        "wheel moves rows under the clip and reveals the last control"
    );
    let after = painted_region(env, tab, "compact-dark-scrolled", header).await?;
    check!(
        before == after,
        "off-viewport scroll content changes no header pixel"
    );
    let before = sheet(tab).await?;
    click(tab, LAST).await?;
    ready(
        tab,
        &format!(
            "d.panel_actions === {}",
            total(&before, "panel_actions")? + 1
        ),
    )
    .await?;
    click(tab, CLOSE).await?;
    anchor(tab, "closed").await?;
    Ok(
        json!({"panel":panel,"header":header,"title":title,"content_clip":clip,
        "description_before":description,"description_after":scrolled_description,
        "last_before":last_before,"last_after":last,"fixed_header_bytes":after.len()}),
    )
}

async fn modal(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M51",
        "modal bottom sheet shields outside actions and restores opener focus",
    );
    select(tab, "bottom-sheet-demo-modal", "d.presentation === 'modal'").await?;
    let main = point(tab, MAIN).await?;
    let before = sheet(tab).await?;
    let outside = [2., 2., 10., 10.];
    let bare = painted_region(env, tab, "modal-before", outside).await?;
    focus(tab, OPEN).await?;
    tab.press("Enter").await?;
    anchor(tab, "peek").await?;
    focused(tab, HANDLE).await?;
    let panel = panel_bounds(tab).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-modal') === 'true' && {} === null && {} === null",
        projected(PANEL),
        projected(MAIN),
        projected(OPEN)
    ))
    .await?;
    let export = dom(tab).await?;
    check!(
        find_node(&export, LAYER)?["style"]["background"][3]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "modal scrim alpha"
    );
    let visible = painted_region(env, tab, "modal-peek", outside).await?;
    check!(
        visible
            .iter()
            .zip(&bare)
            .all(|(after, before)| after < before),
        "WebGPU scrim darkens the outside pixels"
    );
    check!(
        main.y < main.canvas[1] + panel[1],
        "outside probe is above the sheet"
    );
    tab.click(main.x, main.y).await?;
    let after_scrim = anchor(tab, "closed").await?;
    model_focus(tab, OPEN).await?;
    check_eq!(
        after_scrim["main_actions"],
        before["main_actions"],
        "scrim shields the main action"
    );
    check_eq!(
        total(&after_scrim, "dismissals")?,
        total(&before, "dismissals")? + 1,
        "scrim dismisses once"
    );

    step(
        "M52",
        "modal keyboard scope omits the page and the disabled control",
    );
    focus(tab, OPEN).await?;
    tab.press("Enter").await?;
    anchor(tab, "peek").await?;
    focused(tab, HANDLE).await?;
    tab.press("Space").await?;
    anchor(tab, "expanded").await?;
    focused(tab, HANDLE).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-disabled') === 'true'",
        projected(DISABLED)
    ))
    .await?;
    let controls = [HANDLE, CLOSE, ACTION, LAST];
    let mut visited = Vec::new();
    for key in ["Tab"; 4].into_iter().chain(["Shift+Tab"; 4]) {
        tab.press(key).await?;
        tab.settle().await?;
        let snapshot = state(tab).await?;
        let id = snapshot["focused"]
            .as_str()
            .context("focused modal control")?;
        check!(
            controls.contains(&id),
            "{key} stays within the modal controls: {id}"
        );
        focused(tab, id).await?;
        visited.push(id.to_owned());
    }
    check!(
        controls
            .iter()
            .all(|id| visited.iter().any(|seen| seen == id)),
        "each enabled modal control is reachable"
    );
    tab.press("Escape").await?;
    let closed = anchor(tab, "closed").await?;
    focused(tab, OPEN).await?;
    Ok(json!({"peek":panel,"after_scrim":after_scrim,"keyboard_path":visited,"closed":closed}))
}

#[derive(Clone, Copy)]
enum Device {
    Touch,
    Mouse,
}

#[derive(Clone, Copy)]
enum Phase {
    Start,
    Move,
    End,
    Cancel,
}

async fn gesture(tab: &Tab, device: Device, phase: Phase, (x, y): (f64, f64)) -> Result<()> {
    match device {
        Device::Touch => {
            let kind = match phase {
                Phase::Start => "touchStart",
                Phase::Move => "touchMove",
                Phase::End => "touchEnd",
                Phase::Cancel => "touchCancel",
            };
            let points = if matches!(phase, Phase::Start | Phase::Move) {
                json!([{"x":x,"y":y,"id":1}])
            } else {
                json!([])
            };
            tab.cdp(
                "Input.dispatchTouchEvent",
                json!({"type":kind,"touchPoints":points}),
            )
            .await?;
        }
        Device::Mouse => {
            let (kind, buttons) = match phase {
                Phase::Start => {
                    tab.mouse_move(x, y).await?;
                    return tab.mouse_down().await;
                }
                Phase::Move => ("mouseMoved", 1),
                Phase::End => ("mouseReleased", 0),
                Phase::Cancel => bail!("Chrome does not send mouse cancellation through CDP"),
            };
            tab.cdp(
                "Input.dispatchMouseEvent",
                json!({"type":kind,"x":x,"y":y,"button":"left","buttons":buttons,"clickCount":1}),
            )
            .await?;
        }
    }
    Ok(())
}

async fn drag(tab: &Tab, device: Device, dy: f64) -> Result<(f64, f64)> {
    let handle = point(tab, HANDLE).await?;
    let start = (handle.x, handle.y);
    let end = (handle.x, handle.y + dy);
    check!(
        end.1 > handle.canvas[1] && end.1 < handle.canvas[1] + handle.canvas[3],
        "drag lands inside canvas"
    );
    gesture(tab, device, Phase::Start, start).await?;
    gesture(tab, device, Phase::Move, end).await?;
    ready(tab, "d.dragging").await?;
    Ok(end)
}

async fn touch(tab: &Tab, enabled: bool) -> Result<()> {
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":enabled,"maxTouchPoints":1}),
    )
    .await?;
    Ok(())
}

async fn motion(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M54",
        "trusted vertical touch and mouse drag, rollback, collapse, and dismissal",
    );
    resize(tab, 960, 760).await?;
    select(tab, "bottom-sheet-demo-scale", "d.text_scale === 1").await?;
    select(tab, "bottom-sheet-demo-skip-peek", "!d.skip_peek").await?;
    select(
        tab,
        "bottom-sheet-demo-standard",
        "d.presentation === 'standard'",
    )
    .await?;
    click(tab, OPEN).await?;
    let peek = anchor(tab, "peek").await?;
    let peek_bounds = panel_bounds(tab).await?;
    let span = peek["expanded_height"]
        .as_f64()
        .context("expanded height")?
        - peek_bounds[3];
    check!(
        span >= 48.,
        "peek and expanded anchors have a usable drag range"
    );
    tab.eval(
        "(() => {
            window.bottomSheetPointerEvents = [];
            for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
                document.addEventListener(kind, e => {
                    if (e.target?.id === 'surface') {
                        window.bottomSheetPointerEvents.push({kind,device:e.pointerType,trusted:e.isTrusted});
                    }
                }, {capture:true,passive:true});
            }
        })()",
    ).await?;
    touch(tab, true).await?;
    let canceled_end = drag(tab, Device::Touch, -span * 0.72).await?;
    let held_bounds = panel_bounds(tab).await?;
    check!(
        held_bounds[3] > peek_bounds[3] + 20. && held_bounds[3] < peek_bounds[3] + span,
        "held touch renders an intermediate height"
    );
    capture(env, tab, "touch-held-up").await?;
    gesture(tab, Device::Touch, Phase::Cancel, canceled_end).await?;
    let rolled_back = anchor(tab, "peek").await?;
    check_eq!(
        rolled_back["dismissals"],
        peek["dismissals"],
        "cancel keeps original anchor without dismissal"
    );
    check_eq!(
        panel_bounds(tab).await?,
        peek_bounds,
        "cancel restores original rendered height"
    );

    let expanded_end = drag(tab, Device::Touch, -span * 0.82).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    gesture(tab, Device::Touch, Phase::End, expanded_end).await?;
    anchor(tab, "expanded").await?;
    touch(tab, false).await?;
    let collapse_end = drag(tab, Device::Mouse, span * 0.85).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    gesture(tab, Device::Mouse, Phase::End, collapse_end).await?;
    let collapsed = anchor(tab, "peek").await?;
    check_eq!(
        collapsed["dismissals"],
        peek["dismissals"],
        "standard down drag collapses without dismissing"
    );
    check_eq!(
        collapsed["panel_actions"],
        peek["panel_actions"],
        "drag does not activate a panel button"
    );
    click(tab, CLOSE).await?;
    anchor(tab, "closed").await?;

    select(tab, "bottom-sheet-demo-modal", "d.presentation === 'modal'").await?;
    click(tab, OPEN).await?;
    let modal_peek = anchor(tab, "peek").await?;
    touch(tab, true).await?;
    let dismissed_end = drag(tab, Device::Touch, panel_bounds(tab).await?[3] * 0.75).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    gesture(tab, Device::Touch, Phase::End, dismissed_end).await?;
    let dismissed = anchor(tab, "closed").await?;
    model_focus(tab, OPEN).await?;
    touch(tab, false).await?;
    check_eq!(
        total(&dismissed, "dismissals")?,
        total(&modal_peek, "dismissals")? + 1,
        "down drag dismisses modal once"
    );
    check_eq!(
        dismissed["main_actions"],
        modal_peek["main_actions"],
        "modal drag does not reach the main action"
    );
    let events = tab.eval("window.bottomSheetPointerEvents").await?;
    let recorded = events.as_array().context("trusted pointer event log")?;
    for device in ["touch", "mouse"] {
        let expected = ["pointerdown", "pointermove", "pointerup"]
            .into_iter()
            .chain((device == "touch").then_some("pointercancel"));
        for kind in expected {
            check!(
                recorded
                    .iter()
                    .any(|e| e["device"] == device && e["kind"] == kind && e["trusted"] == true),
                "trusted {device} {kind}"
            );
        }
    }
    Ok(
        json!({"peek":peek_bounds,"held":held_bounds,"rolled_back":rolled_back,
        "collapsed":collapsed,"dismissed":dismissed,"events":events}),
    )
}

async fn reduced_motion(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M55",
        "Reduce Motion reaches all anchors while the animation clock is frozen",
    );
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({
            "features":[{"name":"prefers-reduced-motion","value":"reduce"}]
        }),
    )
    .await?;
    tab.wait_for("matchMedia('(prefers-reduced-motion: reduce)').matches")
        .await?;
    tab.eval(
        "(() => {
            const now = performance.now();
            Object.defineProperty(performance, 'now', {configurable:true,value:() => now});
        })()",
    )
    .await?;
    focus(tab, OPEN).await?;
    tab.press("Enter").await?;
    let peek = anchor(tab, "peek").await?;
    check_eq!(
        peek["reduced_motion"],
        json!(true),
        "the runtime observes Reduce Motion"
    );
    focus(tab, HANDLE).await?;
    tab.press("Enter").await?;
    let expanded = anchor(tab, "expanded").await?;
    capture(env, tab, "reduced-motion-expanded").await?;
    tab.press("Escape").await?;
    let closed = anchor(tab, "closed").await?;
    focused(tab, OPEN).await?;
    let export = dom(tab).await?;
    check!(
        find_node(&export, PANEL).is_err() && find_node(&export, LAYER).is_err(),
        "close removes panel and scrim immediately"
    );
    tab.eval("delete performance.now").await?;
    Ok(json!({"peek":peek,"expanded":expanded,"closed":closed}))
}

async fn start_gallery(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 760).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({
            "features":[{"name":"prefers-reduced-motion","value":"no-preference"}]
        }),
    )
    .await?;
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    click(tab, "bottom-sheet-demo-show").await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    start_gallery(tab).await?;
    let standard = standard(env, tab).await?;
    let modal = modal(env, tab).await?;
    // Start from a new Gallery instance so the keyboard pass's focused
    // last action cannot carry its scroll position into the compact check.
    tab.goto(url).await?;
    start_gallery(tab).await?;
    select(tab, "bottom-sheet-demo-modal", "d.presentation === 'modal'").await?;
    let compact = compact_scroll(env, tab).await?;
    let motion = motion(env, tab).await?;
    let reduced = reduced_motion(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "bottom sheet page errors: {errors:?}");
    env.write_json(
        "browser-material-bottom-sheet.json",
        &json!({
            "browser":chrome.version,"standard":standard,"modal":modal,
            "compact":compact,"motion":motion,"reduced_motion":reduced,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-bottom-sheet-failure.json", &dump);
        let _ = capture(env, &tab, "failure").await;
    }
    result
}
