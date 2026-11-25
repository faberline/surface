//! Test Material slider inset icons in the Gallery's actual WebGPU output.

use anyhow::{Context, Result};
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

pub(super) const STEPS: [&str; 3] = ["M313", "M314", "M315"];

const SHOW: &str = "slider-inset-demo-show";
const BACK: &str = "slider-inset-demo-back";
const SCROLL: &str = "slider-inset-demo-scroll";
const SETTINGS: [&str; 3] = [
    "slider-inset-demo-theme",
    "slider-inset-demo-rtl",
    "slider-inset-demo-large-text",
];
const SAMPLES: [(&str, &str, f64); 8] = [
    ("slider-inset-demo-horizontal-medium", "中型水平音量", 24.),
    ("slider-inset-demo-horizontal-large", "大型水平音量", 24.),
    ("slider-inset-demo-horizontal-xl", "特大水平音量", 32.),
    ("slider-inset-demo-short", "短軌道音量", 0.),
    ("slider-inset-demo-vertical-medium", "中型垂直音量", 24.),
    ("slider-inset-demo-vertical-large", "大型垂直音量", 24.),
    ("slider-inset-demo-vertical-xl", "特大垂直音量", 32.),
    ("slider-inset-demo-disabled", "停用垂直音量", 24.),
];
const INITIAL: [f64; 8] = [65., 0., 65., 50., 65., 0., 65., 65.];
const ACTIVE: &str = "inset-icon-active";
const INACTIVE: &str = "inset-icon-inactive";
const LIGHT: &str = "material-slider-inset-light.png";
const MUTED: &str = "material-slider-inset-muted.png";
const QUIET: &str = "material-slider-inset-quiet.png";
const COMPACT: &str = "material-slider-inset-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn ready(tab: &Tab, condition: &str) -> Result<Value> {
    until(tab, &format!("(d => {condition})(s.slider_inset_demo)")).await?;
    tab.settle().await?;
    Ok(state(tab).await?["slider_inset_demo"].clone())
}

async fn part(tab: &Tab, id: &str, suffix: &str) -> Result<Rect> {
    bounds(tab, &format!("{id}-{suffix}")).await
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    let path = env.artifact(name);
    tab.settle().await?;
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn color(palette: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] =
        serde_json::from_value(palette[name].clone()).with_context(|| format!("palette {name}"))?;
    Ok([r, g, b])
}

fn ink(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

fn area([_, _, w, h]: Rect) -> f64 {
    w * h
}

async fn icon(tab: &Tab, id: &str, positive: bool, side: f64) -> Result<Rect> {
    let active = part(tab, id, ACTIVE).await?;
    let inactive = part(tab, id, INACTIVE).await?;
    let (drawn, hidden, track) = if positive {
        (active, inactive, "active")
    } else {
        (inactive, active, "inactive")
    };
    check!(area(hidden) < 1., "{id} has only one inset icon");
    check!(
        near(drawn[2], side)
            && near(drawn[3], side)
            && contains(part(tab, id, track).await?, drawn),
        "{id} has a {side} dp glyph entirely inside the {track} track"
    );
    Ok(drawn)
}

async fn wide(tab: &Tab, env: &Env) -> Result<Value> {
    let model = ready(tab, "d.visible && !d.dark && !d.rtl && d.text_scale === 1").await?;
    let palette = &model["palette"];
    let image = photo(tab, env, LIGHT).await?;
    let tree = ax_tree(tab).await?;
    let mut observations = Vec::new();
    for (index, (id, name, side)) in SAMPLES.into_iter().enumerate() {
        let value = INITIAL[index];
        let vertical = index >= 4;
        let disabled = index == 7;
        let node = ax(&tree, "slider", name)?;
        check_eq!(node["value"]["value"].as_f64(), Some(value), "{id} value");
        check_eq!(
            ax_property(node, "orientation").and_then(Value::as_str),
            Some(if vertical { "vertical" } else { "horizontal" }),
            "{id} orientation"
        );
        check_eq!(
            ax_property(node, "disabled") == Some(&json!(true)),
            disabled,
            "{id} disabled state"
        );
        for suffix in [
            ACTIVE,
            INACTIVE,
            "inset-icon-active-glyph",
            "inset-icon-inactive-glyph",
        ] {
            let glyph = projected(&format!("{id}-{suffix}"));
            check_eq!(
                tab.eval(&format!("{glyph} === null")).await?,
                json!(true),
                "decorative node {suffix} has no accessibility peer"
            );
        }
        let stop = part(tab, id, "stop").await?;
        check!(
            near(stop[2], 4.) && near(stop[3], 4.),
            "{id} keeps the end stop"
        );
        if side == 0. {
            let target = bounds(tab, id).await?;
            check!(
                near(target[2], 88.)
                    && area(part(tab, id, ACTIVE).await?) < 1.
                    && area(part(tab, id, INACTIVE).await?) < 1.
                    && contains(part(tab, id, "inactive").await?, stop),
                "the short rail keeps its stop and omits a glyph that does not fit"
            );
            observations.push(json!({"id":id,"stop":stop,"glyph":null}));
            continue;
        }
        let rect = icon(tab, id, value > 0., side).await?;
        let painted = if disabled {
            None
        } else {
            let lane_color = if value == 0. {
                "on_secondary_container"
            } else {
                "on_primary"
            };
            let painted = ink(&image, rect, color(palette, lane_color)?)?;
            check!(painted > 30, "the {id} icon has actual {lane_color} pixels");
            Some(painted)
        };
        observations.push(json!({"id":id,"stop":stop,"glyph":rect,"ink":painted}));
    }
    Ok(json!({"capture":LIGHT,"palette":palette,"samples":observations}))
}

async fn keys(tab: &Tab, env: &Env) -> Result<Value> {
    let (id, name, side) = SAMPLES[0];
    focus_projected(tab, id).await?;
    let target = bounds(tab, id).await?;
    let rail = part(tab, id, "rail").await?;
    let stop = part(tab, id, "stop").await?;
    tab.press("Home").await?;
    let model = ready(
        tab,
        "d.samples[0].value === 0 && d.samples[0].requests === 1",
    )
    .await?;
    let muted = icon(tab, id, false, side).await?;
    let palette = &model["palette"];
    let foreground = color(palette, "on_secondary_container")?;
    let muted_ink = ink(&photo(tab, env, MUTED).await?, muted, foreground)?;
    tab.press("ArrowRight").await?;
    ready(
        tab,
        "d.samples[0].value === 5 && !d.samples[0].minimum_glyph && d.samples[0].requests === 2",
    )
    .await?;
    let quiet = icon(tab, id, false, side).await?;
    let quiet_ink = ink(&photo(tab, env, QUIET).await?, quiet, foreground)?;
    check_eq!(
        muted,
        quiet,
        "the regular and minimum glyph use the same slot"
    );
    check_eq!(
        part(tab, id, "stop").await?,
        stop,
        "the end stop is unchanged"
    );
    check!(
        muted_ink > 30 && quiet_ink > 30 && muted_ink.abs_diff(quiet_ink) >= 8,
        "the two glyph contours differ in the actual PNG: {muted_ink} versus {quiet_ink}"
    );
    tab.press("End").await?;
    let model = ready(
        tab,
        "d.samples[0].value === 100 && d.samples[0].requests === 3",
    )
    .await?;
    let full = icon(tab, id, true, side).await?;
    check!(
        full[0] < quiet[0],
        "keyboard changes which track contains the icon"
    );
    check_eq!(
        bounds(tab, id).await?,
        target,
        "the hit region is unchanged"
    );
    check_eq!(part(tab, id, "rail").await?, rail, "the rail is unchanged");
    focused(tab, id).await?;
    let tree = ax_tree(tab).await?;
    check_eq!(
        ax(&tree, "slider", name)?["value"]["value"].as_f64(),
        Some(100.),
        "Chrome publishes the new value"
    );
    Ok(
        json!({"values":model["samples"][0],"muted":{"capture":MUTED,"glyph":muted,"ink":muted_ink},"quiet":{"capture":QUIET,"glyph":quiet,"ink":quiet_ink},"full":full}),
    )
}

async fn compact(tab: &Tab, env: &Env) -> Result<Value> {
    resize(tab, 320, 880).await?;
    for control in SETTINGS {
        activate(tab, control).await?;
    }
    let model = ready(tab, "d.visible && d.dark && d.rtl && d.text_scale === 2").await?;
    let palette = &model["palette"];
    let (horizontal, _, side) = SAMPLES[0];
    focus_projected(tab, horizontal).await?;
    let h_icon = icon(tab, horizontal, true, side).await?;
    check!(
        h_icon[0] > part(tab, horizontal, "handle").await?[0],
        "RTL mirrors the active horizontal track"
    );
    for (id, _, _) in &SAMPLES[1..=4] {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    let (vertical, _, side) = SAMPLES[4];
    let v_icon = icon(tab, vertical, true, side).await?;
    let label = part(tab, vertical, "label").await?;
    let ring = part(tab, vertical, "focus").await?;
    let scroll = bounds(tab, SCROLL).await?;
    check!(
        v_icon[1] > part(tab, vertical, "handle").await?[1]
            && contains(scroll, bounds(tab, vertical).await?)
            && contains(scroll, v_icon)
            && contains(scroll, ring)
            && contains(scroll, label)
            && label[3] >= 63.
            && label[0] + label[2] < ring[0],
        "vertical minimum stays at the bottom and Tab reveals the full compact glyph, ring and readout"
    );
    let image = photo(tab, env, COMPACT).await?;
    let glyph_ink = ink(&image, v_icon, color(palette, "on_primary")?)?;
    let label_ink = ink(&image, label, color(palette, "inverse_on_surface")?)?;
    let ring_ink = ink(&image, ring, color(palette, "secondary")?)?;
    check!(
        glyph_ink > 30 && label_ink > 30 && ring_ink > 30,
        "the glyph, readout ink and ring all render in the compact PNG"
    );
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"capture":COMPACT,"palette":palette,"horizontal_icon":h_icon,"vertical_icon":v_icon,"label":label,"ring":ring,"glyph_ink":glyph_ink,"label_ink":label_ink,"ring_ink":ring_ink,"back_focus":SHOW}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1944, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(STEPS[0], "Standard slider inset glyphs paint at both axes and all three sizes with one accessibility peer");
        tab.goto(url).await?;
        crate::smoke::ready(&tab, "gallery").await?;
        resize(&tab, 960, 1900).await?;
        reduced_motion(&tab, true).await?;
        activate(&tab, SHOW).await?;
        resize(&tab, 960, 1040).await?;
        report["wide"] = wide(&tab, env).await?;
        step(STEPS[1], "the inset glyph changes contour at the minimum and moves tracks without moving the rail");
        report["keys"] = keys(&tab, env).await?;
        step(STEPS[2], "dark 320 dp RTL keeps the upright vertical glyph, focus and large label visible");
        report["compact"] = compact(&tab, env).await?;
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-slider-inset-failure.png"))
            .await;
    }
    env.write_json("material-slider-inset.json", &report)?;
    result?;
    check!(tab.errors().is_empty(), "no inset slider page errors");
    Ok(())
}
