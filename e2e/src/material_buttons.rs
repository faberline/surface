//! Common Material Buttons using trusted Chrome input and WebGPU pixel evidence.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::material_list_corners::{distance, freeze, reduced_motion, tick};
use super::material_time_input::{
    activate, ax_property, ax_tree, bounds, contains, focus_projected, focused, near, projected,
    resize, Raster,
};
use super::{state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M316", "M317", "M318", "M319"];

const SHOW: &str = "button-demo-show";
const BACK: &str = "button-demo-back";
const SCROLL: &str = "button-demo-scroll";
const SETTINGS: [&str; 3] = [
    "button-demo-theme",
    "button-demo-rtl",
    "button-demo-large-text",
];
const SIZES: [(&str, f64, f64, f64); 5] = [
    ("xs", 32., 20., 4.),
    ("s", 40., 20., 8.),
    ("m", 56., 24., 8.),
    ("l", 96., 32., 12.),
    ("xl", 136., 40., 16.),
];
const COMMANDS: [(&str, &str, &str); 5] = [
    ("elevated", "surface_container_low", "primary"),
    ("filled", "primary", "on_primary"),
    ("tonal", "secondary_container", "on_secondary_container"),
    ("outlined", "background", "on_surface_variant"),
    ("text", "background", "primary"),
];
const TOGGLES: [(&str, &str, &str); 4] = [
    ("elevated", "primary", "on_primary"),
    ("filled", "primary", "on_primary"),
    ("tonal", "secondary", "on_secondary"),
    ("outlined", "inverse_surface", "inverse_on_surface"),
];
const LIGHT: &str = "material-buttons-light.png";
const CHECKED: &str = "material-buttons-checked.png";
const HELD: &str = "material-buttons-held.png";
const COMPACT: &str = "material-buttons-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Serialize)]
struct Geometry {
    hit: Rect,
    face: Rect,
    label: Rect,
    focus: Rect,
}

fn id(kind: &str, name: &str) -> String {
    format!("button-demo-{kind}-{name}")
}

async fn part(tab: &Tab, id: &str, suffix: &str) -> Result<Rect> {
    bounds(tab, &format!("{id}-{suffix}")).await
}

async fn geometry(tab: &Tab, id: &str) -> Result<Geometry> {
    let g = Geometry {
        hit: bounds(tab, id).await?,
        face: part(tab, id, "face").await?,
        label: part(tab, id, "label").await?,
        focus: part(tab, id, "focus").await?,
    };
    check!(
        g.hit[2] >= 48. && g.hit[3] >= 48. && contains(g.hit, g.face) && contains(g.face, g.label),
        "{id} uses a full hit target around the face and label"
    );
    for axis in 0..2 {
        check!(
            near(g.face[axis] - g.focus[axis], 5.)
                && near(g.focus[axis + 2] - g.face[axis + 2], 10.),
            "{id} has a separate outline with 5 dp outset"
        );
    }
    Ok(g)
}

async fn ready(tab: &Tab, condition: &str) -> Result<Value> {
    until(tab, &format!("(d => {condition})(s.button_demo)")).await?;
    tab.settle().await?;
    Ok(state(tab).await?["button_demo"].clone())
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2200).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.button_demo.visible").await?;
    activate(tab, SHOW).await?;
    ready(tab, "d.visible && !d.dark && !d.rtl && d.text_scale === 1").await?;
    resize(tab, 960, 1160).await
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    let path = env.artifact(name);
    tab.settle().await?;
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn color(model: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(model["palette"][name].clone())
        .with_context(|| format!("button palette {name}"))?;
    Ok([r, g, b])
}

fn ink(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

fn centre([x, y, w, h]: Rect) -> [f64; 2] {
    [x + w / 2., y + h / 2.]
}

fn fill(image: &Raster, face: Rect) -> Result<Rgb> {
    image.sample(centre(face)[0], face[1] + 3.5)
}

fn ax_button(
    tree: &[Value],
    model: &Value,
    id: &str,
    checked: Option<bool>,
    disabled: bool,
) -> Result<()> {
    let name = model["names"][id]
        .as_str()
        .with_context(|| format!("{id} name"))?;
    let matches: Vec<_> = tree
        .iter()
        .filter(|node| {
            node["ignored"] == false
                && node["name"]["value"] == name
                && matches!(
                    node["role"]["value"].as_str(),
                    Some("button" | "toggleButton")
                )
        })
        .collect();
    check_eq!(matches.len(), 1, "{id} is one named AX action");
    let node = matches[0];
    check_eq!(
        ax_property(node, "disabled") == Some(&json!(true)),
        disabled,
        "{id} Chrome AX disabled"
    );
    check_eq!(
        ax_property(node, "pressed").and_then(Value::as_str),
        checked.map(|on| if on { "true" } else { "false" }),
        "{id} Chrome AX pressed state"
    );
    Ok(())
}

async fn wide(tab: &Tab, env: &Env) -> Result<Value> {
    let model = ready(tab, "d.visible").await?;
    let tree = ax_tree(tab).await?;
    let scroller = bounds(tab, SCROLL).await?;
    let image = photo(tab, env, LIGHT).await?;
    let mut sizes = Vec::new();
    let mut commands = Vec::new();
    for (suffix, height, icon, gap) in SIZES {
        let id = id("size", suffix);
        let g = geometry(tab, &id).await?;
        let glyph = part(tab, &id, "icon").await?;
        check!(
            near(g.face[3], height) && near(glyph[2], icon) && near(glyph[3], icon),
            "{id} uses the face and artwork size tokens"
        );
        check!(
            contains(scroller, g.focus) && near(g.label[0] - glyph[0] - icon, gap),
            "{id} exposes the exact leading artwork gap"
        );
        ax_button(&tree, &model, &id, None, false)?;
        sizes.push(json!({"id":id,"geometry":g,"glyph":glyph,"gap":gap}));
    }
    for (kind, background, foreground) in COMMANDS {
        let id = id("command", kind);
        let g = geometry(tab, &id).await?;
        ax_button(&tree, &model, &id, None, false)?;
        let box_color = fill(&image, g.face)?;
        check_eq!(box_color, color(&model, background)?, "{kind} painted face");
        let label_pixels = ink(&image, g.label, color(&model, foreground)?)?;
        check!(label_pixels > 25, "{kind} painted label");
        if kind == "outlined" {
            let outline = image.sample(g.face[0] + 0.5, centre(g.face)[1])?;
            check_eq!(
                outline,
                color(&model, "outline_variant")?,
                "Outlined stroke"
            );
        }
        let label_peer = projected(&format!("{id}-label"));
        check_eq!(
            tab.eval(&format!("{label_peer} === null")).await?,
            json!(true),
            "painted label is part of the button name"
        );
        commands.push(json!({"id":id,"geometry":g,"fill":box_color,"label_ink":label_pixels}));
    }
    ax_button(&tree, &model, "button-demo-disabled", None, true)?;
    ax_button(
        &tree,
        &model,
        "button-demo-disabled-toggle",
        Some(true),
        true,
    )?;
    Ok(json!({"capture":LIGHT,"palette":model["palette"],"sizes":sizes,"commands":commands}))
}

async fn keyboard(tab: &Tab, env: &Env) -> Result<Value> {
    let mut states = Vec::new();
    for (index, (kind, _, _)) in TOGGLES.into_iter().enumerate() {
        let id = id("toggle", kind);
        let query = format!(
            "d.checked[{}] === true && d.counts[{}] === 1",
            js(&id),
            js(&id)
        );
        let model = ready(tab, "d.visible").await?;
        ax_button(&ax_tree(tab).await?, &model, &id, Some(false), false)?;
        focus_projected(tab, &id).await?;
        if index == 0 {
            for repeat in [false, true, true] {
                tab.cdp(
                    "Input.dispatchKeyEvent",
                    json!({"type":"keyDown","key":" ","code":"Space",
                           "windowsVirtualKeyCode":32,"text":" ","autoRepeat":repeat}),
                )
                .await?;
                ready(tab, &query).await?;
            }
            tab.cdp(
                "Input.dispatchKeyEvent",
                json!({"type":"keyUp","key":" ","code":"Space","windowsVirtualKeyCode":32}),
            )
            .await?;
        } else {
            tab.press("Enter").await?;
        }
        let model = ready(tab, &query).await?;
        focused(tab, &id).await?;
        ax_button(&ax_tree(tab).await?, &model, &id, Some(true), false)?;
        states.push(json!({"id":id,"checked":true,"requests":model["counts"][&id]}));
    }
    focus_projected(tab, BACK).await?;
    let model = ready(tab, "d.visible").await?;
    let image = photo(tab, env, CHECKED).await?;
    for (observed, (kind, background, foreground)) in states.iter_mut().zip(TOGGLES) {
        let id = id("toggle", kind);
        let g = geometry(tab, &id).await?;
        let actual_fill = fill(&image, g.face)?;
        let label_ink = ink(&image, g.label, color(&model, foreground)?)?;
        check_eq!(actual_fill, color(&model, background)?, "{kind} on face");
        check!(label_ink > 25, "{kind} on label");
        observed["geometry"] = json!(g);
        observed["fill"] = json!(actual_fill);
        observed["label_ink"] = json!(label_ink);
    }
    Ok(json!({"capture":CHECKED,"states":states,"palette":model["palette"]}))
}

async fn pressed(tab: &Tab, expected: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        json!(expected)
    ))
    .await?;
    tab.settle().await
}

fn face_pixels(image: &Raster, region: Rect, face: Rgb, background: Rgb) -> Result<usize> {
    // The Filled button raises a shadow on hover. Count face pixels independently.
    Ok(image
        .colors(region)?
        .iter()
        .filter(|(rgb, _)| distance(**rgb, face) < distance(**rgb, background))
        .map(|(_, count)| count)
        .sum())
}

async fn pointer(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    let id = id("size", "xs");
    let model = ready(tab, "d.visible").await?;
    let g = geometry(tab, &id).await?;
    let canvas = tab.canvas_box().await?;
    let point = |[x, y]: [f64; 2]| [canvas[0] + x, canvas[1] + y];
    let hit = [centre(g.hit)[0], g.hit[1] + 2.];
    check!(
        hit[1] < g.face[1],
        "XS accepts a touch target larger than its painted face"
    );
    let [x, y] = point(hit);
    reduced_motion(tab, false).await?;
    freeze(tab).await?;
    tab.mouse_move(x, y).await?;
    let background = color(&model, "background")?;
    let fill_color = color(&model, "primary")?;
    let region = [g.face[0], g.face[1], 13., 13.];
    let rest_png = "material-buttons-motion-rest.png";
    let early_png = "material-buttons-motion-early.png";
    let rest = face_pixels(
        &photo(tab, env, rest_png).await?,
        region,
        fill_color,
        background,
    )?;
    tab.mouse_down().await?;
    pressed(tab, Some(&id)).await?;
    tick(tab, 28).await?;
    let early = face_pixels(
        &photo(tab, env, early_png).await?,
        region,
        fill_color,
        background,
    )?;
    tick(tab, 240).await?;
    let held = face_pixels(
        &photo(tab, env, HELD).await?,
        region,
        fill_color,
        background,
    )?;
    check!(
        rest < early && early < held,
        "held contour passes through an intermediate shape: rest={rest} early={early} held={held}"
    );
    let [ox, oy] = point([3., 3.]);
    for (kind, buttons) in [("mouseMoved", 1), ("mouseReleased", 0)] {
        tab.cdp(
            "Input.dispatchMouseEvent",
            json!({"type":kind,"x":ox,"y":oy,"button":"left","buttons":buttons,"clickCount":1}),
        )
        .await?;
    }
    pressed(tab, None).await?;
    let no_request = format!("(d.counts[{}] ?? 0) === 0", js(&id));
    ready(tab, &no_request).await?;
    tick(tab, 480).await?;
    reduced_motion(tab, true).await?;
    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(&id)).await?;
    let reduced_png = "material-buttons-motion-reduced.png";
    let snapped = face_pixels(
        &photo(tab, env, reduced_png).await?,
        region,
        fill_color,
        background,
    )?;
    check!(
        snapped > early && snapped.abs_diff(held) <= 8,
        "Reduce Motion reaches the pressed contour at once"
    );
    tab.mouse_up().await?;
    ready(tab, &format!("d.counts[{}] === 1", js(&id))).await?;
    let touches = json!([{ "x": x, "y": y, "id": 1 }]);
    for (end, count) in [("touchCancel", 1), ("touchEnd", 2)] {
        tab.cdp(
            "Input.dispatchTouchEvent",
            json!({"type":"touchStart","touchPoints":touches}),
        )
        .await?;
        pressed(tab, Some(&id)).await?;
        tab.cdp(
            "Input.dispatchTouchEvent",
            json!({"type":end,"touchPoints":[]}),
        )
        .await?;
        pressed(tab, None).await?;
        ready(tab, &format!("d.counts[{}] === {count}", js(&id))).await?;
    }
    check_eq!(
        geometry(tab, &id).await?.hit,
        g.hit,
        "activation never shifts the hit target"
    );
    Ok(
        json!({"capture":HELD,"originals":[rest_png,early_png,HELD,reduced_png],"region":region,"hit":hit,"corner_pixels":[rest,early,held,snapped],"outside_requests":0,"accepted_mouse":1,"after_touch_cancel":1,"after_touch_tap":2}),
    )
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, 320, 880).await?;
    for control in SETTINGS {
        activate(tab, control).await?;
    }
    let model = ready(tab, "d.dark && d.rtl && d.text_scale === 2").await?;
    let scroller = bounds(tab, SCROLL).await?;
    let leading = "button-demo-leading";
    let trailing = "button-demo-trailing";
    let wrapped = "button-demo-wrapped";
    let initial = geometry(tab, wrapped).await?;
    let mut icon_slots = Vec::new();
    check!(
        !contains(scroller, initial.hit),
        "the long label starts below the visible scroll region"
    );
    let order: Vec<_> = SIZES
        .iter()
        .map(|(size, _, _, _)| id("size", size))
        .chain(COMMANDS.iter().map(|(kind, _, _)| id("command", kind)))
        .chain(TOGGLES.iter().map(|(kind, _, _)| id("toggle", kind)))
        .chain([leading, trailing, wrapped].map(str::to_owned))
        .collect();
    focused(tab, SETTINGS[2]).await?;
    for target in &order {
        tab.press("Tab").await?;
        focused(tab, target).await?;
        if target == leading || target == trailing {
            let glyph = part(tab, target, "icon").await?;
            let label = part(tab, target, "label").await?;
            check_eq!(
                centre(glyph)[0] > centre(label)[0],
                target == leading,
                "RTL mirrors the logical icon slot"
            );
            icon_slots.push(json!({"id":target,"icon":glyph,"label":label}));
        }
    }
    let revealed = geometry(tab, wrapped).await?;
    check!(
        contains(scroller, revealed.label)
            && revealed.label[3] > 48.
            && revealed.hit[1] < initial.hit[1],
        "trusted Tab reveals the full multiline 200 percent label"
    );
    // Scroll the final focus outline into the reserved 6 dp padding.
    let canvas = tab.canvas_box().await?;
    tab.mouse_move(
        canvas[0] + scroller[0] + 2.,
        canvas[1] + centre(scroller)[1],
    )
    .await?;
    tab.wheel(0., 64.).await?;
    tab.settle().await?;
    focused(tab, wrapped).await?;
    let g = geometry(tab, wrapped).await?;
    check!(
        contains(scroller, g.focus),
        "the last focus outline fits inside the scroller"
    );
    let image = photo(tab, env, COMPACT).await?;
    let ring = color(&model, "secondary")?;
    let ring_pixels = ink(&image, g.focus, ring)?.saturating_sub(ink(&image, g.face, ring)?);
    let label_pixels = ink(&image, g.label, color(&model, "on_primary")?)?;
    check!(
        ring_pixels > 60 && label_pixels > 120,
        "the large label and focus are painted in the dark PNG"
    );
    tab.press("Enter").await?;
    ready(tab, "d.counts['button-demo-wrapped'] === 1").await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"capture":COMPACT,"geometry":g,"ring_pixels":ring_pixels,"label_pixels":label_pixels,"tab_order":order,"back_focus":SHOW,
            "palette":model["palette"],"initial_hit":initial.hit,"scroll":scroller,"icon_slots":icon_slots,
            "size":{"dark":model["dark"],"rtl":model["rtl"],"text_scale":model["text_scale"],"width":model["width"],"height":model["height"]}}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1204, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(STEPS[0], "Five sizes and five command appearances use measured targets and actual Material paint");
        route(&tab, url).await?;
        report["light"] = wide(&tab, env).await?;
        step(STEPS[1], "Four caller-controlled toggles expose one stable action and ignore held key repeats");
        report["checked"] = keyboard(&tab, env).await?;
        step(STEPS[2], "Trusted pointer presses the XS hit slack, samples the shape spring and cancels outside");
        report["held"] = pointer(&tab, env, url).await?;
        step(STEPS[3], "A compact dark RTL route keeps logical icon slots, large text and visible exterior focus");
        report["compact"] = compact(&tab, env, url).await?;
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-buttons-failure.png"))
            .await;
    }
    env.write_json("material-buttons.json", &report)?;
    result?;
    check!(tab.errors().is_empty(), "no common button page errors");
    Ok(())
}
