//! Time Dial keyboard entry and clock-hand motion on the real browser host.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::super::{
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, projected,
        resize,
    },
    state,
};
use super::{
    capture, closed, enter_scene, mouse, open, preset, ready, tick, DIAL, FACE, HOUR, MINUTE, OPEN,
    USE_DIAL,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const HANDLE: &str = "time-dial-dial-handle";
const CENTER: &str = "time-dial-dial-center";
const SAVE: &str = "time-dial-confirm";
const HOUR_0: &str = "time-dial-dial-hour-0";
const HOUR_1: &str = "time-dial-dial-hour-1";
const HOUR_9: &str = "time-dial-dial-hour-9";
const HOUR_10: &str = "time-dial-dial-hour-10";
const HOUR_12: &str = "time-dial-dial-hour-12";
const MINUTE_0: &str = "time-dial-dial-minute-0";
const MINUTE_1: &str = "time-dial-dial-minute-1";
const MINUTE_7: &str = "time-dial-dial-minute-7";
const MINUTE_10: &str = "time-dial-dial-minute-10";
const MINUTE_59: &str = "time-dial-dial-minute-59";

async fn observe_keyboard(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            window.timeDialKeys = [];
            document.addEventListener('keydown', event => {
                const id = event.target?.dataset?.surfaceId;
                if (id?.startsWith('time-dial-')) {
                    window.timeDialKeys.push({id, key:event.key, trusted:event.isTrusted});
                }
            }, {capture:true, passive:true});
        })()",
    )
    .await?;
    Ok(())
}

async fn checked(tab: &Tab, id: &str, expected: bool) -> Result<()> {
    let label = tab
        .eval_str(&format!("{}?.getAttribute('aria-label')", projected(id)))
        .await?;
    let tree = ax_tree(tab).await?;
    let option = ax(&tree, "radio", &label)?;
    check_eq!(
        ax_property(option, "checked"),
        Some(&json!(expected.to_string())),
        format!("Chrome reports the checked state of {id}")
    );
    Ok(())
}

async fn painted_digits(tab: &Tab, id: &str) -> Result<Value> {
    tab.eval(&format!(
        "(() => {{
            const nodes = JSON.parse(window.gallery.export_dom()).dom.nodes;
            const option = nodes.find(node => node.stable_id === {});
            return nodes[option?.first_child]?.text ?? null;
        }})()",
        js(id)
    ))
    .await
}

async fn fields(tab: &Tab) -> Result<Value> {
    let time = &state(tab).await?["time_dial"];
    let draft = &time["draft"];
    Ok(json!({
        "raw_hour":draft["raw_hour"], "raw_minute":draft["raw_minute"],
        "period":draft["period"], "candidate":draft["candidate"],
        "saved":time["saved"], "commits":time["commits"],
    }))
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M145",
        "trusted selector Enter focuses the first value without choosing it",
    );
    observe_keyboard(tab).await?;
    resize(tab, 960, 820).await?;
    open(tab).await?;
    tab.press("Tab").await?;
    focused(tab, MINUTE).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, HOUR).await?;

    let twelve_hour = fields(tab).await?;
    tab.press("Enter").await?;
    focused(tab, HOUR_12).await?;
    check_eq!(
        fields(tab).await?,
        twelve_hour,
        "entering Hour retains the draft"
    );
    checked(tab, HOUR_9, true).await?;
    checked(tab, HOUR_12, false).await?;
    capture(env, tab, "keyboard-first-hour").await?;

    tab.press("ArrowRight").await?;
    focused(tab, HOUR_1).await?;
    ready(
        tab,
        "d.draft.stage === 'Hour' && d.draft.candidate === '01:07' && d.commits === 0",
    )
    .await?;
    checked(tab, HOUR_1, true).await?;
    tab.press("Enter").await?;
    focused(tab, MINUTE).await?;
    ready(tab, "d.draft.stage === 'Minute'").await?;
    let minute = fields(tab).await?;
    tab.press("Enter").await?;
    focused(tab, MINUTE_0).await?;
    check_eq!(
        fields(tab).await?,
        minute,
        "entering Minute retains the draft"
    );
    checked(tab, MINUTE_7, true).await?;
    checked(tab, MINUTE_0, false).await?;
    tab.press("ArrowRight").await?;
    focused(tab, MINUTE_1).await?;
    ready(tab, "d.draft.candidate === '01:01' && d.commits === 0").await?;
    checked(tab, MINUTE_1, true).await?;

    focus_projected(tab, SAVE).await?;
    tab.press("Tab").await?;
    focused(tab, HOUR).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, SAVE).await?;
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;
    focused(tab, OPEN).await?;

    preset(tab, "format").await?;
    open(tab).await?;
    ready(tab, "d.clock === 'TwentyFourHour'").await?;
    let twenty_four_hour = fields(tab).await?;
    tab.press("Enter").await?;
    focused(tab, HOUR_0).await?;
    check_eq!(
        fields(tab).await?,
        twenty_four_hour,
        "24-hour entry retains the draft"
    );
    checked(tab, HOUR_9, true).await?;
    checked(tab, HOUR_0, false).await?;
    let events = tab.eval("window.timeDialKeys").await?;
    let delivered = events.as_array().context("Dial key events")?;
    for (id, expected) in [(HOUR, 2), (MINUTE, 1), (HOUR_1, 1)] {
        check_eq!(
            delivered
                .iter()
                .filter(|event| {
                    event["id"] == id && event["key"] == "Enter" && event["trusted"] == true
                })
                .count(),
            expected,
            format!("trusted Enter reached {id}")
        );
    }
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;
    Ok(json!({"twelve_hour":twelve_hour, "minute":minute,
        "twenty_four_hour":twenty_four_hour, "trusted_keys":events}))
}

async fn freeze_clock(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            const start = performance.now();
            window.timeDialMotionTime = 0;
            Object.defineProperty(performance, 'now', {
                configurable:true, value:() => start + window.timeDialMotionTime,
            });
        })()",
    )
    .await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.timeDialMotionTime = {ms}"))
        .await?;
    tab.settle().await
}

async fn reduced_motion(tab: &Tab, reduced: bool) -> Result<()> {
    let preference = if reduced { "reduce" } else { "no-preference" };
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

type Position = [f64; 2];

async fn hand(tab: &Tab) -> Result<Position> {
    let [x, y, width, height] = bounds(tab, HANDLE).await?;
    let [cx, cy, cw, ch] = bounds(tab, CENTER).await?;
    Ok([
        x + width / 2. - cx - cw / 2.,
        y + height / 2. - cy - ch / 2.,
    ])
}

fn slot(value: f64, count: f64, radius: f64) -> Position {
    let (sin, cos) = (value / count * std::f64::consts::TAU).sin_cos();
    [radius * sin, -radius * cos]
}

fn angle([x, y]: Position) -> f64 {
    x.atan2(-y).to_degrees()
}

fn close_pose(got: Position, expected: Position) -> bool {
    (got[0] - expected[0]).hypot(got[1] - expected[1]) < 0.75
}

fn match_pose(got: Position, expected: Position, label: &str) -> Result<()> {
    check!(
        close_pose(got, expected),
        "{label}: expected {expected:?}, got {got:?}"
    );
    Ok(())
}

fn in_arc(pose: Position, first: f64, last: f64) -> Result<()> {
    let degrees = angle(pose);
    check!(
        degrees > first && degrees < last && (pose[0].hypot(pose[1]) - 100.).abs() < 1.,
        "hand remains on the shortest outer arc from {first:.2}° to {last:.2}°: {pose:?} ({degrees:.2}°)"
    );
    Ok(())
}

async fn spring(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M146",
        "the hand takes the short arc through wrap and reverses from its current pose",
    );
    reduced_motion(tab, false).await?;
    resize(tab, 960, 820).await?;
    freeze_clock(tab).await?;
    open(tab).await?;
    activate(tab, MINUTE).await?;
    ready(
        tab,
        "d.draft.stage === 'Minute' && d.draft.candidate === '09:07'",
    )
    .await?;
    advance(tab, 500).await?;
    let start = hand(tab).await?;
    match_pose(start, slot(7., 60., 100.), "initial minute")?;

    focus_projected(tab, MINUTE_7).await?;
    tab.press("End").await?;
    ready(tab, "d.draft.candidate === '09:59' && d.commits === 0").await?;
    checked(tab, MINUTE_59, true).await?;
    check_eq!(
        [
            painted_digits(tab, MINUTE_0).await?,
            painted_digits(tab, MINUTE_59).await?
        ],
        [json!("00"), Value::Null],
        "the selected non-five-minute label leaves the nearby zero clear before the hand moves"
    );
    match_pose(
        hand(tab).await?,
        start,
        "the checked value updates before the hand moves",
    )?;
    advance(tab, 532).await?;
    let midpoint = hand(tab).await?;
    in_arc(midpoint, -5.5, 41.5)?;
    check_eq!(
        painted_digits(tab, MINUTE_59).await?,
        Value::Null,
        "the moving hand does not add a colliding non-five-minute label"
    );
    capture(env, tab, "motion-midpoint-light").await?;

    activate(tab, MINUTE_10).await?;
    ready(tab, "d.draft.candidate === '09:10' && d.commits === 0").await?;
    checked(tab, MINUTE_10, true).await?;
    let reversal_start = hand(tab).await?;
    match_pose(
        reversal_start,
        midpoint,
        "retargeting does not jump to either endpoint",
    )?;
    advance(tab, 607).await?;
    let reversed = hand(tab).await?;
    in_arc(reversed, angle(midpoint) + 0.5, 59.99)?;
    advance(tab, 1100).await?;
    let settled = hand(tab).await?;
    match_pose(
        settled,
        slot(10., 60., 100.),
        "reversed hand settles at minute ten",
    )?;

    tab.press("End").await?;
    ready(tab, "d.draft.candidate === '09:59'").await?;
    advance(tab, 1600).await?;
    let before_wrap = hand(tab).await?;
    match_pose(
        before_wrap,
        slot(59., 60., 100.),
        "minute 59 is just left of zero",
    )?;
    check_eq!(
        [
            painted_digits(tab, MINUTE_0).await?,
            painted_digits(tab, MINUTE_59).await?
        ],
        [Value::Null, json!("59")],
        "the settled hand paints 59 and leaves out overlapping zero ink"
    );
    tab.press("ArrowRight").await?;
    ready(tab, "d.draft.candidate === '09:00' && d.commits === 0").await?;
    checked(tab, MINUTE_0, true).await?;
    match_pose(hand(tab).await?, before_wrap, "wrap starts at minute 59")?;
    advance(tab, 1632).await?;
    let wrapping = hand(tab).await?;
    in_arc(wrapping, -5.95, -0.05)?;
    advance(tab, 2100).await?;
    let wrapped = hand(tab).await?;
    match_pose(wrapped, slot(0., 60., 100.), "wrap settles at zero")?;
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;

    preset(tab, "format").await?;
    open(tab).await?;
    focus_projected(tab, HOUR_9).await?;
    for hour in 10..=12 {
        tab.press("ArrowRight").await?;
        ready(tab, &format!("d.draft.candidate === '{hour}:07'")).await?;
    }
    checked(tab, HOUR_12, true).await?;
    match_pose(
        hand(tab).await?,
        slot(9., 12., 100.),
        "ring change starts at the old radius",
    )?;
    advance(tab, 2132).await?;
    let ring_midpoint = hand(tab).await?;
    let radius = ring_midpoint[0].hypot(ring_midpoint[1]);
    check!(
        radius > 64.5
            && radius < 99.5
            && angle(ring_midpoint) > -89.5
            && angle(ring_midpoint) < -0.5,
        "the ring change interpolates angle and radius: {ring_midpoint:?}"
    );
    advance(tab, 2600).await?;
    let inner = hand(tab).await?;
    match_pose(
        inner,
        slot(12., 12., 64.),
        "the hand settles on the inner ring",
    )?;
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;

    Ok(
        json!({"start":start, "midpoint":midpoint, "reversal_start":reversal_start,
        "reversed":reversed, "settled":settled, "before_wrap":before_wrap,
        "wrapping":wrapping, "wrapped":wrapped,
        "ring_midpoint":ring_midpoint, "inner":inner}),
    )
}

async fn observe_capture(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            window.timeDialMotionCapture = [];
            const canvas = document.getElementById('surface');
            for (const kind of ['gotpointercapture','lostpointercapture']) {
                canvas.addEventListener(kind, event => {
                    if (event.isTrusted && event.pointerType === 'mouse') {
                        window.timeDialMotionCapture.push({kind,id:event.pointerId});
                    }
                });
            }
        })()",
    )
    .await?;
    Ok(())
}

async fn interrupted(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M147",
        "a held hand snaps to input and capture loss retires motion and the draft preview",
    );
    reduced_motion(tab, false).await?;
    resize(tab, 390, 720).await?;
    freeze_clock(tab).await?;
    open(tab).await?;
    observe_capture(tab).await?;
    focus_projected(tab, HOUR_9).await?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.draft.candidate === '10:07' && d.commits === 0").await?;
    advance(tab, 32).await?;
    let moving = hand(tab).await?;
    in_arc(moving, -89.5, -60.5)?;

    let start = tick(tab, 3., 12., 100.).await?;
    let held = tick(tab, 6., 12., 100.).await?;
    mouse(tab, "mousePressed", start, true).await?;
    mouse(tab, "mouseMoved", held, true).await?;
    ready(
        tab,
        "d.draft.stage === 'Hour' && d.draft.preview?.value === 6 && d.draft.candidate === '10:07'",
    )
    .await?;
    checked(tab, HOUR_10, true).await?;
    let preview_pose = hand(tab).await?;
    match_pose(
        preview_pose,
        slot(6., 12., 100.),
        "held input snaps the hand",
    )?;
    advance(tab, 500).await?;
    match_pose(
        hand(tab).await?,
        preview_pose,
        "no old flight resumes during hold",
    )?;
    capture(env, tab, "motion-held-snapped").await?;

    tab.wait_for("window.timeDialMotionCapture.some(event => event.kind === 'gotpointercapture')")
        .await?;
    let owner = tab
        .eval("window.timeDialMotionCapture.find(event => event.kind === 'gotpointercapture').id")
        .await?
        .as_i64()
        .context("DOM capture owner")?;
    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    mouse(tab, "mouseReleased", held, false).await?;
    ready(
        tab,
        "d.draft.preview === null && d.draft.stage === 'Hour' &&
        d.draft.candidate === '10:07' && d.saved === '09:07' && d.commits === 0",
    )
    .await?;
    let restored = hand(tab).await?;
    match_pose(
        restored,
        slot(10., 12., 100.),
        "capture loss snaps to the unchanged selection",
    )?;
    advance(tab, 1000).await?;
    match_pose(
        hand(tab).await?,
        restored,
        "a cancelled flight cannot restart",
    )?;
    let events = tab.eval("window.timeDialMotionCapture").await?;
    check!(
        events.as_array().is_some_and(|events| events
            .iter()
            .any(|event| { event["id"] == owner && event["kind"] == "lostpointercapture" })),
        "Chrome delivered trusted capture loss for the held pointer"
    );

    mouse(tab, "mousePressed", start, true).await?;
    mouse(tab, "mouseMoved", held, true).await?;
    ready(tab, "d.draft.preview?.value === 6").await?;
    mouse(tab, "mouseReleased", held, false).await?;
    ready(
        tab,
        "d.draft.preview === null && d.draft.stage === 'Minute' &&
        d.draft.candidate === '06:07' && d.commits === 0",
    )
    .await?;
    let released = hand(tab).await?;
    match_pose(
        released,
        slot(7., 60., 100.),
        "claimed release snaps to the Minute face",
    )?;
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;
    Ok(
        json!({"moving":moving, "preview":preview_pose, "restored":restored,
        "released":released, "trusted_capture":events}),
    )
}

async fn compact_and_reduce(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M148",
        "compact fallback and Reduce Motion remove stale hand animation at frozen time",
    );
    reduced_motion(tab, false).await?;
    preset(tab, "theme").await?;
    preset(tab, "rtl").await?;
    preset(tab, "large-text").await?;
    preset(tab, "locale").await?;
    resize(tab, 1120, 880).await?;
    freeze_clock(tab).await?;
    open(tab).await?;
    focus_projected(tab, HOUR_9).await?;
    tab.press("ArrowRight").await?;
    ready(
        tab,
        "d.rtl && d.text_scale === 2 && d.locale === 'ja' && d.draft.candidate === '10:07'",
    )
    .await?;
    advance(tab, 32).await?;
    let before_compact = hand(tab).await?;
    in_arc(before_compact, -89.5, -60.5)?;
    let wide_controls = bounds(tab, "time-dial-dial-controls").await?;
    let wide_period_targets = [
        bounds(tab, "time-dial-am").await?,
        bounds(tab, "time-dial-pm").await?,
    ];
    check!(
        wide_period_targets.iter().all(|target| {
            contains(wide_controls, *target) && target[2] >= 47.5 && target[3] >= 47.5
        }),
        "both wide period targets fit at 200%: {wide_controls:?}, {wide_period_targets:?}"
    );
    capture(env, tab, "motion-midpoint-dark-rtl").await?;
    resize(tab, 320, 300).await?;
    ready(
        tab,
        "d.orientation === null && d.draft.mode === 'Input' && !d.draft.fits &&
         d.draft.candidate === '10:07' && d.commits === 0",
    )
    .await?;
    check_eq!(
        tab.eval(&format!(
            "window.gallery.control_bounds({}).length",
            js(HANDLE)
        ))
        .await?,
        json!(0),
        "the fallback removes the hand while time is frozen"
    );
    let panel = bounds(tab, DIAL).await?;
    let actions = [
        bounds(tab, SAVE).await?,
        bounds(tab, "time-dial-cancel").await?,
    ];
    check!(
        contains([0., 0., 320., 300.], panel)
            && actions
                .iter()
                .all(|action| contains(panel, *action) && action[2] >= 47.5 && action[3] >= 47.5),
        "the compact editor and localized action targets fit the viewport: {panel:?}, {actions:?}"
    );
    capture(env, tab, "motion-compact-dark-rtl-input").await?;

    resize(tab, 1120, 880).await?;
    ready(tab, "d.draft.mode === 'Input' && d.draft.fits").await?;
    activate(tab, USE_DIAL).await?;
    ready(
        tab,
        "d.draft.mode === 'Dial' && d.draft.candidate === '10:07'",
    )
    .await?;
    let remounted = hand(tab).await?;
    match_pose(
        remounted,
        slot(10., 12., 100.),
        "returning to Dial cannot resume its old flight",
    )?;
    focus_projected(tab, HOUR_10).await?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.draft.candidate === '11:07' && d.commits === 0").await?;
    advance(tab, 64).await?;
    let before_reduce = hand(tab).await?;
    in_arc(before_reduce, -59.5, -30.5)?;
    reduced_motion(tab, true).await?;
    let reduced = hand(tab).await?;
    match_pose(
        reduced,
        slot(11., 12., 100.),
        "Reduce Motion snaps the interrupted flight",
    )?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.draft.candidate === '00:07' && d.commits === 0").await?;
    checked(tab, HOUR_12, true).await?;
    let next = hand(tab).await?;
    match_pose(
        next,
        slot(12., 12., 100.),
        "subsequent reduced changes snap without time advancing",
    )?;
    check_eq!(
        tab.eval("window.timeDialMotionTime").await?,
        json!(64),
        "spring never advances after enabling Reduce Motion"
    );
    check!(
        bounds(tab, FACE).await?[2] >= 255.5,
        "the full Dial stays reachable at 200% text after resizing"
    );
    capture(env, tab, "motion-reduced-dark-rtl").await?;
    activate(tab, SAVE).await?;
    closed(tab, "00:07", 1).await?;
    Ok(
        json!({"before_compact":before_compact, "wide_controls":wide_controls,
        "wide_period_targets":wide_period_targets, "compact_panel":panel,"compact_actions":actions,"remounted":remounted,
        "before_reduce":before_reduce,"reduced":reduced,"next":next,"clock_ms":64,"saved":"00:07"}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    enter_scene(tab, url).await?;
    let keyboard = keyboard(env, tab).await?;
    enter_scene(tab, url).await?;
    let spring = spring(env, tab).await?;
    enter_scene(tab, url).await?;
    let interrupted = interrupted(env, tab).await?;
    enter_scene(tab, url).await?;
    let compact = compact_and_reduce(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "time dial motion page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-time-dial-motion.json",
        &json!({"browser":chrome.version,"keyboard":keyboard,"spring":spring,
            "interrupted":interrupted,"compact":compact,"errors":errors}),
    )
}

pub(in super::super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1244, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-time-dial-motion-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-time-dial-motion-failure.png"))
            .await;
    }
    result.context("Material 3 time dial keyboard and motion")
}
