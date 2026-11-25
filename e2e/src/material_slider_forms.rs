//! Material Centered and vertical Standard sliders through the browser Gallery.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "slider-forms-demo-show";
const BACK: &str = "slider-forms-demo-back";
const SCROLL: &str = "slider-forms-demo-scroll";
const THEME: &str = "slider-forms-demo-theme";
const RTL: &str = "slider-forms-demo-rtl";
const LARGE_TEXT: &str = "slider-forms-demo-large-text";
const CENTERED: [&str; 4] = [
    "slider-forms-demo-centered-negative",
    "slider-forms-demo-centered-neutral",
    "slider-forms-demo-centered-positive",
    "slider-forms-demo-centered-discrete",
];
const VERTICAL: [&str; 3] = [
    "slider-forms-demo-vertical-continuous",
    "slider-forms-demo-vertical-discrete",
    "slider-forms-demo-vertical-disabled",
];
const VERTICAL_NAMES: [&str; 3] = ["垂直連續", "垂直階梯", "垂直停用"];
const CENTERED_NAMES: [&str; 4] = ["負值", "中立值", "正值", "離散值"];
const WIDE: [u32; 2] = [960, 1160];
const COMPACT: [u32; 2] = [320, 880];
const CENTERED_PNG: &str = "material-slider-forms-centered-light.png";
const KEYS_PNG: &str = "material-slider-forms-centered-keys.png";
const VERTICAL_PNG: &str = "material-slider-forms-vertical-light.png";
const COMPACT_PNG: &str = "material-slider-forms-vertical-compact-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Deserialize, Serialize)]
struct Palette {
    background: [u8; 4],
    primary: [u8; 4],
    secondary: [u8; 4],
    secondary_container: [u8; 4],
    on_secondary_container: [u8; 4],
    inverse_surface: [u8; 4],
    inverse_on_surface: [u8; 4],
}

#[derive(Deserialize, Serialize)]
struct SliderState {
    values: Vec<f64>,
    requests: Vec<u32>,
    engaged: Vec<bool>,
}

#[derive(Deserialize, Serialize)]
struct Model {
    visible: bool,
    rtl: bool,
    text_scale: f64,
    dark: bool,
    centered: SliderState,
    vertical: SliderState,
    palette: Palette,
}

#[derive(Serialize)]
struct Shape {
    target: Rect,
    rail: Rect,
    active: Rect,
    handle: Rect,
}

fn midpoint([x, y, w, h]: Rect) -> [f64; 2] {
    [x + w / 2., y + h / 2.]
}

fn rgb([r, g, b, _]: [u8; 4]) -> Rgb {
    [r, g, b]
}

fn sample(image: &Raster, rect: Rect) -> Result<Rgb> {
    let [x, y] = midpoint(rect);
    image.sample(x, y)
}

fn ink(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

async fn part(tab: &Tab, id: &str, suffix: &str) -> Result<Rect> {
    bounds(tab, &format!("{id}-{suffix}")).await
}

async fn shape(tab: &Tab, id: &str) -> Result<Shape> {
    Ok(Shape {
        target: bounds(tab, id).await?,
        rail: part(tab, id, "rail").await?,
        active: part(tab, id, "active").await?,
        handle: part(tab, id, "handle").await?,
    })
}

async fn ready(tab: &Tab, condition: &str) -> Result<Model> {
    until(tab, &format!("(d => {condition})(s.slider_forms_demo)")).await?;
    tab.settle().await?;
    Ok(serde_json::from_value(
        state(tab).await?["slider_forms_demo"].clone(),
    )?)
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1900).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.slider_forms_demo.visible").await?;
    activate(tab, SHOW).await?;
    ready(
        tab,
        "d.visible && d.centered.values[0] === -50 && d.vertical.values[0] === 25",
    )
    .await?;
    resize(tab, WIDE[0], WIDE[1]).await
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn aria(tab: &Tab, id: &str) -> Result<Value> {
    let element = projected(id);
    tab.eval(&format!(
        "['aria-valuemin','aria-valuemax','aria-valuenow','aria-valuetext','aria-orientation']\
         .map(name => {element}.getAttribute(name))"
    ))
    .await
}

fn logical_left(rect: Rect, rtl: bool) -> f64 {
    if rtl {
        -rect[0] - rect[2]
    } else {
        rect[0]
    }
}

fn check_centered_span(g: &Shape, inactive: [Rect; 2], value: f64, rtl: bool) -> Result<()> {
    let start = |rect| logical_left(rect, rtl);
    let end = |rect: Rect| start(rect) + rect[2];
    let neutral = start(g.rail) + g.rail[2] / 2.;
    let thumb = start(g.handle) + g.handle[2] / 2.;
    check!(
        near(thumb, start(g.rail) + g.rail[2] * ((value + 100.) / 200.)),
        "handle follows the signed value"
    );
    check!(
        inactive
            .iter()
            .all(|segment| segment[2] > 0. && segment[3] > 0.)
            && near(inactive[0][3], inactive[1][3]),
        "both inactive portions have the same positive dimensions across the neutral cut"
    );
    if value == 0. {
        check!(g.active[2] <= 0.75, "no active track at the neutral value");
        check!(
            near(start(inactive[1]) - end(inactive[0]), g.handle[2] + 12.),
            "the centered handle owns the neutral gap"
        );
        return Ok(());
    }
    let [before, after] = if value < 0. {
        [end(g.active), start(inactive[1])]
    } else {
        [end(inactive[0]), start(g.active)]
    };
    let [handle_start, handle_end] = if value < 0. {
        [end(inactive[0]), start(g.active)]
    } else {
        [end(g.active), start(inactive[1])]
    };
    check!(
        g.active[2] > 0.
            && near(g.active[3], inactive[0][3])
            && near(before, neutral - 2.)
            && near(after, neutral + 2.),
        "active paint ends at the four dp neutral cut"
    );
    check!(
        near(start(g.handle) - handle_start, 6.) && near(handle_end - end(g.handle), 6.),
        "each occupied track leaves the six dp handle clearance"
    );
    Ok(())
}

async fn centered_paint(env: &Env, tab: &Tab) -> Result<Value> {
    let model = ready(tab, "d.visible && !d.rtl && !d.dark && d.text_scale === 1").await?;
    let image = photo(tab, env, CENTERED_PNG).await?;
    let tree = ax_tree(tab).await?;
    let viewport = bounds(tab, SCROLL).await?;
    let mut sliders = Vec::new();
    for (index, id) in CENTERED.into_iter().enumerate() {
        let g = shape(tab, id).await?;
        let inactive = [
            part(tab, id, "inactive-min").await?,
            part(tab, id, "inactive-max").await?,
        ];
        let stops = [
            part(tab, id, "stop-min").await?,
            part(tab, id, "stop-max").await?,
        ];
        let value = model.centered.values[index];
        check!(contains(viewport, g.target), "wide target is visible");
        check_centered_span(&g, inactive, value, false)?;
        check_eq!(
            sample(&image, g.handle)?,
            rgb(model.palette.primary),
            "centered handle paints the primary color"
        );
        if value != 0. {
            // Discrete tick dots can overlap the geometric midpoint of a span.
            let active_ink = ink(&image, g.active, rgb(model.palette.primary))?;
            check!(active_ink > 50, "signed active span contains primary paint");
        }
        for (track, stop) in inactive.into_iter().zip(stops) {
            check!(
                contains(track, stop) && near(stop[2], 4.) && near(stop[3], 4.),
                "each surviving inactive end has a four dp stop"
            );
            check!(
                ink(&image, track, rgb(model.palette.secondary_container))? > 80,
                "inactive span contains secondary-container paint"
            );
            check_eq!(
                sample(&image, stop)?,
                rgb(model.palette.on_secondary_container),
                "inactive stop has contrasting paint"
            );
        }
        let node = ax(&tree, "slider", CENTERED_NAMES[index])?;
        check_eq!(
            node["value"]["value"].as_f64(),
            Some(value),
            "Chrome AX reports the signed value"
        );
        let description = aria(tab, id).await?;
        check_eq!(
            description,
            json!(["-100", "100", value.to_string(), description[3], null]),
            "centered slider retains horizontal orientation and signed bounds"
        );
        check!(
            description[3].as_str().is_some_and(|text| !text.is_empty()),
            "signed slider exposes localized value text"
        );
        sliders.push(json!({"id":id,"shape":g,"inactive":inactive,"stops":stops,"value":value,"aria":description}));
    }
    Ok(json!({"viewport":WIDE,"palette":model.palette,"sliders":sliders,"capture":CENTERED_PNG}))
}

async fn centered_keys(env: &Env, tab: &Tab) -> Result<Value> {
    focus_projected(tab, CENTERED[0]).await?;
    tab.press("Home").await?;
    ready(
        tab,
        "d.centered.values[0] === -100 && d.centered.requests[0] === 1",
    )
    .await?;
    tab.press("ArrowLeft").await?;
    ready(
        tab,
        "d.centered.values[0] === -100 && d.centered.requests[0] === 1",
    )
    .await?;
    tab.press("End").await?;
    ready(
        tab,
        "d.centered.values[0] === 100 && d.centered.requests[0] === 2",
    )
    .await?;
    focus_projected(tab, CENTERED[3]).await?;
    tab.press("ArrowRight").await?;
    ready(
        tab,
        "d.centered.values[3] === 0 && d.centered.requests[3] === 1",
    )
    .await?;
    let neutral = shape(tab, CENTERED[3]).await?;
    check!(
        neutral.active[2] <= 0.75,
        "discrete step reaches the neutral gap"
    );
    tab.press("ArrowRight").await?;
    ready(
        tab,
        "d.centered.values[3] === 25 && d.centered.requests[3] === 2",
    )
    .await?;

    activate(tab, RTL).await?;
    focus_projected(tab, CENTERED[3]).await?;
    tab.press("ArrowLeft").await?;
    let model = ready(
        tab,
        "d.rtl && d.centered.values[3] === 50 && d.centered.requests[3] === 3",
    )
    .await?;
    let g = shape(tab, CENTERED[3]).await?;
    let inactive = [
        part(tab, CENTERED[3], "inactive-min").await?,
        part(tab, CENTERED[3], "inactive-max").await?,
    ];
    check_centered_span(&g, inactive, model.centered.values[3], true)?;
    let ring = part(tab, CENTERED[3], "focus").await?;
    let label = part(tab, CENTERED[3], "label").await?;
    check!(
        model.centered.engaged[3] && near(g.handle[2], 2.),
        "engaged centered thumb narrows without changing value"
    );
    let image = photo(tab, env, KEYS_PNG).await?;
    let ring_ink = ink(&image, ring, rgb(model.palette.secondary))?;
    let label_ink = ink(&image, label, rgb(model.palette.inverse_on_surface))?;
    check!(
        ring_ink > 20 && label_ink > 20,
        "centered focus and signed label appear in actual paint"
    );
    Ok(
        json!({"geometry":g,"inactive":inactive,"neutral":neutral,"ring":ring,"label":label,
        "ring_ink":ring_ink,"label_ink":label_ink,"values":model.centered.values,
        "requests":model.centered.requests,"palette":model.palette,"capture":KEYS_PNG}),
    )
}

async fn vertical_ax(tab: &Tab, index: usize, disabled: bool, value: f64) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "slider", VERTICAL_NAMES[index])?;
    check_eq!(
        ax_property(node, "orientation").and_then(Value::as_str),
        Some("vertical"),
        "Chrome AX exposes vertical orientation"
    );
    check_eq!(
        ax_property(node, "disabled") == Some(&json!(true)),
        disabled,
        "Chrome AX disabled state"
    );
    check_eq!(
        node["value"]["value"].as_f64(),
        Some(value),
        "Chrome AX vertical value"
    );
    let description = aria(tab, VERTICAL[index]).await?;
    check_eq!(
        description,
        json!(["0", "100", value.to_string(), description[3], "vertical"]),
        "DOM keeps vertical numeric bounds and orientation"
    );
    check!(
        description[3]
            .as_str()
            .is_some_and(|text| !text.trim().is_empty()),
        "vertical DOM exposes localized value text"
    );
    Ok(description)
}

async fn vertical_point(tab: &Tab, g: &Shape, value: f64) -> Result<[f64; 2]> {
    let canvas = tab.canvas_box().await?;
    Ok([
        canvas[0] + midpoint(g.target)[0],
        canvas[1] + g.rail[1] + g.rail[3] * (1. - value / 100.),
    ])
}

async fn vertical_mouse(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    focus_projected(tab, VERTICAL[0]).await?;
    let g = shape(tab, VERTICAL[0]).await?;
    check!(
        g.target[2] >= 48. && near(g.target[3], 240.),
        "vertical slider has its full hit target"
    );
    check!(
        near(g.rail[1] - g.target[1], 12.) && near(g.target[3] - g.rail[3], 24.),
        "vertical input and paint use the same padded rail"
    );
    check!(
        near(midpoint(g.handle)[1], g.rail[1] + g.rail[3] * 0.75),
        "initial value is measured from the bottom"
    );
    let initial_ax = vertical_ax(tab, 0, false, 25.).await?;
    let disabled_ax = vertical_ax(tab, 2, true, 50.).await?;
    let [x, y] = vertical_point(tab, &g, 20.).await?;
    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    ready(
        tab,
        "Math.abs(d.vertical.values[0] - 20) < 0.4 && d.vertical.requests[0] === 1",
    )
    .await?;
    tab.mouse_move(x + 12., y).await?;
    ready(
        tab,
        "Math.abs(d.vertical.values[0] - 20) < 0.4 && d.vertical.requests[0] === 1",
    )
    .await?;
    let [move_x, move_y] = vertical_point(tab, &g, 73.).await?;
    tab.mouse_move(move_x, move_y).await?;
    let model = ready(tab, "Math.abs(d.vertical.values[0] - 73) < 0.4 && d.vertical.requests[0] === 2 && d.vertical.engaged[0]")
        .await?;
    let held = shape(tab, VERTICAL[0]).await?;
    let inactive = part(tab, VERTICAL[0], "inactive").await?;
    let label = part(tab, VERTICAL[0], "label").await?;
    let ring = part(tab, VERTICAL[0], "focus").await?;
    check!(
        near(held.handle[3], 2.) && near(held.rail[3], g.rail[3]),
        "engaged vertical handle narrows along the fixed rail"
    );
    check!(
        midpoint(held.active)[1] > midpoint(held.handle)[1]
            && midpoint(inactive)[1] < midpoint(held.handle)[1],
        "vertical active paint starts at the physical bottom"
    );
    check!(
        label[0] > ring[0] + ring[2],
        "LTR label stands beside the handle"
    );
    let image = photo(tab, env, VERTICAL_PNG).await?;
    let colors = [
        sample(&image, held.active)?,
        sample(&image, inactive)?,
        sample(&image, held.handle)?,
    ];
    check_eq!(
        colors,
        [
            rgb(model.palette.primary),
            rgb(model.palette.secondary_container),
            rgb(model.palette.primary)
        ],
        "rotated active, inactive and handle paint match their colors"
    );
    check!(
        ink(&image, label, rgb(model.palette.inverse_on_surface))? > 20,
        "vertical value label contains upright ink"
    );
    resize(tab, 900, WIDE[1]).await?;
    tab.mouse_move(move_x, move_y + 45.).await?;
    tab.mouse_up().await?;
    let canceled = ready(tab, "d.vertical.requests[0] === 2").await?;
    check_eq!(
        canceled.vertical.values[0],
        model.vertical.values[0],
        "viewport change retires the old vertical mouse contact"
    );
    focus_projected(tab, VERTICAL[0]).await?;
    tab.press("Home").await?;
    ready(
        tab,
        "d.vertical.values[0] === 0 && d.vertical.requests[0] === 3",
    )
    .await?;
    tab.press("End").await?;
    ready(
        tab,
        "d.vertical.values[0] === 100 && d.vertical.requests[0] === 4",
    )
    .await?;
    let disabled = shape(tab, VERTICAL[2]).await?;
    let [disabled_x, disabled_y] = vertical_point(tab, &disabled, 25.).await?;
    tab.click(disabled_x, disabled_y).await?;
    ready(
        tab,
        "d.vertical.values[2] === 50 && d.vertical.requests[2] === 0",
    )
    .await?;
    Ok(
        json!({"initial":g,"initial_ax":initial_ax,"disabled_ax":disabled_ax,"held":held,
        "value":model.vertical.values[0],"value_after_resize":canceled.vertical.values[0],
            "inactive":inactive,"colors":colors,"label":label,"ring":ring,
        "palette":model.palette,"capture":VERTICAL_PNG}),
    )
}

async fn touch(tab: &Tab, kind: &str, point: Option<[f64; 2]>) -> Result<()> {
    let points = point.map_or_else(|| json!([]), |[x, y]| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn compact_touch(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, COMPACT[0], COMPACT[1]).await?;
    for setting in [THEME, RTL, LARGE_TEXT] {
        activate(tab, setting).await?;
    }
    for target in CENTERED.into_iter().chain(VERTICAL.into_iter().take(2)) {
        tab.press("Tab").await?;
        focused(tab, target).await?;
    }
    tab.press("ArrowRight").await?;
    ready(tab, "d.dark && d.rtl && d.text_scale === 2 && d.vertical.values[1] === 62.5 && d.vertical.requests[1] === 1").await?;
    let g = shape(tab, VERTICAL[1]).await?;
    check!(
        contains(bounds(tab, SCROLL).await?, g.target),
        "keyboard reveals complete vertical target"
    );
    check!(
        near(midpoint(g.handle)[1], g.rail[1] + g.rail[3] * 0.375),
        "RTL keeps vertical bottom to top"
    );
    let first = vertical_point(tab, &g, 13.).await?;
    let moved = vertical_point(tab, &g, 81.).await?;
    touch(tab, "touchStart", Some(first)).await?;
    ready(
        tab,
        "d.vertical.values[1] === 12.5 && d.vertical.requests[1] === 2",
    )
    .await?;
    touch(tab, "touchMove", Some(moved)).await?;
    ready(
        tab,
        "d.vertical.values[1] === 75 && d.vertical.requests[1] === 3",
    )
    .await?;
    touch(tab, "touchCancel", None).await?;
    ready(
        tab,
        "d.vertical.values[1] === 75 && d.vertical.requests[1] === 3",
    )
    .await?;
    let fresh = vertical_point(tab, &g, 31.).await?;
    touch(tab, "touchStart", Some(fresh)).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "d.vertical.values[1] === 25 && d.vertical.requests[1] === 4",
    )
    .await?;
    focus_projected(tab, VERTICAL[1]).await?;
    let model = ready(tab, "d.vertical.engaged[1]").await?;
    let focused_geometry = shape(tab, VERTICAL[1]).await?;
    let label = part(tab, VERTICAL[1], "label").await?;
    let ring = part(tab, VERTICAL[1], "focus").await?;
    let viewport = [0., 0., f64::from(COMPACT[0]), f64::from(COMPACT[1])];
    check!(
        contains(viewport, label)
            && contains(viewport, ring)
            && contains(bounds(tab, SCROLL).await?, focused_geometry.target),
        "320 dp contains the complete focus, target and 200 percent label"
    );
    check!(
        label[0] + label[2] < ring[0],
        "RTL moves the upright label to the left"
    );
    let image = photo(tab, env, COMPACT_PNG).await?;
    let label_ink = ink(&image, label, rgb(model.palette.inverse_on_surface))?;
    let label_fill = ink(&image, label, rgb(model.palette.inverse_surface))?;
    let focus_ink = ink(&image, ring, rgb(model.palette.secondary))?;
    check!(
        label_ink > 40 && label_fill > 100 && focus_ink > 30,
        "compact dark screenshot contains visible label, bubble and focus outline"
    );
    let description = vertical_ax(tab, 1, false, 25.).await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"viewport":COMPACT,"rtl":model.rtl,"dark":model.dark,"text_scale":model.text_scale,
        "geometry":focused_geometry,"label":label,"ring":ring,"label_ink":label_ink,"label_fill":label_fill,
        "focus_ink":focus_ink,"aria":description,"vertical":model.vertical,"back_focus":SHOW,
        "palette":model.palette,"capture":COMPACT_PNG}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(WIDE[0], WIDE[1] + 44, 2.).await?;
    let mut report = json!({"browser":chrome.version,"adapter":null});
    let result: Result<()> = async {
        step("M302", "Centered sliders show negative, neutral, and positive tracks with two end stops");
        route(&tab, url).await?;
        report["adapter"] = tab.eval("(async()=>{ const a=await navigator.gpu.requestAdapter(); const {vendor,architecture,isFallbackAdapter}=a.info; return {vendor,architecture,isFallbackAdapter}; })()").await?;
        report["centered"] = centered_paint(env, &tab).await.context("M302 centered paint and AX")?;
        step("M303", "Centered keyboard reaches the neutral and exact endpoints and mirrors in RTL");
        report["centered_keys"] = centered_keys(env, &tab).await.context("M303 centered keys")?;
        step("M304", "Vertical AX semantics and trusted mouse follow the measured bottom-to-top rail");
        report["vertical_mouse"] = vertical_mouse(env, &tab, url).await.context("M304 vertical mouse")?;
        step("M305", "Vertical compact dark RTL has touch cancellation, a 200 percent label and focus restoration");
        report["compact"] = compact_touch(env, &tab, url).await.context("M305 compact touch")?;
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-slider-forms-failure.png"))
            .await;
    }
    env.write_json("material-slider-forms.json", &report)?;
    result?;
    check!(
        tab.errors().is_empty(),
        "Slider forms Chrome page has no errors"
    );
    Ok(())
}
