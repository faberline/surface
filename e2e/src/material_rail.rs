//! Navigation Rail through the release Gallery and Chrome's input paths.

use super::{click, point, state, until, Point};
use crate::browser::{js, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

const INLINE: &str = "wide_navigation_rail";
const MODAL: &str = "modal_wide_navigation_rail";
const TOGGLE: &str = "wide-rail-toggle";
const RAIL: &str = "wide-rail";
const PAGE: &str = "wide-rail-content";
const BOTTOM: &str = "wide-rail-bottom";
const INBOX: &str = "wide-rail-inbox";
const DIALOG: &str = "modal-wide-rail";
const OPEN: &str = "modal-wide-rail-open";
const CLOSE: &str = "modal-wide-rail-close";
const HEADER: &str = "modal-wide-rail-header";
const HOME: &str = "modal-wide-rail-home";
const MODAL_INBOX: &str = "modal-wide-rail-inbox";
const NAVIGATION: &str = "modal-wide-rail-navigation";
const CONTENT: &str = "modal-wide-rail-content";

async fn model(tab: &Tab, family: &str) -> Result<Value> {
    Ok(state(tab).await?[family].clone())
}

async fn ready(tab: &Tab, family: &str, predicate: &str) -> Result<()> {
    until(tab, &format!("(r => {predicate})(s[{}])", js(family))).await?;
    tab.settle().await
}

async fn viewport(tab: &Tab, width: u32, height: u32, scale: u32) -> Result<()> {
    tab.cdp(
        "Emulation.setDeviceMetricsOverride",
        json!({"width":width,"height":height+44,"deviceScaleFactor":scale,"mobile":false}),
    )
    .await?;
    tab.wait_for(&format!(
        "(() => {{
            const c = document.querySelector('canvas');
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return c.width === {pixel_width} && c.height === {pixel_height} &&
                   v?.width === {width} && v.height === {height};
        }})()",
        pixel_width = width * scale,
        pixel_height = height * scale,
    ))
    .await?;
    tab.settle().await
}

async fn reveal(tab: &Tab, id: &str) -> Result<()> {
    let sample = point(tab, id).await?;
    if sample.b[1] < 8. || sample.b[1] + sample.b[3] > sample.canvas[3] - 8. {
        let scale = tab.eval("devicePixelRatio").await?.as_f64().unwrap_or(1.);
        // The page margin is outside nested scroll containers.
        tab.mouse_move(sample.canvas[0] + 10., sample.canvas[1] + 26.)
            .await?;
        tab.wheel(0., (sample.b[1] - 32.) * scale).await?;
    }
    tab.wait_for(&format!(
        "(() => {{
            const [x,y,w,h] = window.gallery.control_bounds({});
            const c = document.querySelector('canvas').getBoundingClientRect();
            return w > 0 && h > 0 && x >= -1 && y >= -1 &&
                x+w <= c.width+1 && y+h <= c.height+1;
        }})()",
        js(id),
    ))
    .await?;
    tab.settle().await
}

async fn modes(
    tab: &Tab,
    family: &str,
    dark: bool,
    rtl: bool,
    scale: u8,
    retain: bool,
) -> Result<()> {
    if state(tab).await?["section"] != 1 {
        reveal(tab, "section-1").await?;
        click(tab, "section-1").await?;
        until(tab, "s.section === 1").await?;
    }
    if state(tab).await?["dark"] != dark {
        reveal(tab, "theme").await?;
        click(tab, "theme").await?;
        until(tab, &format!("s.dark === {dark}")).await?;
    }
    let key = if family == MODAL {
        "modal-wide-rail"
    } else {
        "wide-rail"
    };
    let current = model(tab, family).await?;
    for (changed, control, predicate) in [
        (
            current["rtl"] != rtl,
            "direction",
            format!("r.rtl === {rtl}"),
        ),
        (
            current["text_scale"].as_f64() != Some(f64::from(scale)),
            "scale",
            format!("r.text_scale === {scale}"),
        ),
        (
            family == MODAL && current["retain_inline"] != retain,
            "retain",
            format!("r.retain_inline === {retain}"),
        ),
    ] {
        if changed {
            let id = format!("{key}-{control}");
            reveal(tab, &id).await?;
            click(tab, &id).await?;
            ready(tab, family, &predicate).await?;
        }
    }
    Ok(())
}

fn element(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn projected_focus(tab: &Tab, id: &str) -> Result<()> {
    focus(tab, id).await?;
    tab.wait_for(&format!("document.activeElement === {}", element(id)))
        .await
}

async fn project(tab: &Tab, id: &str) -> Result<()> {
    let element = element(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    projected_focus(tab, id).await
}

fn node<'a>(dom: &'a Value, id: &str) -> Result<&'a Value> {
    dom["dom"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["stable_id"] == id))
        .with_context(|| format!("missing DOM node {id}"))
}

async fn export(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

async fn png(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.screenshot(&env.artifact(&format!("material-navigation-rail-{name}.png")))
        .await
}

async fn start_clock(tab: &Tab) -> Result<()> {
    // Pin the host clock while preserving actual Chrome input and rendering.
    tab.eval("(() => {
        const original = performance.now.bind(performance);
        const start = original();
        window.railClock = {original, start, now:start};
        Object.defineProperty(performance, 'now', {configurable:true, value:() => window.railClock.now});
    })()").await?;
    Ok(())
}

async fn clock_at(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!(
        "window.railClock.now = window.railClock.start + {ms}"
    ))
    .await?;
    tab.settle().await
}

async fn restore_clock(tab: &Tab) -> Result<()> {
    tab.eval(
        "new Promise(resolve => {
        const clock = window.railClock;
        const restore = () => {
            if (clock.original() < clock.now) requestAnimationFrame(restore);
            else { delete performance.now; delete window.railClock; resolve(); }
        };
        restore();
    })",
    )
    .await?;
    Ok(())
}

async fn inline(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M24",
        "inline rail reflows, keeps one selection, and hands focus to the compact bar",
    );
    viewport(tab, 960, 760, 2).await?;
    modes(tab, INLINE, false, false, 1, false).await?;
    reveal(tab, TOGGLE).await?;
    let collapsed = point(tab, RAIL).await?;
    let page = point(tab, PAGE).await?;
    check_eq!(collapsed.b[2], 96., "collapsed width");
    png(env, tab, "inline-collapsed").await?;

    project(tab, TOGGLE).await?;
    tab.press("Tab").await?;
    projected_focus(tab, "wide-rail-home").await?;
    tab.press("ArrowDown").await?;
    projected_focus(tab, INBOX).await?;
    check_eq!(
        model(tab, INLINE).await?["actions"],
        0,
        "roving skips Archive without selection"
    );
    tab.press("Enter").await?;
    ready(
        tab,
        INLINE,
        "r.selected === 'wide-rail-inbox' && r.actions === 1",
    )
    .await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-current') === 'page'",
        element(INBOX)
    ))
    .await?;

    project(tab, TOGGLE).await?;
    start_clock(tab).await?;
    tab.press("Enter").await?;
    ready(tab, INLINE, "r.expanded && r.settling && r.rail === 0").await?;
    clock_at(tab, 40).await?;
    ready(
        tab,
        INLINE,
        "r.expanded && r.settling && r.rail > .2 && r.rail < .4",
    )
    .await?;
    let moving = point(tab, RAIL).await?;
    check!(
        (105. ..=240.).contains(&moving.b[2]),
        "measured interior width: {:?}",
        moving.b
    );
    check_eq!(
        page.b[2] - point(tab, PAGE).await?.b[2],
        moving.b[2] - 96.,
        "page reflows by rail width"
    );
    let dom = export(tab).await?;
    check_eq!(
        node(&dom, "wide-rail-inbox-motion-label")?["text"],
        "跨部門協作與專案通知收件匣",
        "moving label remains complete"
    );
    png(env, tab, "inline-40ms").await?;
    clock_at(tab, 500).await?;
    ready(tab, INLINE, "r.expanded && !r.settling && r.rail === 1").await?;
    let expanded = point(tab, RAIL).await?;
    check!(
        expanded.b[2] >= 220. && expanded.b[2] > moving.b[2],
        "expanded rail settles wider"
    );
    check_eq!(
        point(tab, PAGE).await?.b[2] + expanded.b[2],
        page.b[2] + 96.,
        "page and rail share the same width"
    );
    restore_clock(tab).await?;

    modes(tab, INLINE, true, true, 2, false).await?;
    reveal(tab, TOGGLE).await?;
    let rtl = point(tab, RAIL).await?;
    check!(
        point(tab, PAGE).await?.b[0] < rtl.b[0],
        "RTL places the rail on the right"
    );
    check_eq!(
        model(tab, INLINE).await?["text_scale"].as_f64(),
        Some(2.),
        "rail at 200% text"
    );
    png(env, tab, "inline-rtl-200-expanded").await?;
    project(tab, TOGGLE).await?;
    viewport(tab, 320, 240, 1).await?;
    ready(
        tab,
        INLINE,
        "r.mode === 'bottom' && r.selected === 'wide-rail-inbox'",
    )
    .await?;
    focus(tab, INBOX).await?;
    reveal(tab, BOTTOM).await?;
    let bottom = point(tab, BOTTOM).await?;
    let home = point(tab, "wide-rail-home").await?;
    check_eq!(bottom.canvas[2..], [320., 240.], "true compact canvas");
    check!(
        home.b[0] > point(tab, INBOX).await?.b[0],
        "bottom bar follows RTL order"
    );
    check_eq!(
        model(tab, INLINE).await?["actions"],
        1,
        "handoff does not activate another destination"
    );
    png(env, tab, "inline-rtl-320-bottom").await?;
    Ok(
        json!({"collapsed":collapsed.b,"moving":moving.b,"expanded":expanded.b,"rtl":rtl.b,
              "bottom":bottom.b,"restored_focus":INBOX,"state":model(tab,INLINE).await?}),
    )
}

#[derive(Clone, Copy)]
enum Input {
    Mouse,
    Touch,
}

impl Input {
    async fn configure(self, tab: &Tab, enabled: bool) -> Result<()> {
        if matches!(self, Self::Touch) {
            tab.cdp(
                "Emulation.setTouchEmulationEnabled",
                json!({"enabled":enabled,"maxTouchPoints":2}),
            )
            .await?;
        }
        Ok(())
    }

    async fn start(self, tab: &Tab, (x, y): (f64, f64)) -> Result<()> {
        match self {
            Self::Mouse => {
                tab.mouse_move(x, y).await?;
                tab.mouse_down().await
            }
            Self::Touch => touch(tab, "touchStart", Some((x, y))).await,
        }
    }

    async fn drag(self, tab: &Tab, (x, y): (f64, f64)) -> Result<()> {
        match self {
            Self::Mouse => mouse(tab, "mouseMoved", x, y, 1).await,
            Self::Touch => touch(tab, "touchMove", Some((x, y))).await,
        }
    }

    async fn release(self, tab: &Tab, (x, y): (f64, f64)) -> Result<()> {
        // Let the velocity window expire so distance decides the anchor.
        tokio::time::sleep(Duration::from_millis(140)).await;
        match self {
            Self::Mouse => mouse(tab, "mouseReleased", x, y, 0).await,
            Self::Touch => touch(tab, "touchEnd", None).await,
        }
    }
}

async fn mouse(tab: &Tab, phase: &str, x: f64, y: f64, buttons: u8) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":phase,"x":x,"y":y,"buttons":buttons,
        "button":"left","clickCount":1}),
    )
    .await?;
    Ok(())
}

async fn touch(tab: &Tab, phase: &str, p: Option<(f64, f64)>) -> Result<()> {
    let touches: Vec<_> = p
        .into_iter()
        .map(|(x, y)| json!({"x":x,"y":y,"id":1}))
        .collect();
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":phase,"touchPoints":touches}),
    )
    .await?;
    Ok(())
}

fn origin(hit: &Point, rtl: bool) -> (f64, f64) {
    (
        hit.canvas[0] + hit.b[0] + hit.b[2] * if rtl { 0.16 } else { 0.84 },
        hit.y,
    )
}

fn travel((x, y): (f64, f64), span: f64, ratio: f64, rtl: bool) -> (f64, f64) {
    (x + span * ratio * if rtl { 1. } else { -1. }, y)
}

async fn endpoint(tab: &Tab, open: bool) -> Result<()> {
    let n = u8::from(open);
    ready(tab, MODAL, &format!("r.open === {open} && !r.settling && !r.swipe_settling && r.swipe === null && r.slide === {n} && r.scrim === {n}")).await
}

async fn open(tab: &Tab) -> Result<()> {
    reveal(tab, OPEN).await?;
    click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    focus(tab, CLOSE).await
}

async fn closed(tab: &Tab) -> Result<()> {
    endpoint(tab, false).await?;
    focus(tab, OPEN).await
}

async fn held(tab: &Tab, fraction: f64) -> Result<()> {
    ready(tab, MODAL, &format!("r.open && !r.swipe_settling && typeof r.swipe === 'number' && Math.abs(r.swipe - {fraction}) < .04")).await
}

async fn sample(tab: &Tab, x: f64, y: f64) -> Result<[u8; 3]> {
    // Decode a pixel captured from the WebGPU output using Chrome's PNG decoder.
    let png = tab
        .cdp(
            "Page.captureScreenshot",
            json!({"format":"png","captureBeyondViewport":false,
        "clip":{"x":x,"y":y,"width":1,"height":1,"scale":1}}),
        )
        .await?;
    let encoded = png["data"].as_str().context("Chrome pixel PNG")?;
    let colors = tab
        .eval(&format!(
            "(async () => {{
        const image = new Image();
        image.src = 'data:image/png;base64,' + {};
        await image.decode();
        const context = new OffscreenCanvas(1,1).getContext('2d');
        context.drawImage(image,0,0,1,1);
        return Array.from(context.getImageData(0,0,1,1).data).slice(0,3);
    }})()",
            js(encoded)
        ))
        .await?;
    Ok(serde_json::from_value(colors)?)
}

fn error(a: [u8; 3], b: [u8; 3]) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(x, y)| u16::from(x.abs_diff(y)))
        .sum()
}

fn counts(state: &Value) -> Result<(u64, u64)> {
    Ok((
        state["actions"].as_u64().context("actions")?,
        state["dismissals"].as_u64().context("dismissals")?,
    ))
}

async fn swipe(env: &Env, tab: &Tab, input: Input, rtl: bool) -> Result<Value> {
    modes(tab, MODAL, rtl, rtl, 1, false).await?;
    reveal(tab, OPEN).await?;
    let old = model(tab, MODAL).await?;
    let [left, top, width, height] = tab.canvas_box().await?;
    let (x, y) = (left + width * 0.5, top + height * 0.65);
    let plain = sample(tab, x, y).await?;
    let page = point(tab, CONTENT).await?;
    open(tab).await?;
    let full = sample(tab, x, y).await?;
    let panel = point(tab, DIALOG).await?;
    let width = panel.b[2];
    let logical_x = if rtl { panel.canvas[2] - width } else { 0. };
    check_eq!(panel.b[0], logical_x, "standalone opens at logical start");
    let hand = origin(&point(tab, HOME).await?, rtl);
    input.configure(tab, true).await?;
    input.start(tab, hand).await?;
    let short = travel(hand, width, 0.29, rtl);
    input.drag(tab, short).await?;
    held(tab, 0.71).await?;
    let preview = point(tab, DIALOG).await?;
    let progress = model(tab, MODAL).await?["swipe"]
        .as_f64()
        .context("swipe progress")?;
    let moved = (1. - progress) * width * if rtl { 1. } else { -1. };
    check!(
        (preview.b[0] - panel.b[0] - moved).abs() <= 1. && preview.b[1..] == panel.b[1..],
        "held panel translates with captured width: {:?}",
        preview.b
    );
    check_eq!(
        point(tab, CONTENT).await?.b,
        page.b,
        "modal does not reflow the page"
    );
    let dom = export(tab).await?;
    let alpha = node(&dom, "modal-wide-rail-layer")?["style"]["background"][3]
        .as_f64()
        .context("scrim alpha")?;
    check_eq!(alpha, (82. * progress).round(), "scrim alpha follows panel");
    let observed = sample(tab, x, y).await?;
    let mix = alpha / 82.;
    let expected = std::array::from_fn(|i| {
        (f64::from(plain[i]) * (1. - mix) + f64::from(full[i]) * mix).round() as u8
    });
    check!(error(observed,expected) <= 6 && error(observed,plain) > 6 && error(observed,full) > 3,
        "held scrim pixel transfer: {observed:?}, expected {expected:?}, plain {plain:?}, full {full:?}");
    png(
        env,
        tab,
        if rtl {
            "modal-touch-rtl-held"
        } else {
            "modal-mouse-ltr-held"
        },
    )
    .await?;
    input.release(tab, short).await?;
    endpoint(tab, true).await?;
    check_eq!(
        sample(tab, x, y).await?,
        full,
        "short release returns to full scrim"
    );
    check_eq!(
        counts(&model(tab, MODAL).await?)?,
        counts(&old)?,
        "short release does not select or dismiss"
    );

    input.start(tab, hand).await?;
    let long = travel(hand, width, 0.66, rtl);
    input.drag(tab, long).await?;
    held(tab, 0.34).await?;
    input.release(tab, long).await?;
    closed(tab).await?;
    input.configure(tab, false).await?;
    let (actions, dismissals) = counts(&old)?;
    let after = model(tab, MODAL).await?;
    check_eq!(
        counts(&after)?,
        (actions, dismissals + 1),
        "long release dismisses once without activating pressed row"
    );
    check_eq!(
        after["selected"],
        old["selected"],
        "destination stays selected"
    );
    Ok(
        json!({"rtl":rtl,"settled":panel.b,"held":preview.b,"fraction":progress,"alpha":alpha,
        "scrim":{"canvas_point":[x-left,y-top],"plain":plain,"full":full,"observed":observed,"expected":expected},
        "restored_focus":OPEN}),
    )
}

async fn compact_modal(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M26",
        "modal rail scrolls at 320×240 with 200% text and supports retained layout",
    );
    modes(tab, MODAL, true, true, 2, false).await?;
    open(tab).await?;
    viewport(tab, 320, 240, 1).await?;
    focus(tab, CLOSE).await?;
    let panel = point(tab, DIALOG).await?;
    check_eq!(
        panel.b,
        [0., 0., 320., 240.],
        "compact panel matches canvas"
    );
    let page = point(tab, CONTENT).await?;
    let navigation = point(tab, NAVIGATION).await?;
    let header = point(tab, HEADER).await?;
    let before = model(tab, MODAL).await?;
    check_eq!(
        before["text_scale"].as_f64(),
        Some(2.),
        "compact modal at 200% text"
    );
    let at = (navigation.x, navigation.y);
    Input::Touch.configure(tab, true).await?;
    Input::Touch.start(tab, at).await?;
    Input::Touch.drag(tab, (at.0, at.1 - 54.)).await?;
    Input::Touch.release(tab, (at.0, at.1 - 54.)).await?;
    Input::Touch.configure(tab, false).await?;
    check_eq!(
        counts(&model(tab, MODAL).await?)?,
        counts(&before)?,
        "vertical touch sends no activation or dismissal"
    );
    check_eq!(
        model(tab, MODAL).await?["swipe"],
        Value::Null,
        "vertical touch does not claim horizontal drag"
    );
    tab.mouse_move(navigation.x, navigation.y).await?;
    tab.wheel(0., 112.).await?;
    ready(tab, MODAL, "r.scroll > 0").await?;
    let wheel = model(tab, MODAL).await?["scroll"].clone();
    check_eq!(
        point(tab, HEADER).await?.b,
        header.b,
        "wheel keeps header in place"
    );
    check_eq!(
        point(tab, CONTENT).await?.b,
        page.b,
        "wheel keeps background in place"
    );
    project(tab, CLOSE).await?;
    tab.press("Tab").await?;
    projected_focus(tab, HOME).await?;
    tab.press("ArrowDown").await?;
    projected_focus(tab, MODAL_INBOX).await?;
    let inbox = point(tab, MODAL_INBOX).await?;
    let nav = point(tab, NAVIGATION).await?;
    check!(
        inbox.b[1] >= nav.b[1] && inbox.b[1] + inbox.b[3] <= nav.b[1] + nav.b[3] + 1.,
        "keyboard reveals the wrapped Inbox: {:?} in {:?}",
        inbox.b,
        nav.b
    );
    let keyboard = model(tab, MODAL).await?["scroll"].clone();
    png(env, tab, "modal-rtl-320-focused-inbox").await?;
    tab.press("Escape").await?;
    closed(tab).await?;

    viewport(tab, 960, 760, 2).await?;
    modes(tab, MODAL, false, false, 1, true).await?;
    reveal(tab, OPEN).await?;
    let retained = point(tab, "modal-wide-rail-inline").await?;
    check_eq!(retained.b[2], 96., "retained rail is 96 dp");
    open(tab).await?;
    let begin = model(tab, MODAL).await?;
    let panel = point(tab, DIALOG).await?;
    let at = origin(&point(tab, HEADER).await?, false);
    Input::Mouse.start(tab, at).await?;
    let end = travel(at, panel.b[2], 0.65, false);
    Input::Mouse.drag(tab, end).await?;
    Input::Mouse.release(tab, end).await?;
    endpoint(tab, true).await?;
    check_eq!(
        counts(&model(tab, MODAL).await?)?,
        counts(&begin)?,
        "retained mode does not arm the standalone swipe"
    );
    check_eq!(
        point(tab, DIALOG).await?.b,
        panel.b,
        "retained modal stays in place"
    );
    tab.press("Escape").await?;
    closed(tab).await?;
    Ok(
        json!({"compact":[320,240],"wheel_scroll":wheel,"key_scroll":keyboard,
              "inbox":inbox.b,"retained":retained.b}),
    )
}

async fn accessibility(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M27",
        "Chrome keeps a named modal during drag and restores focus with Reduce Motion",
    );
    modes(tab, MODAL, false, false, 1, false).await?;
    reveal(tab, OPEN).await?;
    project(tab, OPEN).await?;
    tab.press("Enter").await?;
    endpoint(tab, true).await?;
    projected_focus(tab, CLOSE).await?;
    let panel = point(tab, DIALOG).await?;
    let at = origin(&point(tab, HEADER).await?, false);
    let old = model(tab, MODAL).await?;
    let short = travel(at, panel.b[2], 0.10, false);
    Input::Mouse.start(tab, at).await?;
    Input::Mouse.drag(tab, short).await?;
    held(tab, 0.90).await?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let nodes = tree["nodes"]
        .as_array()
        .context("Chrome accessibility tree")?;
    let exposed = |role: &str, name: &str| {
        nodes.iter().find(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"] == name
        })
    };
    let dialog = exposed("dialog", "模態導覽列").context("named modal")?;
    check!(
        dialog["properties"].as_array().is_some_and(|p| p
            .iter()
            .any(|item| item["name"] == "modal" && item["value"]["value"] == true)),
        "Chrome marks the dialog modal"
    );
    check!(
        exposed("navigation", "工作區目的地").is_some(),
        "Chrome exposes the named navigation"
    );
    tab.wait_for(&format!(
        "{} === null && {}?.getAttribute('aria-current') === 'page' &&
        {}?.getAttribute('aria-disabled') === 'true'",
        element(OPEN),
        element(HOME),
        element("modal-wide-rail-archive")
    ))
    .await?;
    focus(tab, CLOSE).await?;
    tab.press("Tab").await?;
    endpoint(tab, true).await?;
    focus(tab, HOME).await?;
    Input::Mouse.release(tab, short).await?;
    check_eq!(
        counts(&model(tab, MODAL).await?)?,
        counts(&old)?,
        "Tab cancels the swipe and stays in the modal"
    );

    let long = travel(at, panel.b[2], 0.65, false);
    Input::Mouse.start(tab, at).await?;
    Input::Mouse.drag(tab, long).await?;
    held(tab, 0.35).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    tab.wait_for("matchMedia('(prefers-reduced-motion: reduce)').matches")
        .await?;
    tab.settle().await?;
    start_clock(tab).await?;
    Input::Mouse.release(tab, long).await?;
    closed(tab).await?;
    restore_clock(tab).await?;
    let dom = export(tab).await?;
    check!(
        node(&dom, DIALOG).is_err() && node(&dom, "modal-wide-rail-closing").is_err(),
        "Reduce Motion retires paint immediately"
    );
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-expanded') === 'false'",
        element(OPEN)
    ))
    .await?;
    let (actions, dismissals) = counts(&old)?;
    check_eq!(
        counts(&model(tab, MODAL).await?)?,
        (actions, dismissals + 1),
        "reduced motion releases exactly one dismissal"
    );
    png(env, tab, "modal-focus-restored").await?;
    check!(
        tab.errors().is_empty(),
        "rail page errors: {:?}",
        tab.errors()
    );
    Ok(json!({"held_dialog":dialog,"held_focus":CLOSE,"tab_focus":HOME,"restored_focus":OPEN}))
}

pub(super) async fn journey(env: &Env, tab: &Tab) -> Result<()> {
    tab.cdp("Accessibility.enable", json!({})).await?;
    let inline = inline(env, tab).await?;
    step(
        "M25",
        "standalone modal follows mouse and touch swipes and releases at the right anchor",
    );
    viewport(tab, 960, 760, 2).await?;
    let ltr = swipe(env, tab, Input::Mouse, false).await?;
    let rtl = swipe(env, tab, Input::Touch, true).await?;
    let compact = compact_modal(env, tab).await?;
    let ax = accessibility(env, tab).await?;
    env.write_json(
        "browser-material-navigation-rail.json",
        &json!({"inline":inline,
        "swipe_ltr":ltr,"swipe_rtl":rtl,"compact":compact,"accessibility":ax}),
    )
}
