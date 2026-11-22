//! Snackbar behavior through the real Gallery, Chrome input and WebGPU.

use super::{click, point, state, until, Point};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};
use anyhow::{Context, Result};
use serde_json::{json, Value};

const SHOW_TEXT: &str = "snackbar-show-text";
const SHOW_UNDO: &str = "snackbar-show-undo";
const SHOW_LONG: &str = "snackbar-show-long";
const TEXT_MESSAGE: &str = "變更已儲存";
const UNDO_MESSAGE: &str = "專案已移到封存";
const LONG_MESSAGE: &str = "已將跨部門協作專案移到另一個工作區。你可以復原這項變更。";
const LIVE: &str = "document.querySelector('#surface-accessibility > [aria-live=polite]')";

async fn visible_click(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    if p.b[1] < 0. || p.b[1] + p.b[3] > p.canvas[3] {
        let scale = tab.eval("devicePixelRatio").await?;
        tab.mouse_move(p.canvas[0] + 10., p.canvas[1] + 26.).await?;
        tab.wheel(0., (p.b[1] - 32.) * scale.as_f64().context("pixel ratio")?)
            .await?;
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
    click(tab, id).await
}

async fn ready(tab: &Tab, test: &str) -> Result<()> {
    until(tab, &format!("(r => {test})(s.snackbar)")).await?;
    tab.settle().await
}

async fn panel_id(tab: &Tab) -> Result<String> {
    let snack = &state(tab).await?["snackbar"];
    let id = snack["id"].as_u64().context("active Snackbar ID")?;
    let view = snack["view_id"]
        .as_str()
        .context("active Snackbar view ID")?;
    check_eq!(view, format!("gallery-snackbar-{id}"), "unique view ID");
    Ok(view.to_owned())
}

fn element(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn projected_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    tab.wait_for(&format!("document.activeElement === {}", element(id)))
        .await
}

async fn project(tab: &Tab, id: &str) -> Result<()> {
    let target = element(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    projected_focus(tab, id).await
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    // Real tool input and animation frames still run. Only the host's reading
    // of the monotonic clock is pinned, so a slow CI runner cannot miss a toast.
    tab.eval(
        "(() => {
            window.snackbarClock = { start: performance.now(), elapsed: 0 };
            Object.defineProperty(performance, 'now', {
                configurable: true,
                value: () => window.snackbarClock.start + window.snackbarClock.elapsed,
            });
        })()",
    )
    .await?;
    tab.settle().await
}

async fn clock_at(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.snackbarClock.elapsed = {ms}"))
        .await?;
    tab.settle().await
}

async fn finish_entrance(tab: &Tab) -> Result<()> {
    tab.eval("window.snackbarClock.elapsed += 200").await?;
    tab.settle().await
}

async fn observe_announcements(tab: &Tab) -> Result<()> {
    tab.wait_for(&format!("{LIVE}?.getAttribute('aria-atomic') === 'true'"))
        .await?;
    tab.eval(&format!(
        "(() => {{
            const region = {LIVE};
            const messages = [];
            const observer = new MutationObserver(() => {{
                const message = region.textContent.trim();
                if (message) messages.push(message);
            }});
            observer.observe(region, {{childList:true, characterData:true, subtree:true}});
            window.snackbarAnnouncements = {{region,messages,observer}};
        }})()"
    ))
    .await?;
    Ok(())
}

async fn show(tab: &Tab, control: &str, kind: &str) -> Result<String> {
    visible_click(tab, control).await?;
    ready(tab, &format!("r.visible === {}", js(kind))).await?;
    until(tab, &format!("s.focused === {}", js(control))).await?;
    finish_entrance(tab).await?;
    panel_id(tab).await
}

async fn passive(env: &Env, tab: &Tab) -> Result<()> {
    step(
        "M28",
        "Snackbar keeps trigger focus and expires a short message",
    );
    visible_click(tab, "section-2").await?;
    until(tab, "s.section === 2 && !s.dark").await?;
    pin_clock(tab).await?;
    observe_announcements(tab).await?;
    let id = show(tab, SHOW_TEXT, "text").await?;
    let status = element(&format!("{id}-text"));
    let action = element(&format!("{id}-action"));
    let dismiss = element(&format!("{id}-dismiss"));
    tab.wait_for(&format!(
        "{status}?.getAttribute('role') === 'status' &&
         {status}.getAttribute('aria-live') === 'off' && {status}.textContent === {} &&
         !{status}.hasAttribute('tabindex') && {action} === null && {dismiss} === null &&
         {LIVE}.textContent === {}",
        js(TEXT_MESSAGE),
        js(TEXT_MESSAGE)
    ))
    .await?;
    tab.screenshot(&env.artifact("material-snackbar-light-text.png"))
        .await?;
    clock_at(tab, 3900).await?;
    ready(tab, "r.visible === 'text' && r.expired === 0").await?;
    clock_at(tab, 4100).await?;
    ready(
        tab,
        "r.visible === null && r.expired === 1 && r.pending === 0",
    )
    .await?;
    until(tab, "s.focused === 'snackbar-show-text'").await?;
    tab.wait_for(&format!("{status} === null")).await
}

async fn queued(env: &Env, tab: &Tab) -> Result<()> {
    step(
        "M29",
        "Queued actions advance once on Enter and pointer release",
    );
    let first = show(tab, SHOW_UNDO, "undo").await?;
    let action = format!("{first}-action");
    let dismiss = format!("{first}-dismiss");
    // Enqueue a second copy while the first is still present.
    visible_click(tab, SHOW_UNDO).await?;
    ready(tab, "r.visible === 'undo' && r.pending === 1").await?;
    check_eq!(
        panel_id(tab).await?,
        first,
        "FIFO keeps the first item visible"
    );
    clock_at(tab, 21000).await?;
    project(tab, &action).await?;
    tab.press("Tab").await?;
    projected_focus(tab, &dismiss).await?;
    ready(tab, "r.acted === 0 && r.dismissed === 0 && r.expired === 1").await?;
    tab.press("Shift+Tab").await?;
    projected_focus(tab, &action).await?;
    tab.eval(&format!("window.oldSnackbarAction = {}", element(&action)))
        .await?;
    tab.screenshot(&env.artifact("material-snackbar-light-action.png"))
        .await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "r.visible === 'undo' && r.pending === 0 && r.acted === 1",
    )
    .await?;
    projected_focus(tab, SHOW_UNDO).await?;
    finish_entrance(tab).await?;

    let second = panel_id(tab).await?;
    check!(
        second != first,
        "FIFO replacement retains a distinct view ID"
    );
    tab.wait_for(&format!("{} === null", element(&action)))
        .await?;
    tab.eval("window.oldSnackbarAction.click()").await?;
    tab.settle().await?;
    check_eq!(
        state(tab).await?["snackbar"]["acted"],
        1,
        "stale action ignored"
    );
    let at = point(tab, &format!("{second}-action")).await?;
    tab.mouse_move(at.x, at.y).await?;
    tab.mouse_down().await?;
    tab.settle().await?;
    check_eq!(
        state(tab).await?["snackbar"]["acted"],
        1,
        "pointer-down does not act"
    );
    tab.mouse_up().await?;
    ready(
        tab,
        "r.visible === null && r.acted === 2 && r.dismissed === 0 && r.expired === 1",
    )
    .await?;
    until(tab, "s.focused === 'snackbar-show-undo'").await
}

fn contains(parent: &Point, child: &Point) -> bool {
    child.b[0] >= parent.b[0] - 1.
        && child.b[1] >= parent.b[1] - 1.
        && child.b[0] + child.b[2] <= parent.b[0] + parent.b[2] + 1.
        && child.b[1] + child.b[3] <= parent.b[1] + parent.b[3] + 1.
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M30",
        "Dark RTL Snackbar wraps large CJK text in a 320 dp viewport",
    );
    let light_id = show(tab, SHOW_LONG, "long").await?;
    let light_panel = point(tab, &light_id).await?;
    let light_body = point(tab, &format!("{light_id}-text")).await?;
    tab.screenshot(&env.artifact("material-snackbar-light-long.png"))
        .await?;
    click(tab, &format!("{light_id}-dismiss")).await?;
    ready(
        tab,
        "r.visible === null && r.dismissed === 1 && r.acted === 2",
    )
    .await?;
    until(tab, "s.focused === 'snackbar-show-long'").await?;

    visible_click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    visible_click(tab, "snackbar-direction").await?;
    ready(tab, "r.rtl").await?;
    visible_click(tab, "snackbar-scale").await?;
    ready(tab, "r.text_scale === 2").await?;
    let id = show(tab, SHOW_LONG, "long").await?;
    tab.viewport(320, 364).await?;
    tab.wait_for(
        "(() => {
            const canvas = document.querySelector('canvas');
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return v?.width === 320 && v.height === 320 &&
                canvas.width === 320*devicePixelRatio && canvas.height === 320*devicePixelRatio;
        })()",
    )
    .await?;
    tab.settle().await?;
    check_eq!(
        panel_id(tab).await?,
        id,
        "resize preserves the notification ID"
    );
    let panel = point(tab, &id).await?;
    let body = point(tab, &format!("{id}-text")).await?;
    let action = point(tab, &format!("{id}-action")).await?;
    let dismiss = point(tab, &format!("{id}-dismiss")).await?;
    check!(
        panel.b[0] >= 0.
            && panel.b[1] >= 0.
            && panel.b[0] + panel.b[2] <= 320.
            && panel.b[1] + panel.b[3] <= 320.,
        "Snackbar fits inside the compact viewport: {:?}",
        panel.b
    );
    check!(
        body.b[3] > 40. && body.b[3] > light_body.b[3] && panel.b[3] > light_panel.b[3],
        "large text wraps to multiple lines: {:?} vs {:?}",
        body.b,
        light_body.b
    );
    for control in [&action, &dismiss] {
        check!(
            contains(&panel, control)
                && control.b[2] >= 48.
                && control.b[3] >= 48.
                && control.b[1] >= body.b[1] + body.b[3] - 1.,
            "full-sized control is below the message: {:?}",
            control.b
        );
    }
    check!(contains(&panel, &body), "message stays inside the panel");
    check!(
        dismiss.b[0] + dismiss.b[2] <= action.b[0] + 1.,
        "RTL mirrors both independent controls"
    );
    tab.screenshot(&env.artifact("material-snackbar-dark-rtl-compact-200.png"))
        .await?;
    Ok(
        json!({"id":id,"panel":panel.b,"body":body.b,"action":action.b,"dismiss":dismiss.b,
        "light_panel":light_panel.b,"light_body":light_body.b}),
    )
}

async fn accessibility(tab: &Tab, compact: &Value) -> Result<Value> {
    step(
        "M31",
        "Snackbar exposes passive status and separate named controls",
    );
    let id = compact["id"].as_str().context("compact ID")?;
    let status = element(&format!("{id}-text"));
    let action = element(&format!("{id}-action"));
    let dismiss = element(&format!("{id}-dismiss"));
    tab.wait_for(&format!(
        "{status}?.getAttribute('role') === 'status' && {status}.textContent === {} &&
         {status}.getAttribute('aria-live') === 'off' && !{status}.hasAttribute('tabindex') &&
         {LIVE} === window.snackbarAnnouncements.region && {LIVE}.textContent === {} &&
         {action}?.getAttribute('role') === 'button' &&
         {action}.getAttribute('aria-label') === '復原工作區移動' &&
         {dismiss}?.getAttribute('role') === 'button' &&
         {dismiss}.getAttribute('aria-label') === '關閉通知' &&
         document.querySelector('#surface-accessibility [aria-modal=true]') === null",
        js(LONG_MESSAGE),
        js(LONG_MESSAGE)
    ))
    .await?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let nodes = tree["nodes"]
        .as_array()
        .context("Chrome accessibility tree")?;
    let exposed = |role: &str, name: &str| {
        nodes.iter().any(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"] == name
        })
    };
    check!(
        exposed("button", "復原工作區移動"),
        "Chrome exposes the action by name"
    );
    check!(
        exposed("button", "關閉通知"),
        "Chrome exposes the dismiss action by name"
    );
    check!(
        nodes
            .iter()
            .any(|node| node["ignored"] == false && node["role"]["value"] == "status"),
        "Chrome exposes the passive status role"
    );
    let messages = tab.eval("window.snackbarAnnouncements.messages").await?;
    check_eq!(
        messages,
        json!([
            TEXT_MESSAGE,
            UNDO_MESSAGE,
            UNDO_MESSAGE,
            LONG_MESSAGE,
            LONG_MESSAGE
        ]),
        "a stable polite region updates once per notification, including repeated text"
    );

    click(tab, &format!("{id}-dismiss")).await?;
    ready(
        tab,
        "r.visible === null && r.acted === 2 && r.dismissed === 2 && r.expired === 1",
    )
    .await?;
    until(tab, "s.focused === 'snackbar-show-long'").await?;
    tab.wait_for(&format!(
        "{status} === null && {action} === null && {dismiss} === null"
    ))
    .await?;
    Ok(
        json!({"status":LONG_MESSAGE,"action":"復原工作區移動","dismiss":"關閉通知",
        "announcements":messages}),
    )
}

async fn journey(env: &Env, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    let adapter = tab
        .eval(
            "(async () => { const gpu = await navigator.gpu.requestAdapter();
            if (!gpu) return null;
            const {vendor, architecture, isFallbackAdapter} = gpu.info;
            return {vendor, architecture, isFallbackAdapter};
        })()",
        )
        .await?;
    check!(!adapter.is_null(), "WebGPU adapter is available");
    passive(env, tab).await?;
    queued(env, tab).await?;
    let compact = compact(env, tab).await?;
    let accessibility = accessibility(tab, &compact).await?;
    check!(
        tab.errors().is_empty(),
        "Snackbar page errors: {:?}",
        tab.errors()
    );
    env.write_json(
        "browser-material-snackbar.json",
        &json!({"adapter":adapter,"compact":compact,"accessibility":accessibility,
            "state":state(tab).await?["snackbar"]}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, &tab).await;
    if result.is_err() {
        let receipt = state(&tab)
            .await
            .unwrap_or_else(|error| json!({"error":error.to_string()}));
        let _ = env.write_json("material-snackbar-failure.json", &receipt);
        let _ = tab
            .screenshot(&env.artifact("material-snackbar-failure.png"))
            .await;
    }
    result
}
