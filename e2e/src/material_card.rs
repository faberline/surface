//! Material cards through Chromium WebGPU, accessibility and trusted input.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

const PRIMARY: &str = "card-primary";
const PASSIVE: &str = "card-secondary";
const DISABLED: &str = "card-disabled";
const NESTED: &str = "card-nested-action";
const MOVE: &str = "card-move";
const SCROLL: &str = "card-demo-scroll";
const PRIMARY_MEDIA: &str = "card-primary-media";
const PRIMARY_TITLE: &str = "card-primary-title";
const PASSIVE_TITLE: &str = "card-secondary-title";
const PRIMARY_NAME: &str = "山徑週末";
const PASSIVE_NAME: &str = "行程草稿";
const DISABLED_NAME: &str = "封存行程";

type Rect = [f64; 4];

async fn cards(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["cards"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.cards)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, id: &str, condition: &str) -> Result<()> {
    click(tab, id).await?;
    ready(tab, condition).await
}

async fn variant(tab: &Tab, kind: &str) -> Result<()> {
    select(
        tab,
        &format!("card-{kind}"),
        &format!("d.variant === {}", js(kind)),
    )
    .await
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
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

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return v?.width === {width} && v?.height === {height}
        }})()"
    ))
    .await?;
    tab.settle().await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .as_slice()
        .try_into()
        .context("four card bounds")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn contains([x, y, w, h]: Rect, [cx, cy, cw, ch]: Rect) -> bool {
    cx >= x - 0.75 && cy >= y - 0.75 && cx + cw <= x + w + 0.75 && cy + ch <= y + h + 0.75
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
        let path = env.artifact(&format!("material-card-{label}.png"));
        tab.screenshot(&path).await?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let frame = reader.next_frame(&mut pixels)?;
        let channels = match frame.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG color type {other:?}"),
        };
        let width = tab
            .eval("innerWidth")
            .await?
            .as_f64()
            .context("window width")?;
        Ok(Self {
            pixels,
            channels,
            scale: f64::from(frame.width) / width,
            frame,
            canvas: tab.canvas_box().await?,
        })
    }

    fn pixel(&self, x: f64, y: f64) -> Result<[u8; 3]> {
        let x = ((x + self.canvas[0]) * self.scale).floor() as usize;
        let y = ((y + self.canvas[1]) * self.scale).floor() as usize;
        if x >= self.frame.width as usize || y >= self.frame.height as usize {
            bail!("card probe is outside the screenshot");
        }
        let offset = y * self.frame.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }
}

fn rgb(value: &Value) -> Result<[u8; 3]> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(value.clone()).context("card RGBA color")?;
    Ok([r, g, b])
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

async fn surface(image: &Image, tab: &Tab, id: &str, exported: &Value) -> Result<[u8; 3]> {
    let [x, y, w, h] = bounds(tab, id).await?;
    let expected = rgb(&find_node(exported, id)?["style"]["background"])?;
    let pixel = image.pixel(x + w - 12., y + h * 0.5)?;
    check!(
        distance(pixel, expected) <= 8,
        "{id} paints {pixel:?}, expected {expected:?}"
    );
    Ok(pixel)
}

async fn appearance(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M78",
        "elevated, filled and outlined cards paint their Material surfaces in both themes",
    );
    let mut samples = Vec::new();
    for dark in [false, true] {
        if dark {
            click(tab, "card-theme").await?;
            until(tab, "s.dark").await?;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let mut shadows = Vec::new();
        let mut fills = Vec::new();
        for kind in ["elevated", "filled", "outlined"] {
            variant(tab, kind).await?;
            tokio::time::sleep(Duration::from_millis(250)).await;
            let export = dom(tab).await?;
            let card = find_node(&export, PRIMARY)?;
            let overlay = &find_node(&export, "card-primary-state")?["style"];
            let border = overlay["border_width"]
                .as_f64()
                .context("card border width")?;
            let layers = card["box_shadows"].as_array().map_or(0, Vec::len);
            check_eq!(
                border > 0.,
                kind == "outlined",
                "outlined variant owns the stroke"
            );
            check_eq!(
                layers > 0,
                kind == "elevated",
                "elevated variant owns the shadow"
            );
            let label = format!("{kind}-{}", if dark { "dark" } else { "light" });
            let image = Image::capture(env, tab, &label).await?;
            let primary = surface(&image, tab, PRIMARY, &export).await?;
            let passive = surface(&image, tab, PASSIVE, &export).await?;
            let [x, y, w, h] = bounds(tab, PRIMARY).await?;
            let shadow = [
                image.pixel(x + w / 2., y + h + 0.25)?,
                image.pixel(x + w / 2., y + h + 0.75)?,
                image.pixel(x + w / 2., y + h + 1.25)?,
                image.pixel(x - 0.75, y + h / 2.)?,
            ];
            shadows.push(shadow);
            fills.push(primary);
            let stroke = if kind == "outlined" {
                let expected = rgb(&overlay["border_color"])?;
                let pixels = [-0.25, 0.25, 0.75, 1.25]
                    .into_iter()
                    .map(|dx| image.pixel(x + dx, y + h / 2.))
                    .collect::<Result<Vec<_>>>()?;
                let actual = pixels
                    .iter()
                    .min_by_key(|&&pixel| distance(pixel, expected))
                    .copied()
                    .context("stroke samples")?;
                check!(
                    distance(actual, expected) <= 16 && distance(actual, primary) > 20,
                    "WebGPU paints the outline at the straight card edge"
                );
                Some(actual)
            } else {
                None
            };
            samples.push(
                json!({"variant":kind,"dark":dark,"primary":primary,"passive":passive,
                "stroke":stroke,"shadow_edge_pixels":shadow,"shadow_layers":layers}),
            );
        }
        let darkened_samples = shadows[0]
            .iter()
            .zip(shadows[1])
            .filter(|(elevated, flat)| {
                distance(**elevated, *flat) > 3 && elevated.iter().zip(flat).all(|(&a, &b)| a <= b)
            })
            .count();
        check!(
            darkened_samples >= 2,
            "elevated card darkens its edge pixels"
        );
        check!(
            fills[0] != fills[1] && fills[0] != fills[2] && fills[1] != fills[2],
            "filled, outlined and elevated tokens differ"
        );
    }
    click(tab, "card-theme").await?;
    until(tab, "!s.dark").await?;
    variant(tab, "elevated").await?;
    Ok(json!(samples))
}

async fn accessibility_button(tab: &Tab, name: &str) -> Result<Value> {
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    tree["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
                node["ignored"] == false
                    && matches!(
                        node["role"]["value"].as_str(),
                        Some("button" | "toggleButton")
                    )
                    && node["name"]["value"] == name
            })
        })
        .cloned()
        .with_context(|| format!("Chrome accessibility has the {name} button"))
}

fn ax_boolean(node: &Value, property: &str, value: bool) -> bool {
    let text = value.to_string();
    node["properties"].as_array().is_some_and(|properties| {
        properties.iter().any(|entry| {
            let actual = &entry["value"]["value"];
            entry["name"] == property && (actual == value || actual == &text)
        })
    })
}

async fn whole_card(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M79",
        "whole card activates once through pointer and keyboard and exposes its toggle to Chrome",
    );
    let before = cards(tab).await?["activations"]
        .as_u64()
        .context("activation count")?;
    click(tab, PRIMARY).await?;
    ready(
        tab,
        &format!("d.activations === {} && !d.selected", before + 1),
    )
    .await?;
    focus(tab, PRIMARY).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        &format!("d.activations === {} && !d.selected", before + 2),
    )
    .await?;
    focused(tab, PRIMARY).await?;
    select(tab, "card-toggle-mode", "d.toggle_mode").await?;
    focus(tab, PRIMARY).await?;
    tab.press("Space").await?;
    ready(
        tab,
        &format!("d.activations === {} && d.selected", before + 3),
    )
    .await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-pressed') === 'true'",
        projected(PRIMARY)
    ))
    .await?;
    let pressed = accessibility_button(tab, PRIMARY_NAME).await?;
    check!(
        ax_boolean(&pressed, "pressed", true),
        "Chrome AX reports the pressed card"
    );
    Image::capture(env, tab, "toggle-selected").await?;
    click(tab, PRIMARY).await?;
    ready(
        tab,
        &format!("d.activations === {} && !d.selected", before + 4),
    )
    .await?;
    let unpressed = accessibility_button(tab, PRIMARY_NAME).await?;
    check!(
        ax_boolean(&unpressed, "pressed", false),
        "Chrome AX reports the released card"
    );

    select(tab, "card-disable", "d.disabled").await?;
    let disabled = accessibility_button(tab, PRIMARY_NAME).await?;
    let always_disabled = accessibility_button(tab, DISABLED_NAME).await?;
    check!(
        ax_boolean(&disabled, "disabled", true) && ax_boolean(&always_disabled, "disabled", true),
        "Chrome AX exposes dynamic and permanent disabled cards"
    );
    for id in [PRIMARY, DISABLED] {
        tab.wait_for(&format!(
            "{}?.getAttribute('aria-disabled') === 'true' && !{}.hasAttribute('tabindex')",
            projected(id),
            projected(id)
        ))
        .await?;
        click(tab, id).await?;
    }
    tab.settle().await?;
    check_eq!(
        cards(tab).await?["activations"],
        json!(before + 4),
        "disabled cards ignore pointer activation"
    );
    Image::capture(env, tab, "disabled").await?;
    select(tab, "card-disable", "!d.disabled").await?;
    select(tab, "card-toggle-mode", "!d.toggle_mode").await?;
    Ok(
        json!({"activations":before+4,"pressed":pressed,"unpressed":unpressed,
        "disabled":disabled,"permanently_disabled":always_disabled}),
    )
}

async fn passive_card(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M80",
        "a passive card keeps its own role and nested actions activate independently",
    );
    let export = dom(tab).await?;
    let passive = find_node(&export, PASSIVE)?;
    check_eq!(passive["name"], json!(PASSIVE_NAME), "passive card name");
    check_eq!(
        passive["role"],
        json!("Group"),
        "passive card has a grouping role"
    );
    tab.wait_for(&format!(
        "{}?.getAttribute('role') === 'group' && !{}.hasAttribute('tabindex')",
        projected(PASSIVE),
        projected(PASSIVE)
    ))
    .await?;
    let before = cards(tab).await?;
    let nested = before["nested_activations"]
        .as_u64()
        .context("nested count")?;
    let [x, y, w, _] = bounds(tab, PASSIVE).await?;
    let canvas = tab.canvas_box().await?;
    tab.click(canvas[0] + x + w - 12., canvas[1] + y + 12.)
        .await?;
    tab.settle().await?;
    check_eq!(
        cards(tab).await?["nested_activations"],
        json!(nested),
        "passive surface does not invoke its action"
    );
    click(tab, NESTED).await?;
    ready(tab, &format!("d.nested_activations === {}", nested + 1)).await?;
    focus(tab, NESTED).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.nested_activations === {}", nested + 2)).await?;
    focused(tab, NESTED).await?;
    let after = cards(tab).await?;
    for field in ["activations", "moves", "position"] {
        check_eq!(
            after[field],
            before[field],
            format!("nested action leaves {field} unchanged")
        );
    }
    Image::capture(env, tab, "passive-action").await?;
    Ok(
        json!({"nested_activations":nested+2,"focused":NESTED,"primary_activations":after["activations"]}),
    )
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

async fn mouse(tab: &Tab, kind: &str, (x, y): (f64, f64), buttons: u8) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,
        "button":"left","buttons":buttons,"clickCount":1}),
    )
    .await?;
    tab.settle().await
}

async fn drag_points(tab: &Tab, dy: f64) -> Result<((f64, f64), (f64, f64))> {
    let handle = point(tab, MOVE).await?;
    let end_y = handle.y + dy;
    check!(
        end_y > handle.canvas[1] && end_y < handle.canvas[1] + handle.canvas[3],
        "drag stays inside the canvas"
    );
    Ok(((handle.x, handle.y), (handle.x, end_y)))
}

async fn preview(tab: &Tab, initial: Rect, dy: f64) -> Result<Rect> {
    ready(
        tab,
        &format!("d.dragging && Math.abs(d.drag_offset - {dy}) <= 0.75"),
    )
    .await?;
    let held = bounds(tab, PASSIVE).await?;
    check!(
        near(held[1] - initial[1], dy),
        "captured drag moves the visible passive card by {dy} dp"
    );
    Ok(held)
}

async fn gestures(env: &Env, tab: &Tab) -> Result<Value> {
    step("M81","captured mouse and touch move the passive card, cancel safely and match the keyboard action");
    let original = cards(tab).await?;
    let count = original["moves"].as_u64().context("move count")?;
    ready(tab, "d.position === 0 && !d.dragging").await?;
    let baseline = bounds(tab, PASSIVE).await?;
    let primary = bounds(tab, PRIMARY).await?;
    check!(
        baseline[1] < primary[1],
        "position zero places the passive card first"
    );
    tab.eval(
        "(() => {
        window.cardPointerEvents = [];
        for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
            document.addEventListener(kind, e => {
                if (e.target?.id === 'surface') {
                    window.cardPointerEvents.push({kind,device:e.pointerType,trusted:e.isTrusted});
                }
            }, {capture:true,passive:true});
        }
    })()",
    )
    .await?;
    let (start, end) = drag_points(tab, 80.).await?;
    tab.mouse_move(start.0, start.1).await?;
    tab.mouse_down().await?;
    mouse(tab, "mouseMoved", end, 1).await?;
    let mouse_held = preview(tab, baseline, 80.).await?;
    Image::capture(env, tab, "mouse-drag").await?;
    tab.press("Escape").await?;
    ready(
        tab,
        &format!("d.position === 0 && !d.dragging && d.moves === {count}"),
    )
    .await?;
    mouse(tab, "mouseReleased", end, 0).await?;
    check_eq!(
        bounds(tab, PASSIVE).await?,
        baseline,
        "mouse Escape rolls back the visible card"
    );

    tab.mouse_move(start.0, start.1).await?;
    tab.mouse_down().await?;
    mouse(tab, "mouseMoved", end, 1).await?;
    preview(tab, baseline, 80.).await?;
    mouse(tab, "mouseReleased", end, 0).await?;
    ready(
        tab,
        &format!(
            "d.position === 1 && !d.dragging && d.moves === {}",
            count + 1
        ),
    )
    .await?;
    check!(
        bounds(tab, PASSIVE).await?[1] > bounds(tab, PRIMARY).await?[1],
        "committed move changes card order"
    );
    focus(tab, MOVE).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        &format!(
            "d.position === 0 && !d.dragging && d.moves === {}",
            count + 2
        ),
    )
    .await?;
    focused(tab, MOVE).await?;
    check_eq!(
        bounds(tab, PASSIVE).await?,
        baseline,
        "Move button returns the same card to position zero"
    );

    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    let (start, end) = drag_points(tab, 80.).await?;
    touch(tab, "touchStart", Some(start)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    let touch_held = preview(tab, baseline, 80.).await?;
    Image::capture(env, tab, "touch-drag").await?;
    touch(tab, "touchCancel", None).await?;
    ready(
        tab,
        &format!(
            "d.position === 0 && !d.dragging && d.moves === {}",
            count + 2
        ),
    )
    .await?;
    check_eq!(
        bounds(tab, PASSIVE).await?,
        baseline,
        "canceled touch rolls back the visible card"
    );
    touch(tab, "touchStart", Some(start)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    preview(tab, baseline, 80.).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        &format!(
            "d.position === 1 && !d.dragging && d.moves === {}",
            count + 3
        ),
    )
    .await?;
    let second = bounds(tab, PASSIVE).await?;
    let (start, end) = drag_points(tab, -80.).await?;
    touch(tab, "touchStart", Some(start)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    preview(tab, second, -80.).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        &format!(
            "d.position === 0 && !d.dragging && d.moves === {}",
            count + 4
        ),
    )
    .await?;
    check_eq!(
        bounds(tab, PASSIVE).await?,
        baseline,
        "reverse touch restores the original order"
    );
    let after = cards(tab).await?;
    for field in ["activations", "nested_activations"] {
        check_eq!(
            after[field],
            original[field],
            format!("card drag leaves {field} unchanged")
        );
    }
    let events = tab.eval("window.cardPointerEvents").await?;
    let recorded = events.as_array().context("Chrome pointer events")?;
    for device in ["mouse", "touch"] {
        for kind in ["pointerdown", "pointermove", "pointerup"]
            .into_iter()
            .chain((device == "touch").then_some("pointercancel"))
        {
            check!(
                recorded.iter().any(|event| event["kind"] == kind
                    && event["device"] == device
                    && event["trusted"] == true),
                "Chrome delivers trusted {device} {kind}"
            );
        }
    }
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    Ok(
        json!({"baseline":baseline,"mouse_held":mouse_held,"touch_held":touch_held,
        "moves":count+4,"events":events}),
    )
}

async fn wheel_at(tab: &Tab, x: f64, y: f64, dy: f64) -> Result<()> {
    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("device scale")?;
    tab.mouse_move(canvas_x + x, canvas_y + y).await?;
    tab.wheel(0., dy * scale).await?;
    tab.settle().await
}

async fn reveal(tab: &Tab, id: &str) -> Result<Rect> {
    let control = bounds(tab, id).await?;
    let clip = bounds(tab, SCROLL).await?;
    if !contains(clip, control) {
        let y = clip[1] + clip[3] / 2.;
        let dy = control[1] + control[3] / 2. - y;
        wheel_at(tab, clip[0] + clip[2] / 2., y, dy).await?;
    }
    let bounds = bounds(tab, id).await?;
    check!(contains(clip, bounds), "compact scroll reveals all of {id}");
    Ok(bounds)
}

async fn scroll_over_media(tab: &Tab, title: Rect) -> Result<Value> {
    let media = bounds(tab, PRIMARY_MEDIA).await?;
    let clip = bounds(tab, SCROLL).await?;
    let top = media[1].max(clip[1]);
    let bottom = (media[1] + media[3]).min(clip[1] + clip[3]);
    check!(
        bottom > top + 2.,
        "some card media is visible before wheeling"
    );
    let x = media[0] + media[2] / 2.;
    let y = (top + bottom) / 2.;
    wheel_at(tab, x, y, 24.).await?;
    let after = bounds(tab, PRIMARY_TITLE).await?;
    check!(
        after[1] < title[1] - 12.,
        "wheel over art scrolls the card list"
    );
    Ok(json!({"point":[x,y],"title_before":title,"title_after":after}))
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M82",
        "dark RTL cards at 320 dp keep 200% text and separate action targets reachable",
    );
    variant(tab, "outlined").await?;
    select(tab, "card-direction", "d.rtl").await?;
    select(tab, "card-large-text", "d.text_scale === 2").await?;
    click(tab, "card-theme").await?;
    until(tab, "s.dark").await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    resize(tab, 320, 580).await?;
    let mut title_layout = Vec::new();
    let mut media_wheel = Value::Null;
    for (card, title) in [(PRIMARY, PRIMARY_TITLE), (PASSIVE, PASSIVE_TITLE)] {
        let text = reveal(tab, title).await?;
        let bounds = bounds(tab, card).await?;
        let export = dom(tab).await?;
        let node = find_node(&export, title)?;
        let label = node["text"].as_str().context("compact title text")?;
        let line = node["style"]["line_height"]
            .as_f64()
            .context("compact title line height")?
            * 2.;
        check!(
            contains(bounds, text)
                && text[3] >= line - 0.75
                && bounds[0] >= 0.
                && bounds[0] + bounds[2] <= 320.75,
            "{card} fits its scaled Chinese title without horizontal clipping"
        );
        check!(
            label.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}')),
            "card retains its Chinese title"
        );
        Image::capture(env, tab, &format!("compact-dark-{card}")).await?;
        title_layout.push(json!({"card":bounds,"text":text,"title":label}));
        if card == PRIMARY {
            media_wheel = scroll_over_media(tab, text).await?;
        }
    }
    let seen = reveal(tab, NESTED).await?;
    check!(
        contains(bounds(tab, PASSIVE).await?, seen),
        "compact nested action stays inside the passive card"
    );
    let before = cards(tab).await?["nested_activations"]
        .as_u64()
        .context("nested count")?;
    focus(tab, NESTED).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.nested_activations === {}", before + 1)).await?;
    focused(tab, NESTED).await?;
    Image::capture(env, tab, "compact-dark-action").await?;
    Ok(json!({"titles":title_layout,"media_wheel":media_wheel,
        "action":seen,"nested_activations":before+1}))
}

async fn open_gallery(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 760).await?;
    click(tab, "section-3").await?;
    until(tab, "s.section === 3").await?;
    click(tab, "card-demo-show").await?;
    ready(tab, "d.open").await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    open_gallery(tab).await?;
    let appearance = appearance(env, tab).await?;
    let whole = whole_card(env, tab).await?;
    let passive = passive_card(env, tab).await?;
    let gestures = gestures(env, tab).await?;
    tab.goto(url).await?;
    open_gallery(tab).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "card page errors: {errors:?}");
    env.write_json(
        "browser-material-card.json",
        &json!({
            "browser":chrome.version,"appearance":appearance,"whole":whole,
            "passive":passive,"gestures":gestures,"compact":compact,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-card-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-card-failure.png"))
            .await;
    }
    result
}
