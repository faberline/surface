//! Browser proof for the controlled Material switch and its projected semantics.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{focus, point, state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "switch-demo-show";
const BACK: &str = "switch-demo-back";
const THEME: &str = "switch-demo-theme";
const RTL: &str = "switch-demo-rtl";
const LARGE: &str = "switch-demo-large-text";
const SCROLL: &str = "switch-demo-scroll";
const PLAIN: &str = "switch-demo-plain";
const SELECTED: &str = "switch-demo-selected-icon";
const BOTH: &str = "switch-demo-both-icons";
const DISABLED_OFF: &str = "switch-demo-disabled-off";
const DISABLED_ON: &str = "switch-demo-disabled-on";
const IDS: [&str; 5] = [PLAIN, SELECTED, BOTH, DISABLED_OFF, DISABLED_ON];
const NAMES: [&str; 5] = ["通知", "相機", "行動網路同步", "位置存取", "自動備份"];
const INITIAL: [bool; 5] = [false, true, false, false, true];
const AFTER_SPACE: [bool; 5] = [true, true, false, false, true];
const AFTER_CLICK: [bool; 5] = [true, false, false, false, true];
const AFTER_LABEL: [bool; 5] = [true, false, true, false, true];
const STEM: &str = "material-switch";

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Serialize)]
struct Geometry {
    row: Rect,
    track: Rect,
    handle: Rect,
    layer: Rect,
    label: Rect,
    icon: Option<Rect>,
}

fn size(rect: Rect) -> [f64; 2] {
    [rect[2], rect[3]]
}

fn center(rect: Rect) -> (f64, f64) {
    (rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.)
}

async fn values(tab: &Tab, expected: [bool; 5], changes: u32) -> Result<Value> {
    until(
        tab,
        &format!(
            "s.switch_demo.changes === {changes} && JSON.stringify(s.switch_demo.checked) === {}",
            json!(json!(expected).to_string())
        ),
    )
    .await?;
    tab.settle().await?;
    let actual = state(tab).await?["switch_demo"].clone();
    check_eq!(
        actual["checked"],
        json!(expected),
        "controlled switch values"
    );
    check_eq!(actual["changes"], json!(changes), "effective value changes");
    Ok(actual)
}

async fn interaction(tab: &Tab, key: &str, id: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.{key} === {}",
        json!(id)
    ))
    .await
}

async fn optional_bounds(tab: &Tab, id: &str) -> Result<Option<Rect>> {
    let raw = tab
        .eval_f64s(&format!("window.gallery.control_bounds({id:?})"))
        .await?;
    match raw {
        empty if empty.is_empty() => Ok(None),
        rect => Ok(Some(rect.try_into().map_err(|rect| {
            anyhow::anyhow!("expected four bounds for {id}: {rect:?}")
        })?)),
    }
}

async fn geometry(
    tab: &Tab,
    id: &str,
    checked: bool,
    diameter: f64,
    icon: bool,
    rtl: bool,
) -> Result<Geometry> {
    let handle_id = format!("{id}-handle");
    let track_id = format!("{id}-track");
    let offset = if checked != rtl { 36. } else { 16. };
    tab.wait_for(&format!(
        "(() => {{ const h=window.gallery.control_bounds({handle_id:?});
            const track=window.gallery.control_bounds({track_id:?});
            return h[2] === {diameter} && Math.abs(h[0] + h[2] / 2 - track[0] - {offset}) < 0.05;
        }})()"
    ))
    .await?;
    tab.settle().await?;
    let row = bounds(tab, id).await?;
    let track = bounds(tab, &track_id).await?;
    let handle = bounds(tab, &handle_id).await?;
    let layer = bounds(tab, &format!("{id}-state-layer")).await?;
    let label = bounds(tab, &format!("{id}-label")).await?;
    let artwork = optional_bounds(tab, &format!("{id}-icon")).await?;
    check_eq!(size(track), [52., 32.], format!("{id} track"));
    check_eq!(size(handle), [diameter, diameter], format!("{id} handle"));
    check_eq!(size(layer), [40., 40.], format!("{id} state layer"));
    check_eq!(
        artwork.map(size),
        icon.then_some([16., 16.]),
        format!("{id} returned icon")
    );
    let (handle_x, handle_y) = center(handle);
    check!(
        near(handle_x - track[0], offset)
            && near(handle_y, center(track).1)
            && near(handle_x, center(layer).0)
            && near(handle_y, center(layer).1),
        "{id} handle and layer share the expected center"
    );
    check!(
        artwork.is_none_or(|rect| {
            contains(handle, rect)
                && near(center(rect).0, handle_x)
                && near(center(rect).1, handle_y)
        }),
        "{id} icon stays centered inside the handle"
    );
    check!(
        row[2] >= 48.
            && row[3] >= 48.
            && contains(row, label)
            && contains(row, track)
            && contains(row, layer)
            && (if rtl {
                track[0] + track[2] <= label[0]
            } else {
                label[0] + label[2] <= track[0]
            }),
        "{id} has a full hit row and mirrors the label and track by direction: {row:?}"
    );
    Ok(Geometry {
        row,
        track,
        handle,
        layer,
        label,
        icon: artwork,
    })
}

async fn switch_nodes(tab: &Tab, expected: [bool; 5]) -> Result<Vec<Value>> {
    let tree = ax_tree(tab).await?;
    let mut nodes = Vec::with_capacity(IDS.len());
    for (index, (id, name)) in IDS.into_iter().zip(NAMES).enumerate() {
        let node = ax(&tree, "switch", name)?;
        let expected_state = json!(if expected[index] { "true" } else { "false" });
        check_eq!(
            ax_property(node, "checked"),
            Some(&expected_state),
            format!("{id} AX value")
        );
        let disabled = index >= 3;
        check_eq!(
            ax_property(node, "disabled") == Some(&json!(true)),
            disabled,
            format!("{id} AX disabled state")
        );
        let proxy = projected(id);
        check_eq!(
            tab.eval(&format!("{proxy}?.hasAttribute('data-surface-action')"))
                .await?,
            !disabled,
            format!("{id} projected activation")
        );
        nodes.push(node.clone());
    }
    Ok(nodes)
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    until(tab, "s.section === 0 && !s.dark && !s.switch_demo.visible").await?;
    activate(tab, SHOW).await?;
    until(
        tab,
        "s.switch_demo.visible && !s.switch_demo.rtl && s.switch_demo.text_scale === 1",
    )
    .await?;
    focused(tab, BACK).await?;
    resize(tab, 960, 800).await
}

fn token(palette: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(palette[name].clone())
        .with_context(|| format!("authored switch token {name}"))?;
    check_eq!(alpha, 255, format!("{name} is an opaque theme token"));
    Ok([r, g, b])
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    let path = env.artifact(&format!("{STEM}-{name}.png"));
    tab.settle().await?;
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn semantics(env: &Env, tab: &Tab) -> Result<Value> {
    let before = values(tab, INITIAL, 0).await?;
    let plain = geometry(tab, PLAIN, false, 16., false, false).await?;
    let selected = geometry(tab, SELECTED, true, 24., true, false).await?;
    let both = geometry(tab, BOTH, false, 24., true, false).await?;
    let disabled_off = geometry(tab, DISABLED_OFF, false, 24., true, false).await?;
    let disabled_on = geometry(tab, DISABLED_ON, true, 24., true, false).await?;
    let ink = &before["palette"];
    let frame = capture(env, tab, "wide-settled").await?;
    let (plain_x, plain_y) = center(plain.handle);
    check_eq!(
        frame.sample(plain_x, plain_y)?,
        token(ink, "outline")?,
        "plain off handle paint"
    );
    let (track_x, track_y) = (selected.track[0] + 10., center(selected.track).1);
    check_eq!(
        frame.sample(track_x, track_y)?,
        token(ink, "primary")?,
        "selected track paint"
    );
    let selected_icon = selected.icon.context("selected-only icon rectangle")?;
    let primary = token(ink, "primary")?;
    let icon_pixels = frame
        .colors(selected_icon)?
        .get(&primary)
        .copied()
        .unwrap_or_default();
    check!(
        icon_pixels > 16,
        "selected-only variant paints its actual icon"
    );

    let nodes = switch_nodes(tab, INITIAL).await?;
    let order = [THEME, RTL, LARGE, PLAIN, SELECTED, BOTH, BACK];
    for id in order {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    tab.press("Shift+Tab").await?;
    focused(tab, BOTH).await?;
    let tree = ax_tree(tab).await?;
    let focused_node = ax(&tree, "switch", NAMES[2])?;
    check_eq!(
        ax_property(focused_node, "focused"),
        Some(&json!(true)),
        "real AX focus on the enabled switch"
    );
    values(tab, INITIAL, 0).await?;
    Ok(json!({
        "nodes": nodes, "focused_node": focused_node, "tab_order": order, "wrap_reverse": BOTH,
        "controls": [plain, selected, both, disabled_off, disabled_on], "selected_icon_primary_pixels": icon_pixels,
        "state": before, "capture": "material-switch-wide-settled.png",
    }))
}

async fn contact(tab: &Tab, id: &str) -> Result<(f64, f64)> {
    let target = point(tab, id).await?;
    let area: Rect = target
        .b
        .try_into()
        .map_err(|_| anyhow::anyhow!("hit bounds for {id}"))?;
    let scroller = bounds(tab, SCROLL).await?;
    check!(
        contains(scroller, area) && contains([0., 0., target.canvas[2], target.canvas[3]], area),
        "{id} is visible inside the scroll viewport before pointer input: {area:?}"
    );
    Ok((target.x, target.y))
}

async fn activation(tab: &Tab) -> Result<Value> {
    focus_projected(tab, PLAIN).await?;
    tab.press("Space").await?;
    let space = values(tab, AFTER_SPACE, 1).await?;
    focused(tab, PLAIN).await?;
    let enabled_on = geometry(tab, PLAIN, true, 24., false, false).await?;

    let (x, y) = contact(tab, &format!("{SELECTED}-track")).await?;
    tab.click(x, y).await?;
    let click = values(tab, AFTER_CLICK, 2).await?;
    let enabled_off = geometry(tab, SELECTED, false, 16., false, false).await?;
    for id in [DISABLED_OFF, DISABLED_ON] {
        let (x, y) = contact(tab, &format!("{id}-track")).await?;
        tab.click(x, y).await?;
        values(tab, AFTER_CLICK, 2).await?;
    }
    let nodes = switch_nodes(tab, AFTER_CLICK).await?;
    Ok(json!({
        "space": {"id":PLAIN, "state":space, "geometry":enabled_on},
        "click": {"id":SELECTED, "state":click, "geometry":enabled_off},
        "disabled_pointer": [DISABLED_OFF, DISABLED_ON], "nodes_after":nodes,
    }))
}

async fn press_geometry(env: &Env, tab: &Tab) -> Result<Value> {
    focus_projected(tab, BACK).await?;
    let (x, y) = contact(tab, &format!("{SELECTED}-track")).await?;
    tab.mouse_move(x, y).await?;
    interaction(tab, "hovered", Some(SELECTED)).await?;
    let hover = geometry(tab, SELECTED, false, 16., false, false).await?;
    let hover_image = capture(env, tab, "hovered").await?;
    let (cx, cy) = center(hover.handle);
    let hover_center = hover_image.sample(cx, cy)?;
    let hover_edge = hover_image.sample(cx + 11., cy)?;
    check!(
        hover_center != hover_edge,
        "hover leaves the pixel beyond the 16 dp handle in its track"
    );

    tab.mouse_down().await?;
    interaction(tab, "pressed", Some(SELECTED)).await?;
    let held = geometry(tab, SELECTED, false, 28., false, false).await?;
    values(tab, AFTER_CLICK, 2).await?;
    let held_image = capture(env, tab, "held").await?;
    let held_center = held_image.sample(cx, cy)?;
    let held_edge = held_image.sample(cx + 11., cy)?;
    check_eq!(
        held_center,
        held_edge,
        "held pixels reach inside the expanded 28 dp handle"
    );
    check!(
        hover_edge != held_edge,
        "the real handle footprint expands under contact"
    );

    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    let outside = json!({"x":canvas_x + 1.,"y":canvas_y + 1.});
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":outside["x"],"y":outside["y"],"button":"none","buttons":1}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseReleased","x":outside["x"],"y":outside["y"],"button":"left","buttons":0,"clickCount":1}),
    )
    .await?;
    interaction(tab, "pressed", None).await?;
    let outside_state = values(tab, AFTER_CLICK, 2).await?;
    let release = geometry(tab, SELECTED, false, 16., false, false).await?;

    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":x,"y":y,"id":1}]}),
    )
    .await?;
    interaction(tab, "pressed", Some(SELECTED)).await?;
    let touch_held = geometry(tab, SELECTED, false, 28., false, false).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchCancel","touchPoints":[]}),
    )
    .await?;
    interaction(tab, "pressed", None).await?;
    let canceled_state = values(tab, AFTER_CLICK, 2).await?;
    let canceled = geometry(tab, SELECTED, false, 16., false, false).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    Ok(json!({
        "hover": hover, "mouse_held": held, "mouse_release_outside": release, "touch_held": touch_held,
        "touch_cancel":canceled, "outside_state":outside_state, "canceled_state":canceled_state,
        "paint":{"hover_center":hover_center,"hover_edge":hover_edge,"held_center":held_center,"held_edge":held_edge},
        "captures":["material-switch-hovered.png","material-switch-held.png"],
    }))
}

async fn compact(env: &Env, tab: &Tab, wide_label_height: f64) -> Result<Value> {
    for setting in [THEME, RTL, LARGE] {
        activate(tab, setting).await?;
    }
    until(
        tab,
        "s.dark && s.switch_demo.rtl && s.switch_demo.text_scale === 2",
    )
    .await?;
    resize(tab, 320, 640).await?;
    let before = geometry(tab, BOTH, false, 24., true, true).await?;
    check!(
        before.label[3] >= 96. && before.label[3] > 2. * wide_label_height,
        "200% CJK label occupies at least two 48 dp lines: {:?}",
        before.label
    );
    let (x, y) = contact(tab, &format!("{BOTH}-label")).await?;
    tab.click(x, y).await?;
    let after = values(tab, AFTER_LABEL, 3).await?;
    let selected = geometry(tab, BOTH, true, 24., true, true).await?;
    check_eq!(
        before.row,
        selected.row,
        "toggling does not move the hit cell"
    );
    check_eq!(
        before.label,
        selected.label,
        "toggling preserves the wrapped label"
    );
    check!(
        center(selected.handle).0 < center(before.handle).0,
        "RTL moves the selected handle to the physical left"
    );
    let nodes = switch_nodes(tab, AFTER_LABEL).await?;
    // Physical pointer input keeps the canvas focused. The switch owns
    // Runtime focus, and M274 checks its browser focus from the keyboard.
    focus(tab, BOTH, "surface").await?;

    let frame = capture(env, tab, "compact-dark-rtl-200").await?;
    let primary = token(&after["palette"], "primary")?;
    check_eq!(
        frame.sample(selected.track[0] + 42., center(selected.track).1)?,
        primary,
        "selected dark RTL track paints the primary token"
    );
    let icon = selected.icon.context("RTL both-state icon rectangle")?;
    let icon_pixels = frame
        .colors(icon)?
        .get(&primary)
        .copied()
        .unwrap_or_default();
    check!(
        icon_pixels > 16,
        "the checked icon still paints at compact RTL"
    );
    // The returning launcher is inside the main gallery scroll view.
    resize(tab, 960, 1800).await?;
    activate(tab, BACK).await?;
    until(tab, "!s.switch_demo.visible").await?;
    focused(tab, SHOW).await?;
    values(tab, AFTER_LABEL, 3).await?;
    Ok(json!({
        "viewport":[320,640], "dark":true, "wide_label_height":wide_label_height, "state":after,
        "before":before, "selected":selected, "label_contact":[x,y], "nodes":nodes,
        "icon_primary_pixels":icon_pixels, "label_focus":{"runtime":BOTH,"browser":"surface"},
        "focus_restored":SHOW,
        "capture":"material-switch-compact-dark-rtl-200.png",
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"no-preference"}]}),
    )
    .await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step("M274", "switch sizes, icons and checked/disabled names in the real browser accessibility tree");
        route(&tab, url).await.context("M274 route")?;
        report["reduced_motion"] = tab.eval("matchMedia('(prefers-reduced-motion: reduce)').matches").await?;
        check_eq!(report["reduced_motion"], false, "the ordinary interaction runs with motion enabled");
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter();if(!a)return null;const {isFallbackAdapter,vendor,architecture}=a.info;return {isFallbackAdapter,vendor,architecture};})()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(), "WebGPU reports actual adapter metadata");
        report["accessibility"] = semantics(env, &tab).await.context("M274")?;
        let wide_label_height = report["accessibility"]["controls"][2]["label"][3]
            .as_f64()
            .context("wide both-icon label height")?;
        step("M275", "Space and actual pointer input each request one controlled switch update");
        report["activation"] = activation(&tab).await.context("M275")?;
        step("M276", "hover, held handle enlargement, outside release and touch cancellation");
        report["press_geometry"] = press_geometry(env, &tab).await.context("M276")?;
        step("M277", "dark compact RTL at 200% wraps text and accepts a click within the full label");
        report["compact"] = compact(env, &tab, wide_label_height).await.context("M277")?;
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }.await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        report["dom_at_failure"] = tab
            .eval_json("window.gallery.export_dom()")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact(&format!("{STEM}-failure.png")))
            .await;
    }
    env.write_json(&format!("browser-{STEM}.json"), &report)?;
    result
}
