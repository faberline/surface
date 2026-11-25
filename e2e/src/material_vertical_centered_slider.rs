//! Exercise the vertical Centered Slider through the rendered Gallery.

use anyhow::{Context, Result};
use serde::Serialize;
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

pub(super) const STEPS: [&str; 3] = ["M310", "M311", "M312"];

const SHOW: &str = "vertical-centered-slider-demo-show";
const BACK: &str = "vertical-centered-slider-demo-back";
const SCROLL: &str = "vertical-centered-slider-demo-scroll";
const SETTINGS: [&str; 3] = [
    "vertical-centered-slider-demo-theme",
    "vertical-centered-slider-demo-rtl",
    "vertical-centered-slider-demo-large-text",
];
const IDS: [&str; 5] = [
    "vertical-centered-slider-demo-negative",
    "vertical-centered-slider-demo-neutral",
    "vertical-centered-slider-demo-positive",
    "vertical-centered-slider-demo-discrete",
    "vertical-centered-slider-demo-disabled",
];
const NAMES: [&str; 5] = ["負值平衡", "中立平衡", "正值平衡", "階梯平衡", "停用平衡"];
const WIDE: [u32; 2] = [960, 1160];
const COMPACT: [u32; 2] = [320, 880];
const WIDE_PNG: &str = "material-vertical-centered-light.png";
const KEYS_PNG: &str = "material-vertical-centered-keyboard.png";
const COMPACT_PNG: &str = "material-vertical-centered-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Serialize)]
struct Shape {
    target: Rect,
    rail: Rect,
    handle: Rect,
    active: Rect,
    inactive: [Rect; 2],
    stops: [Rect; 2],
}

async fn part(tab: &Tab, id: &str, suffix: &str) -> Result<Rect> {
    bounds(tab, &format!("{id}-{suffix}")).await
}

async fn shape(tab: &Tab, id: &str) -> Result<Shape> {
    Ok(Shape {
        target: bounds(tab, id).await?,
        rail: part(tab, id, "rail").await?,
        handle: part(tab, id, "handle").await?,
        active: part(tab, id, "active").await?,
        inactive: [
            part(tab, id, "inactive-min").await?,
            part(tab, id, "inactive-max").await?,
        ],
        stops: [
            part(tab, id, "stop-min").await?,
            part(tab, id, "stop-max").await?,
        ],
    })
}

async fn ready(tab: &Tab, condition: &str) -> Result<Value> {
    until(
        tab,
        &format!("(d => {condition})(s.vertical_centered_slider_demo)"),
    )
    .await?;
    tab.settle().await?;
    Ok(state(tab).await?["vertical_centered_slider_demo"].clone())
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1900).await?;
    reduced_motion(tab, true).await?;
    until(
        tab,
        "s.section === 0 && !s.vertical_centered_slider_demo.visible",
    )
    .await?;
    activate(tab, SHOW).await?;
    ready(
        tab,
        "d.visible && d.neutral === 0 && d.samples.map(s => s.value).join(',') === '-50,0,50,-25,50'",
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

fn bottom([_, y, _, h]: Rect) -> f64 {
    y + h
}

fn middle_y([_, y, _, h]: Rect) -> f64 {
    y + h / 2.
}

fn color(palette: &Value, name: &str) -> Result<Rgb> {
    let [red, green, blue, _]: [u8; 4] =
        serde_json::from_value(palette[name].clone()).with_context(|| format!("palette {name}"))?;
    Ok([red, green, blue])
}

fn sample(image: &Raster, [x, y, w, h]: Rect) -> Result<Rgb> {
    image.sample(x + w / 2., y + h / 2.)
}

fn ink(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

fn measured(g: &Shape, value: f64) -> Result<()> {
    check!(
        g.target[2] >= 48.
            && near(g.target[3], 240.)
            && near(g.rail[1] - g.target[1], 12.)
            && near(g.target[3] - g.rail[3], 24.),
        "the 240 dp target and rail have the same axis"
    );
    let seam = middle_y(g.rail);
    let expected = g.rail[1] + g.rail[3] * (100. - value) / 200.;
    check!(
        near(middle_y(g.handle), expected),
        "signed values increase from the physical bottom"
    );
    let [low, high] = g.inactive;
    if value == 0. {
        check!(
            g.active[3] <= 0.75 && near(low[1] - bottom(high), g.handle[3] + 12.),
            "only the handle and its gap interrupt the two inactive sections at neutral"
        );
    } else if value < 0. {
        check!(
            g.active[3] > 0.
                && near(g.active[1], seam + 2.)
                && near(bottom(high), seam - 2.)
                && near(g.handle[1] - bottom(g.active), 6.)
                && near(low[1] - bottom(g.handle), 6.),
            "negative active paint runs from just below neutral down to the handle"
        );
    } else {
        check!(
            g.active[3] > 0.
                && near(bottom(g.active), seam - 2.)
                && near(low[1], seam + 2.)
                && near(g.active[1] - bottom(g.handle), 6.)
                && near(g.handle[1] - bottom(high), 6.),
            "positive active paint runs from just above neutral up to the handle"
        );
    }
    Ok(())
}

async fn accessibility(tab: &Tab, index: usize, expected: f64, disabled: bool) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "slider", NAMES[index])?;
    check_eq!(
        ax_property(node, "orientation").and_then(Value::as_str),
        Some("vertical"),
        "Chrome publishes the vertical orientation"
    );
    check_eq!(
        [
            ax_property(node, "valuemin").and_then(Value::as_f64),
            ax_property(node, "valuemax").and_then(Value::as_f64),
            node["value"]["value"].as_f64(),
        ],
        [Some(-100.), Some(100.), Some(expected)],
        "signed accessible range and current value"
    );
    check_eq!(
        ax_property(node, "disabled") == Some(&json!(true)),
        disabled,
        "Chrome publishes the disabled state"
    );
    let element = projected(IDS[index]);
    let values = tab.eval(&format!(
        "['aria-valuenow','aria-valuetext','aria-orientation'].map(name => {element}.getAttribute(name))"
    )).await?;
    let expected_text = expected.to_string();
    check_eq!(
        values[0].as_str(),
        Some(expected_text.as_str()),
        "ARIA numeric value"
    );
    check_eq!(
        values[2].as_str(),
        Some("vertical"),
        "DOM vertical orientation"
    );
    check!(
        values[1].as_str().is_some_and(|value| !value.is_empty()),
        "localized value text"
    );
    Ok(values)
}

async fn wide(tab: &Tab, env: &Env) -> Result<Value> {
    let model = ready(tab, "d.visible && !d.dark && !d.rtl && d.text_scale === 1").await?;
    let palette = &model["palette"];
    let image = photo(tab, env, WIDE_PNG).await?;
    let mut samples = Vec::new();
    for (index, value) in [-50., 0., 50., -25., 50.].into_iter().enumerate() {
        let g = shape(tab, IDS[index]).await?;
        measured(&g, value)?;
        let colors = [
            sample(&image, g.handle)?,
            sample(&image, g.inactive[0])?,
            sample(&image, g.inactive[1])?,
        ];
        if index == 4 {
            let toned = colors[0] != color(palette, "primary")?
                && colors[0] != color(palette, "background")?;
            check!(toned, "disabled handle remains visible in muted ink");
        } else {
            check_eq!(
                colors[0],
                color(palette, "primary")?,
                "handle primary paint"
            );
            check!(
                g.inactive.iter().all(|&track| near(track[2], 24.)),
                "both inactive sections preserve Small track thickness"
            );
            for (track, stop) in g.inactive.into_iter().zip(g.stops) {
                check!(
                    contains(track, stop) && near(stop[2], 4.) && near(stop[3], 4.),
                    "both physical ends have a four dp stop"
                );
                check_eq!(
                    sample(&image, stop)?,
                    color(palette, "on_secondary_container")?,
                    "actual stop paint"
                );
                check!(
                    ink(&image, track, color(palette, "secondary_container")?)? > 80,
                    "actual inactive paint"
                );
            }
            if value != 0. {
                check!(
                    ink(&image, g.active, color(palette, "primary")?)? > 50,
                    "actual signed active paint"
                );
            }
        }
        let values = accessibility(tab, index, value, index == 4).await?;
        samples.push(json!({"geometry":g,"value":value,"colors":colors,"aria":values}));
    }
    Ok(json!({"viewport":WIDE,"palette":palette,"samples":samples,"capture":WIDE_PNG}))
}

async fn paint(
    tab: &Tab,
    env: &Env,
    id: &str,
    capture: &str,
    palette: &Value,
    rtl: bool,
) -> Result<Value> {
    let label = part(tab, id, "label").await?;
    let ring = part(tab, id, "focus").await?;
    check!(
        if rtl {
            label[0] + label[2] < ring[0]
        } else {
            label[0] > ring[0] + ring[2]
        },
        "the upright label stands beside the trailing side of the focus ring"
    );
    let image = photo(tab, env, capture).await?;
    let text = ink(&image, label, color(palette, "inverse_on_surface")?)?;
    let fill = ink(&image, label, color(palette, "inverse_surface")?)?;
    let focus = ink(&image, ring, color(palette, "secondary")?)?;
    check!(
        text > 20 && fill > 100 && focus > 20,
        "actual indicator and focus paint"
    );
    Ok(json!({"label":label,"ring":ring,"text_ink":text,"label_fill":fill,"focus_ink":focus}))
}

async fn keys(tab: &Tab, env: &Env) -> Result<Value> {
    focus_projected(tab, IDS[0]).await?;
    tab.press("Home").await?;
    ready(
        tab,
        "d.samples[0].value === -100 && d.samples[0].requests === 1",
    )
    .await?;
    tab.press("End").await?;
    ready(
        tab,
        "d.samples[0].value === 100 && d.samples[0].requests === 2",
    )
    .await?;
    tab.press("ArrowRight").await?;
    ready(tab, "d.samples[0].requests === 2").await?;
    focus_projected(tab, IDS[3]).await?;
    tab.press("ArrowUp").await?;
    ready(
        tab,
        "d.samples[3].value === 0 && d.samples[3].requests === 1",
    )
    .await?;
    measured(&shape(tab, IDS[3]).await?, 0.)?;
    tab.press("ArrowRight").await?;
    let model = ready(
        tab,
        "d.samples[3].value === 25 && d.samples[3].requests === 2 && d.samples[3].engaged",
    )
    .await?;
    let g = shape(tab, IDS[3]).await?;
    measured(&g, 25.)?;
    check!(
        near(g.handle[3], 2.),
        "focused handle narrows on its vertical axis"
    );
    let paint = paint(tab, env, IDS[3], KEYS_PNG, &model["palette"], false).await?;
    let aria = accessibility(tab, 3, 25., false).await?;
    Ok(
        json!({"viewport":WIDE,"palette":model["palette"],"geometry":g,"paint":paint,
        "values":model["samples"],"aria":aria,"capture":KEYS_PNG}),
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

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, COMPACT[0], COMPACT[1]).await?;
    for setting in SETTINGS {
        activate(tab, setting).await?;
    }
    for id in IDS.into_iter().take(4) {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    tab.press("ArrowRight").await?;
    ready(tab, "d.dark && d.rtl && d.text_scale === 2 && d.samples[3].value === 0 && d.samples[3].requests === 1")
        .await?;
    let g = shape(tab, IDS[3]).await?;
    measured(&g, 0.)?;
    let canvas = tab.canvas_box().await?;
    let point = |value| {
        [
            canvas[0] + g.rail[0] + g.rail[2] / 2.,
            canvas[1] + g.rail[1] + g.rail[3] * (100. - value) / 200.,
        ]
    };
    touch(tab, "touchStart", Some(point(-75.))).await?;
    ready(
        tab,
        "d.samples[3].value === -75 && d.samples[3].requests === 2",
    )
    .await?;
    touch(tab, "touchMove", Some(point(50.))).await?;
    ready(
        tab,
        "d.samples[3].value === 50 && d.samples[3].requests === 3",
    )
    .await?;
    touch(tab, "touchCancel", None).await?;
    ready(
        tab,
        "d.samples[3].value === 50 && d.samples[3].requests === 3",
    )
    .await?;
    touch(tab, "touchStart", Some(point(25.))).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "d.samples[3].value === 25 && d.samples[3].requests === 4",
    )
    .await?;
    focus_projected(tab, IDS[3]).await?;
    let model = ready(tab, "d.samples[3].engaged").await?;
    let g = shape(tab, IDS[3]).await?;
    measured(&g, 25.)?;
    let paint = paint(tab, env, IDS[3], COMPACT_PNG, &model["palette"], true).await?;
    let label: Rect = serde_json::from_value(paint["label"].clone())?;
    let ring: Rect = serde_json::from_value(paint["ring"].clone())?;
    let screen = [0., 0., f64::from(COMPACT[0]), f64::from(COMPACT[1])];
    check!(
        contains(bounds(tab, SCROLL).await?, g.target)
            && contains(screen, label)
            && contains(screen, ring)
            && label[3] >= 63.,
        "Tab reveals the entire vertical target and full 200 percent label"
    );
    let aria = accessibility(tab, 3, 25., false).await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"viewport":COMPACT,"rtl":model["rtl"],"dark":model["dark"],
        "text_scale":model["text_scale"],"palette":model["palette"],"geometry":g,"paint":paint,
        "values":model["samples"],"aria":aria,"back_focus":SHOW,"capture":COMPACT_PNG}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(WIDE[0], WIDE[1] + 44, 2.).await?;
    let mut report = json!({"browser":chrome.version,"adapter":null});
    let result: Result<()> = async {
        step(STEPS[0], "Vertical Centered paints both signs, neutral and outer stops with separate ARIA values");
        route(&tab, url).await?;
        report["adapter"] = tab.eval("(async()=>{ const a=await navigator.gpu.requestAdapter(); const {vendor,architecture,isFallbackAdapter}=a.info; return {vendor,architecture,isFallbackAdapter}; })()").await?;
        report["wide"] = wide(&tab, env).await.context("M310 vertical centered paint and AX")?;
        step(STEPS[1], "Vertical Centered reaches neutral and signed endpoints with a localized focused readout");
        report["keys"] = keys(&tab, env).await.context("M311 vertical centered keys")?;
        step(STEPS[2], "Vertical Centered keeps bottom-to-top touch and 200 percent labels in compact dark RTL");
        report["compact"] = compact(&tab, env, url).await.context("M312 compact vertical centered touch")?;
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-vertical-centered-failure.png"))
            .await;
    }
    env.write_json("material-vertical-centered-slider.json", &report)?;
    result?;
    check!(tab.errors().is_empty(), "no Vertical Centered page errors");
    Ok(())
}
