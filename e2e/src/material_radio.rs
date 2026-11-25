//! Browser proof for controlled Material radios and group keyboard behavior.

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
use crate::{check, check_eq, keys, Env};

const SHOW: &str = "radio-demo-show";
const BACK: &str = "radio-demo-back";
const THEME: &str = "radio-demo-theme";
const RTL: &str = "radio-demo-rtl";
const LARGE: &str = "radio-demo-large-text";
const LAYOUT: &str = "radio-demo-layout";
const CLEAR: &str = "radio-demo-clear";
const SCROLL: &str = "radio-demo-scroll";
const GROUP: &str = "radio-demo-frequency";
const DAILY: &str = "radio-demo-daily";
const LOCKED: &str = "radio-demo-locked";
const WEEKLY: &str = "radio-demo-weekly";
const MONTHLY: &str = "radio-demo-monthly";
const IDS: [&str; 4] = [DAILY, LOCKED, WEEKLY, MONTHLY];
const NAMES: [&str; 4] = ["每天", "即時通知", "每週一通知", "每月"];
const STEM: &str = "material-radio";

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn size(rect: Rect) -> [f64; 2] {
    [rect[2], rect[3]]
}

fn center(rect: Rect) -> (f64, f64) {
    (rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.)
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

#[derive(Serialize)]
struct Geometry {
    row: Rect,
    ring: Rect,
    dot: Rect,
    layer: Rect,
    focus_ring: Rect,
    label: Rect,
}

async fn geometry(tab: &Tab, id: &str, selected: bool, rtl: bool) -> Result<Geometry> {
    let row = bounds(tab, id).await?;
    let ring = bounds(tab, &format!("{id}-ring")).await?;
    let dot = bounds(tab, &format!("{id}-dot")).await?;
    let layer = bounds(tab, &format!("{id}-state-layer")).await?;
    let focus_ring = bounds(tab, &format!("{id}-focus-ring")).await?;
    let label = bounds(tab, &format!("{id}-label")).await?;
    check_eq!(size(ring), [20., 20.], format!("{id} indicator"));
    let dot_size = if selected { 10. } else { 0. };
    check_eq!(size(dot), [dot_size; 2], format!("{id} settled dot"));
    check_eq!(size(layer), [40., 40.], format!("{id} state layer"));
    check_eq!(size(focus_ring), [30., 30.], format!("{id} focus ring"));
    let (x, y) = center(ring);
    check!(
        [dot, layer, focus_ring].into_iter().all(|rect| {
            let (cx, cy) = center(rect);
            near(cx, x) && near(cy, y)
        }),
        "{id} layers share the indicator center"
    );
    check!(
        row[2] >= 48. && row[3] >= 48. && contains(row, label) && contains(row, layer),
        "{id} keeps its complete artwork and label in a 48 dp hit target: {row:?}"
    );
    check!(
        if rtl {
            near(right(row) - x, 24.) && near(x - right(label), 32.)
        } else {
            near(x - row[0], 24.) && near(label[0] - x, 32.)
        },
        "{id} keeps the leading 48 dp artwork and 8 dp label gap when mirrored"
    );
    Ok(Geometry {
        row,
        ring,
        dot,
        layer,
        focus_ring,
        label,
    })
}

async fn selection(tab: &Tab, selected: Option<&str>, requests: u32) -> Result<Value> {
    until(
        tab,
        &format!(
            "s.radio_demo.selected === {} && s.radio_demo.requests === {requests}",
            json!(selected)
        ),
    )
    .await?;
    tab.settle().await?;
    let model = state(tab).await?["radio_demo"].clone();
    check_eq!(
        model["selected"],
        json!(selected),
        "controlled radio selection"
    );
    check_eq!(
        model["requests"],
        json!(requests),
        "number of delivered selection requests"
    );
    Ok(model)
}

async fn pressed(tab: &Tab, id: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        json!(id)
    ))
    .await?;
    tab.settle().await
}

fn token(model: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(model["palette"][name].clone())
        .with_context(|| format!("the authored radio {name} token"))?;
    check_eq!(alpha, 255, format!("{name} token is opaque"));
    Ok([r, g, b])
}

async fn capture(env: &Env, tab: &Tab, stage: &str) -> Result<Raster> {
    let path = env.artifact(&format!("{STEM}-{stage}.png"));
    tab.settle().await?;
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    until(tab, "s.section === 0 && !s.dark && !s.radio_demo.visible").await?;
    activate(tab, SHOW).await?;
    until(
        tab,
        "s.radio_demo.visible && !s.radio_demo.rtl && !s.radio_demo.horizontal && s.radio_demo.text_scale === 1",
    )
    .await?;
    focused(tab, BACK).await?;
    resize(tab, 960, 800).await
}

async fn radio_tree(tab: &Tab, selected: Option<&str>) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let group = ax(&tree, "radiogroup", "通知頻率")?;
    let children = group["childIds"]
        .as_array()
        .context("direct radio group children")?;
    let mut nodes = Vec::with_capacity(IDS.len());
    for (id, name) in IDS.into_iter().zip(NAMES) {
        let node = ax(&tree, "radio", name)?;
        let checked = selected == Some(id);
        let disabled = id == LOCKED;
        check!(
            children.contains(&node["nodeId"]),
            "{name} belongs to the real Chrome radio group"
        );
        check_eq!(
            ax_property(node, "checked"),
            Some(&json!(if checked { "true" } else { "false" })),
            format!("{id} Chrome checked value")
        );
        check_eq!(
            ax_property(node, "disabled") == Some(&json!(true)),
            disabled,
            format!("{id} Chrome disabled state")
        );
        let proxy = projected(id);
        let actionable = tab
            .eval(&format!("{proxy}?.hasAttribute('data-surface-action')"))
            .await?;
        check_eq!(
            actionable,
            !disabled && !checked,
            format!("{id} only offers activation when it can change the selection")
        );
        nodes.push(node.clone());
    }
    Ok(json!({"group":group,"nodes":nodes}))
}

async fn layouts(tab: &Tab, selected: &str, rtl: bool) -> Result<Vec<Geometry>> {
    let mut layouts = Vec::with_capacity(IDS.len());
    for id in IDS {
        layouts.push(geometry(tab, id, id == selected, rtl).await?);
    }
    Ok(layouts)
}

async fn accessibility(env: &Env, tab: &Tab) -> Result<(Value, f64)> {
    let model = selection(tab, Some(WEEKLY), 0).await?;
    let order = [THEME, RTL, LARGE, LAYOUT, WEEKLY, CLEAR];
    for id in order {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    tab.press("Shift+Tab").await?;
    focused(tab, WEEKLY).await?;
    let ax = radio_tree(tab, Some(WEEKLY)).await?;
    check_eq!(
        ax_property(&ax["nodes"][2], "focused"),
        Some(&json!(true)),
        "the checked middle option owns real Chrome keyboard focus"
    );
    selection(tab, Some(WEEKLY), 0).await?;

    let layouts = layouts(tab, WEEKLY, false).await?;
    let frame = capture(env, tab, "wide-focused").await?;
    let checked = &layouts[2];
    let (x, y) = center(checked.ring);
    let indicator = frame.sample(x, checked.ring[1] + 1.)?;
    let dot = frame.sample(x, y)?;
    let focus_ring = frame.sample(x, checked.focus_ring[1] + 1.5)?;
    let on = token(&model, "primary")?;
    check_eq!(indicator, on, "selected indicator paints Primary");
    check_eq!(dot, on, "selected dot paints Primary");
    check_eq!(
        focus_ring,
        token(&model, "secondary")?,
        "the checked focus ring paints Secondary"
    );
    let unselected = &layouts[0];
    let (off_x, off_y) = center(unselected.ring);
    let off_ring = frame.sample(off_x, unselected.ring[1] + 1.)?;
    let off_center = frame.sample(off_x, off_y)?;
    check_eq!(
        off_ring,
        token(&model, "on_surface_variant")?,
        "idle unselected ring paints On Surface Variant"
    );
    check_eq!(
        off_center,
        token(&model, "surface")?,
        "idle unselected center stays empty"
    );
    let wide_label_height = checked.label[3];
    Ok((
        json!({
            "state":model,"accessibility":ax,"layouts":layouts,"tab_order":order,"reverse_tab":WEEKLY,
            "paint":{"indicator":indicator,"dot":dot,"focus_ring":focus_ring,"off_ring":off_ring,"off_center":off_center},
            "capture":"material-radio-wide-focused.png",
        }),
        wide_label_height,
    ))
}

async fn repeat_space(tab: &Tab) -> Result<()> {
    let key = keys::key("Space")?;
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({"type":"keyDown","key":key.key,"code":key.code,"windowsVirtualKeyCode":key.key_code,
               "text":key.text,"unmodifiedText":key.text,"autoRepeat":true}),
    )
    .await?;
    tab.key_up(&key).await
}

async fn arrow_sequence(
    tab: &Tab,
    starting_requests: u32,
    moves: &[(&str, &str)],
) -> Result<Vec<Value>> {
    let mut receipt = Vec::with_capacity(moves.len());
    for (index, &(key, target)) in moves.iter().enumerate() {
        tab.press(key).await?;
        let model = selection(tab, Some(target), starting_requests + index as u32 + 1).await?;
        focused(tab, target).await?;
        receipt.push(json!({"key":key,"target":target,"state":model}));
    }
    Ok(receipt)
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    let arrows = arrow_sequence(
        tab,
        0,
        &[
            ("ArrowRight", MONTHLY),
            ("ArrowDown", DAILY),
            ("ArrowRight", WEEKLY),
            ("ArrowUp", DAILY),
            ("ArrowLeft", MONTHLY),
        ],
    )
    .await?;
    let selected_ax = radio_tree(tab, Some(MONTHLY)).await?;
    tab.press("Space").await?;
    let already_checked = selection(tab, Some(MONTHLY), 5).await?;

    tab.press("Tab").await?;
    focused(tab, CLEAR).await?;
    tab.press("Enter").await?;
    let cleared = selection(tab, None, 5).await?;
    let cleared_ax = radio_tree(tab, None).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, MONTHLY).await?;
    tab.eval(
        r#"
        window.radioSpaceTrace = [];
        document.addEventListener('keydown', event => {
            if (event.code === 'Space') {
                window.radioSpaceTrace.push({trusted:event.isTrusted, repeat:event.repeat});
            }
        }, {capture:true});
    "#,
    )
    .await?;
    repeat_space(tab).await?;
    let protected = selection(tab, None, 5).await?;
    focused(tab, MONTHLY).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, LAYOUT).await?;
    tab.press("Tab").await?;
    focused(tab, DAILY).await?;
    selection(tab, None, 5).await?;
    tab.press("Space").await?;
    let normal = selection(tab, Some(DAILY), 6).await?;
    focused(tab, DAILY).await?;
    repeat_space(tab).await?;
    let repeated = selection(tab, Some(DAILY), 6).await?;
    let trace = tab.eval("window.radioSpaceTrace").await?;
    check_eq!(
        trace,
        json!([{"trusted":true,"repeat":true},{"trusted":true,"repeat":false},{"trusted":true,"repeat":true}]),
        "Chrome delivered the trusted repeat and fresh Space presses"
    );
    let horizontal = horizontal(env, tab).await?;
    Ok(json!({
        "arrows":arrows,"selected_ax":selected_ax,"selected_space":already_checked,"cleared":cleared,"cleared_ax":cleared_ax,
        "empty_entry":{"forward":DAILY,"reverse":MONTHLY},
        "space":{"ignored_repeat":protected,"normal":normal,"after_selection_repeat":repeated,"native_events":trace},
        "horizontal":horizontal,
    }))
}

async fn horizontal(env: &Env, tab: &Tab) -> Result<Value> {
    activate(tab, LAYOUT).await?;
    until(tab, "s.radio_demo.horizontal").await?;
    let model = selection(tab, Some(DAILY), 6).await?;
    let group = bounds(tab, GROUP).await?;
    let layouts = layouts(tab, DAILY, false).await?;
    check!(
        layouts.iter().all(|art| contains(group, art.row)),
        "horizontal wrapping retains each full target inside the group"
    );
    check!(
        layouts.windows(2).any(
            |pair| near(pair[0].row[1], pair[1].row[1]) && pair[1].row[0] >= right(pair[0].row)
        ),
        "two fitting choices share a real horizontal row"
    );
    for pair in layouts.windows(2) {
        let [before, after] = [&pair[0].row, &pair[1].row];
        check!(
            (near(before[1], after[1]) && after[0] >= right(*before) - 0.5)
                || after[1] >= bottom(*before) - 0.5,
            "options follow authored order within and across wrapped rows"
        );
    }
    let image = capture(env, tab, "wide-horizontal").await?;
    let (x, y) = center(layouts[0].dot);
    let ink = image.sample(x, y)?;
    check_eq!(
        ink,
        token(&model, "primary")?,
        "horizontal choice still paints the selected dot"
    );
    let ax = radio_tree(tab, Some(DAILY)).await?;
    activate(tab, LAYOUT).await?;
    until(tab, "!s.radio_demo.horizontal").await?;
    selection(tab, Some(DAILY), 6).await?;
    Ok(
        json!({"state":model,"group":group,"layouts":layouts,"dot":ink,"accessibility":ax,
              "capture":"material-radio-wide-horizontal.png"}),
    )
}

async fn contact(tab: &Tab, id: &str, part: &str) -> Result<(f64, f64)> {
    let hit = point(tab, &format!("{id}-{part}")).await?;
    let rect: Rect = hit
        .b
        .try_into()
        .map_err(|_| anyhow::anyhow!("the bounds of {id}-{part}"))?;
    let row = bounds(tab, id).await?;
    let clip = bounds(tab, SCROLL).await?;
    check!(
        contains(row, rect)
            && contains(clip, rect)
            && contains([0., 0., hit.canvas[2], hit.canvas[3]], rect),
        "{id}-{part} is fully visible inside its real hit row: {rect:?}"
    );
    Ok((hit.x, hit.y))
}

async fn press_receipt(env: &Env, tab: &Tab) -> Result<Value> {
    let (x, y) = contact(tab, DAILY, "ring").await?;
    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(DAILY)).await?;
    focus(tab, DAILY, "surface").await?;
    let held = selection(tab, Some(MONTHLY), 8).await?;
    let layout = geometry(tab, DAILY, false, false).await?;
    let image = capture(env, tab, "wide-held").await?;
    let (cx, cy) = center(layout.ring);
    let press_layer = image.sample(cx + 17., cy)?;
    let background = image.sample(cx + 23., cy)?;
    let press_indicator = image.sample(cx, layout.ring[1] + 1.)?;
    let focus_ring = image.sample(cx, layout.focus_ring[1] + 1.5)?;
    check_eq!(
        background,
        token(&held, "surface")?,
        "pixels outside the state layer show the surface"
    );
    check_eq!(
        press_indicator,
        token(&held, "on_surface")?,
        "pressed unselected ring paints On Surface"
    );
    check_eq!(
        focus_ring,
        token(&held, "secondary")?,
        "the pressed focus ring remains visible"
    );
    check!(
        press_layer != background && press_layer != token(&held, "primary")?,
        "a translucent pressed state layer appears outside the indicator and focus ring"
    );
    let [left, top, _, _] = tab.canvas_box().await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":left+1.,"y":top+1.,"button":"none","buttons":1}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseReleased","x":left+1.,"y":top+1.,"button":"left","buttons":0,"clickCount":1}),
    )
    .await?;
    pressed(tab, None).await?;
    let outside = selection(tab, Some(MONTHLY), 8).await?;

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
    pressed(tab, Some(DAILY)).await?;
    selection(tab, Some(MONTHLY), 8).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchCancel","touchPoints":[]}),
    )
    .await?;
    pressed(tab, None).await?;
    let canceled = selection(tab, Some(MONTHLY), 8).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    Ok(json!({
        "id":DAILY,"host":"surface","state":held,"layout":layout,"outside":outside,"touch_cancel":canceled,
        "paint":{"layer":press_layer,"background":background,"indicator":press_indicator,"focus_ring":focus_ring},
        "capture":"material-radio-wide-held.png",
    }))
}

async fn pointer(env: &Env, tab: &Tab) -> Result<Value> {
    let (x, y) = contact(tab, WEEKLY, "label").await?;
    tab.click(x, y).await?;
    let label = selection(tab, Some(WEEKLY), 7).await?;
    focus(tab, WEEKLY, "surface").await?;
    tab.click(x, y).await?;
    let (ring_x, ring_y) = contact(tab, WEEKLY, "ring").await?;
    tab.click(ring_x, ring_y).await?;
    let selected_again = selection(tab, Some(WEEKLY), 7).await?;
    focus(tab, WEEKLY, "surface").await?;

    let monthly = geometry(tab, MONTHLY, false, false).await?;
    let [left, top, _, _] = tab.canvas_box().await?;
    let (cx, cy) = center(monthly.ring);
    let hit_x = cx + 28.;
    check!(
        hit_x > right(monthly.layer)
            && hit_x < monthly.label[0]
            && contains(monthly.row, [hit_x, cy, 1., 1.]),
        "the pointer lands in the gap inside the whole monthly row"
    );
    tab.click(left + hit_x, top + cy).await?;
    let whole_row = selection(tab, Some(MONTHLY), 8).await?;
    focus(tab, MONTHLY, "surface").await?;
    let held = press_receipt(env, tab).await?;

    let (x, y) = contact(tab, LOCKED, "ring").await?;
    tab.click(x, y).await?;
    let blocked = selection(tab, Some(MONTHLY), 8).await?;
    check!(
        state(tab).await?["focused"] != LOCKED,
        "disabled radio cannot take pointer focus"
    );
    let ax = radio_tree(tab, Some(MONTHLY)).await?;
    Ok(json!({
        "label":{"id":WEEKLY,"host":"surface","state":label},
        "same_label_and_ring":selected_again,
        "whole_row":{"id":MONTHLY,"host":"surface","state":whole_row,"contact":[left+hit_x,top+cy],"layout":monthly},
        "held":held,"disabled":{"id":LOCKED,"state":blocked},"accessibility":ax,
    }))
}

async fn compact(env: &Env, tab: &Tab, wide_label_height: f64) -> Result<Value> {
    for id in [THEME, RTL, LARGE] {
        activate(tab, id).await?;
    }
    until(
        tab,
        "s.dark && s.radio_demo.rtl && !s.radio_demo.horizontal && s.radio_demo.text_scale === 2",
    )
    .await?;
    resize(tab, 320, 640).await?;
    focus_projected(tab, LAYOUT).await?;
    tab.press("Tab").await?;
    focused(tab, MONTHLY).await?;
    selection(tab, Some(MONTHLY), 8).await?;
    let arrows = arrow_sequence(
        tab,
        8,
        &[
            ("ArrowRight", WEEKLY),
            ("ArrowLeft", MONTHLY),
            ("ArrowUp", WEEKLY),
        ],
    )
    .await?;
    let model = selection(tab, Some(WEEKLY), 11).await?;
    let layout = geometry(tab, WEEKLY, true, true).await?;
    let clip = bounds(tab, SCROLL).await?;
    check!(
        contains(clip, layout.row),
        "wide text and radio remain in the compact scroll clip"
    );
    check!(
        layout.label[3] >= 96. && layout.label[3] > 2. * wide_label_height,
        "200% CJK text wraps within its full RTL label cell: {:?}",
        layout.label
    );
    let image = capture(env, tab, "compact-dark-rtl-200").await?;
    let dark_surface = image.sample(1., 1.)?;
    check_eq!(
        dark_surface,
        token(&model, "surface")?,
        "the compact frame paints the dark Surface token"
    );
    let (cx, cy) = center(layout.ring);
    let dot = image.sample(cx, cy)?;
    let ring = image.sample(cx, layout.ring[1] + 1.)?;
    let focus_ink = image.sample(cx, layout.focus_ring[1] + 1.5)?;
    let primary = token(&model, "primary")?;
    check_eq!(dot, primary, "compact selected dot uses Primary");
    check_eq!(ring, primary, "compact selected ring uses Primary");
    check_eq!(
        focus_ink,
        token(&model, "secondary")?,
        "compact keyboard focus ring uses Secondary"
    );
    let [lx, ly, lw, _] = layout.label;
    let text = token(&model, "on_surface")?;
    let mut label_pixels = Vec::new();
    for line in 0..2 {
        let y = ly + f64::from(line) * 48.;
        let ink = image
            .colors([lx, y, lw, 48.])?
            .get(&text)
            .copied()
            .unwrap_or_default();
        check!(
            ink > 24,
            "wrapped line {} paints real glyph pixels",
            line + 1
        );
        label_pixels.push(ink);
    }
    let tree = ax_tree(tab).await?;
    let group = ax(&tree, "radiogroup", "通知頻率")?;
    let radio = ax(&tree, "radio", NAMES[2])?;
    check_eq!(
        ax_property(radio, "focused"),
        Some(&json!(true)),
        "weekly choice keeps projected focus in RTL"
    );
    check_eq!(
        ax_property(radio, "checked"),
        Some(&json!("true")),
        "weekly choice is checked in RTL"
    );
    resize(tab, 960, 1800).await?;
    activate(tab, BACK).await?;
    until(tab, "!s.radio_demo.visible").await?;
    focused(tab, SHOW).await?;
    selection(tab, Some(WEEKLY), 11).await?;
    Ok(json!({
        "viewport":[320,640],"dark":true,"state":model,"layout":layout,"clip":clip,"arrows":arrows,
        "accessibility":{"group":group,"radio":radio},"wide_label_height":wide_label_height,
        "paint":{"surface":dark_surface,"dot":dot,"indicator":ring,"focus_ring":focus_ink,"label_pixels":label_pixels},
        "focus_restored":SHOW,"capture":"material-radio-compact-dark-rtl-200.png",
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step("M282", "named radio group, checked and disabled states, shared Tab stop and radio paint");
        route(&tab, url).await.context("M282 route")?;
        report["reduced_motion"] = tab.eval("matchMedia('(prefers-reduced-motion: reduce)').matches").await?;
        check_eq!(report["reduced_motion"], true, "paint checks use stable reduced-motion endpoints");
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter();if(!a)return null;const {isFallbackAdapter,vendor,architecture}=a.info;return {isFallbackAdapter,vendor,architecture};})()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(), "WebGPU reports actual adapter metadata");
        let (accessible, wide_label_height) = accessibility(env, &tab).await.context("M282")?;
        report["accessibility"] = accessible;
        step("M283", "arrows select and wrap, empty entry has first and last stops, trusted Space repeats stay inert");
        report["keyboard"] = keyboard(env, &tab).await.context("M283")?;
        step("M284", "whole-row and label hits select once, with actual press paint and cancellation");
        report["pointer"] = pointer(env, &tab).await.context("M284")?;
        step("M285", "320 dp dark RTL paints wrapped 200% labels and mirrors horizontal arrows");
        report["compact"] = compact(env, &tab, wide_label_height).await.context("M285")?;
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }
    .await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        report["dom_at_failure"] = tab
            .eval_json("window.gallery.export_dom()")
            .await
            .unwrap_or(Value::Null);
        report["focus_host_at_failure"] = tab
            .eval("document.activeElement?.id ?? null")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact(&format!("{STEM}-failure.png")))
            .await;
    }
    env.write_json(&format!("browser-{STEM}.json"), &report)?;
    result
}
