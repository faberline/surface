//! Standard Material sliders through the real Gallery and browser inputs.

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

const SHOW: &str = "slider-demo-show";
const BACK: &str = "slider-demo-back";
const THEME: &str = "slider-demo-theme";
const RTL: &str = "slider-demo-rtl";
const LARGE_TEXT: &str = "slider-demo-large-text";
const SCROLL: &str = "slider-demo-scroll";
const IDS: [&str; 6] = [
    "slider-demo-xs",
    "slider-demo-s",
    "slider-demo-m",
    "slider-demo-l",
    "slider-demo-xl",
    "slider-demo-disabled",
];
const NAMES: [&str; 6] = [
    "特小連續",
    "小型階梯",
    "中型滑桿",
    "大型滑桿",
    "特大滑桿",
    "停用滑桿",
];
const TRACK: [f64; 5] = [16., 24., 40., 56., 96.];
const HANDLE: [f64; 5] = [44., 44., 52., 68., 108.];
const WIDE: [u32; 2] = [960, 1040];
const COMPACT: [u32; 2] = [320, 880];
const WIDE_PNG: &str = "material-standard-slider-wide-light.png";
const KEY_PNG: &str = "material-standard-slider-keyboard.png";
const POINTER_PNG: &str = "material-standard-slider-live-drag.png";
const COMPACT_PNG: &str = "material-standard-slider-compact-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Deserialize, Serialize)]
struct Palette {
    background: [u8; 4],
    primary: [u8; 4],
    on_primary: [u8; 4],
    secondary: [u8; 4],
    secondary_container: [u8; 4],
    on_secondary_container: [u8; 4],
    inverse_surface: [u8; 4],
    inverse_on_surface: [u8; 4],
}

#[derive(Deserialize, Serialize)]
struct Model {
    rtl: bool,
    text_scale: f64,
    engaged: [bool; 6],
    values: [f64; 6],
    requests: [u32; 6],
    palette: Palette,
}

#[derive(Serialize)]
struct Geometry {
    id: &'static str,
    hit: Rect,
    rail: Rect,
    active: Rect,
    inactive: Rect,
    handle: Rect,
}

fn part(id: &str, suffix: &str) -> String {
    format!("{id}-{suffix}")
}

fn center([x, y, w, h]: Rect) -> [f64; 2] {
    [x + w / 2., y + h / 2.]
}

fn rgb([r, g, b, _]: [u8; 4]) -> Rgb {
    [r, g, b]
}

async fn ready(tab: &Tab, condition: &str) -> Result<Model> {
    until(tab, &format!("(d => {condition})(s.slider_demo)")).await?;
    tab.settle().await?;
    Ok(serde_json::from_value(
        state(tab).await?["slider_demo"].clone(),
    )?)
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1900).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.slider_demo.visible").await?;
    activate(tab, SHOW).await?;
    ready(
        tab,
        "d.visible && d.values[0] === 25 && d.requests.every(n => n === 0)",
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

async fn geometry(tab: &Tab, id: &'static str) -> Result<Geometry> {
    Ok(Geometry {
        id,
        hit: bounds(tab, id).await?,
        rail: bounds(tab, &part(id, "rail")).await?,
        active: bounds(tab, &part(id, "active")).await?,
        inactive: bounds(tab, &part(id, "inactive")).await?,
        handle: bounds(tab, &part(id, "handle")).await?,
    })
}

fn pixel(image: &Raster, rect: Rect) -> Result<Rgb> {
    let [x, y] = center(rect);
    image.sample(x, y)
}

fn ink(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

async fn ax_range(tab: &Tab, index: usize, disabled: bool) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "slider", NAMES[index])?;
    check_eq!(
        ax_property(node, "valuemin").and_then(Value::as_f64),
        Some(0.),
        "AX minimum"
    );
    check_eq!(
        ax_property(node, "valuemax").and_then(Value::as_f64),
        Some(100.),
        "AX maximum"
    );
    check_eq!(
        ax_property(node, "disabled") == Some(&json!(true)),
        disabled,
        "AX disabled state"
    );
    let element = projected(IDS[index]);
    let value = tab
        .eval(&format!(
            "[{element}.getAttribute('aria-valuenow'),{element}.getAttribute('aria-valuetext')]"
        ))
        .await?;
    check!(
        value[0]
            .as_str()
            .and_then(|text| text.parse::<f64>().ok())
            .is_some(),
        "numeric value remains in the projected range"
    );
    check!(
        value[1].as_str().is_some_and(|text| text.ends_with('％')),
        "localized value text is exposed"
    );
    Ok(value)
}

async fn wide(env: &Env, tab: &Tab) -> Result<Value> {
    let model = ready(tab, "d.visible && !d.rtl && d.text_scale === 1").await?;
    let pane = bounds(tab, SCROLL).await?;
    let mut presets = Vec::new();
    for (index, id) in IDS.into_iter().take(5).enumerate() {
        let g = geometry(tab, id).await?;
        check!(
            contains(pane, g.hit),
            format!("{id}: reachable in the wide Gallery")
        );
        check!(
            near(g.hit[3], HANDLE[index].max(48.)),
            "target height keeps the minimum and full handle"
        );
        check!(
            near(g.rail[0] - g.hit[0], 12.) && near(g.hit[2] - g.rail[2], 24.),
            "rendered and input rails share the same padding"
        );
        check!(
            near(g.active[3], TRACK[index]) && near(g.inactive[3], TRACK[index]),
            "both track halves use the same size token"
        );
        check!(
            near(g.handle[2], 4.) && near(g.handle[3], HANDLE[index]),
            "ready handle dimensions"
        );
        let fraction = (center(g.handle)[0] - g.rail[0]) / g.rail[2];
        check!(
            (fraction - model.values[index] / 100.).abs() < 0.005,
            "handle position follows controlled value"
        );
        check!(
            near(g.handle[0] - (g.active[0] + g.active[2]), 6.)
                && near(g.inactive[0] - (g.handle[0] + g.handle[2]), 6.),
            "logical inner edges leave the measured handle gap"
        );
        presets.push(g);
    }
    let ax = ax_range(tab, 0, false).await?;
    check_eq!(
        ax,
        json!(["25", "25％"]),
        "AX numeric and localized continuous value"
    );
    let disabled_ax = ax_range(tab, 5, true).await?;
    let image = photo(tab, env, WIDE_PNG).await?;
    let first = &presets[0];
    let colors = [
        pixel(&image, first.active)?,
        pixel(&image, first.inactive)?,
        pixel(&image, first.handle)?,
    ];
    check_eq!(
        colors,
        [
            rgb(model.palette.primary),
            rgb(model.palette.secondary_container),
            rgb(model.palette.primary)
        ],
        "actual active, inactive and handle token paint"
    );
    let stop = bounds(tab, &part(IDS[0], "stop")).await?;
    check!(
        near(stop[2], 4.) && near(stop[3], 4.),
        "continuous slider has a four dp end stop"
    );
    check_eq!(
        pixel(&image, stop)?,
        rgb(model.palette.on_secondary_container),
        "end stop color is visible in the inactive segment"
    );
    let tick = bounds(tab, &part(IDS[1], "tick-1")).await?;
    check!(
        near(tick[2], 4.) && ink(&image, tick, rgb(model.palette.on_primary))? > 0,
        "discrete active tick has visible paint"
    );
    Ok(
        json!({"viewport":WIDE,"geometry":presets,"colors":colors,"stop":stop,"tick":tick,
              "ax":ax,"disabled_ax":disabled_ax,"palette":model.palette,"capture":WIDE_PNG}),
    )
}

async fn tab_into_first(tab: &Tab) -> Result<()> {
    focused(tab, BACK).await?;
    for target in [THEME, RTL, LARGE_TEXT, IDS[0]] {
        tab.press("Tab").await?;
        focused(tab, target).await?;
    }
    Ok(())
}

async fn keyboard(tab: &Tab, env: &Env) -> Result<Value> {
    tab_into_first(tab).await?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.values[0] === 30 && d.requests[0] === 1").await?;
    tab.press("End").await?;
    ready(tab, "d.values[0] === 100 && d.requests[0] === 2").await?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.values[0] === 100 && d.requests[0] === 2").await?;
    tab.press("Home").await?;
    ready(tab, "d.values[0] === 0 && d.requests[0] === 3").await?;
    tab.press("PageUp").await?;
    let model = ready(
        tab,
        "d.values[0] === 10 && d.requests[0] === 4 && d.engaged[0]",
    )
    .await?;
    let g = geometry(tab, IDS[0]).await?;
    check!(
        model.engaged[0] && near(g.handle[2], 2.) && near(g.handle[3], 44.),
        "focus narrows handle paint without moving the rail"
    );
    let ring = bounds(tab, &part(IDS[0], "focus")).await?;
    let label = bounds(tab, &part(IDS[0], "label")).await?;
    check!(
        label[1] + label[3] <= g.handle[1] + 0.75,
        "localized value fits above the handle"
    );
    let aria = ax_range(tab, 0, false).await?;
    check_eq!(
        aria,
        json!(["10", "10％"]),
        "AX value text tracks key updates"
    );
    let image = photo(tab, env, KEY_PNG).await?;
    let focus_ink = ink(&image, ring, rgb(model.palette.secondary))?;
    let label_ink = ink(&image, label, rgb(model.palette.inverse_on_surface))?;
    check!(
        focus_ink > 15
            && label_ink > 15
            && ink(&image, label, rgb(model.palette.inverse_surface))? > 15,
        "actual focus outline and localized value paint"
    );
    tab.press("Tab").await?;
    focused(tab, IDS[1]).await?;
    tab.press("ArrowRight").await?;
    let discrete = ready(tab, "d.values[1] === 62.5 && d.requests[1] === 1").await?;
    Ok(
        json!({"geometry":g,"value_label":label,"ring":ring,"focus_ink":focus_ink,"label_ink":label_ink,
              "continuous_requests":model.requests[0],"discrete":discrete.values[1],"ax":aria,
              "palette":model.palette,"capture":KEY_PNG}),
    )
}

async fn point(tab: &Tab, rail: Rect, fraction: f64) -> Result<[f64; 2]> {
    let origin = tab.canvas_box().await?;
    Ok([
        origin[0] + rail[0] + rail[2] * fraction,
        origin[1] + center(rail)[1],
    ])
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

async fn pointer(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    let g = geometry(tab, IDS[0]).await?;
    let [x, y] = point(tab, g.rail, 0.333).await?;
    tab.click(x, y).await?;
    let tap = ready(
        tab,
        "d.requests[0] === 1 && Math.abs(d.values[0] - 33.3) < 0.4",
    )
    .await?;
    check!(
        tap.values[0] % 5. > 0.1,
        "continuous pointer follows the exact rail position"
    );

    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    let [x, y] = point(tab, g.rail, 0.731).await?;
    tab.mouse_move(x, y).await?;
    let held = ready(
        tab,
        "d.requests[0] === 2 && Math.abs(d.values[0] - 73.1) < 0.4 && d.engaged[0]",
    )
    .await?;
    let contact = geometry(tab, IDS[0]).await?;
    check!(
        near(contact.handle[2], 2.) && near(contact.rail[2], g.rail[2]),
        "held paint does not move the input rail"
    );
    photo(tab, env, POINTER_PNG).await?;
    resize(tab, 900, WIDE[1]).await?;
    tab.mouse_move(x - 80., y).await?;
    tab.mouse_up().await?;
    let canceled = ready(tab, "d.requests[0] === 2").await?;
    check_eq!(
        canceled.values[0],
        held.values[0],
        "resize retires the stale mouse route"
    );

    let small = geometry(tab, IDS[1]).await?;
    let first = point(tab, small.rail, 0.13).await?;
    let moved = point(tab, small.rail, 0.81).await?;
    touch(tab, "touchStart", Some(first)).await?;
    ready(tab, "d.values[1] === 12.5 && d.requests[1] === 1").await?;
    touch(tab, "touchMove", Some(moved)).await?;
    ready(tab, "d.values[1] === 75 && d.requests[1] === 2").await?;
    touch(tab, "touchCancel", None).await?;
    let touch_end = ready(tab, "d.values[1] === 75 && d.requests[1] === 2").await?;
    let disabled = bounds(tab, IDS[5]).await?;
    let [x, y] = point(tab, disabled, 0.8).await?;
    tab.click(x, y).await?;
    let disabled = ready(tab, "d.values[5] === 50 && d.requests[5] === 0").await?;
    Ok(
        json!({"tap":tap.values[0],"held":held.values[0],"canceled":canceled.values[0],"handle":contact.handle,
              "touch":touch_end.values[1],"requests":disabled.requests,"capture":POINTER_PNG}),
    )
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, COMPACT[0], COMPACT[1]).await?;
    for setting in [THEME, RTL, LARGE_TEXT] {
        activate(tab, setting).await?;
    }
    focus_projected(tab, IDS[0]).await?;
    tab.press("ArrowLeft").await?;
    let model = ready(
        tab,
        "d.rtl && d.text_scale === 2 && d.values[0] === 30 && d.engaged[0]",
    )
    .await?;
    let g = geometry(tab, IDS[0]).await?;
    let label = bounds(tab, &part(IDS[0], "label")).await?;
    let viewport = [0., 0., f64::from(COMPACT[0]), f64::from(COMPACT[1])];
    check!(
        contains(viewport, g.hit) && contains(viewport, label),
        "compact 200% value label and slider fit without clipping"
    );
    check!(
        center(g.active)[0] > center(g.handle)[0] && center(g.inactive)[0] < center(g.handle)[0],
        "RTL mirrors both track portions"
    );
    let image = photo(tab, env, COMPACT_PNG).await?;
    let ink_count = ink(&image, label, rgb(model.palette.inverse_on_surface))?;
    check!(ink_count > 40, "200% value label contains readable ink");
    let compact_ax = ax_range(tab, 0, false).await?;
    for target in IDS.into_iter().skip(1).take(4) {
        tab.press("Tab").await?;
        focused(tab, target).await?;
    }
    let xl = bounds(tab, IDS[4]).await?;
    check!(
        contains(bounds(tab, SCROLL).await?, xl),
        "keyboard can reveal the extra large compact slider"
    );
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"viewport":COMPACT,"rtl":model.rtl,"text_scale":model.text_scale,"geometry":g,
              "label":label,"label_ink":ink_count,"ax":compact_ax,"back_focus":SHOW,
              "palette":model.palette,"capture":COMPACT_PNG}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(WIDE[0], WIDE[1] + 44, 2.).await?;
    let mut report = json!({"browser":chrome.version,"adapter":null,"wide":null,"keyboard":null,
                           "pointer":null,"compact":null});
    let result: Result<()> = async {
        step("M298", "Standard slider size tokens, track/stop paint and Chrome range semantics");
        route(&tab, url).await?;
        report["adapter"] = tab.eval("(async()=>{ const a=await navigator.gpu.requestAdapter(); const {vendor,architecture,isFallbackAdapter}=a.info; return {vendor,architecture,isFallbackAdapter}; })()").await?;
        report["wide"] = wide(env, &tab).await.context("M298 slider presets")?;
        step("M299", "Standard slider keys, exact endpoints, discrete stops, focus and accessible value text");
        report["keyboard"] = keyboard(&tab, env).await.context("M299 slider keyboard")?;
        step("M300", "Standard slider target geometry, continuous mouse drag, discrete touch and cancellation");
        report["pointer"] = pointer(&tab, env, url).await.context("M300 slider pointer")?;
        step("M301", "Standard slider dark 320 dp RTL, 200 percent value text and scroll reveal");
        report["compact"] = compact(&tab, env, url).await.context("M301 slider compact")?;
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-standard-slider-failure.png"))
            .await;
    }
    env.write_json("material-standard-slider.json", &report)?;
    result?;
    check!(
        tab.errors().is_empty(),
        "Standard slider Chrome page has no errors"
    );
    Ok(())
}
