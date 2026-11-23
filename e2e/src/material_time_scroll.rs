//! Expressive Scroll Time Picker on the real Chrome WebGPU host.

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

const INLINE_HOUR: &str = "time-scroll-inline-hour-wheel";
const INLINE_MINUTE: &str = "time-scroll-inline-minute-wheel";
const HOUR: &str = "time-scroll-modal-hour-wheel";
const MINUTE: &str = "time-scroll-modal-minute-wheel";
const HOUR_INPUT: &str = "time-scroll-modal-hour-input";
const MINUTE_INPUT: &str = "time-scroll-modal-minute-input";
const USE_INPUT: &str = "time-scroll-modal-use-input";
const USE_SCROLL: &str = "time-scroll-modal-use-scroll";
const PAGE: &str = "time-scroll-demo-page";
const OPEN: &str = "time-scroll-demo-open";
const INLINE_SAVE: &str = "time-scroll-demo-confirm-inline";
const OUTSIDE: &str = "time-scroll-demo-outside";
const MODAL: &str = "time-scroll-modal";
const CONFIRM: &str = "time-scroll-modal-confirm";
const CANCEL: &str = "time-scroll-modal-cancel";
const AM: &str = "time-scroll-modal-am";
const PM: &str = "time-scroll-modal-pm";

type Position = (f64, f64);
type Rect = [f64; 4];

async fn read(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["time_scroll"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.time_scroll)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1244).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1200).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "show-time-scroll-demo").await?;
    ready(
        tab,
        "d.inline.mode === 'Scroll' && d.inline.candidate === '09:07' &&
         d.saved === '09:07' && d.inline_saved === '09:07' && d.modal === null",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("time-scroll-demo-{name}")).await
}

async fn open(tab: &Tab) -> Result<()> {
    activate(tab, OPEN).await?;
    ready(tab, "d.modal?.mode === 'Scroll'").await?;
    focused(tab, HOUR).await
}

async fn closed(tab: &Tab, saved: &str, commits: u8) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.modal === null && d.saved === {} && d.commits === {commits} &&
             d.outside_actions === 0",
            js(saved)
        ),
    )
    .await?;
    until(tab, &format!("s.focused === {}", js(OPEN))).await?;
    tab.wait_for(&format!(
        "{} === null && {} !== null",
        projected(MODAL),
        projected(OUTSIDE)
    ))
    .await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-time-scroll-{name}.png")))
        .await
}

async fn freeze(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            const start = performance.now();
            window.timeScrollClock = 0;
            Object.defineProperty(performance, 'now', {
                configurable: true, value: () => start + window.timeScrollClock,
            });
        })()",
    )
    .await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.timeScrollClock = {ms}")).await?;
    tab.settle().await
}

async fn reduced(tab: &Tab, value: bool) -> Result<()> {
    let preference = if value { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":preference}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {preference})"))
    ))
    .await?;
    tab.settle().await
}

struct Rotor {
    center: Position,
    row: f64,
    rect: Rect,
}

async fn rotor(tab: &Tab, id: &str) -> Result<Rotor> {
    let target = point(tab, id).await?;
    let rect: Rect = target.b.try_into().ok().context("rotor rectangle")?;
    Ok(Rotor {
        center: (target.x, target.y),
        row: rect[3] / 3.,
        rect,
    })
}

async fn wheel(tab: &Tab, at: Position, deltas: &[f64]) -> Result<()> {
    tab.mouse_move(at.0, at.1).await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("DPR")?;
    // Send the burst with no explicit frame wait between physical samples.
    for delta in deltas {
        tab.wheel(0., delta * scale).await?;
    }
    tab.settle().await
}

async fn page_offset(tab: &Tab) -> Result<f64> {
    let value = tab
        .eval(&format!(
            "JSON.parse(window.gallery.export_dom()).dom.nodes
             .find(node => node.stable_id === {})?.style.scroll_y",
            js(PAGE)
        ))
        .await?;
    value.as_f64().context("page scroll offset")
}

async fn observe_wheel(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            window.timeScrollWheelEvents = [];
            document.getElementById('surface').addEventListener('wheel', event => {
                window.timeScrollWheelEvents.push({
                    x:event.clientX, y:event.clientY, dy:event.deltaY, trusted:event.isTrusted,
                });
            }, {capture:true,passive:true});
        })()",
    )
    .await?;
    Ok(())
}

async fn wheels(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M149",
        "two inline wheels settle independently and the surrounding page still scrolls",
    );
    reduced(tab, false).await?;
    observe_wheel(tab).await?;
    freeze(tab).await?;
    let hour = rotor(tab, INLINE_HOUR).await?;
    let minute = rotor(tab, INLINE_MINUTE).await?;
    check!(
        (100. ..=132.).contains(&hour.rect[2])
            && near(hour.rect[2], minute.rect[2])
            && near(hour.row, 48.)
            && near(minute.row, 48.),
        "two equally sized wide rotors each paint three 48 dp rows: {:?}, {:?}",
        hour.rect,
        minute.rect
    );
    let start = read(tab).await?;
    let initial_page = page_offset(tab).await?;
    let initial_hour = start["inline"]["hour"]["offset"]
        .as_f64()
        .context("Hour offset")?;
    wheel(tab, hour.center, &[hour.row * 0.30, hour.row * 0.30]).await?;
    ready(
        tab,
        "d.inline.hour.preview === 10 && d.inline.hour.accepted === 9 &&
         d.inline.minute.accepted === 7 && d.inline.minute.preview === null",
    )
    .await?;
    let paired = read(tab).await?;
    let moved = paired["inline"]["hour"]["offset"]
        .as_f64()
        .context("burst offset")?;
    check!(
        (0.52..0.70).contains(&(moved - initial_hour)),
        "both rapid samples reached the same rotor: {initial_hour} → {moved}"
    );
    wheel(tab, minute.center, &[minute.row * 1.24]).await?;
    ready(
        tab,
        "d.inline.hour.preview === 10 && d.inline.minute.preview === 8 &&
         d.inline.hour.accepted === 9 && d.inline.minute.accepted === 7 && d.inline_commits === 0",
    )
    .await?;
    let moving = read(tab).await?["inline"].clone();
    check_eq!(
        page_offset(tab).await?,
        initial_page,
        "inside wheel events do not scroll the page"
    );
    advance(tab, 100).await?;
    ready(
        tab,
        "d.inline.candidate === '09:07' && d.inline.hour.preview === 10",
    )
    .await?;
    advance(tab, 112).await?;
    ready(
        tab,
        "d.inline.candidate === '10:08' && d.inline.hour.accepted === 10 &&
         d.inline.minute.accepted === 8 && d.inline_commits === 0",
    )
    .await?;
    advance(tab, 400).await?;
    ready(
        tab,
        "!d.inline.hour.settling && !d.inline.minute.settling &&
         d.inline.hour.preview === null && d.inline.minute.preview === null",
    )
    .await?;
    let settled = read(tab).await?["inline"].clone();
    check_eq!(
        settled["hour"]["offset"],
        json!(9.),
        "Hour snaps to its tenth label"
    );
    check_eq!(
        settled["minute"]["offset"],
        json!(8.),
        "Minute snaps independently"
    );
    spinbutton(tab, INLINE_HOUR, "小時", 1, 12, Some(10), Some("10")).await?;
    spinbutton(tab, INLINE_MINUTE, "分鐘", 0, 59, Some(8), Some("08")).await?;
    capture(env, tab, "inline-independent-wheel").await?;
    activate(tab, INLINE_SAVE).await?;
    ready(
        tab,
        "d.inline_saved === '10:08' && d.inline_commits === 1 && d.commits === 0",
    )
    .await?;

    let outside = point(tab, OUTSIDE).await?;
    let before_outside = page_offset(tab).await?;
    wheel(tab, (outside.x, outside.y), &[72.]).await?;
    let scroll_after = page_offset(tab).await?;
    check!(
        scroll_after > before_outside + 1.,
        "a real wheel event outside both rotors scrolls the page: {before_outside} → {scroll_after}"
    );
    check_eq!(
        read(tab).await?["inline"]["candidate"],
        json!("10:08"),
        "page scroll does not edit either rotor"
    );
    let events = tab.eval("window.timeScrollWheelEvents").await?;
    check_eq!(
        events
            .as_array()
            .context("trusted wheel log")?
            .iter()
            .filter(|event| event["trusted"] == true)
            .count(),
        4,
        "both burst samples, independent Minute and page wheel arrived as trusted events"
    );
    Ok(
        json!({"initial":start["inline"],"paired":paired["inline"],"preview":moving,
        "settled":settled,"page_offsets":[initial_page,before_outside,scroll_after],"trusted_wheels":events}),
    )
}

async fn mouse(tab: &Tab, kind: &str, (x, y): Position, held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,"button":"left","buttons":u8::from(held),"clickCount":1}),
    )
    .await?;
    tab.settle().await
}

async fn touch(tab: &Tab, kind: &str, at: Option<Position>) -> Result<()> {
    let points = at.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn observe_pointers(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            window.timeScrollPointers = [];
            const canvas = document.getElementById('surface');
            for (const kind of ['pointerdown','pointermove','pointerup','gotpointercapture','lostpointercapture']) {
                canvas.addEventListener(kind, event => {
                    if (event.isTrusted) window.timeScrollPointers.push({
                        kind, id:event.pointerId, device:event.pointerType, buttons:event.buttons,
                    });
                }, {capture:true,passive:true});
            }
        })()",
    )
    .await?;
    Ok(())
}

async fn pointer(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M150",
        "trusted pointer preview, capture loss, swipe and adjacent tap choose only on acceptance",
    );
    reduced(tab, true).await?;
    open(tab).await?;
    observe_pointers(tab).await?;
    let hour = rotor(tab, HOUR).await?;
    let (x, y) = hour.center;
    let held = (x, y - hour.row * 1.23);
    mouse(tab, "mousePressed", hour.center, true).await?;
    mouse(tab, "mouseMoved", held, true).await?;
    ready(
        tab,
        "d.modal.hour.preview === 10 && d.modal.hour.accepted === 9 &&
         d.modal.candidate === '09:07' && d.modal.minute.preview === null",
    )
    .await?;
    let preview = read(tab).await?["modal"].clone();
    spinbutton(tab, HOUR, "小時", 1, 12, Some(9), Some("9")).await?;
    capture(env, tab, "modal-held-preview").await?;
    tab.wait_for(
        "window.timeScrollPointers.some(event => event.kind === 'gotpointercapture' && event.device === 'mouse')",
    )
    .await?;
    let owner = tab.eval(
        "window.timeScrollPointers.find(event => event.kind === 'gotpointercapture' && event.device === 'mouse').id",
    ).await?.as_i64().context("mouse capture ID")?;
    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    mouse(tab, "mouseReleased", held, false).await?;
    ready(
        tab,
        "d.modal.candidate === '09:07' && d.modal.hour.accepted === 9 &&
         d.modal.hour.preview === null && !d.modal.hour.settling && d.commits === 0",
    )
    .await?;
    let restored = read(tab).await?["modal"].clone();
    check_eq!(
        restored["hour"]["offset"],
        json!(8.),
        "loss restores the accepted Hour position"
    );

    touch(tab, "touchStart", Some(hour.center)).await?;
    touch(tab, "touchMove", Some(held)).await?;
    ready(
        tab,
        "d.modal.hour.preview === 10 && d.modal.hour.accepted === 9",
    )
    .await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "d.modal.candidate === '10:07' && d.modal.hour.accepted === 10 &&
         d.modal.hour.preview === null && d.commits === 0",
    )
    .await?;
    tab.click(x, y + hour.row).await?;
    ready(
        tab,
        "d.modal.candidate === '11:07' && d.modal.hour.accepted === 11 &&
         d.modal.minute.accepted === 7 && d.commits === 0",
    )
    .await?;
    mouse(tab, "mousePressed", hour.center, true).await?;
    mouse(tab, "mouseMoved", (x + 31., y + 1.), true).await?;
    mouse(tab, "mouseReleased", (x + 31., y + 1.), false).await?;
    ready(
        tab,
        "d.modal.candidate === '11:07' && d.modal.hour.preview === null",
    )
    .await?;

    let events = tab.eval("window.timeScrollPointers").await?;
    let trace = events.as_array().context("trusted pointer trace")?;
    let loss = trace
        .iter()
        .position(|event| {
            event["kind"] == "lostpointercapture"
                && event["device"] == "mouse"
                && event["id"] == owner
        })
        .context("trusted loss from the original contact")?;
    let up = trace
        .iter()
        .position(|event| {
            event["kind"] == "pointerup" && event["device"] == "mouse" && event["id"] == owner
        })
        .context("trusted mouse up")?;
    check!(
        loss < up,
        "unexpected trusted capture loss precedes its late up: {events}"
    );
    for kind in ["pointerdown", "pointermove", "pointerup"] {
        check!(
            trace
                .iter()
                .any(|event| event["device"] == "touch" && event["kind"] == kind),
            "trusted touch {kind} reached the wheel"
        );
    }
    capture(env, tab, "modal-touch-and-tap").await?;
    activate(tab, CANCEL).await?;
    closed(tab, "09:07", 0).await?;
    Ok(
        json!({"preview":preview,"restored":restored,"swipe":"10:07","tap":"11:07",
        "horizontal_kept":"11:07","trusted_contacts":events}),
    )
}

async fn spinbutton(
    tab: &Tab,
    id: &str,
    name: &str,
    minimum: u8,
    maximum: u8,
    expected: Option<u8>,
    text: Option<&str>,
) -> Result<Value> {
    let selector = projected(id);
    let attributes = tab.eval(&format!(
        "(() => {{
            const node = {selector};
            return node && {{
                role:node.getAttribute('role'), label:node.getAttribute('aria-label'),
                minimum:node.getAttribute('aria-valuemin'), maximum:node.getAttribute('aria-valuemax'),
                value:node.getAttribute('aria-valuenow'), text:node.getAttribute('aria-valuetext'),
                children:node.querySelectorAll('[tabindex]').length,
            }};
        }})()"
    )).await?;
    check!(
        attributes["role"] == "spinbutton"
            && attributes["label"] == name
            && attributes["minimum"]
                .as_str()
                .and_then(|value| value.parse().ok())
                == Some(minimum)
            && attributes["maximum"]
                .as_str()
                .and_then(|value| value.parse().ok())
                == Some(maximum)
            && attributes["value"] == json!(expected.map(|value| value.to_string()))
            && attributes["children"] == 0,
        "one named adjustable with passive rows: {id}: {attributes}"
    );
    if let Some(text) = text {
        check_eq!(
            attributes["text"],
            json!(text),
            "the DOM retains the localized value text"
        );
    }
    let tree = ax_tree(tab).await?;
    let exposed = ax(&tree, "spinbutton", name)?;
    check!(
        ax_property(exposed, "focusable") == Some(&json!(true))
            && ax_property(exposed, "valuemin").and_then(Value::as_f64) == Some(f64::from(minimum))
            && ax_property(exposed, "valuemax").and_then(Value::as_f64) == Some(f64::from(maximum)),
        "Chrome exposes a focusable spinbutton with actual bounds: {exposed}"
    );
    let value = exposed["value"]["value"].as_f64();
    let description = &exposed["description"]["value"];
    if let Some(expected) = expected {
        check_eq!(
            value,
            Some(f64::from(expected)),
            format!("Chrome accepted value for {id}")
        );
    } else {
        check!(
            attributes["text"].as_str().is_some_and(|text| !text.trim().is_empty()) &&
            description == &attributes["text"],
            "Chrome exposes a distinct missing-value description even if it supplies implicit zero: {exposed}"
        );
    }
    Ok(json!({"dom":attributes,"chrome_value":value,"chrome_description":description}))
}

async fn replace(tab: &Tab, value: &str) -> Result<()> {
    tab.shortcut("a").await?;
    if value.is_empty() {
        tab.press("Backspace").await?;
    } else {
        tab.insert_text(value).await?;
    }
    tab.settle().await
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M151",
        "projected spinbuttons keep keyboard scope, bounds and a distinct missing value",
    );
    reduced(tab, true).await?;
    open(tab).await?;
    let initial = spinbutton(tab, HOUR, "小時", 1, 12, Some(9), Some("9")).await?;
    check_eq!(
        tab.eval(&format!("{} === null", projected(INLINE_HOUR)))
            .await?,
        json!(true),
        "the inline rotor is absent from the modal accessibility scope"
    );
    tab.press("ArrowDown").await?;
    ready(tab, "d.modal.candidate === '10:07' && d.commits === 0").await?;
    focused(tab, HOUR).await?;
    tab.press("ArrowUp").await?;
    ready(tab, "d.modal.candidate === '09:07' && d.commits === 0").await?;
    tab.press("End").await?;
    ready(tab, "d.modal.hour.accepted === 12").await?;
    tab.press("ArrowDown").await?;
    ready(
        tab,
        "d.modal.candidate === '00:07' && d.modal.hour.accepted === 12 && d.commits === 0",
    )
    .await?;
    focused(tab, HOUR).await?;
    let bound = spinbutton(tab, HOUR, "小時", 1, 12, Some(12), Some("12")).await?;
    tab.press("ArrowRight").await?;
    focused(tab, MINUTE).await?;
    ready(tab, "d.modal.candidate === '00:07' && d.commits === 0").await?;
    tab.press("Shift+Tab").await?;
    focused(tab, HOUR).await?;
    tab.press("Tab").await?;
    focused(tab, MINUTE).await?;
    tab.press("ArrowUp").await?;
    ready(
        tab,
        "d.modal.candidate === '00:06' && d.modal.hour.accepted === 12",
    )
    .await?;
    spinbutton(tab, MINUTE, "分鐘", 0, 59, Some(6), Some("06")).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, HOUR).await?;

    activate(tab, USE_INPUT).await?;
    ready(tab, "d.modal.mode === 'Input'").await?;
    focused(tab, HOUR_INPUT).await?;
    replace(tab, "").await?;
    ready(tab, "d.modal.raw_hour === '' && d.modal.candidate === null").await?;
    click(tab, USE_SCROLL).await?;
    ready(
        tab,
        "d.modal.mode === 'Scroll' && d.modal.raw_hour === '' && d.modal.hour.accepted === null &&
         d.modal.minute.accepted === 6 && d.commits === 0",
    )
    .await?;
    let missing = spinbutton(tab, HOUR, "小時", 1, 12, None, None).await?;
    let missing_text = missing["dom"]["text"]
        .as_str()
        .context("missing Hour text")?;
    check!(
        missing_text == "尚未設定" || missing_text.contains("小時"),
        "the missing Hour has a localized unmet-value description: {missing_text}"
    );
    capture(env, tab, "modal-missing-spinbutton").await?;
    focus_projected(tab, HOUR).await?;
    tab.press("Home").await?;
    ready(
        tab,
        "d.modal.candidate === '01:06' && d.modal.hour.accepted === 1 && d.commits === 0",
    )
    .await?;
    let minimum = spinbutton(tab, HOUR, "小時", 1, 12, Some(1), Some("1")).await?;
    focus_projected(tab, CONFIRM).await?;
    tab.press("Tab").await?;
    focused(tab, HOUR).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, CONFIRM).await?;
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;
    focused(tab, OPEN).await?;
    Ok(
        json!({"initial":initial,"upper_bound":bound,"missing":missing,"minimum":minimum,"modal_tab_loop":true}),
    )
}

fn draft_values(draft: &Value) -> Value {
    json!({
        "candidate":draft["candidate"], "raw_hour":draft["raw_hour"], "raw_minute":draft["raw_minute"],
        "period":draft["period"], "hour":draft["hour"]["accepted"], "minute":draft["minute"]["accepted"],
    })
}

fn target_fits(outer: Rect, target: Rect) -> bool {
    contains(outer, target) && target[2] >= 47.5 && target[3] >= 47.5
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M152",
        "dark RTL at 200% keeps both localized wheels and preserves the draft in compact Input",
    );
    reduced(tab, true).await?;
    preset(tab, "theme").await?;
    preset(tab, "direction").await?;
    preset(tab, "scale").await?;
    preset(tab, "locale").await?;
    resize(tab, 1120, 880).await?;
    open(tab).await?;
    ready(
        tab,
        "d.locale === 'ja' && d.rtl && d.text_scale === 2 && d.modal.candidate === '09:07'",
    )
    .await?;
    let wide = bounds(tab, MODAL).await?;
    let hour = bounds(tab, HOUR).await?;
    let minute = bounds(tab, MINUTE).await?;
    let wide_period_targets = [bounds(tab, AM).await?, bounds(tab, PM).await?];
    check!(
        contains([0., 0., 1120., 880.], wide)
            && target_fits(wide, hour)
            && target_fits(wide, minute)
            && hour[0] > minute[0]
            && near(hour[2], minute[2]),
        "both equally sized rotors fit in dark RTL: {wide:?}, {hour:?}, {minute:?}"
    );
    check!(
        wide_period_targets
            .iter()
            .all(|target| target_fits(wide, *target) && target[2] >= 71.5 && near(target[3], 56.)),
        "both Japanese period labels fit a single line in dark RTL: {wide_period_targets:?}"
    );
    tab.press("ArrowLeft").await?;
    focused(tab, MINUTE).await?;
    tab.press("ArrowDown").await?;
    ready(tab, "d.modal.candidate === '09:08' && d.commits === 0").await?;
    focused(tab, MINUTE).await?;
    spinbutton(tab, MINUTE, "分", 0, 59, Some(8), Some("０８")).await?;
    let draft = draft_values(&read(tab).await?["modal"]);
    capture(env, tab, "modal-wide-dark-rtl").await?;

    resize(tab, 320, 240).await?;
    ready(
        tab,
        "d.modal.mode === 'Input' && !d.modal.fits && d.modal.candidate === '09:08' && d.commits === 0",
    )
    .await?;
    focused(tab, MINUTE_INPUT).await?;
    check_eq!(
        draft_values(&read(tab).await?["modal"]),
        draft,
        "forced Input preserves all draft values"
    );
    for id in [HOUR, MINUTE] {
        check_eq!(
            tab.eval(&format!("window.gallery.control_bounds({}).length", js(id)))
                .await?,
            json!(0),
            "the compact fallback removes each rotor's paint"
        );
    }
    let panel = bounds(tab, MODAL).await?;
    let actions = [bounds(tab, CONFIRM).await?, bounds(tab, CANCEL).await?];
    check!(
        contains([0., 0., 320., 240.], panel)
            && actions.iter().all(|target| target_fits(panel, *target)),
        "the Japanese dialog and complete fixed actions fit at 320 by 240: {panel:?}, {actions:?}"
    );
    capture(env, tab, "compact-dark-rtl-input").await?;
    resize(tab, 1120, 880).await?;
    ready(
        tab,
        "d.modal.mode === 'Input' && d.modal.fits && d.modal.candidate === '09:08'",
    )
    .await?;
    activate(tab, USE_SCROLL).await?;
    ready(
        tab,
        "d.modal.mode === 'Scroll' && d.modal.candidate === '09:08'",
    )
    .await?;
    check_eq!(
        draft_values(&read(tab).await?["modal"]),
        draft,
        "explicit return keeps every draft field"
    );
    spinbutton(tab, MINUTE, "分", 0, 59, Some(8), Some("０８")).await?;
    activate(tab, CANCEL).await?;
    closed(tab, "09:07", 0).await?;
    Ok(
        json!({"wide_panel":wide,"wide_rotors":[hour,minute],"draft":draft,
        "wide_period_targets":wide_period_targets,
        "compact_panel":panel,"compact_actions":actions,"minute_focus_retained":true}),
    )
}

async fn input_and_save(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M153",
        "IME Enter stays in Input, and explicit Save resolves queued wheel input once",
    );
    reduced(tab, false).await?;
    preset(tab, "clock").await?;
    ready(tab, "d.clock === 'TwentyFourHour'").await?;
    open(tab).await?;
    check_eq!(
        tab.eval(&format!("{} === null", projected(AM))).await?,
        json!(true),
        "the 24-hour wheel has no period target"
    );
    spinbutton(tab, HOUR, "小時", 0, 23, Some(9), Some("09")).await?;
    activate(tab, USE_INPUT).await?;
    ready(tab, "d.modal.mode === 'Input'").await?;
    focused(tab, HOUR_INPUT).await?;
    tab.press("Tab").await?;
    focused(tab, MINUTE_INPUT).await?;
    tab.shortcut("a").await?;
    tab.ime_compose("５９", 2, 2).await?;
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(MINUTE_INPUT)
    );
    tab.wait_for(&composing)
        .await
        .context("the Minute editor receives preedit before the mode button is pressed")?;
    click(tab, USE_SCROLL).await?;
    ready(
        tab,
        "d.modal.mode === 'Input' && d.modal.candidate === '09:07' && d.commits === 0",
    )
    .await?;
    tab.wait_for(&composing)
        .await
        .context("a trusted press on the mode button preserves Minute preedit")?;
    focused(tab, MINUTE_INPUT).await?;
    capture(env, tab, "modal-ime-preedit").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.modal.mode === 'Input' && d.saved === '09:07' && d.commits === 0",
    )
    .await?;
    if read(tab).await?["modal"]["raw_minute"] == "07" {
        tab.insert_text("５９").await?;
    }
    ready(tab, "d.modal.candidate === '09:59' && d.commits === 0").await?;
    tab.wait_for(&format!("!({composing})")).await?;
    focused(tab, MINUTE_INPUT).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.modal.mode === 'Input' && d.modal.candidate === '09:59' && d.commits === 0",
    )
    .await?;
    activate(tab, USE_SCROLL).await?;
    ready(
        tab,
        "d.modal.mode === 'Scroll' && d.modal.candidate === '09:59' && d.commits === 0",
    )
    .await?;
    spinbutton(tab, MINUTE, "分鐘", 0, 59, Some(59), Some("59")).await?;
    freeze(tab).await?;
    let hour = rotor(tab, HOUR).await?;
    wheel(tab, hour.center, &[hour.row * 1.10]).await?;
    ready(
        tab,
        "d.modal.hour.preview === 10 && d.modal.hour.accepted === 9 &&
         d.modal.candidate === '09:59' && d.commits === 0",
    )
    .await?;
    activate(tab, CONFIRM).await?;
    closed(tab, "10:59", 1).await?;
    check_eq!(
        tab.eval("window.timeScrollClock").await?,
        json!(0),
        "Save resolved the queued wheel before time advanced"
    );
    check_eq!(
        read(tab).await?["inline_commits"],
        json!(0),
        "modal Save does not touch the inline form"
    );
    Ok(
        json!({"mode_press_preserves_preedit":true,"ime_enter_commits":0,
        "queued_hour":10,"saved":"10:59","commits":1}),
    )
}

async fn dismissal(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M154",
        "Cancel, Escape and a real scrim click discard the draft and restore the opener",
    );
    reduced(tab, true).await?;
    for dismiss in ["cancel", "escape", "scrim"] {
        open(tab).await?;
        tab.press("ArrowDown").await?;
        ready(
            tab,
            "d.modal.candidate === '10:07' && d.saved === '09:07' &&
             d.commits === 0 && d.outside_actions === 0",
        )
        .await?;
        match dismiss {
            "cancel" => {
                capture(env, tab, "modal-before-dismiss").await?;
                activate(tab, CANCEL).await?;
            }
            "escape" => tab.press("Escape").await?,
            _ => {
                let panel = bounds(tab, MODAL).await?;
                let outside = bounds(tab, OUTSIDE).await?;
                let center = [
                    outside[0] + outside[2] / 2.,
                    outside[1] + outside[3] / 2.,
                    0.,
                    0.,
                ];
                check!(
                    !contains(panel, center),
                    "the outside action is behind the scrim but outside the panel"
                );
                click(tab, OUTSIDE).await?;
            }
        }
        closed(tab, "09:07", 0).await?;
        if dismiss != "scrim" {
            focused(tab, OPEN).await?;
        }
    }
    Ok(
        json!({"dismissals":["cancel","escape","scrim"],"saved":"09:07","commits":0,"outside_actions":0}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let wheels = wheels(env, tab).await?;
    reload(tab, url).await?;
    let pointer = pointer(env, tab).await?;
    reload(tab, url).await?;
    let keyboard = keyboard(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    reload(tab, url).await?;
    let input = input_and_save(env, tab).await?;
    reload(tab, url).await?;
    let dismissal = dismissal(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "time scroll page errors: {errors:?}");
    env.write_json(
        "browser-material-time-scroll.json",
        &json!({"browser":chrome.version,"wheels":wheels,"pointer":pointer,"keyboard":keyboard,
            "compact":compact,"input":input,"dismissal":dismissal,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1244, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-time-scroll-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-time-scroll-failure.png"))
            .await;
    }
    result.context("Material 3 Scroll time picker")
}
