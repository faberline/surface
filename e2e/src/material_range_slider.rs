//! Material Range Slider through the real Chrome canvas and accessibility tree.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{focus, state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M306", "M307", "M308", "M309"];

const SHOW: &str = "range-slider-demo-show";
const BACK: &str = "range-slider-demo-back";
const SCROLL: &str = "range-slider-demo-scroll";
const COLLAPSE: &str = "range-slider-demo-collapse";
const SETTINGS: [&str; 3] = [
    "range-slider-demo-theme",
    "range-slider-demo-rtl",
    "range-slider-demo-large-text",
];
const IDS: [&str; 4] = [
    "range-slider-demo-continuous",
    "range-slider-demo-discrete",
    "range-slider-demo-equal",
    "range-slider-demo-disabled",
];
const GROUP_NAMES: [&str; 4] = ["連續區間", "階梯區間", "相等端點", "停用區間"];
const NAMES: [[&str; 2]; 4] = [
    ["連續下限", "連續上限"],
    ["階梯下限", "階梯上限"],
    ["相等下限", "相等上限"],
    ["停用下限", "停用上限"],
];
const WIDE: [u32; 2] = [960, 1160];
const COMPACT: [u32; 2] = [320, 880];
const WIDE_PNG: &str = "material-range-slider-wide-light.png";
const OVERLAP_PNG: &str = "material-range-slider-overlap-focus.png";
const HELD_PNG: &str = "material-range-slider-held-loss.png";
const COMPACT_PNG: &str = "material-range-slider-compact-dark-rtl-200.png";
const TRACE: &str = r#"
    (() => {
        window.rangeSliderContacts = [];
        const canvas = document.getElementById('surface');
        for (const kind of ['pointerdown', 'gotpointercapture', 'pointermove',
                            'lostpointercapture', 'pointerup', 'pointercancel']) {
            document.addEventListener(kind, event => {
                if (event.target !== canvas) return;
                window.rangeSliderContacts.push({
                    kind, id: event.pointerId, device: event.pointerType,
                    trusted: event.isTrusted, buttons: event.buttons,
                    captured: canvas.hasPointerCapture(event.pointerId)
                });
            }, {capture: true, passive: true});
        }
    })()
"#;

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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Sample {
    id: String,
    min: f64,
    max: f64,
    intervals: Option<u16>,
    values: [f64; 2],
    labels: [String; 2],
    requests: [u32; 2],
    engaged: Option<String>,
    disabled: bool,
}

#[derive(Deserialize)]
struct Model {
    visible: bool,
    rtl: bool,
    dark: bool,
    text_scale: f64,
    collapses: u32,
    samples: [Sample; 4],
    palette: Palette,
}

#[derive(Serialize)]
struct Shape {
    owner: Rect,
    rail: Rect,
    active: Rect,
    inactive: [Rect; 2],
    handles: [Rect; 2],
    targets: [Rect; 2],
    stops: [Rect; 2],
}

fn thumb(index: usize, upper: bool) -> String {
    let end = if upper { "upper" } else { "lower" };
    format!("{}-{end}", IDS[index])
}

fn center([x, y, width, height]: Rect) -> [f64; 2] {
    [x + width / 2., y + height / 2.]
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn rgb([r, g, b, _]: [u8; 4]) -> Rgb {
    [r, g, b]
}

fn sample(image: &Raster, rect: Rect) -> Result<Rgb> {
    let [x, y] = center(rect);
    image.sample(x, y)
}

fn track_sample(image: &Raster, rect: Rect) -> Result<Rgb> {
    // The middle of a discrete span can contain a dot.
    image.sample(center(rect)[0], rect[1] + rect[3] * 0.3)
}

fn ink(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

async fn part(tab: &Tab, owner: &str, name: &str) -> Result<Rect> {
    bounds(tab, &format!("{owner}-{name}")).await
}

async fn shape(tab: &Tab, index: usize) -> Result<Shape> {
    let id = IDS[index];
    Ok(Shape {
        owner: bounds(tab, id).await?,
        rail: part(tab, id, "rail").await?,
        active: part(tab, id, "active").await?,
        inactive: [
            part(tab, id, "inactive-min").await?,
            part(tab, id, "inactive-max").await?,
        ],
        handles: [
            part(tab, id, "lower-handle").await?,
            part(tab, id, "upper-handle").await?,
        ],
        targets: [part(tab, id, "lower").await?, part(tab, id, "upper").await?],
        stops: [
            part(tab, id, "stop-min").await?,
            part(tab, id, "stop-max").await?,
        ],
    })
}

async fn ready(tab: &Tab, condition: &str) -> Result<Model> {
    until(tab, &format!("(d => {condition})(s.range_slider_demo)")).await?;
    tab.settle().await?;
    Ok(serde_json::from_value(
        state(tab).await?["range_slider_demo"].clone(),
    )?)
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1900).await?;
    reduced_motion(tab, true).await?;
    activate(tab, "section-0").await?;
    until(tab, "s.section === 0 && !s.range_slider_demo.visible").await?;
    activate(tab, SHOW).await?;
    ready(
        tab,
        "d.visible && d.samples[0].values[0] === 25 && d.collapses === 0",
    )
    .await?;
    resize(tab, WIDE[0], WIDE[1]).await
}

async fn photo(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn rail_checks(g: &Shape, state: &Sample, rtl: bool) -> Result<()> {
    check!(
        near(g.rail[0] - g.owner[0], 24.)
            && near(g.owner[2] - g.rail[2], 48.)
            && g.targets.iter().all(|&target| near(target[2], 48.)
                && target[3] >= 48.
                && contains(g.owner, target)),
        "a shared measured rail has two independent 48 dp thumb targets"
    );
    for (index, value) in state.values.into_iter().enumerate() {
        let expected = (value - state.min) / (state.max - state.min);
        let fraction = if rtl { 1. - expected } else { expected };
        let x = g.rail[0] + g.rail[2] * fraction;
        check!(
            near(center(g.targets[index])[0], x) && near(center(g.handles[index])[0], x),
            "thumb {index} uses the same measured value position for targets and paint"
        );
    }
    Ok(())
}

async fn accessibility(tab: &Tab, index: usize, model: &Sample) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let group = ax(&tree, "group", GROUP_NAMES[index])?;
    let limits = [[model.min, model.values[1]], [model.values[0], model.max]];
    let mut twins = Vec::new();
    for (i, is_upper) in [false, true].into_iter().enumerate() {
        let id = thumb(index, is_upper);
        let element = projected(&id);
        let attributes = tab.eval(&format!(
            "['aria-valuemin','aria-valuemax','aria-valuenow','aria-valuetext','aria-disabled'].map(name => {element}.getAttribute(name))"
        )).await?;
        check_eq!(
            attributes,
            json!([limits[i][0].to_string(), limits[i][1].to_string(), model.values[i].to_string(),
                   model.labels[i], if model.disabled { Some("true") } else { None }]),
            "each projected thumb exposes the peer-constrained interval, value, text and disability"
        );
        let node = ax(&tree, "slider", NAMES[index][i])?;
        let minimum = ax_property(node, "valuemin").and_then(Value::as_f64);
        let maximum = ax_property(node, "valuemax").and_then(Value::as_f64);
        let current = node["value"]["value"].as_f64();
        check_eq!(
            [minimum, maximum, current],
            [
                Some(limits[i][0]),
                Some(limits[i][1]),
                Some(model.values[i])
            ],
            "Chrome exposes the separate constrained accessible range"
        );
        check_eq!(
            ax_property(node, "disabled") == Some(&json!(true)),
            model.disabled,
            "Chrome exposes thumb disabled state"
        );
        twins.push(
            json!({"id":id,"name":NAMES[index][i],"attributes":attributes,"limits":limits[i],
            "value":current,"disabled":model.disabled,"node":node["nodeId"]}),
        );
    }
    check!(
        twins[0]["node"] != twins[1]["node"],
        "Chrome retains both thumb nodes at equal values"
    );
    Ok(json!({"group":GROUP_NAMES[index],"node":group["nodeId"],"thumbs":twins}))
}

async fn canvas_point(
    tab: &Tab,
    shape: &Shape,
    value: f64,
    min: f64,
    max: f64,
    rtl: bool,
) -> Result<[f64; 2]> {
    let canvas = tab.canvas_box().await?;
    let fraction = (value - min) / (max - min);
    let along = if rtl { 1. - fraction } else { fraction };
    Ok([
        canvas[0] + shape.rail[0] + shape.rail[2] * along,
        canvas[1] + center(shape.rail)[1],
    ])
}

async fn wide(env: &Env, tab: &Tab) -> Result<Value> {
    let model = ready(tab, "d.visible && !d.dark && !d.rtl && d.text_scale === 1").await?;
    check!(
        model.visible && model.collapses == 0,
        "wide route uses initial Range fixture"
    );
    check_eq!(
        model.samples.each_ref().map(|sample| sample.values),
        [[25., 75.], [15., 65.], [50., 50.], [25., 75.]],
        "all initial ranges have their authored values"
    );
    check_eq!(
        model.samples[1].intervals,
        Some(8),
        "signed discrete Range retains eight intervals"
    );
    let image = photo(env, tab, WIDE_PNG).await?;
    let mut shapes = Vec::new();
    let mut names = Vec::new();
    let mut colors = Vec::new();
    for (index, fixture) in model.samples.iter().enumerate() {
        check_eq!(fixture.id.as_str(), IDS[index], "stable Range owner");
        check_eq!(
            fixture.requests,
            [0, 0],
            "initial layout does not issue adjustment callbacks"
        );
        let g = shape(tab, index).await?;
        rail_checks(&g, fixture, false)?;
        check_eq!(
            g.inactive[0][3],
            [16., 24., 16., 40.][index],
            "inactive track size token"
        );
        if index == 2 {
            check!(
                g.active[2] == 0. && g.targets[0] == g.targets[1],
                "coincident targets preserve both logical thumbs"
            );
        } else {
            check!(
                near(g.active[0] - right(g.handles[0]), 6.)
                    && near(g.handles[1][0] - right(g.active), 6.),
                "active track respects both handle clearances"
            );
        }
        let active = if g.active[2] > 0. {
            Some(track_sample(&image, g.active)?)
        } else {
            None
        };
        let inactive = [
            track_sample(&image, g.inactive[0])?,
            track_sample(&image, g.inactive[1])?,
        ];
        let handles = [sample(&image, g.handles[0])?, sample(&image, g.handles[1])?];
        let stops = [sample(&image, g.stops[0])?, sample(&image, g.stops[1])?];
        if fixture.disabled {
            check!(
                handles
                    .into_iter()
                    .all(|color| color != rgb(model.palette.primary)
                        && color != rgb(model.palette.background)),
                "disabled handles remain visible in muted ink"
            );
        } else {
            check_eq!(
                inactive,
                [rgb(model.palette.secondary_container); 2],
                "inactive track matches its palette"
            );
            check_eq!(
                handles,
                [rgb(model.palette.primary); 2],
                "both handle centers are painted"
            );
            check_eq!(
                stops,
                [rgb(model.palette.on_secondary_container); 2],
                "both outer stops are painted"
            );
            if let Some(active) = active {
                check_eq!(
                    active,
                    rgb(model.palette.primary),
                    "the selected interval is painted"
                );
            }
        }
        colors.push(json!({"active":active,"inactive":inactive,"handles":handles,"stops":stops}));
        shapes.push(g);
        names.push(accessibility(tab, index, fixture).await?);
    }
    let [x, y] = canvas_point(tab, &shapes[3], 42., 0., 100., false).await?;
    tab.click(x, y).await?;
    let after = ready(tab, "d.samples[3].requests.every(n => n === 0)").await?;
    check_eq!(
        after.samples[3].values,
        model.samples[3].values,
        "a disabled shared-rail tap remains inert"
    );
    Ok(
        json!({"viewport":WIDE,"shapes":shapes,"colors":colors,"ax":names,"palette":model.palette,"capture":WIDE_PNG}),
    )
}

async fn engaged_paint(
    env: &Env,
    tab: &Tab,
    capture: &str,
    owner: &str,
    upper: bool,
    g: &Shape,
    palette: &Palette,
) -> Result<Value> {
    let end = if upper { "upper" } else { "lower" };
    let label = part(tab, owner, &format!("{end}-label")).await?;
    let ring = part(tab, owner, &format!("{end}-focus")).await?;
    check!(
        near(g.handles[usize::from(upper)][2], 2.)
            && near(ring[1] - (label[1] + label[3]), 4.)
            && label[0] >= g.owner[0]
            && right(label) <= right(g.owner) + 0.75,
        "one engaged thumb narrows and its measured label clears the focus outline by four dp"
    );
    let image = photo(env, tab, capture).await?;
    let label_ink = ink(&image, label, rgb(palette.inverse_on_surface))?;
    let label_fill = ink(&image, label, rgb(palette.inverse_surface))?;
    let focus_ink = ink(&image, ring, rgb(palette.secondary))?;
    check!(
        label_ink > 20 && label_fill > 100,
        "actual Canvas has the readable value label"
    );
    Ok(
        json!({"label":label,"ring":ring,"label_ink":label_ink,"label_fill":label_fill,"focus_ink":focus_ink}),
    )
}

async fn overlap(env: &Env, tab: &Tab) -> Result<Value> {
    let lower = thumb(0, false);
    let upper = thumb(0, true);
    activate(tab, COLLAPSE).await?;
    ready(
        tab,
        "d.collapses === 1 && d.samples[0].values.every(v => v === 50)",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, &lower).await?;
    tab.press("Tab").await?;
    focused(tab, &upper).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &lower).await?;
    let at_same = ready(tab, "d.samples[0].engaged === 'lower'").await?;
    check_eq!(
        at_same.samples[0].requests,
        [0, 0],
        "Tab into both equal targets sends no value request"
    );
    let g = shape(tab, 0).await?;
    check!(
        g.targets[0] == g.targets[1] && g.active[2] == 0. && near(g.handles[1][2], 4.),
        "coincident endpoints have a single focused face and distinct target IDs"
    );
    let paint = engaged_paint(env, tab, OVERLAP_PNG, IDS[0], false, &g, &at_same.palette).await?;
    check!(
        paint["focus_ink"]
            .as_u64()
            .is_some_and(|pixels| pixels > 30),
        "the focused coincident handle has real focus paint"
    );
    let equal_ax = accessibility(tab, 0, &at_same.samples[0]).await?;

    tab.press("End").await?;
    let no_change = ready(tab, "d.samples[0].values.every(v => v === 50)").await?;
    check_eq!(
        no_change.samples[0].requests,
        [0, 0],
        "End cannot cross the peer or send a duplicate"
    );
    tab.press("Home").await?;
    ready(
        tab,
        "d.samples[0].values[0] === 0 && d.samples[0].requests[0] === 1",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, &upper).await?;
    tab.press("Home").await?;
    let locked = ready(
        tab,
        "d.samples[0].values.every(v => v === 0) && d.samples[0].requests[1] === 1",
    )
    .await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &lower).await?;
    let locks = accessibility(tab, 0, &locked.samples[0]).await?;
    tab.press("Tab").await?;
    focused(tab, &upper).await?;
    tab.press("End").await?;
    let extremes = ready(
        tab,
        "d.samples[0].values[1] === 100 && d.samples[0].requests[1] === 2",
    )
    .await?;
    check_eq!(
        extremes.samples[0].values,
        [0., 100.],
        "Home/End keep exact global endpoints"
    );

    focus_projected(tab, &thumb(1, false)).await?;
    tab.press("ArrowRight").await?;
    ready(
        tab,
        "d.samples[1].values[0] === 27.5 && d.samples[1].requests[0] === 1",
    )
    .await?;
    tab.press("End").await?;
    ready(
        tab,
        "d.samples[1].values[0] === 65 && d.samples[1].requests[0] === 2",
    )
    .await?;
    tab.press("ArrowRight").await?;
    let constrained = ready(tab, "d.samples[1].values[0] === 65").await?;
    check_eq!(
        constrained.samples[1].requests,
        [2, 0],
        "crossing the signed peer is a no-op"
    );
    tab.press("Tab").await?;
    focused(tab, &thumb(1, true)).await?;
    tab.press("ArrowRight").await?;
    let stepped = ready(
        tab,
        "d.samples[1].values[1] === 77.5 && d.samples[1].requests[1] === 1",
    )
    .await?;
    check_eq!(
        stepped.samples[1].values,
        [65., 77.5],
        "moving the peer keeps the original fractional grid"
    );
    Ok(
        json!({"viewport":WIDE,"shape":g,"paint":paint,"initial_ax":equal_ax,"locked_ax":locks,
        "extremes":extremes.samples[0],"discrete":stepped.samples[1],"palette":at_same.palette,"capture":OVERLAP_PNG}),
    )
}

async fn pressed(tab: &Tab, id: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        json!(id)
    ))
    .await?;
    tab.settle().await
}

async fn mouse(tab: &Tab, kind: &str, point: [f64; 2], held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":point[0],"y":point[1],"button":"left",
        "buttons":u8::from(held),"clickCount":1,"pointerType":"mouse"}),
    )
    .await?;
    Ok(())
}

async fn touch(tab: &Tab, kind: &str, point: Option<[f64; 2]>) -> Result<()> {
    let points = point.map_or_else(|| json!([]), |[x, y]| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    Ok(())
}

async fn capture_owner(tab: &Tab) -> Result<i64> {
    tab.wait_for("window.rangeSliderContacts.some(e => e.kind === 'gotpointercapture' && e.device === 'mouse' && e.trusted && e.captured)").await?;
    tab.eval("window.rangeSliderContacts.find(e => e.kind === 'gotpointercapture' && e.device === 'mouse' && e.trusted && e.captured).id")
        .await?
        .as_i64().context("Chrome must report a real mouse capture on the canvas")
}

async fn trusted_order(tab: &Tab, owner: i64, order: &[&str]) -> Result<Value> {
    let trace = tab.eval("window.rangeSliderContacts").await?;
    let mut events = trace
        .as_array()
        .context("expected Chrome pointer trace")?
        .iter();
    check!(
        order
            .iter()
            .all(|kind| events.any(|event| event["trusted"] == true
                && event["device"] == "mouse"
                && event["id"] == owner
                && event["kind"] == *kind)),
        "trusted owner {owner} must emit {order:?} in order: {trace}"
    );
    Ok(trace)
}

async fn mouse_capture(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    let g = shape(tab, 0).await?;
    let at = |value| canvas_point(tab, &g, value, 0., 100., false);
    let [x, y] = at(40.).await?;
    tab.click(x, y).await?;
    let before = ready(
        tab,
        "Math.abs(d.samples[0].values[0] - 40) < 0.3 &&
        d.samples[0].values[1] === 75 && d.samples[0].requests[0] === 1",
    )
    .await?;
    focus(tab, &thumb(0, false), "surface").await?;
    check_eq!(
        before.samples[0].requests,
        [1, 0],
        "one tap on the shared rail selects only its nearer lower thumb"
    );
    tab.eval(TRACE).await?;

    let upper = thumb(0, true);
    mouse(tab, "mousePressed", at(77.).await?, true).await?;
    ready(
        tab,
        "Math.abs(d.samples[0].values[1] - 77) < 0.3 && d.samples[0].requests[1] === 1",
    )
    .await?;
    mouse(tab, "mouseMoved", at(86.).await?, true).await?;
    let held = ready(
        tab,
        "Math.abs(d.samples[0].values[1] - 86) < 0.3 && d.samples[0].requests[1] === 2",
    )
    .await?;
    let owner = capture_owner(tab).await?;
    pressed(tab, Some(&upper)).await?;
    focus(tab, &upper, "surface").await?;
    check_eq!(
        held.samples[0].values[0],
        before.samples[0].values[0],
        "holding the upper thumb preserves its peer"
    );
    let held_shape = shape(tab, 0).await?;
    rail_checks(&held_shape, &held.samples[0], false)?;
    let paint = engaged_paint(env, tab, HELD_PNG, IDS[0], true, &held_shape, &held.palette).await?;

    let foreign = at(14.).await?;
    touch(tab, "touchStart", Some(foreign)).await?;
    touch(tab, "touchEnd", None).await?;
    tab.wait_for("window.rangeSliderContacts.some(e => e.kind === 'pointerup' && e.device === 'touch' && e.trusted)").await?;
    let foreign_trace = tab
        .eval("window.rangeSliderContacts.filter(e => e.device === 'touch' && e.trusted)")
        .await?;
    check!(
        foreign_trace.as_array().is_some_and(|events| {
            events
                .iter()
                .any(|event| event["kind"] == "pointerdown" && event["id"] != owner)
                && events
                    .iter()
                    .any(|event| event["kind"] == "pointerup" && event["id"] != owner)
        }),
        "a real foreign touch pressed and released while the mouse owned the Range"
    );
    pressed(tab, Some(&upper)).await?;
    let after_foreign = ready(tab, "d.samples[0].requests[1] === 2").await?;
    check_eq!(
        after_foreign.samples[0],
        held.samples[0],
        "foreign touch cannot move or release the mouse-owned thumb"
    );

    tab.eval(&format!("document.getElementById('surface').dispatchEvent(
      new PointerEvent('lostpointercapture',{{pointerId:{owner},pointerType:'mouse',buttons:1,bubbles:true}}))")).await?;
    pressed(tab, Some(&upper)).await?;
    check_eq!(tab.eval(&format!("document.getElementById('surface').hasPointerCapture({owner}) &&
        window.rangeSliderContacts.some(e=>!e.trusted && e.id==={owner} && e.kind==='lostpointercapture')")).await?,
        json!(true), "synthetic loss leaves actual owner capture intact");

    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    // Chrome delivers pending loss before processing this different release coordinate.
    mouse(tab, "mouseReleased", at(95.).await?, false).await?;
    let loss = trusted_order(
        tab,
        owner,
        &[
            "pointerdown",
            "gotpointercapture",
            "pointermove",
            "lostpointercapture",
            "pointerup",
        ],
    )
    .await?;
    pressed(tab, None).await?;
    let canceled = ready(tab, "d.samples[0].requests[1] === 2").await?;
    check_eq!(
        canceled.samples[0].values,
        held.samples[0].values,
        "loss retains the last accepted live pairs"
    );
    check_eq!(
        canceled.samples[0].requests,
        held.samples[0].requests,
        "late release issues zero callbacks"
    );
    until(tab, &format!("s.focused === {}", json!(upper))).await?;

    tab.eval("window.rangeSliderContacts.length = 0").await?;
    mouse(tab, "mousePressed", at(80.).await?, true).await?;
    ready(
        tab,
        "Math.abs(d.samples[0].values[1] - 80) < 0.3 && d.samples[0].requests[1] === 3",
    )
    .await?;
    mouse(tab, "mouseMoved", at(90.).await?, true).await?;
    let fresh_owner = capture_owner(tab).await?;
    mouse(tab, "mouseReleased", at(90.).await?, false).await?;
    let normal = trusted_order(
        tab,
        fresh_owner,
        &[
            "pointerdown",
            "gotpointercapture",
            "pointermove",
            "pointerup",
            "lostpointercapture",
        ],
    )
    .await?;
    pressed(tab, None).await?;
    let fresh = ready(
        tab,
        "Math.abs(d.samples[0].values[1] - 90) < 0.3 && d.samples[0].requests[1] === 4",
    )
    .await?;
    check_eq!(
        fresh.samples[0].requests,
        [1, 4],
        "a fresh gesture keeps ordinary updates and releases once"
    );
    Ok(
        json!({"viewport":WIDE,"shape":held_shape,"paint":paint,"before":before.samples[0],"held":held.samples[0],
        "after":canceled.samples[0],"fresh":fresh.samples[0],"foreign_events":foreign_trace,
        "loss_events":loss,"normal_events":normal,"palette":held.palette,"capture":HELD_PNG}),
    )
}

async fn compact(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, COMPACT[0], COMPACT[1]).await?;
    for setting in SETTINGS {
        activate(tab, setting).await?;
    }
    ready(tab, "d.dark && d.rtl && d.text_scale === 2").await?;
    focus_projected(tab, &thumb(2, true)).await?;
    let equal = shape(tab, 2).await?;
    check!(
        equal.targets[0] == equal.targets[1] && equal.active[2] == 0.,
        "compact RTL keeps two separate overlapping thumb targets"
    );
    let at = |value| canvas_point(tab, &equal, value, 0., 100., true);
    tab.eval(TRACE).await?;
    touch(tab, "touchStart", Some(at(50.).await?)).await?;
    pressed(tab, None).await?;
    let pending = ready(tab, "d.samples[2].values.every(v => v === 50)").await?;
    check_eq!(
        pending.samples[2].requests,
        [0, 0],
        "a touch at coincident endpoints waits for direction"
    );
    touch(tab, "touchMove", Some(at(28.).await?)).await?;
    let held = ready(
        tab,
        "Math.abs(d.samples[2].values[0] - 28) < 0.8 &&
        d.samples[2].values[1] === 50 && d.samples[2].requests[0] === 1",
    )
    .await?;
    pressed(tab, Some(&thumb(2, false))).await?;
    touch(tab, "touchCancel", None).await?;
    pressed(tab, None).await?;
    let after = ready(tab, "d.samples[2].requests[0] === 1").await?;
    check_eq!(
        after.samples[2].values,
        held.samples[2].values,
        "touch cancel preserves the accepted RTL pair"
    );
    check_eq!(
        after.samples[2].requests,
        [1, 0],
        "touch cancel sends no additional adjustment"
    );
    let canceled_trace = tab.eval("window.rangeSliderContacts").await?;
    let owner = canceled_trace
        .as_array()
        .and_then(|events| {
            events.iter().find(|event| {
                event["kind"] == "pointerdown"
                    && event["device"] == "touch"
                    && event["trusted"] == true
            })
        })
        .and_then(|event| event["id"].as_i64())
        .context("Chrome must send the trusted initial touch")?;
    check!(
        canceled_trace.as_array().is_some_and(|events| events
            .iter()
            .any(|event| event["kind"] == "pointercancel"
                && event["id"] == owner
                && event["trusted"] == true)),
        "the coincident touch had actual trusted cancellation"
    );

    tab.eval("window.rangeSliderContacts.length = 0").await?;
    touch(tab, "touchStart", Some(at(73.).await?)).await?;
    touch(tab, "touchEnd", None).await?;
    let fresh = ready(
        tab,
        "Math.abs(d.samples[2].values[1] - 73) < 0.8 &&
        d.samples[2].requests[1] === 1",
    )
    .await?;
    check_eq!(
        fresh.samples[2].values[0],
        held.samples[2].values[0],
        "fresh touch adjusts the other thumb only"
    );
    let fresh_trace = tab.eval("window.rangeSliderContacts").await?;
    check!(
        fresh_trace.as_array().is_some_and(|events| {
            let mut events = events.iter();
            ["pointerdown", "pointerup"].into_iter().all(|kind| {
                events.any(|event| {
                    event["kind"] == kind && event["device"] == "touch" && event["trusted"] == true
                })
            })
        }),
        "the fresh touch tap uses a trusted press and release"
    );

    focus_projected(tab, &thumb(2, true)).await?;
    tab.press("Tab").await?;
    focused(tab, BACK).await?;
    focus_projected(tab, &thumb(1, false)).await?;
    tab.press("ArrowLeft").await?;
    let model = ready(
        tab,
        "d.dark && d.rtl && d.text_scale === 2 && d.samples[1].values[0] === 27.5 &&
        d.samples[1].requests[0] === 1 && d.samples[1].engaged === 'lower'",
    )
    .await?;
    let g = shape(tab, 1).await?;
    rail_checks(&g, &model.samples[1], true)?;
    check!(
        center(g.handles[0])[0] > center(g.handles[1])[0]
            && contains(bounds(tab, SCROLL).await?, g.owner),
        "RTL mirrors the shared rail while keeping the full target inside its scroll view"
    );
    let paint = engaged_paint(env, tab, COMPACT_PNG, IDS[1], false, &g, &model.palette).await?;
    let label = serde_json::from_value::<Rect>(paint["label"].clone())?;
    let ring = serde_json::from_value::<Rect>(paint["ring"].clone())?;
    let viewport = [0., 0., f64::from(COMPACT[0]), f64::from(COMPACT[1])];
    check!(
        label[3] >= 63.
            && contains(viewport, ring)
            && contains(viewport, label)
            && paint["focus_ink"]
                .as_u64()
                .is_some_and(|pixels| pixels > 30),
        "dark RTL at 320 dp contains the scaled label and its actual focus paint"
    );
    let image = Raster::read(tab, env.artifact(COMPACT_PNG)).await?;
    let colors = [
        track_sample(&image, g.active)?,
        track_sample(&image, g.inactive[0])?,
        track_sample(&image, g.inactive[1])?,
        sample(&image, g.handles[0])?,
    ];
    check_eq!(
        colors,
        [
            rgb(model.palette.primary),
            rgb(model.palette.secondary_container),
            rgb(model.palette.secondary_container),
            rgb(model.palette.primary)
        ],
        "dark active and inactive paint use the real palette"
    );
    let semantics = accessibility(tab, 1, &model.samples[1]).await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"viewport":COMPACT,"rtl":model.rtl,"dark":model.dark,"text_scale":model.text_scale,
        "shape":g,"paint":paint,"colors":colors,"ax":semantics,"held":held.samples[2],"after":after.samples[2],
        "fresh":fresh.samples[2],"touch_cancel":canceled_trace,"touch_fresh":fresh_trace,
        "discrete":model.samples[1],"back_focus":SHOW,"palette":model.palette,"capture":COMPACT_PNG}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(WIDE[0], WIDE[1] + 44, 2.).await?;
    let mut report = json!({"browser":chrome.version,"adapter":null});
    let result: Result<()> = async {
        step(STEPS[0], "Range shows two constrained Chrome sliders, one rail and actual light paint");
        route(&tab, url).await?;
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu.requestAdapter(); const {vendor,architecture,isFallbackAdapter}=a.info; return {vendor,architecture,isFallbackAdapter}})()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(), "the actual WebGPU adapter reports fallback status");
        report["wide"] = wide(env, &tab).await.context("M306 Range light paint and AX")?;
        step(STEPS[1], "Range keyboard keeps both coincident stops, locked bounds and the signed discrete grid");
        report["overlap"] = overlap(env, &tab).await.context("M307 Range coincident keyboard and AX")?;
        step(STEPS[2], "Range drops a trusted lost mouse capture and ignores a real foreign touch release");
        report["capture_loss"] = mouse_capture(env, &tab, url).await.context("M308 Range real capture loss")?;
        step(STEPS[3], "Range touch cancellation and focus keep the 320 dp dark RTL 200 percent layout usable");
        report["compact"] = compact(env, &tab, url).await.context("M309 Range compact touch and paint")?;
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["pointer_trace"] = tab
            .eval("window.rangeSliderContacts ?? []")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-range-slider-failure.png"))
            .await;
    }
    env.write_json("material-range-slider.json", &report)?;
    result?;
    check!(
        tab.errors().is_empty(),
        "Range Slider Chrome page has no errors"
    );
    Ok(())
}
