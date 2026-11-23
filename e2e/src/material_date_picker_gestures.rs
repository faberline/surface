//! Trusted touch paging for the modal calendar and docked focus shortcuts.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const MODAL: &str = "date-picker-demo-modal";
const DOCKED: &str = "docked-date-picker-demo-picker";
const OPEN: &str = "date-picker-demo-open";
const DOCKED_OPEN: &str = "docked-date-picker-demo-picker-calendar-trigger";
const DOCKED_INPUT: &str = "docked-date-picker-demo-picker-input";
const DOCKED_OUTSIDE: &str = "docked-date-picker-demo-outside";
const PREVIEW: &str = "date-picker-demo-modal-swipe-preview";
const CURRENT: &str = "date-picker-demo-modal-swipe-current";
const GRID: &str = "date-picker-demo-modal-grid";
const GRID_SCROLL: &str = "date-picker-demo-modal-grid-scroll";
const INITIAL: &str = "2024-02-28";

type Rect = [f64; 4];
type TouchPoint = (f64, f64);

fn part(base: &str, name: &str) -> String {
    format!("{base}-{name}")
}

fn day(base: &str, date: &str) -> String {
    part(base, &format!("day-{date}"))
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    runtime_focus(tab, id).await?;
    let target = if id == DOCKED_INPUT {
        "document.getElementById('surface-text-input')".to_owned()
    } else {
        projected(id)
    };
    tab.wait_for(&format!(
        "{target} !== null && document.activeElement === {target}"
    ))
    .await?;
    tab.settle().await
}

async fn focus_projected(tab: &Tab, id: &str) -> Result<()> {
    let target = projected(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    focused(tab, id).await
}

async fn activate(tab: &Tab, id: &str) -> Result<()> {
    focus_projected(tab, id).await?;
    tab.press("Enter").await?;
    tab.settle().await
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

async fn reload(tab: &Tab, url: &str, docked: bool) -> Result<()> {
    tab.viewport(960, 1024).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 980).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    let (show, visible) = if docked {
        (
            "docked-date-picker-demo-show",
            "s.docked_date_picker.visible",
        )
    } else {
        ("date-picker-demo-show", "s.date_picker.visible")
    };
    activate(tab, show).await?;
    until(tab, visible).await?;
    resize(tab, 960, 820).await
}

async fn modal(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.date_picker)")).await?;
    tab.settle().await
}

async fn docked(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.docked_date_picker)")).await?;
    tab.settle().await
}

async fn open_calendar(tab: &Tab, base: &str) -> Result<()> {
    if base == MODAL {
        activate(tab, OPEN).await?;
        modal(tab, "d.session?.mode === 'Calendar'").await?;
    } else {
        activate(tab, DOCKED_OPEN).await?;
        docked(tab, "d.mode === 'Calendar'").await?;
    }
    focused(tab, &day(base, INITIAL)).await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .try_into()
        .map_err(|rect| anyhow::anyhow!("expected four bounds for {id}: {rect:?}"))
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner[0] >= outer[0] - 0.75
        && inner[1] >= outer[1] - 0.75
        && inner[0] + inner[2] <= outer[0] + outer[2] + 0.75
        && inner[1] + inner[3] <= outer[1] + outer[3] + 0.75
}

fn coordinate(id: &str, axis: char) -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {}).style.scroll_{axis}",
        js(id),
    )
}

async fn scroll(tab: &Tab, id: &str, axis: char) -> Result<f64> {
    tab.eval(&coordinate(id, axis))
        .await?
        .as_f64()
        .context("computed scroll offset")
}

async fn touch(tab: &Tab, event: &str, location: Option<TouchPoint>) -> Result<()> {
    let points = location.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":event,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn grip(tab: &Tab, date: &str, fraction: f64) -> Result<TouchPoint> {
    let grid = point(tab, GRID_SCROLL).await?;
    let day = point(tab, &day(MODAL, date)).await?;
    let body = bounds(tab, &part(MODAL, "body")).await?;
    let leading = grid.b[1].max(body[1]) + 12.;
    let trailing = (grid.b[1] + grid.b[3]).min(body[1] + body[3]) - 12.;
    check!(
        trailing > leading,
        "the calendar offers a visible capture surface"
    );
    let y = day
        .y
        .clamp(grid.canvas[1] + leading, grid.canvas[1] + trailing);
    Ok((grid.canvas[0] + grid.b[0] + grid.b[2] * fraction, y))
}

async fn move_touch(tab: &Tab, (x, y): TouchPoint, dx: f64, dy: f64) -> Result<()> {
    touch(tab, "touchMove", Some((x + dx, y + dy))).await
}

async fn horizontal_contact(tab: &Tab, start: TouchPoint, delta: f64) -> Result<()> {
    touch(tab, "touchStart", Some(start)).await?;
    move_touch(tab, start, delta, 0.).await?;
    touch(tab, "touchEnd", None).await
}

async fn media(tab: &Tab, preference: &str) -> Result<()> {
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":preference}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {preference})")),
    ))
    .await?;
    tab.settle().await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Vec<u8>> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-date-picker-gestures-{name}.png"));
    tab.screenshot(&path).await?;
    std::fs::read(path).context("read the captured Chrome PNG")
}

async fn no_preview(tab: &Tab) -> Result<()> {
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(PREVIEW),
    ))
    .await
}

async fn one_calendar(tab: &Tab, month: &str) -> Result<Value> {
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let grids: Vec<_> = tree["nodes"]
        .as_array()
        .context("Chrome accessibility tree")?
        .iter()
        .filter(|node| node["ignored"] == false && node["role"]["value"] == "grid")
        .map(|node| node["name"]["value"].clone())
        .collect();
    check_eq!(
        grids.len(),
        1,
        "only one interactive calendar reaches Chrome AX"
    );
    check!(
        grids[0].as_str().is_some_and(|name| name.contains(month)),
        "the active Chrome grid describes {month}: {grids:?}"
    );
    Ok(grids[0].clone())
}

async fn record_touch(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
           window.calendarTouch = [];
           for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
             document.addEventListener(kind, e => {
               if (e.target?.id === 'surface' && e.pointerType === 'touch')
                 window.calendarTouch.push({kind, trusted:e.isTrusted});
             }, {capture:true,passive:true});
           }
         })()",
    )
    .await?;
    Ok(())
}

async fn wide_swipe(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M128",
        "trusted calendar swipe previews one passive page and restores or turns on release",
    );
    media(tab, "no-preference").await?;
    open_calendar(tab, MODAL).await?;
    record_touch(tab).await?;
    let rest = bounds(tab, GRID).await?;
    let rest_png = capture(env, tab, "wide-rest").await?;
    let a = grip(tab, INITIAL, 0.72).await?;
    touch(tab, "touchStart", Some(a)).await?;
    move_touch(tab, a, -78., 0.).await?;
    let preview = bounds(tab, PREVIEW).await?;
    let held = bounds(tab, GRID).await?;
    check!(
        held[0] < rest[0] - 40. && near(held[1], rest[1]) && near(held[2], rest[2]),
        "the current grid translates horizontally at its measured size: {rest:?} -> {held:?}"
    );
    check_eq!(
        tab.eval(&format!("{} === null", projected(PREVIEW)))
            .await?,
        json!(true),
        "the preview is passive in the browser"
    );
    let visible_grid = one_calendar(tab, "2024年2月").await?;
    modal(tab, "d.swipe.offset < -40 && d.session.month[1] === 2 && d.session.draft === '2024-02-28' && d.commits === 0").await?;
    runtime_focus(tab, &day(MODAL, INITIAL)).await?;
    let held_png = capture(env, tab, "wide-preview").await?;
    check!(
        rest_png != held_png,
        "the passive preview changes the real PNG"
    );

    touch(tab, "touchCancel", None).await?;
    no_preview(tab).await?;
    modal(
        tab,
        "d.swipe.offset === null && d.session.month[1] === 2 && d.commits === 0",
    )
    .await?;
    runtime_focus(tab, &day(MODAL, INITIAL)).await?;
    check_eq!(
        bounds(tab, GRID).await?,
        rest,
        "cancel returns the grid to its starting rectangle"
    );

    let b = grip(tab, INITIAL, 0.72).await?;
    touch(tab, "touchStart", Some(b)).await?;
    move_touch(tab, b, -80., 0.).await?;
    bounds(tab, PREVIEW).await?;
    tab.press("Escape").await?;
    touch(tab, "touchEnd", None).await?;
    no_preview(tab).await?;
    modal(
        tab,
        "d.session?.mode === 'Calendar' && d.session.month[1] === 2 && d.commits === 0",
    )
    .await?;
    runtime_focus(tab, &day(MODAL, INITIAL)).await?;

    let c = grip(tab, INITIAL, 0.72).await?;
    touch(tab, "touchStart", Some(c)).await?;
    move_touch(tab, c, -147., 0.).await?;
    bounds(tab, PREVIEW).await?;
    runtime_focus(tab, &day(MODAL, INITIAL)).await?;
    touch(tab, "touchEnd", None).await?;
    modal(tab, "d.session.month[1] === 3 && d.session.active === '2024-03-28' && d.session.draft === '2024-02-28' && d.committed === '2024-02-28' && d.commits === 0").await?;
    no_preview(tab).await?;
    runtime_focus(tab, &day(MODAL, "2024-03-28")).await?;
    let settled_grid = one_calendar(tab, "2024年3月").await?;
    let settled = bounds(tab, &day(MODAL, "2024-03-28")).await?;
    check!(
        contains(bounds(tab, GRID_SCROLL).await?, settled),
        "page turn reveals the new focused date"
    );
    capture(env, tab, "wide-settled").await?;
    let events = tab.eval("window.calendarTouch").await?;
    let events_list = events.as_array().context("Chrome pointer events")?;
    for kind in ["pointermove", "pointerup", "pointercancel"] {
        check!(
            events_list
                .iter()
                .any(|event| event["kind"] == kind && event["trusted"] == true),
            "actual Chrome touch event: {kind}"
        );
    }
    Ok(
        json!({"grid":visible_grid,"settled_grid":settled_grid,"rest":rest,"held":held,
        "preview":preview,"focused_day":settled,"touch_events":events}),
    )
}

async fn compact_swipe(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M129",
        "320 dp RTL contacts pan first and a new edge contact pages",
    );
    for toggle in ["theme", "rtl", "large-text"] {
        activate(tab, &format!("date-picker-demo-{toggle}")).await?;
    }
    resize(tab, 320, 520).await?;
    until(
        tab,
        "s.dark && s.date_picker.rtl && s.date_picker.text_scale === 2",
    )
    .await?;
    open_calendar(tab, MODAL).await?;
    let clip = bounds(tab, GRID_SCROLL).await?;
    let full = bounds(tab, GRID).await?;
    let initial_x = scroll(tab, GRID_SCROLL, 'x').await?;
    check!(
        near(full[2], 336.) && full[2] - clip[2] >= 15.5 && near(initial_x, full[2] - clip[2]),
        "RTL starts at the logical edge of the 48 dp columns: {clip:?}, {full:?}, {initial_x}"
    );

    let start = grip(tab, INITIAL, 0.25).await?;
    touch(tab, "touchStart", Some(start)).await?;
    move_touch(tab, start, 40., 0.).await?;
    tab.wait_for(&format!("{} <= 0.5", coordinate(GRID_SCROLL, 'x')))
        .await?;
    move_touch(tab, start, 145., 0.).await?;
    modal(tab, "d.session.month[1] === 2 && d.swipe.offset === null").await?;
    touch(tab, "touchEnd", None).await?;
    let left_x = scroll(tab, GRID_SCROLL, 'x').await?;
    modal(tab, "d.session.month[1] === 2 && d.session.draft === '2024-02-28' && d.swipe.offset === null && d.commits === 0").await?;
    no_preview(tab).await?;
    runtime_focus(tab, &day(MODAL, INITIAL)).await?;

    let reverse = grip(tab, INITIAL, 0.75).await?;
    horizontal_contact(tab, reverse, -145.).await?;
    tab.wait_for(&format!("{} >= 15.5", coordinate(GRID_SCROLL, 'x')))
        .await?;
    let right_x = scroll(tab, GRID_SCROLL, 'x').await?;
    modal(tab, "d.session.month[1] === 2 && d.swipe.offset === null").await?;
    no_preview(tab).await?;

    let wheel = grip(tab, INITIAL, 0.55).await?;
    tab.mouse_move(wheel.0, wheel.1).await?;
    tab.wheel(-160., 0.).await?;
    tab.wait_for(&format!("{} <= 0.5", coordinate(GRID_SCROLL, 'x')))
        .await?;
    modal(tab, "d.session.month[1] === 2 && d.swipe.offset === null").await?;
    let wheel_x = scroll(tab, GRID_SCROLL, 'x').await?;

    let body_id = part(MODAL, "body");
    let before_y = scroll(tab, &body_id, 'y').await?;
    let vertical = grip(tab, INITIAL, 0.50).await?;
    let distance = if before_y > 10. { 55. } else { -55. };
    touch(tab, "touchStart", Some(vertical)).await?;
    move_touch(tab, vertical, 0., distance).await?;
    touch(tab, "touchEnd", None).await?;
    tab.wait_for(&format!(
        "Math.abs({} - {before_y}) > 5",
        coordinate(&body_id, 'y'),
    ))
    .await?;
    let after_y = scroll(tab, &body_id, 'y').await?;
    check!(
        near(scroll(tab, GRID_SCROLL, 'x').await?, wheel_x),
        "vertical touch preserves horizontal pan"
    );
    modal(
        tab,
        "d.session.month[1] === 2 && d.swipe.offset === null && d.commits === 0",
    )
    .await?;

    let page = grip(tab, INITIAL, 0.24).await?;
    touch(tab, "touchStart", Some(page)).await?;
    move_touch(tab, page, 149., 0.).await?;
    let preview = bounds(tab, PREVIEW).await?;
    let moving = bounds(tab, CURRENT).await?;
    check!(
        near(preview[2], 336.) && near(moving[2], 336.) && near(preview[0] + preview[2], moving[0]),
        "the two RTL pages meet without overlapping the 48 dp columns: {preview:?}, {moving:?}"
    );
    modal(
        tab,
        "d.swipe.offset > 50 && d.session.month[1] === 2 && d.commits === 0",
    )
    .await?;
    one_calendar(tab, "2024年2月").await?;
    capture(env, tab, "compact-rtl-preview").await?;
    touch(tab, "touchEnd", None).await?;
    modal(tab, "d.session.month[1] === 3 && d.session.active === '2024-03-28' && d.session.draft === '2024-02-28' && d.committed === '2024-02-28' && d.commits === 0").await?;
    no_preview(tab).await?;
    let target = day(MODAL, "2024-03-28");
    runtime_focus(tab, &target).await?;
    let cell = bounds(tab, &target).await?;
    let body = bounds(tab, &part(MODAL, "body")).await?;
    check!(
        near(cell[2], 48.) && near(cell[3], 48.) && contains(body, cell),
        "the actual focused 48 dp date is fully visible after RTL paging: {cell:?}, {body:?}"
    );
    capture(env, tab, "compact-rtl-settled").await?;
    Ok(
        json!({"clip":clip,"full":full,"pan_offsets":[initial_x,left_x,right_x,wheel_x],
        "vertical_offsets":[before_y,after_y],"preview":preview,"current":moving,
        "settled_day":cell,"body":body}),
    )
}

async fn reduced_swipe(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M130",
        "Reduce Motion turns on release with no preview while retaining the saved date",
    );
    media(tab, "reduce").await?;
    open_calendar(tab, MODAL).await?;
    // Keep real rAF and touch. Freeze the animation clock to catch an
    // accidental dependency on elapsed animation time.
    tab.eval(
        "(() => { const start = performance.now(); Object.defineProperty(performance, 'now',
            {configurable:true,value:() => start}); })()",
    )
    .await?;
    let rest = bounds(tab, GRID).await?;
    let start = grip(tab, INITIAL, 0.72).await?;
    touch(tab, "touchStart", Some(start)).await?;
    move_touch(tab, start, -76., 0.).await?;
    check_eq!(
        bounds(tab, GRID).await?,
        rest,
        "reduced motion does not translate the held grid"
    );
    check_eq!(
        tab.eval(&format!(
            "window.gallery.control_bounds({}).length",
            js(PREVIEW)
        ))
        .await?,
        json!(0),
        "reduced motion paints no passive preview during the real drag"
    );
    modal(tab, "d.swipe.offset === null && d.session.month[1] === 2").await?;
    move_touch(tab, start, -153., 0.).await?;
    no_preview(tab).await?;
    touch(tab, "touchEnd", None).await?;
    let d = state(tab).await?["date_picker"].clone();
    check!(
        d["session"]["month"] == json!([2024, 3])
            && d["session"]["draft"] == INITIAL
            && d["committed"] == INITIAL
            && d["commits"] == 0,
        "the reduced-motion page settles within the next render without the clock: {d}"
    );
    no_preview(tab).await?;
    runtime_focus(tab, &day(MODAL, "2024-03-28")).await?;
    capture(env, tab, "reduced-motion-settled").await?;
    activate(tab, &part(MODAL, "mode")).await?;
    modal(
        tab,
        "d.session?.mode === 'Input' && d.session.draft === '2024-02-28' && d.commits === 0",
    )
    .await?;
    no_preview(tab).await?;
    Ok(
        json!({"no_preview":true,"clock_frozen":true,"month":d["session"]["month"],
        "unchanged_draft":d["session"]["draft"],"mode_change":true}),
    )
}

async fn raw_shift_letter(tab: &Tab, letter: char, repeat: bool) -> Result<()> {
    let code = format!("Key{}", letter.to_ascii_uppercase());
    for kind in ["rawKeyDown", "keyUp"] {
        tab.cdp(
            "Input.dispatchKeyEvent",
            json!({
                "type":kind,
                "key":letter.to_string(),
                "code":code,
                "windowsVirtualKeyCode":u32::from(letter.to_ascii_uppercase()),
                "modifiers":crate::keys::SHIFT,
                "autoRepeat":repeat && kind == "rawKeyDown"
            }),
        )
        .await?;
    }
    tab.settle().await
}

async fn unchanged_docked(tab: &Tab, mode: &str) -> Result<()> {
    docked(
        tab,
        &format!(
            "d.mode === {} && d.month[0] === 2024 && d.month[1] === 2 && 
         d.raw === '2024/02/28' && d.committed === '2024-02-28' && d.commits === 0",
            js(mode)
        ),
    )
    .await
}

async fn shortcuts(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M131",
        "Shift+M and Shift+Y focus docked selectors and Enter or Space opens the list",
    );
    open_calendar(tab, DOCKED).await?;
    tab.eval(
        "(() => { window.dockedKeyEvents = [];
           document.addEventListener('keydown', e => {
             if (/^[my]$/i.test(e.key))
               window.dockedKeyEvents.push({key:e.key,shift:e.shiftKey,repeat:e.repeat,trusted:e.isTrusted});
           }, {capture:true});
         })()"
    ).await?;
    let month = part(DOCKED, "month");
    let year = part(DOCKED, "year");
    raw_shift_letter(tab, 'M', true).await?;
    focused(tab, &day(DOCKED, INITIAL)).await?;
    unchanged_docked(tab, "Calendar").await?;

    tab.press("Shift+M").await?;
    focused(tab, &month).await?;
    unchanged_docked(tab, "Calendar").await?;
    tab.press("Enter").await?;
    unchanged_docked(tab, "Month").await?;
    focused(tab, &part(DOCKED, "month-02")).await?;
    capture(env, tab, "docked-month-list").await?;
    tab.press("Escape").await?;
    unchanged_docked(tab, "Calendar").await?;
    focused(tab, &month).await?;

    tab.press("Shift+y").await?;
    focused(tab, &year).await?;
    unchanged_docked(tab, "Calendar").await?;
    tab.press("Space").await?;
    unchanged_docked(tab, "Year").await?;
    focused(tab, &part(DOCKED, "year-2024")).await?;
    capture(env, tab, "docked-year-list").await?;
    tab.press("Escape").await?;
    unchanged_docked(tab, "Calendar").await?;
    focused(tab, &year).await?;
    tab.press("Escape").await?;
    docked(tab, "d.mode === null && d.commits === 0").await?;
    focused(tab, DOCKED_OPEN).await?;
    let keys = tab.eval("window.dockedKeyEvents").await?;
    let events = keys.as_array().context("Chrome keyboard events")?;
    for (letter, repeat) in [("M", true), ("M", false), ("y", false)] {
        check!(
            events.iter().any(|event| event["key"] == letter
                && event["shift"] == true
                && event["repeat"] == repeat
                && event["trusted"] == true),
            "actual trusted shifted letter {letter}, repeat={repeat}"
        );
    }
    Ok(
        json!({"month_target":month,"year_target":year,"events":keys,
        "focus_restored":DOCKED_OPEN,"commits":0}),
    )
}

async fn shortcut_scope(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M132",
        "docked shortcuts stay scoped to Calendar focus and remain reachable in compact RTL",
    );
    focus_projected(tab, DOCKED_OUTSIDE).await?;
    tab.press("Shift+M").await?;
    tab.press("Shift+Y").await?;
    focused(tab, DOCKED_OUTSIDE).await?;
    docked(
        tab,
        "d.mode === null && d.commits === 0 && d.outside_actions === 0",
    )
    .await?;
    focus_projected(tab, DOCKED_OPEN).await?;
    tab.press("Shift+Y").await?;
    focused(tab, DOCKED_OPEN).await?;
    docked(tab, "d.mode === null && d.commits === 0").await?;

    for toggle in ["theme", "rtl", "large-text"] {
        activate(tab, &format!("docked-date-picker-demo-{toggle}")).await?;
    }
    resize(tab, 320, 720).await?;
    until(
        tab,
        "s.dark && s.docked_date_picker.rtl && s.docked_date_picker.text_scale === 2",
    )
    .await?;
    open_calendar(tab, DOCKED).await?;
    let initial = day(DOCKED, INITIAL);
    for chord in ["m", "y", "Control+Shift+M", "Alt+Shift+Y"] {
        tab.press(chord).await?;
        tab.settle().await?;
        focused(tab, &initial).await?;
        unchanged_docked(tab, "Calendar").await?;
    }
    let popup = bounds(tab, &part(DOCKED, "popup")).await?;
    let mut targets = Vec::new();
    for (chord, suffix) in [("Shift+Y", "year"), ("Shift+M", "month")] {
        tab.press(chord).await?;
        let id = part(DOCKED, suffix);
        focused(tab, &id).await?;
        unchanged_docked(tab, "Calendar").await?;
        let target = bounds(tab, &id).await?;
        check!(
            target[2] >= 47.5 && target[3] >= 47.5 && contains(popup, target),
            "the focused {suffix} stays fully inside the compact popup: {target:?}, {popup:?}"
        );
        targets.push((id, target));
    }
    capture(env, tab, "docked-compact-focused").await?;
    tab.press("Enter").await?;
    unchanged_docked(tab, "Month").await?;
    let current = part(DOCKED, "month-02");
    focused(tab, &current).await?;
    tab.press("Shift+Y").await?;
    focused(tab, &current).await?;
    unchanged_docked(tab, "Month").await?;
    tab.press("Escape").await?;
    focused(tab, &part(DOCKED, "month")).await?;
    tab.press("Escape").await?;
    docked(tab, "d.mode === null && d.commits === 0").await?;
    focused(tab, DOCKED_OPEN).await?;
    Ok(json!({"popup":popup,"targets":targets,"scope_closed":true,"mode_list_isolated":true}))
}

async fn replace_docked(tab: &Tab, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    docked(tab, &format!("d.raw === {} && d.commits === 0", js(raw))).await
}

async fn editor_and_ime(tab: &Tab) -> Result<Value> {
    step(
        "M133",
        "the docked textarea and live IME keep shifted letters and Enter in the editor",
    );
    focus_projected(tab, DOCKED_INPUT).await?;
    replace_docked(tab, "2024/02/").await?;
    tab.press("Shift+M").await?;
    tab.press("Shift+Y").await?;
    docked(tab, "d.mode === null && d.raw === '2024/02/MY' && d.committed === '2024-02-28' && d.commits === 0").await?;
    focused(tab, DOCKED_INPUT).await?;
    replace_docked(tab, "2024/02/28").await?;

    open_calendar(tab, DOCKED).await?;
    tab.press("Tab").await?;
    focused(tab, DOCKED_INPUT).await?;
    replace_docked(tab, "2024/02/").await?;
    tab.press("Shift+M").await?;
    tab.press("Shift+Y").await?;
    docked(tab, "d.mode === 'Calendar' && d.raw === '2024/02/MY' && d.committed === '2024-02-28' && d.commits === 0").await?;
    focused(tab, DOCKED_INPUT).await?;
    replace_docked(tab, "2024/02/").await?;

    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(DOCKED_INPUT)
    );
    tab.ime_compose("２９", 2, 2).await?;
    tab.wait_for(&format!(
        "{composing} && document.getElementById('surface-text-input').value.includes('２９')"
    ))
    .await?;
    raw_shift_letter(tab, 'M', false).await?;
    raw_shift_letter(tab, 'Y', false).await?;
    tab.wait_for(&composing).await?;
    focused(tab, DOCKED_INPUT).await?;
    docked(tab, "d.mode === 'Calendar' && d.raw === '2024/02/' && d.committed === '2024-02-28' && d.commits === 0").await?;
    tab.press("Enter").await?;
    focused(tab, DOCKED_INPUT).await?;
    docked(
        tab,
        "d.mode === 'Calendar' && d.committed === '2024-02-28' && d.commits === 0",
    )
    .await?;

    if tab.eval(&composing).await? == json!(true) {
        tab.ime_compose("", 0, 0).await?;
        tab.wait_for(&format!("!({composing})")).await?;
    }
    tab.press("Escape").await?;
    docked(
        tab,
        "d.mode === null && d.committed === '2024-02-28' && d.commits === 0",
    )
    .await?;
    focused(tab, DOCKED_OPEN).await?;
    Ok(json!({"typed_into_closed_and_open_field":"2024/02/MY",
        "shortcuts_preserved_preedit":true,"ime_enter_commits":0}))
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url, false).await?;
    let wide = wide_swipe(env, tab).await?;
    reload(tab, url, false).await?;
    let compact = compact_swipe(env, tab).await?;
    reload(tab, url, false).await?;
    let reduce = reduced_swipe(env, tab).await?;
    reload(tab, url, true).await?;
    let keys = shortcuts(env, tab).await?;
    reload(tab, url, true).await?;
    let scope = shortcut_scope(env, tab).await?;
    reload(tab, url, true).await?;
    let editor = editor_and_ime(tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "date picker gesture page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-date-picker-gestures.json",
        &json!({"browser":chrome.version,"wide_swipe":wide,"compact_swipe":compact,
            "reduced_motion":reduce,"shortcuts":keys,"scope":scope,"editor":editor,"errors":errors})
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-date-picker-gestures-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
                "touch":tab.eval("window.calendarTouch").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-date-picker-gestures-failure.png"))
            .await;
    }
    result
}
