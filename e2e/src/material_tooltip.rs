//! Material tooltips through Chrome WebGPU and trusted pointer and keyboard input.

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

const PLAIN: &str = "tooltip-demo-plain";
const RICH: &str = "tooltip-demo-rich";
const PLAIN_POPUP: &str = "tooltip-demo-plain-popup";
const RICH_POPUP: &str = "tooltip-demo-rich-popup";
const BODY: &str = "tooltip-demo-body-action";
const ACTION: &str = "tooltip-demo-action";
const DESCRIPTION: &str = "複製目前所選的文字。";

type Rect = [f64; 4];

async fn tooltip(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["tooltip"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.tooltip)")).await?;
    tab.settle().await
}

async fn shown(tab: &Tab, id: &str) -> Result<()> {
    ready(tab, &format!("d.shown && d.active === {}", js(id))).await
}

async fn hidden(tab: &Tab) -> Result<()> {
    ready(tab, "!d.shown && d.active === null").await
}

async fn select(tab: &Tab, id: &str, condition: &str) -> Result<()> {
    click(tab, id).await?;
    ready(tab, condition).await
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
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
            const viewport = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return viewport?.width === {width} && viewport?.height === {height}
        }})()"
    ))
    .await?;
    tab.settle().await
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .as_slice()
        .try_into()
        .context("four tooltip bounds")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn contains([x, y, w, h]: Rect, [cx, cy, cw, ch]: Rect) -> bool {
    cx >= x - 0.75 && cy >= y - 0.75 && cx + cw <= x + w + 0.75 && cy + ch <= y + h + 0.75
}

async fn position(tab: &Tab, owner: &str, popup: &str, side: &str) -> Result<Rect> {
    let [_, _, viewport_width, viewport_height] = tab.canvas_box().await?;
    let anchor = bounds(tab, owner).await?;
    let tip = bounds(tab, popup).await?;
    check!(
        contains([0., 0., viewport_width, viewport_height], tip),
        "tooltip fits the viewport {tip:?}"
    );
    let center = anchor[0] + anchor[2] / 2.;
    let expected_x = (center - tip[2] / 2.).clamp(0., (viewport_width - tip[2]).max(0.));
    check!(
        near(tip[0], expected_x),
        "tooltip centers on the owner and clamps at the edge"
    );
    let gap = if side == "above" {
        anchor[1] - tip[1] - tip[3]
    } else {
        tip[1] - anchor[1] - anchor[3]
    };
    check!(near(gap, 4.), "{side} tooltip leaves a 4 dp gap, got {gap}");
    check_eq!(
        tooltip(tab).await?["side"],
        json!(side),
        "reported popup side"
    );
    Ok(tip)
}

async fn move_over(tab: &Tab, id: &str) -> Result<()> {
    let target = point(tab, id).await?;
    tab.mouse_move(target.x, target.y).await?;
    tab.settle().await
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    // Keep Chromium events and animation frames real. Only set elapsed time.
    tab.eval(
        "(() => {
        const start = Math.ceil(performance.now());
        window.tooltipTestElapsed = 0;
        Object.defineProperty(performance, 'now', {
            configurable: true, value: () => start + window.tooltipTestElapsed,
        });
    })()",
    )
    .await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.tooltipTestElapsed = {ms}"))
        .await?;
    tab.settle().await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-tooltip-{name}.png")))
        .await
}

async fn description(tab: &Tab) -> Result<Value> {
    let check_link = format!(
        "(() => {{
            const owner = {};
            const ref = owner?.getAttribute('aria-describedby');
            const tip = ref && document.getElementById(ref);
            return tip === {} && tip?.getAttribute('role') === 'tooltip'
                && (tip.textContent.includes({}) || tip.getAttribute('aria-label') === {});
        }})()",
        projected(PLAIN),
        projected(PLAIN_POPUP),
        js(DESCRIPTION),
        js(DESCRIPTION)
    );
    tab.wait_for(&check_link).await?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let anchor = tree["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
                node["ignored"] == false
                    && node["role"]["value"] == "button"
                    && node["name"]["value"] == "複製"
            })
        })
        .context("Chrome accessibility tree includes the Copy button")?;
    check_eq!(
        anchor["description"]["value"],
        json!(DESCRIPTION),
        "Chrome resolves the tooltip as the button description"
    );
    Ok(anchor["description"].clone())
}

async fn plain(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M69",
        "passive tooltip waits 500 ms, describes its owner, and dismisses without stealing focus",
    );
    focus(tab, BODY).await?;
    pin_clock(tab).await?;
    move_over(tab, PLAIN).await?;
    advance(tab, 499).await?;
    hidden(tab).await?;
    focused(tab, BODY).await?;
    advance(tab, 500).await?;
    shown(tab, PLAIN).await?;
    focused(tab, BODY).await?;
    let description = description(tab).await?;
    let tip = position(tab, PLAIN, PLAIN_POPUP, "above").await?;
    capture(env, tab, "plain-hover").await?;

    let anchor = point(tab, PLAIN).await?;
    let middle = (tip[1] + tip[3] + anchor.b[1]) / 2.;
    tab.mouse_move(anchor.x, anchor.canvas[1] + middle).await?;
    tab.settle().await?;
    shown(tab, PLAIN).await?;
    move_over(tab, PLAIN_POPUP).await?;
    shown(tab, PLAIN).await?;
    focused(tab, BODY).await?;
    move_over(tab, PLAIN).await?;

    tab.press("Escape").await?;
    hidden(tab).await?;
    focused(tab, BODY).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-describedby') === null && {} === null",
        projected(PLAIN),
        projected(PLAIN_POPUP)
    ))
    .await?;
    advance(tab, 1800).await?;
    hidden(tab).await?;
    move_over(tab, BODY).await?;
    move_over(tab, PLAIN).await?;
    advance(tab, 2299).await?;
    hidden(tab).await?;
    advance(tab, 2300).await?;
    shown(tab, PLAIN).await?;
    focused(tab, BODY).await?;
    move_over(tab, BODY).await?;
    hidden(tab).await?;
    tab.eval("delete performance.now").await?;
    Ok(json!({"owner":anchor.b,"popup":tip,"gap_probe_y":middle,
        "threshold_ms":500,"description":description,"dismissed_without_focus_change":true}))
}

async fn rich(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M70",
        "rich tooltip has a nonmodal action, returns focus, and leaves outside actions live",
    );
    focus(tab, RICH).await?;
    shown(tab, RICH).await?;
    let popup = position(tab, RICH, RICH_POPUP, "above").await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('role') === 'dialog' && {}?.getAttribute('aria-label') === '分享設定'
            && {}?.getAttribute('aria-modal') !== 'true' && {} !== null
            && {}?.getAttribute('aria-expanded') === 'true'",
        projected(RICH_POPUP),
        projected(RICH_POPUP),
        projected(RICH_POPUP),
        projected(BODY),
        projected(RICH)
    ))
    .await?;
    focused(tab, RICH).await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    shown(tab, RICH).await?;
    capture(env, tab, "rich-action-focused").await?;
    let before = tooltip(tab).await?;
    let actions = before["actions"].as_u64().context("rich action count")?;
    tab.press("Enter").await?;
    ready(tab, &format!("!d.shown && d.actions === {}", actions + 1)).await?;
    focused(tab, RICH).await?;

    tab.press("Enter").await?;
    shown(tab, RICH).await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Escape").await?;
    hidden(tab).await?;
    focused(tab, RICH).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-expanded') === 'false'",
        projected(RICH)
    ))
    .await?;

    tab.press("Enter").await?;
    shown(tab, RICH).await?;
    let body_actions = before["body_actions"]
        .as_u64()
        .context("outside action count")?;
    click(tab, BODY).await?;
    ready(
        tab,
        &format!("!d.shown && d.body_actions === {}", body_actions + 1),
    )
    .await?;
    check_eq!(
        tooltip(tab).await?["actions"],
        json!(actions + 1),
        "outside click does not trigger the tooltip action"
    );
    Ok(
        json!({"popup":popup,"body_actions":body_actions+1,"actions":actions+1,
        "focus_restored_to":RICH}),
    )
}

async fn surface_pixel(
    env: &Env,
    tab: &Tab,
    name: &str,
    tip: Rect,
    field: &str,
) -> Result<[u8; 3]> {
    capture(env, tab, name).await?;
    let file = File::open(env.artifact(&format!("material-tooltip-{name}.png")))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
    let frame = reader.next_frame(&mut pixels)?;
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => bail!("unexpected Chrome PNG color type {other:?}"),
    };
    let width = tab
        .eval("innerWidth")
        .await?
        .as_f64()
        .context("window width")?;
    let scale = f64::from(frame.width) / width;
    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    let x = ((canvas_x + tip[0] + tip[2] / 2.) * scale).floor() as usize;
    let y = ((canvas_y + tip[1] + 2.) * scale).floor() as usize;
    check!(
        x < frame.width as usize && y < frame.height as usize,
        "tooltip paint probe fits the PNG"
    );
    let offset = y * frame.line_size + x * channels;
    let pixel: [u8; 3] = pixels[offset..offset + 3].try_into()?;
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(tooltip(tab).await?[field].clone())?;
    let distance = pixel
        .into_iter()
        .zip([r, g, b])
        .map(|(actual, expected)| u16::from(actual.abs_diff(expected)))
        .sum::<u16>();
    check!(
        distance <= 8,
        "WebGPU paints {field}: {pixel:?}, expected {:?}",
        [r, g, b]
    );
    Ok(pixel)
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M71",
        "dark 320 dp RTL tooltips flip at the top, clamp at both edges, and wrap text at 200%",
    );
    select(tab, "tooltip-demo-scale", "d.text_scale === 2").await?;
    select(tab, "tooltip-demo-rtl", "d.rtl").await?;
    click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    // Capture the surrounding controls after the short dark-theme transition.
    tokio::time::sleep(Duration::from_millis(250)).await;
    select(tab, "tooltip-demo-top", "d.top").await?;
    resize(tab, 320, 580).await?;
    let mut edges = Vec::new();
    for edge in ["start", "end"] {
        click(tab, &format!("tooltip-demo-{edge}")).await?;
        tab.settle().await?;
        focus(tab, PLAIN).await?;
        shown(tab, PLAIN).await?;
        let tip = position(tab, PLAIN, PLAIN_POPUP, "below").await?;
        let anchor = bounds(tab, PLAIN).await?;
        let rich_anchor = bounds(tab, RICH).await?;
        check!(
            if edge == "start" {
                near(anchor[0] + anchor[2], 312.)
            } else {
                near(rich_anchor[0], 8.)
            },
            "RTL logical {edge} puts the trigger row against its physical edge"
        );
        if edge == "start" {
            check!(
                near(tip[0] + tip[2], 320.),
                "plain tip clamps against the right edge"
            );
        }
        let text_id = format!("{PLAIN_POPUP}-text");
        let export = dom(tab).await?;
        let text_node = find_node(&export, &text_id)?;
        check_eq!(
            text_node["text"],
            json!(DESCRIPTION),
            "localized tooltip text"
        );
        let line = text_node["style"]["line_height"]
            .as_f64()
            .context("tooltip line height")?
            * 2.;
        let text = bounds(tab, &text_id).await?;
        check!(
            contains(tip, text) && text[3] >= 2. * line - 0.75,
            "200% tooltip text wraps inside its measured bubble"
        );
        let caret = bounds(tab, &format!("{PLAIN_POPUP}-caret")).await?;
        check!(
            near(caret[1] + caret[3], tip[1]),
            "flipped caret points up from the tooltip"
        );
        let color = surface_pixel(
            env,
            tab,
            &format!("compact-dark-rtl-{edge}"),
            tip,
            "plain_surface",
        )
        .await?;
        edges.push(
            json!({"edge":edge,"anchor":anchor,"tip":tip,"text":text,"caret":caret,"color":color}),
        );
        tab.press("Escape").await?;
        hidden(tab).await?;
    }

    select(tab, "tooltip-demo-caret", "!d.caret").await?;
    focus(tab, RICH).await?;
    shown(tab, RICH).await?;
    let popup = position(tab, RICH, RICH_POPUP, "below").await?;
    check!(near(popup[0], 0.), "rich tip clamps against the left edge");
    let text = bounds(tab, &format!("{RICH_POPUP}-text")).await?;
    let title = bounds(tab, &format!("{RICH_POPUP}-title")).await?;
    let action = bounds(tab, ACTION).await?;
    check!(
        [text, title, action]
            .into_iter()
            .all(|bounds| contains(popup, bounds)),
        "compact rich popup keeps its title, text and action in view"
    );
    check!(
        find_node(&dom(tab).await?, &format!("{RICH_POPUP}-caret")).is_err(),
        "caret switch removes the visual pointer"
    );
    let color = surface_pixel(env, tab, "compact-dark-rich", popup, "rich_surface").await?;
    tab.press("Escape").await?;
    hidden(tab).await?;
    Ok(
        json!({"edges":edges,"rich":{"bounds":popup,"text":text,"title":title,"action":action,"color":color}}),
    )
}

async fn touch(tab: &Tab, kind: &str, point: Option<(f64, f64)>) -> Result<()> {
    let points = point.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn touch_journey(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M72",
        "trusted touch tap, long press, cancellation, and moved contact have distinct actions",
    );
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    tab.eval(
        "(() => {
        window.tooltipTouchEvents = [];
        for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
            document.addEventListener(kind, event => {
                if (event.target?.id === 'surface' && event.pointerType === 'touch') {
                    window.tooltipTouchEvents.push({kind, trusted:event.isTrusted});
                }
            }, {capture:true,passive:true});
        }
    })()",
    )
    .await?;
    pin_clock(tab).await?;
    let anchor = point(tab, PLAIN).await?;
    let start = Some((anchor.x, anchor.y));
    let away = Some((anchor.x - anchor.b[2] / 2. - 24., anchor.y));
    let before = tooltip(tab).await?["copies"]
        .as_u64()
        .context("copy count")?;

    touch(tab, "touchStart", start).await?;
    advance(tab, 100).await?;
    hidden(tab).await?;
    touch(tab, "touchEnd", None).await?;
    ready(tab, &format!("d.copies === {}", before + 1)).await?;
    hidden(tab).await?;
    focus(tab, BODY).await?;

    advance(tab, 1000).await?;
    touch(tab, "touchStart", start).await?;
    advance(tab, 1499).await?;
    hidden(tab).await?;
    advance(tab, 1500).await?;
    shown(tab, PLAIN).await?;
    capture(env, tab, "touch-held").await?;
    touch(tab, "touchEnd", None).await?;
    shown(tab, PLAIN).await?;
    check_eq!(
        tooltip(tab).await?["copies"],
        json!(before + 1),
        "long press consumes the underlying copy action"
    );
    advance(tab, 2999).await?;
    shown(tab, PLAIN).await?;
    advance(tab, 3000).await?;
    hidden(tab).await?;
    focus(tab, BODY).await?;

    advance(tab, 4000).await?;
    touch(tab, "touchStart", start).await?;
    advance(tab, 4500).await?;
    shown(tab, PLAIN).await?;
    touch(tab, "touchCancel", None).await?;
    hidden(tab).await?;
    check_eq!(
        tooltip(tab).await?["copies"],
        json!(before + 1),
        "canceled long press does not copy"
    );
    focus(tab, BODY).await?;

    advance(tab, 5000).await?;
    touch(tab, "touchStart", start).await?;
    advance(tab, 5100).await?;
    touch(tab, "touchMove", away).await?;
    advance(tab, 5600).await?;
    hidden(tab).await?;
    touch(tab, "touchEnd", None).await?;
    hidden(tab).await?;
    check_eq!(
        tooltip(tab).await?["copies"],
        json!(before + 1),
        "off-anchor contact neither opens nor copies"
    );

    let events = tab.eval("window.tooltipTouchEvents").await?;
    let observed = events.as_array().context("trusted Chrome touch events")?;
    for kind in ["pointerdown", "pointermove", "pointerup", "pointercancel"] {
        check!(
            observed
                .iter()
                .any(|event| event["kind"] == kind && event["trusted"] == true),
            "Chrome delivered trusted touch {kind}"
        );
    }
    tab.eval("delete performance.now").await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    Ok(json!({"copies":before+1,"hold_ms":500,"visible_after_release_ms":1500,"events":events}))
}

async fn start_gallery(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 760).await?;
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    click(tab, "tooltip-demo-show").await?;
    hidden(tab).await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    start_gallery(tab).await?;
    let plain = plain(env, tab).await?;
    // Start the interactive scenario with a fresh browser clock and runtime.
    tab.goto(url).await?;
    start_gallery(tab).await?;
    let rich = rich(env, tab).await?;
    let compact = compact(env, tab).await?;
    tab.goto(url).await?;
    start_gallery(tab).await?;
    let touch = touch_journey(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "tooltip page errors: {errors:?}");
    env.write_json(
        "browser-material-tooltip.json",
        &json!({
            "browser":chrome.version,"plain":plain,"rich":rich,"compact":compact,
            "touch":touch,"errors":errors,
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
        let _ = env.write_json("material-tooltip-failure.json", &dump);
        let _ = capture(env, &tab, "failure").await;
    }
    result
}
