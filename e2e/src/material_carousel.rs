//! Material carousel through Chromium WebGPU and trusted keyboard, wheel and pointer input.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

const TRACK: &str = "carousel-track";
const PREV: &str = "carousel-prev";
const NEXT: &str = "carousel-next";
const SHOW_ALL: &str = "carousel-show-all";
const LIST_BACK: &str = "carousel-show-all-back";
const COUNT: usize = 6;
type Rect = [f64; 4];

fn card(index: usize) -> String {
    format!("carousel-card-{index}")
}

async fn carousel(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["carousel"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.carousel)")).await?;
    tab.settle().await
}

async fn selected(tab: &Tab, index: usize) -> Result<()> {
    ready(
        tab,
        &format!("d.active === {index} && !d.dragging && !d.settling"),
    )
    .await
}

async fn choice(tab: &Tab, kind: &str) -> Result<()> {
    click(tab, &format!("carousel-{kind}")).await?;
    ready(tab, &format!("d.variant === {} && !d.settling", js(kind))).await
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

async fn current(tab: &Tab) -> Result<usize> {
    Ok(carousel(tab).await?["active"]
        .as_u64()
        .context("active item")? as usize)
}

async fn offset(tab: &Tab) -> Result<f64> {
    carousel(tab).await?["offset"]
        .as_f64()
        .context("carousel offset")
}

async fn changes(tab: &Tab) -> Result<u64> {
    carousel(tab).await?["activations"]
        .as_u64()
        .context("card activation count")
}

async fn start_of_track(tab: &Tab) -> Result<()> {
    let active = current(tab).await?;
    focus(tab, &card(active)).await?;
    tab.press("Home").await?;
    selected(tab, 0).await?;
    focused(tab, &card(0)).await
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const viewport = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return viewport?.width === {width} && viewport?.height === {height}
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
        .context("four carousel bounds")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn intersect([x, y, w, h]: Rect, [cx, cy, cw, ch]: Rect) -> Rect {
    let left = x.max(cx);
    let top = y.max(cy);
    [
        left,
        top,
        ((x + w).min(cx + cw) - left).max(0.),
        ((y + h).min(cy + ch) - top).max(0.),
    ]
}

fn center(rect: Rect, axis: usize) -> f64 {
    rect[axis] + rect[axis + 2] / 2.
}

async fn visible_card(tab: &Tab, index: usize) -> Result<Rect> {
    let rect = intersect(bounds(tab, TRACK).await?, bounds(tab, &card(index)).await?);
    check!(
        rect[2] > 0. && rect[3] > 0.,
        "card {index} is visible inside the track"
    );
    Ok(rect)
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
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
        let path = env.artifact(&format!("material-carousel-{label}.png"));
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

    fn sample(&self, x: f64, y: f64) -> Result<[u8; 3]> {
        let x = ((x + self.canvas[0]) * self.scale).floor() as usize;
        let y = ((y + self.canvas[1]) * self.scale).floor() as usize;
        if x >= self.frame.width as usize || y >= self.frame.height as usize {
            bail!("carousel probe is outside the screenshot");
        }
        let offset = y * self.frame.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }

    fn card_fill(&self, rect: Rect) -> Result<[u8; 3]> {
        self.sample(center(rect, 0), rect[1] + rect[3] * 0.24)
    }
}

fn color_distance(a: [u8; 3], b: [u8; 3]) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn item_views(data: &Value, track: Rect) -> Result<Vec<(usize, Rect, Rect)>> {
    let items = data["items"].as_array().context("carousel items array")?;
    check_eq!(items.len(), COUNT, "six stable fixture items");
    let mut visible = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if item["visible"] != true {
            continue;
        }
        let bounds: Rect = serde_json::from_value(item["bounds"].clone()).context("card bounds")?;
        let clipped = intersect(track, bounds);
        check!(
            clipped[2] > 0. && clipped[3] > 0.,
            "visible card {index} intersects the track"
        );
        visible.push((index, bounds, clipped));
    }
    Ok(visible)
}

async fn layouts(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M73",
        "six carousel layouts paint visible cards and constrain them to the track",
    );
    let mut layouts = Vec::new();
    for kind in [
        "multi-browse",
        "uncontained",
        "mixed",
        "hero-start",
        "hero-center",
        "full-screen",
    ] {
        choice(tab, kind).await?;
        start_of_track(tab).await?;
        if kind == "hero-center" {
            tab.press("ArrowRight").await?;
            selected(tab, 1).await?;
            focused(tab, &card(1)).await?;
        }
        let track = bounds(tab, TRACK).await?;
        let data = carousel(tab).await?;
        let visible = item_views(&data, track)?;
        let active = current(tab).await?;
        let primary = visible
            .iter()
            .find(|(id, _, _)| *id == active)
            .context("active card has visible geometry")?
            .2;
        let export = dom(tab).await?;
        check_eq!(
            find_node(&export, TRACK)?["style"]["clip"],
            json!(true),
            "carousel track clips paint"
        );

        match kind {
            "multi-browse" | "mixed" => {
                check!(
                    visible.len() >= 2
                        && visible
                            .iter()
                            .any(|(_, _, rect)| (rect[2] - primary[2]).abs() > 4.),
                    "{kind} shows different card widths"
                );
            }
            "uncontained" => {
                check!(visible.len() >= 2, "uncontained shows multiple cards");
                let whole = visible
                    .iter()
                    .filter(|(_, _, slot)| {
                        slot[0] > track[0] + 0.75 && slot[0] + slot[2] < track[0] + track[2] - 0.75
                    })
                    .collect::<Vec<_>>();
                check!(
                    whole.len() >= 2 && whole.iter().all(|(_, _, slot)| near(slot[2], primary[2])),
                    "uncontained uses equal full item widths beside its clipped peek"
                );
            }
            "hero-start" | "hero-center" => {
                check!(
                    visible
                        .iter()
                        .any(|(id, _, rect)| *id != active && rect[2] > 2. && rect[2] < primary[2]),
                    "{kind} shows a smaller preview next to the hero"
                );
                if kind == "hero-center" {
                    check!(
                        near(center(primary, 0), center(track, 0)),
                        "middle hero centers in the track"
                    );
                } else {
                    check!(
                        primary[0] - track[0] <= 32.,
                        "start hero sits at the logical start"
                    );
                }
            }
            "full-screen" => {
                check!(
                    near(primary[2], track[2]) && near(primary[3], track[3]),
                    "full-screen fits one complete card on its vertical page"
                );
                check_eq!(
                    visible.len(),
                    1,
                    "only one full-screen card is visible at a settled page"
                );
            }
            _ => bail!("unknown carousel fixture {kind}"),
        }

        let image = Image::capture(env, tab, &format!("layout-{kind}")).await?;
        let paint = image.card_fill(primary)?;
        let background = image.sample(center(track, 0), track[1] - 3.)?;
        check!(
            color_distance(paint, background) > 24,
            "visible {kind} card paints inside its track"
        );
        if kind != "full-screen" {
            let samples: BTreeSet<_> = visible
                .iter()
                .filter(|(_, _, part)| part[2] > 12.)
                .map(|(_, _, part)| image.card_fill(*part))
                .collect::<Result<_>>()?;
            check!(
                samples.len() >= 2,
                "{kind} shows distinct fitted card paint"
            );
            if kind == "uncontained" {
                let (_, _, clipped) = visible
                    .iter()
                    .find(|(_, _, part)| {
                        near(part[0] + part[2], track[0] + track[2])
                            && part[2] > 12.
                            && part[2] < primary[2] - 4.
                    })
                    .context("uncontained carousel exposes a clipped peek at the right edge")?;
                let edge = track[0] + track[2];
                let y = clipped[1] + clipped[3] * 0.24;
                let before = image.sample(edge - 3., y)?;
                let after = image.sample(edge - 3., track[1] - 3.)?;
                check!(
                    color_distance(before, after) > 24,
                    "WebGPU paints the clipped peek up to the right edge"
                );
            }
            if kind == "multi-browse" {
                let (_, _, clipped) = visible
                    .iter()
                    .max_by(|(_, _, a), (_, _, b)| (a[0] + a[2]).total_cmp(&(b[0] + b[2])))
                    .context("multi-browse has a trailing fitted card")?;
                let edge = clipped[0] + clipped[2];
                let y = clipped[1] + clipped[3] * 0.24;
                check!(
                    near(track[0] + track[2] - edge, 16.)
                        && color_distance(image.sample(edge - 3., y)?, image.sample(edge + 3., y)?)
                            > 24,
                    "multi-browse clips card paint before the 16 dp side space"
                );
            }
        }
        layouts.push(
            json!({"variant":kind,"track":track,"active":active,"items":visible,"paint":paint}),
        );
    }
    Ok(json!(layouts))
}

async fn wheel(tab: &Tab, index: usize, dx: f64, dy: f64) -> Result<()> {
    let visible = visible_card(tab, index).await?;
    let canvas = tab.canvas_box().await?;
    tab.mouse_move(
        canvas[0] + center(visible, 0),
        canvas[1] + center(visible, 1),
    )
    .await?;
    // CDP uses physical wheel units under the emulated device scale.
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("device scale")?;
    tab.wheel(dx * scale, dy * scale).await?;
    tab.settle().await
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
        const start = Math.ceil(performance.now());
        window.carouselTestElapsed = 0;
        Object.defineProperty(performance, 'now', {
            configurable:true, value: () => start + window.carouselTestElapsed,
        });
    })()",
    )
    .await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.carouselTestElapsed = {ms}"))
        .await?;
    tab.settle().await
}

async fn resume_clock(tab: &Tab) -> Result<()> {
    // Resume real time at the reached timestamp so theme fades stay monotonic.
    tab.eval(
        "(() => {
        const reached = performance.now();
        delete performance.now;
        const now = performance.now.bind(performance);
        const offset = Math.max(0, reached - now());
        Object.defineProperty(performance, 'now', {
            configurable:true, value: () => now() + offset,
        });
    })()",
    )
    .await?;
    Ok(())
}

async fn wheel_journey(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M74",
        "wheel moves cards continuously, snaps only fitted variants, and clamps",
    );
    choice(tab, "uncontained").await?;
    start_of_track(tab).await?;
    tab.eval(
        "(() => {
        window.carouselWheelEvents = [];
        document.addEventListener('wheel', e => {
            if (e.target?.id === 'surface') {
                window.carouselWheelEvents.push({x:e.deltaX,y:e.deltaY,trusted:e.isTrusted});
            }
        }, {capture:true,passive:true});
    })()",
    )
    .await?;
    let base = offset(tab).await?;
    let interior = bounds(tab, &card(1)).await?;
    let track = bounds(tab, TRACK).await?;
    let before = changes(tab).await?;
    let increments = [24., 38.];
    let mut samples = Vec::new();
    let mut sum = 0.;
    for dx in increments {
        wheel(tab, 0, dx, 0.).await?;
        sum += dx;
        ready(tab, &format!("Math.abs(d.offset - {}) <= 0.75", base + sum)).await?;
        let after = bounds(tab, &card(1)).await?;
        check!(
            near(interior[0] - after[0], sum)
                && near(interior[2], after[2])
                && bounds(tab, TRACK).await? == track,
            "wheel moves one continuous track without resizing cards"
        );
        samples.push(json!({"offset":offset(tab).await?,"interior":after}));
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    check!(
        near(offset(tab).await?, base + sum),
        "uncontained keeps its free-wheel position after idle"
    );
    Image::capture(env, tab, "uncontained-wheel").await?;
    wheel(tab, 0, -900., 0.).await?;
    ready(tab, &format!("Math.abs(d.offset - {base}) <= 0.75")).await?;
    wheel(tab, 0, 9000., 0.).await?;
    ready(tab, &format!("d.offset > {}", base + sum)).await?;
    let end = offset(tab).await?;
    wheel(tab, COUNT - 1, 120., 0.).await?;
    check!(
        near(offset(tab).await?, end),
        "further wheel at the last card clamps the offset"
    );
    check_eq!(
        changes(tab).await?,
        before,
        "wheel does not activate a card"
    );

    choice(tab, "hero-start").await?;
    start_of_track(tab).await?;
    click(tab, NEXT).await?;
    selected(tab, 1).await?;
    let snap = offset(tab).await?;
    click(tab, PREV).await?;
    selected(tab, 0).await?;
    let origin = offset(tab).await?;
    check!(snap > origin + 20., "hero has a distinct next offset");
    let delta = (snap - origin) * 0.72;
    pin_clock(tab).await?;
    wheel(tab, 0, delta, 0.).await?;
    let held = offset(tab).await?;
    check!(
        held > origin + 2. && held < snap - 2.,
        "hero wheel paints an intermediate position before settling"
    );
    Image::capture(env, tab, "hero-wheel-intermediate").await?;
    advance(tab, 100).await?;
    advance(tab, 500).await?;
    selected(tab, 1).await?;
    check!(
        near(offset(tab).await?, snap),
        "hero wheel settles to the same target as Next"
    );
    let hero = offset(tab).await?;
    resume_clock(tab).await?;

    choice(tab, "full-screen").await?;
    start_of_track(tab).await?;
    let height = bounds(tab, TRACK).await?[3];
    wheel(tab, 0, 50., 0.).await?;
    selected(tab, 0).await?;
    let vertical_start = offset(tab).await?;
    wheel(tab, 0, 0., height * 0.72).await?;
    selected(tab, 1).await?;
    let vertical_end = offset(tab).await?;
    check!(
        vertical_end > vertical_start + 2.,
        "full-screen follows vertical wheel only"
    );
    let events = tab.eval("window.carouselWheelEvents").await?;
    let recorded = events.as_array().context("Chrome wheel events")?;
    check!(
        recorded
            .iter()
            .any(|e| e["trusted"] == true && e["x"].as_f64().is_some_and(|x| x > 0.))
            && recorded
                .iter()
                .any(|e| e["trusted"] == true && e["y"].as_f64().is_some_and(|y| y > 0.)),
        "Chrome sends trusted horizontal and vertical wheel events"
    );
    Ok(
        json!({"uncontained":samples,"uncontained_max_offset":end,"hero_held":held,
        "hero_settled":hero,"vertical":[vertical_start,vertical_end],"events":events}),
    )
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M75",
        "keyboard moves card focus; explicit Enter and Show All select once",
    );
    choice(tab, "hero-start").await?;
    start_of_track(tab).await?;
    let before = changes(tab).await?;
    tab.press("ArrowRight").await?;
    selected(tab, 1).await?;
    focused(tab, &card(1)).await?;
    tab.press("End").await?;
    selected(tab, COUNT - 1).await?;
    focused(tab, &card(COUNT - 1)).await?;
    check_eq!(
        changes(tab).await?,
        before,
        "arrow and End move focus without selecting"
    );
    tab.press("ArrowRight").await?;
    selected(tab, COUNT - 1).await?;
    focused(tab, &card(COUNT - 1)).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.activations === {}", before + 1)).await?;
    tab.press("Home").await?;
    selected(tab, 0).await?;
    focused(tab, &card(0)).await?;
    tab.press("ArrowLeft").await?;
    selected(tab, 0).await?;
    focused(tab, &card(0)).await?;

    focus(tab, SHOW_ALL).await?;
    tab.press("Enter").await?;
    ready(tab, "d.show_all").await?;
    focused(tab, LIST_BACK).await?;
    Image::capture(env, tab, "show-all").await?;
    let export = dom(tab).await?;
    for id in 0..COUNT {
        find_node(&export, &format!("carousel-list-{id}"))?;
    }
    tab.press("Enter").await?;
    ready(tab, "!d.show_all").await?;
    focused(tab, SHOW_ALL).await?;
    tab.press("Enter").await?;
    ready(tab, "d.show_all").await?;
    focus(tab, "carousel-list-3").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        &format!(
            "!d.show_all && d.active === 3 && d.activations === {}",
            before + 2
        ),
    )
    .await?;
    focused(tab, SHOW_ALL).await?;
    Image::capture(env, tab, "selected-from-list").await?;

    choice(tab, "full-screen").await?;
    start_of_track(tab).await?;
    tab.press("ArrowDown").await?;
    selected(tab, 1).await?;
    focused(tab, &card(1)).await?;
    tab.press("ArrowUp").await?;
    selected(tab, 0).await?;
    focused(tab, &card(0)).await?;
    check_eq!(
        changes(tab).await?,
        before + 2,
        "vertical keyboard navigation does not activate"
    );
    Ok(json!({"last":COUNT-1,"list_selection":3,"focus_return":SHOW_ALL,"activations":before+2}))
}

fn trusted_events_script() -> &'static str {
    "(() => {
        window.carouselInputEvents = [];
        for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
            document.addEventListener(kind, e => {
                if (e.target?.id === 'surface') {
                    window.carouselInputEvents.push({kind,device:e.pointerType,trusted:e.isTrusted});
                }
            }, {capture:true,passive:true});
        }
    })()"
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
        json!({"type":kind,"x":x,"y":y,"button":"left","buttons":buttons,"clickCount":1}),
    )
    .await?;
    tab.settle().await
}

async fn drag_points(tab: &Tab, index: usize, negative: bool) -> Result<((f64, f64), (f64, f64))> {
    let track = bounds(tab, TRACK).await?;
    let slot = visible_card(tab, index).await?;
    let canvas = tab.canvas_box().await?;
    let origin_x = if negative {
        slot[0] + slot[2] - 12.
    } else {
        slot[0] + 12.
    };
    let span = if negative {
        origin_x - track[0] - 10.
    } else {
        track[0] + track[2] - origin_x - 10.
    };
    let distance = (slot[2] * 0.76).min(span);
    check!(
        distance >= 32.,
        "drag crosses a useful distance inside the carousel"
    );
    let y = canvas[1] + center(slot, 1);
    let start = (canvas[0] + origin_x, y);
    let end = (start.0 + if negative { -distance } else { distance }, y);
    Ok((start, end))
}

async fn gestures(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M76",
        "trusted mouse and touch drag without card activation; canceled touch rolls back",
    );
    choice(tab, "uncontained").await?;
    start_of_track(tab).await?;
    let baseline = offset(tab).await?;
    let action_count = changes(tab).await?;
    tab.eval(trusted_events_script()).await?;
    let (start, end) = drag_points(tab, 0, true).await?;
    tab.mouse_move(start.0, start.1).await?;
    tab.mouse_down().await?;
    let halfway = ((start.0 + end.0) / 2., start.1);
    mouse(tab, "mouseMoved", halfway, 1).await?;
    ready(tab, "d.dragging").await?;
    let first = offset(tab).await?;
    mouse(tab, "mouseMoved", end, 1).await?;
    let last = offset(tab).await?;
    check!(
        last > first
            && near(first - baseline, (start.0 - end.0) / 2.)
            && near(last - baseline, start.0 - end.0),
        "held mouse follows both horizontal positions without a page jump"
    );
    Image::capture(env, tab, "mouse-drag-held").await?;
    mouse(tab, "mouseReleased", end, 0).await?;
    ready(tab, "!d.dragging && !d.settling").await?;
    check!(
        offset(tab).await? > baseline + 30.,
        "mouse release keeps the carousel moved"
    );

    start_of_track(tab).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    let (start, end) = drag_points(tab, 0, true).await?;
    touch(tab, "touchStart", Some(start)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    ready(tab, "d.dragging").await?;
    let touch_held = offset(tab).await?;
    check!(
        near(touch_held - baseline, start.0 - end.0),
        "trusted touch follows the held pointer"
    );
    Image::capture(env, tab, "touch-drag-held").await?;
    touch(tab, "touchCancel", None).await?;
    selected(tab, 0).await?;
    check!(
        near(offset(tab).await?, baseline),
        "touch cancellation restores the original offset"
    );
    touch(tab, "touchStart", Some(start)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    ready(tab, "d.dragging").await?;
    touch(tab, "touchEnd", None).await?;
    ready(tab, "!d.dragging && !d.settling").await?;
    check!(
        offset(tab).await? > baseline + 30.,
        "touch release keeps the carousel moved"
    );
    check_eq!(
        changes(tab).await?,
        action_count,
        "neither drag nor canceled touch activates a card"
    );
    let events = tab.eval("window.carouselInputEvents").await?;
    let observed = events.as_array().context("trusted Chrome event log")?;
    for device in ["mouse", "touch"] {
        for kind in ["pointerdown", "pointermove", "pointerup"]
            .into_iter()
            .chain((device == "touch").then_some("pointercancel"))
        {
            check!(
                observed
                    .iter()
                    .any(|e| e["kind"] == kind && e["device"] == device && e["trusted"] == true),
                "Chrome delivered trusted {device} {kind}"
            );
        }
    }
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    Ok(json!({"mouse":[first,last],"touch_held":touch_held,"events":events}))
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M77",
        "320 dp dark RTL stays readable at 200% and honors Reduce Motion",
    );
    choice(tab, "hero-start").await?;
    click(tab, "carousel-direction").await?;
    ready(tab, "d.rtl").await?;
    click(tab, "carousel-large-text").await?;
    ready(tab, "d.text_scale === 2").await?;
    click(tab, "carousel-theme").await?;
    until(tab, "s.dark").await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    resize(tab, 320, 580).await?;
    start_of_track(tab).await?;
    let scroll = bounds(tab, "carousel-demo-scroll").await?;
    let mut chips = Vec::new();
    for kind in [
        "multi-browse",
        "uncontained",
        "mixed",
        "hero-start",
        "hero-center",
        "full-screen",
    ] {
        let chip = bounds(tab, &format!("carousel-{kind}")).await?;
        check!(
            chip.into_iter()
                .zip(intersect(scroll, chip))
                .all(|(actual, visible)| near(actual, visible)),
            "{kind} chip is fully visible at 320 dp and 200% text"
        );
        chips.push(chip);
    }
    let track = bounds(tab, TRACK).await?;
    let first = visible_card(tab, 0).await?;
    check!(
        first[0] + first[2] >= track[0] + track[2] - 24.,
        "RTL starts at the physical right edge"
    );
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    tab.wait_for("matchMedia('(prefers-reduced-motion: reduce)').matches")
        .await?;
    pin_clock(tab).await?;
    let before = offset(tab).await?;
    let actions = changes(tab).await?;
    tab.press("ArrowLeft").await?;
    selected(tab, 1).await?;
    focused(tab, &card(1)).await?;
    ready(tab, "d.reduced_motion").await?;
    let after = offset(tab).await?;
    check!(
        after > before,
        "physical Left advances logical offset even with a frozen clock"
    );
    check_eq!(
        changes(tab).await?,
        actions,
        "RTL keyboard navigation only moves focus"
    );
    let second = visible_card(tab, 1).await?;
    let image = Image::capture(env, tab, "compact-dark-rtl").await?;
    check!(
        color_distance(
            image.card_fill(second)?,
            image.sample(center(track, 0), track[1] - 3.)?
        ) > 24,
        "compact focused card paints at the settled anchor"
    );
    let export = dom(tab).await?;
    let label = &find_node(&export, &card(1))?["name"];
    let label = label.as_str().context("compact card accessible name")?;
    check!(
        label.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}')),
        "compact card keeps its Chinese name"
    );
    tab.press("ArrowRight").await?;
    selected(tab, 0).await?;
    focused(tab, &card(0)).await?;
    check!(
        near(offset(tab).await?, before),
        "physical Right returns to the RTL start anchor"
    );
    let delta = (after - before) * 0.36;
    wheel(tab, 0, -delta, 0.).await?;
    let one = offset(tab).await?;
    wheel(tab, 0, -delta, 0.).await?;
    let two = offset(tab).await?;
    check!(
        near(one, before + delta) && near(two, before + 2. * delta),
        "negative X wheel moves continuously toward the next RTL item"
    );
    advance(tab, 89).await?;
    check!(
        near(offset(tab).await?, two),
        "Reduce Motion preserves the idle wheel debounce"
    );
    advance(tab, 90).await?;
    selected(tab, 1).await?;
    check!(
        near(offset(tab).await?, after) && changes(tab).await? == actions,
        "after idle, the RTL wheel settles immediately without activating a card"
    );
    resume_clock(tab).await?;
    Ok(
        json!({"track":track,"first":first,"second":second,"offsets":[before,after],
        "wheel_offsets":[one,two,offset(tab).await?],"chips":chips,"label":label}),
    )
}

async fn open_gallery(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 760).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, "carousel-demo-show").await?;
    ready(tab, "d.open").await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    open_gallery(tab).await?;
    let layouts = layouts(env, tab).await?;
    let wheel = wheel_journey(env, tab).await?;
    let keyboard = keyboard(env, tab).await?;
    let drag = gestures(env, tab).await?;
    // Measure the compact entry layout before a selection adds a status row.
    tab.goto(url).await?;
    open_gallery(tab).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "carousel page errors: {errors:?}");
    env.write_json(
        "browser-material-carousel.json",
        &json!({
            "browser":chrome.version,"layouts":layouts,"wheel":wheel,
            "keyboard":keyboard,"drag":drag,"compact":compact,"errors":errors
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
        let _ = env.write_json("material-carousel-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-carousel-failure.png"))
            .await;
    }
    result
}
