//! Button group geometry, Chrome accessibility, and trusted input in WebGPU.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, time::Duration};

const TRACK: &str = "button-group-track";
const VIEWPORT: &str = "button-group-track-viewport";
const PAGE_SCROLL: &str = "button-group-demo-scroll";
const OVERFLOW: &str = "button-group-track-overflow";
const MENU: &str = "button-group-track-menu";
const LABELS: [&str; 5] = ["裁切", "標示", "鎖定", "對齊", "匯出"];
const SIZES: [(&str, f64); 5] = [
    ("xs", 32.),
    ("s", 40.),
    ("m", 56.),
    ("l", 96.),
    ("xl", 136.),
];
type Rect = [f64; 4];
type Rgb = [u8; 3];

fn item(index: usize) -> String {
    format!("button-group-item-{index}")
}

fn face(index: usize) -> String {
    format!("{}-face", item(index))
}

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["button_group"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.button_group)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, choice: &str, condition: &str) -> Result<()> {
    click(tab, &format!("button-group-{choice}")).await?;
    ready(tab, condition).await
}

async fn choice(tab: &Tab, field: &str, value: &str, choice: &str) -> Result<()> {
    select(tab, choice, &format!("d.{field} === {}", js(value))).await
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

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .try_into()
        .ok()
        .context("four button group bounds")
}

fn face_measurement() -> String {
    let ids = (0..LABELS.len()).map(face).collect::<Vec<_>>();
    format!(
        "{}.map(id => Array.from(window.gallery.control_bounds(id)))",
        json!(ids)
    )
}

async fn faces(tab: &Tab) -> Result<[Rect; 5]> {
    let measurement = face_measurement();
    tab.wait_for(&format!("{measurement}.every(b => b.length === 4)"))
        .await?;
    Ok(serde_json::from_value(tab.eval(&measurement).await?)?)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn same_rect(a: Rect, b: Rect) -> bool {
    a.into_iter().zip(b).all(|(a, b)| near(a, b))
}

fn contains([x, y, w, h]: Rect, [a, b, c, d]: Rect) -> bool {
    a >= x - 0.75 && b >= y - 0.75 && a + c <= x + w + 0.75 && b + d <= y + h + 0.75
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    tab.wait_for(&format!("document.activeElement === {}", projected(id)))
        .await
}

async fn focus(tab: &Tab, id: &str) -> Result<()> {
    let element = projected(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    focused(tab, id).await
}

async fn selection(tab: &Tab, selected: &[usize]) -> Result<()> {
    ready(
        tab,
        &format!(
            "JSON.stringify(d.selected) === {}",
            js(json!(selected).to_string())
        ),
    )
    .await
}

async fn color_mode(tab: &Tab, dark: bool) -> Result<()> {
    let current = state(tab).await?["dark"] == true;
    if current != dark {
        click(tab, "button-group-theme").await?;
    }
    until(tab, &format!("s.dark === {dark}")).await?;
    tab.settle().await
}

struct Image {
    pixels: Vec<u8>,
    frame: png::OutputInfo,
    channels: usize,
    scale: f64,
    canvas: Rect,
}

impl Image {
    async fn capture(env: &Env, tab: &Tab, label: &str) -> Result<Self> {
        tab.settle().await?;
        let path = env.artifact(&format!("material-button-group-{label}.png"));
        tab.screenshot(&path).await?;
        let decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let frame = reader.next_frame(&mut pixels)?;
        let channels = match frame.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG color type {other:?}"),
        };
        let viewport = tab
            .eval("innerWidth")
            .await?
            .as_f64()
            .context("viewport width")?;
        Ok(Self {
            pixels,
            scale: f64::from(frame.width) / viewport,
            frame,
            channels,
            canvas: tab.canvas_box().await?,
        })
    }

    fn sample(&self, x: f64, y: f64) -> Result<Rgb> {
        let x = ((x + self.canvas[0]) * self.scale).floor() as usize;
        let y = ((y + self.canvas[1]) * self.scale).floor() as usize;
        check!(
            x < self.frame.width as usize && y < self.frame.height as usize,
            "button pixel within PNG"
        );
        let offset = y * self.frame.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }

    fn fill(&self, [x, y, w, h]: Rect) -> Result<Rgb> {
        self.sample(x + (w / 8.).min(6.), y + h / 2.)
    }
}

fn distance(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn painted_fill(value: &Value, background: Rgb) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] =
        serde_json::from_value(value.clone()).context("button fill RGBA")?;
    let opacity = f64::from(alpha) / 255.;
    Ok(std::array::from_fn(|i| {
        (f64::from([r, g, b][i]) * opacity + f64::from(background[i]) * (1. - opacity)).round()
            as u8
    }))
}

fn face_style(export: &Value, n: usize) -> Result<&Value> {
    Ok(&find_node(export, &face(n))?["style"])
}

fn corners(style: &Value) -> Result<[f64; 4]> {
    let c = &style["corners"];
    let uniform = style["radius"].as_f64().unwrap_or(0.);
    if c.is_null() {
        return Ok([uniform; 4]);
    }
    let mut radii = [0.; 4];
    for (value, side) in
        radii
            .iter_mut()
            .zip(["top_left", "top_right", "bottom_right", "bottom_left"])
    {
        *value = c[side]
            .as_f64()
            .with_context(|| format!("button radius {side}"))?;
    }
    Ok(radii)
}

async fn sizes_and_paint(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M83",
        "all five sizes keep full hit targets and both themes paint the group states",
    );
    ready(
        tab,
        "d.kind === 'standard' && d.shape === 'round' && d.size === 's'",
    )
    .await?;
    selection(tab, &[1]).await?;
    let mut dimensions = Vec::new();
    for (size, height) in SIZES {
        choice(tab, "size", size, &format!("size-{size}")).await?;
        let actual = faces(tab).await?;
        let track = bounds(tab, TRACK).await?;
        for (index, face) in actual.into_iter().enumerate() {
            let hit = bounds(tab, &item(index)).await?;
            check!(
                near(face[3], height)
                    && hit[2] >= 47.25
                    && hit[3] >= 47.25
                    && contains(track, hit)
                    && contains(hit, face),
                "{size} item {index} paints at {height} dp with a full hit area"
            );
        }
        dimensions.push(json!({"size":size,"first_face":actual[0],"track":track}));
    }
    choice(tab, "size", "s", "size-s").await?;
    let round = corners(face_style(&dom(tab).await?, 0)?)?;
    choice(tab, "shape", "square", "square").await?;
    let square = corners(face_style(&dom(tab).await?, 0)?)?;
    check!(
        round[0] > square[0] + 3.,
        "round and square options have distinct radii"
    );
    choice(tab, "shape", "round", "round").await?;

    let mut themes = Vec::new();
    for dark in [false, true] {
        color_mode(tab, dark).await?;
        tab.mouse_move(10., 50.).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let name = if dark {
            "standard-dark"
        } else {
            "standard-light"
        };
        let image = Image::capture(env, tab, name).await?;
        let export = dom(tab).await?;
        let actual = faces(tab).await?;
        let page = find_node(&export, "button-group-demo-page")?;
        let background = painted_fill(&page["style"]["background"], [0; 3])?;
        let mut colors = Vec::new();
        for index in [0, 1, 2] {
            let style = face_style(&export, index)?;
            let expected = painted_fill(&style["background"], background)?;
            let fill = image.fill(actual[index])?;
            check!(
                distance(fill, expected) <= 10,
                "{name} item {index} paints its token {fill:?}/{expected:?}"
            );
            colors.push(fill);
        }
        check!(
            distance(colors[0], colors[1]) > 15,
            "checked and unchecked buttons look distinct"
        );
        themes.push(json!({"name":name,"colors":colors}));
    }
    check!(
        themes[0]["colors"][0] != themes[1]["colors"][0],
        "light and dark buttons use distinct palettes"
    );
    color_mode(tab, false).await?;
    Ok(json!({"sizes":dimensions,"round":round,"square":square,"themes":themes}))
}

async fn wait_for_faces(tab: &Tab, baseline: &[Rect; 5]) -> Result<()> {
    tab.wait_for_within(
        &format!(
            "{}.every((rect, i) => rect.length === 4 && rect.every(
                (n, j) => Math.abs(n - {}[i][j]) <= 0.75))",
            face_measurement(),
            json!(baseline)
        ),
        Duration::from_secs(4),
    )
    .await?;
    tab.settle().await
}

async fn wait_for_growth(tab: &Tab, item_zero_width: f64) -> Result<[Rect; 5]> {
    tab.wait_for_within(
        &format!("{}[0][2] > {}", face_measurement(), item_zero_width + 1.),
        Duration::from_secs(4),
    )
    .await?;
    faces(tab).await
}

fn redistribution(before: &[Rect; 5], held: &[Rect; 5]) -> Result<()> {
    let expanded = held[0][2] - before[0][2];
    let contracted = before[1][2] - held[1][2];
    check!(
        expanded > 1. && contracted > 1. && near(expanded, contracted),
        "the pressed item borrows width from its direct neighbor ({expanded} vs {contracted})"
    );
    for index in 2..before.len() {
        check!(
            same_rect(before[index], held[index]),
            "nonadjacent item {index} stays in place"
        );
    }
    Ok(())
}

async fn reduce_motion(tab: &Tab, reduce: bool) -> Result<()> {
    let setting = if reduce { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":setting}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {setting})"))
    ))
    .await?;
    ready(tab, &format!("d.reduced_motion === {reduce}")).await
}

async fn touch(tab: &Tab, kind: &str, point: Option<(f64, f64)>) -> Result<()> {
    let points = point.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn gesture_width(env: &Env, tab: &Tab) -> Result<Value> {
    step("M84", "trusted hold reallocates adjacent width and cancellation restores the group, including Reduce Motion");
    reduce_motion(tab, false).await?;
    let baseline = faces(tab).await?;
    let original = data(tab).await?;
    let endpoint = point(tab, &item(0)).await?;
    tab.eval(
        "(() => {
            window.buttonGroupEvents = [];
            for (const kind of ['pointerdown','pointerup','pointercancel']) {
                document.addEventListener(kind, e => {
                    if (e.target?.id === 'surface')
                        window.buttonGroupEvents.push([kind,e.pointerType,e.isTrusted]);
                }, {capture:true,passive:true});
            }
        })()",
    )
    .await?;
    tab.mouse_move(endpoint.x, endpoint.y).await?;
    tab.mouse_down().await?;
    ready(tab, "d.pressed === 0").await?;
    wait_for_growth(tab, baseline[0][2]).await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let held = faces(tab).await?;
    redistribution(&baseline, &held)?;
    Image::capture(env, tab, "standard-mouse-hold").await?;
    tab.press("Escape").await?;
    ready(tab, "d.pressed === null").await?;
    wait_for_faces(tab, &baseline).await?;
    tab.mouse_up().await?;

    reduce_motion(tab, true).await?;
    tab.eval(
        "(() => { const now = performance.now();
            Object.defineProperty(performance, 'now', {configurable:true, value:() => now});
        })()",
    )
    .await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    let endpoint = point(tab, &item(0)).await?;
    touch(tab, "touchStart", Some((endpoint.x, endpoint.y))).await?;
    ready(tab, "d.pressed === 0").await?;
    wait_for_faces(tab, &baseline).await?;
    let reduced = faces(tab).await?;
    let reduced_radii = corners(face_style(&dom(tab).await?, 0)?)?;
    check!(
        reduced_radii.iter().all(|&radius| near(radius, 8.)),
        "reduced motion preserves the pressed contour without moving widths"
    );
    Image::capture(env, tab, "standard-touch-reduced-motion").await?;
    touch(tab, "touchCancel", None).await?;
    ready(tab, "d.pressed === null").await?;
    wait_for_faces(tab, &baseline).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    tab.eval("delete performance.now").await?;
    reduce_motion(tab, false).await?;

    let after_cancel = data(tab).await?;
    check_eq!(
        &after_cancel["activations"],
        &original["activations"],
        "canceled holds invoke no action"
    );
    check_eq!(
        &after_cancel["selected"],
        &original["selected"],
        "canceled holds change no selection"
    );
    let events = tab.eval("window.buttonGroupEvents").await?;
    let list = events.as_array().context("CDP pointer events")?;
    for (kind, device) in [
        ("pointerdown", "mouse"),
        ("pointerup", "mouse"),
        ("pointerdown", "touch"),
        ("pointercancel", "touch"),
    ] {
        check!(
            list.contains(&json!([kind, device, true])),
            "Chrome emits a trusted {device} {kind}"
        );
    }
    Ok(
        json!({"baseline":baseline,"held":held,"reduced_motion":reduced,
        "reduced_radii":reduced_radii,"events":events}),
    )
}

fn check_connected_edges(rects: &[Rect; 5], rtl: bool) -> Result<()> {
    for adjacent in rects.windows(2) {
        let [a, b] = [adjacent[0], adjacent[1]];
        let gap = if rtl {
            a[0] - b[0] - b[2]
        } else {
            b[0] - a[0] - a[2]
        };
        check!(
            near(gap, 2.) && near(a[1], b[1]) && near(a[3], b[3]),
            "connected faces align at a common height with a 2 dp gap, got {gap}"
        );
    }
    Ok(())
}

fn diagonal(radius: f64) -> f64 {
    radius * (1. - std::f64::consts::FRAC_1_SQRT_2)
}

async fn connected(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M85",
        "connected buttons have a smaller join, a pressed contour, and mirrored end shapes",
    );
    choice(tab, "kind", "connected", "connected").await?;
    tab.mouse_move(10., 50.).await?;
    let rests = faces(tab).await?;
    check_connected_edges(&rests, false)?;
    let export = dom(tab).await?;
    let leading = corners(face_style(&export, 0)?)?;
    let trailing = corners(face_style(&export, 4)?)?;
    check!(
        leading[0] > leading[1] + 4. && trailing[1] > trailing[0] + 4.,
        "connected end caps round out while the joining corners stay small"
    );
    let image = Image::capture(env, tab, "connected-rest").await?;
    let selected = corners(face_style(&export, 1)?)?;
    let endpoint = point(tab, &item(1)).await?;
    tab.mouse_move(endpoint.x, endpoint.y).await?;
    tab.mouse_down().await?;
    ready(tab, "d.pressed === 1").await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let pressed_export = dom(tab).await?;
    let pressed = corners(face_style(&pressed_export, 1)?)?;
    check!(
        selected[0] > pressed[0] + 4. && selected[1] > pressed[1] + 4.,
        "press shrinks both joining corners of the selected button"
    );
    let held = faces(tab).await?;
    check!(
        same_rect(held[1], rests[1]),
        "connected press preserves the item footprint"
    );
    let held_image = Image::capture(env, tab, "connected-pressed").await?;
    let [x, y, w, h] = rests[1];
    let inset = (diagonal(selected[0].min(w.min(h) / 2.)) + diagonal(pressed[0])) / 2.;
    let [hx, hy, _, _] = held[1];
    let before_corner = image.sample(x + inset, y + inset)?;
    let held_corner = held_image.sample(hx + inset, hy + inset)?;
    let after_fill = held_image.fill(held[1])?;
    check!(
        distance(before_corner, image.fill(rests[1])?) > 12
            && distance(held_corner, after_fill) <= 12,
        "actual WebGPU pixels fill the corner newly enclosed by the pressed contour"
    );
    tab.press("Escape").await?;
    ready(tab, "d.pressed === null").await?;
    tab.mouse_up().await?;
    wait_for_faces(tab, &rests).await?;

    select(tab, "direction", "d.rtl").await?;
    let mirrored = faces(tab).await?;
    check_connected_edges(&mirrored, true)?;
    let mirrored_export = dom(tab).await?;
    let rtl_lead = corners(face_style(&mirrored_export, 0)?)?;
    let rtl_tail = corners(face_style(&mirrored_export, 4)?)?;
    check!(
        rtl_lead[1] > rtl_lead[0] + 4. && rtl_tail[0] > rtl_tail[1] + 4.,
        "logical end caps move to the other physical edge in RTL"
    );
    Image::capture(env, tab, "connected-rtl").await?;
    Ok(json!({"rest":rests,"held":held,"rtl":mirrored,
        "lead_radii":leading,"selected_radii":selected,"held_radii":pressed,"rtl_lead_radii":rtl_lead,
        "rest_corner":before_corner,"held_corner":held_corner,"held_fill":after_fill}))
}

async fn check_pressed(tab: &Tab, selected: &[usize]) -> Result<()> {
    for (index, label) in LABELS.iter().enumerate() {
        let toggle = projected(&item(index));
        let pressed = selected.contains(&index).to_string();
        tab.wait_for(&format!(
            "{toggle}?.getAttribute('aria-label') === {} && {toggle}.getAttribute('aria-pressed') === {}",
            js(label), js(pressed)
        ))
        .await?;
    }
    Ok(())
}

async fn keyboard_and_selection(env: &Env, tab: &Tab) -> Result<Value> {
    step("M86", "Tab enters the first enabled button, arrows skip disabled, and Enter or Space updates selection");
    select(tab, "direction", "!d.rtl").await?;
    choice(tab, "selection", "single", "single").await?;
    selection(tab, &[1]).await?;
    check_pressed(tab, &[1]).await?;
    let group = projected(TRACK);
    tab.wait_for(&format!(
        "{group}?.getAttribute('aria-label') === '圖片工具'"
    ))
    .await?;
    let before = data(tab).await?["activations"].clone();
    focus(tab, "button-group-large-text").await?;
    tab.press("Tab").await?;
    focused(tab, &item(0)).await?;
    tab.press("ArrowRight").await?;
    focused(tab, &item(1)).await?;
    tab.press("ArrowRight").await?;
    focused(tab, &item(3)).await?;
    selection(tab, &[1]).await?;
    check_eq!(
        &data(tab).await?["activations"],
        &before,
        "focus movement invokes no action"
    );
    tab.press("Enter").await?;
    selection(tab, &[3]).await?;
    focused(tab, &item(3)).await?;
    let disabled = projected(&item(2));
    tab.wait_for(&format!(
        "{disabled}?.getAttribute('aria-disabled') === 'true' && {disabled}.tabIndex < 0"
    ))
    .await?;
    click(tab, &item(2)).await?;
    selection(tab, &[3]).await?;
    check_eq!(
        &data(tab).await?["activations"][2],
        &before[2],
        "disabled pointer click invokes no action"
    );
    let ax = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let selected_ax = ax["nodes"]
        .as_array()
        .and_then(|array| {
            array.iter().find(|node| {
                node["ignored"] == false
                    && node["name"]["value"] == LABELS[3]
                    && node["properties"].as_array().is_some_and(|props| {
                        props.iter().any(|prop| {
                            let value = &prop["value"]["value"];
                            prop["name"] == "pressed" && (value == true || value == "true")
                        })
                    })
            })
        })
        .context("Chrome accessibility exposes the selected icon-only button as pressed")?;

    choice(tab, "selection", "multi", "multi").await?;
    focus(tab, &item(0)).await?;
    tab.press("Space").await?;
    selection(tab, &[0, 3]).await?;
    check_pressed(tab, &[0, 3]).await?;
    select(tab, "required", "d.required").await?;
    click(tab, &item(3)).await?;
    selection(tab, &[0]).await?;
    click(tab, &item(0)).await?;
    selection(tab, &[0]).await?;
    focus(tab, &item(0)).await?;
    tab.press("Space").await?;
    selection(tab, &[0]).await?;
    check_pressed(tab, &[0]).await?;
    Image::capture(env, tab, "keyboard-required").await?;
    Ok(json!({"icon_only_accessibility":selected_ax,"before":before,"after":data(tab).await?}))
}

async fn wheel_at(tab: &Tab, viewport: Rect, dx: f64, dy: f64) -> Result<()> {
    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("device scale")?;
    tab.mouse_move(
        canvas_x + viewport[0] + viewport[2] / 2.,
        canvas_y + viewport[1] + viewport[3] / 2.,
    )
    .await?;
    tab.wheel(dx * scale, dy * scale).await?;
    tab.settle().await
}

async fn reveal_row(tab: &Tab) -> Result<Rect> {
    let area = bounds(tab, PAGE_SCROLL).await?;
    let bar = bounds(tab, VIEWPORT).await?;
    if bar[1] < area[1] || bar[1] + bar[3] > area[1] + area[3] {
        let dy = bar[1] + bar[3] / 2. - area[1] - area[3] / 2.;
        wheel_at(tab, area, 0., dy).await?;
    }
    let area = bounds(tab, PAGE_SCROLL).await?;
    let view = bounds(tab, VIEWPORT).await?;
    check!(
        contains(area, view),
        "the whole button group viewport is reachable"
    );
    Ok(view)
}

async fn reveal_item(tab: &Tab, id: &str) -> Result<Rect> {
    let view = reveal_row(tab).await?;
    let target = bounds(tab, id).await?;
    if !contains(view, target) {
        let dx = target[0] + target[2] / 2. - view[0] - view[2] / 2.;
        wheel_at(tab, view, dx, 0.).await?;
    }
    let seen = bounds(tab, id).await?;
    check!(
        contains(view, seen),
        "{id} is fully reachable inside the horizontal viewport"
    );
    Ok(seen)
}

fn label_layout(export: &Value, index: usize, name: &str) -> Result<Rect> {
    let face = find_node(export, &face(index))?;
    let label_id = face["last_child"]
        .as_u64()
        .context("button face label id")?;
    let label = &export["dom"]["nodes"][label_id as usize];
    check_eq!(label["text"].as_str(), Some(name), "rendered label text");
    let layout = export["layout"]["nodes"]
        .as_array()
        .context("layout nodes")?;
    let bounds = &layout
        .iter()
        .find(|entry| entry["id"] == label_id)
        .context("rendered label bounds")?["bounds"];
    let mut rect = [0.; 4];
    for (coordinate, field) in rect.iter_mut().zip(["x", "y", "width", "height"]) {
        *coordinate = bounds[field]
            .as_f64()
            .with_context(|| format!("label {field}"))?;
    }
    let font = label["style"]["font_size"]
        .as_f64()
        .context("button label font")?;
    let line = label["style"]["line_height"]
        .as_f64()
        .context("button label line height")?;
    check!(
        rect[2] >= 2. * font - 0.75 && rect[3] >= 2. * line - 0.75,
        "label {name} has room for a complete scaled character and line"
    );
    Ok(rect)
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M87",
        "320 dp dark RTL keeps 200% button names and touch targets accessible on a single row",
    );
    choice(tab, "kind", "connected", "connected").await?;
    choice(tab, "size", "xl", "size-xl").await?;
    select(tab, "direction", "d.rtl").await?;
    select(tab, "large-text", "d.text_scale === 2").await?;
    color_mode(tab, true).await?;
    resize(tab, 320, 620).await?;
    let view = reveal_row(tab).await?;
    check!(
        view[0] >= 0. && view[0] + view[2] <= 320.75,
        "the group viewport fits the compact app"
    );
    let all_faces = faces(tab).await?;
    check_connected_edges(&all_faces, true)?;
    let export = dom(tab).await?;
    let mut labels = Vec::new();
    for (index, name) in LABELS.iter().enumerate() {
        let item_id = item(index);
        let node = find_node(&export, &item_id)?;
        check_eq!(
            node["name"].as_str(),
            Some(*name),
            "compact accessible name"
        );
        let hit = bounds(tab, &item_id).await?;
        check!(
            hit[2] >= 47.25 && hit[3] >= 47.25 && contains(hit, all_faces[index]),
            "compact item {index} keeps its full face and touch area"
        );
        if index != 3 {
            let label = label_layout(&export, index, name)?;
            check!(
                contains(all_faces[index], label),
                "compact label {name} fits its face"
            );
            labels.push(label);
        }
    }
    let leading = bounds(tab, &item(0)).await?;
    check!(
        contains(view, leading) && near(leading[0] + leading[2], view[0] + view[2]),
        "RTL starts with the whole logical first button visible at the right edge"
    );
    focus(tab, &item(0)).await?;
    tab.press("ArrowLeft").await?;
    focused(tab, &item(1)).await?;
    tab.press("ArrowLeft").await?;
    focused(tab, &item(3)).await?;
    let inner = bounds(tab, &item(3)).await?;
    check!(
        contains(view, inner),
        "arrow navigation reveals the focused button"
    );
    tab.press("Enter").await?;
    selection(tab, &[3]).await?;
    focused(tab, &item(3)).await?;
    Image::capture(env, tab, "compact-dark-rtl").await?;
    let trailing = reveal_item(tab, &item(4)).await?;
    Image::capture(env, tab, "compact-dark-rtl-end").await?;
    Ok(json!({"viewport":view,"faces":all_faces,"labels":labels,
            "leading":leading,"last":trailing,"keyboard_target":inner}))
}

async fn command_overflow(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M88",
        "compact trailing commands overflow, activate once, and restore the menu trigger focus",
    );
    choice(tab, "selection", "commands", "commands").await?;
    choice(tab, "size", "xl", "size-xl").await?;
    select(tab, "large-text", "d.text_scale === 2").await?;
    resize(tab, 320, 620).await?;
    let trigger = reveal_item(tab, OVERFLOW).await?;
    tab.wait_for(&format!("{} === null", projected(&item(4))))
        .await?;
    let before = data(tab).await?["activations"][4]
        .as_u64()
        .context("export action count")?;
    focus(tab, OVERFLOW).await?;
    tab.press("Enter").await?;
    ready(tab, "d.overflow_open").await?;
    bounds(tab, MENU).await?;
    let export_item = projected(&item(4));
    tab.wait_for(&format!(
        "{export_item}?.getAttribute('role') === 'menuitem' && {export_item}.getAttribute('aria-label') === '匯出'"
    )).await?;
    Image::capture(env, tab, "compact-command-overflow").await?;
    tab.press("Escape").await?;
    ready(tab, "!d.overflow_open").await?;
    focused(tab, OVERFLOW).await?;
    tab.press("Enter").await?;
    ready(tab, "d.overflow_open").await?;
    focus(tab, &item(4)).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        &format!("!d.overflow_open && d.activations[4] === {}", before + 1),
    )
    .await?;
    focused(tab, OVERFLOW).await?;
    selection(tab, &[]).await?;
    tab.press("Enter").await?;
    ready(tab, "d.overflow_open").await?;
    focus(tab, &item(4)).await?;
    resize(tab, 1600, 890).await?;
    ready(tab, "!d.overflow_open").await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(MENU)
    ))
    .await?;
    tab.wait_for(&format!("{export_item}?.getAttribute('role') === 'button'"))
        .await?;
    focused(tab, &item(4)).await?;
    let inline = bounds(tab, &item(4)).await?;
    check_eq!(
        data(tab).await?["activations"][4],
        json!(before + 1),
        "widening and transferring focus invokes no command"
    );
    Image::capture(env, tab, "command-overflow-widened").await?;
    Ok(
        json!({"trigger":trigger,"export_count_before":before,"export_count_after":before+1,
        "focused_inline_command":inline}),
    )
}

async fn open(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 1100, 890).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, "button-group-demo-show").await?;
    ready(tab, "d.open").await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    open(tab).await?;
    let appearance = sizes_and_paint(env, tab).await?;
    let press = gesture_width(env, tab).await?;
    let connected = connected(env, tab).await?;
    let selection = keyboard_and_selection(env, tab).await?;
    tab.goto(url).await?;
    open(tab).await?;
    let compact = compact(env, tab).await?;
    tab.goto(url).await?;
    open(tab).await?;
    let overflow = command_overflow(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "button group page errors: {errors:?}");
    env.write_json(
        "browser-material-button-group.json",
        &json!({"browser":chrome.version,"appearance":appearance,"press":press,"connected":connected,
            "selection":selection,"compact":compact,"overflow":overflow,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(1100, 934, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-button-group-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-button-group-failure.png"))
            .await;
    }
    result
}
