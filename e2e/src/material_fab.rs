//! Expressive icon FAB in headless Chrome WebGPU.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, time::Duration};

const NAME: &str = "新增專案筆記";
const ACTION: &str = "fab-demo-action";
const FACE: &str = "fab-demo-action-face";
const ICON: &str = "fab-demo-action-icon";
const RING: &str = "fab-demo-action-focus-ring";
const STATE_LAYER: &str = "fab-demo-action-state-layer";
const RIPPLE: &str = "fab-demo-action-ripple";
const TIP: &str = "fab-demo-tooltip";
const PAGE: &str = "fab-demo-page";
const ANCHOR: &str = "fab-demo-anchor";
const SCROLL: &str = "fab-demo-scroll";
const VISIBILITY: &str = "fab-demo-toggle-visibility";
const SIZES: [(&str, f64, f64, f64); 3] = [
    ("regular", 56., 16., 24.),
    ("medium", 80., 20., 28.),
    ("large", 96., 28., 36.),
];
const PALETTES: [&str; 6] = [
    "tonal-primary",
    "tonal-secondary",
    "tonal-tertiary",
    "primary",
    "secondary",
    "tertiary",
];
type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["expressive_fab"].clone())
}

async fn ready(tab: &Tab, expression: &str) -> Result<()> {
    until(tab, &format!("(d => {expression})(s.expressive_fab)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, control: &str, condition: &str) -> Result<()> {
    click(tab, &format!("fab-demo-{control}")).await?;
    ready(tab, condition).await
}

async fn choose(tab: &Tab, control: &str, field: &str, value: &str) -> Result<()> {
    select(
        tab,
        &format!("{control}-{value}"),
        &format!("d.{field} === {}", js(value)),
    )
    .await
}

async fn viewport(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{ const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
                   return v?.width === {width} && v?.height === {height}; }})()"
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
        .context("four expressive FAB bounds")
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn contains([x, y, w, h]: Rect, [a, b, c, d]: Rect) -> bool {
    a >= x - 0.75 && b >= y - 0.75 && a + c <= x + w + 0.75 && b + d <= y + h + 0.75
}

fn same_rect(a: Rect, b: Rect) -> bool {
    a.into_iter().zip(b).all(|(a, b)| near(a, b))
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
    let target = projected(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    focused(tab, id).await
}

async fn theme(tab: &Tab, dark: bool) -> Result<()> {
    if (state(tab).await?["dark"] == true) != dark {
        click(tab, "fab-demo-theme").await?;
    }
    until(tab, &format!("s.dark === {dark}")).await?;
    tab.settle().await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-fab-{name}.png")))
        .await
}

struct Image {
    pixels: Vec<u8>,
    frame: png::OutputInfo,
    channels: usize,
    scale: f64,
    canvas: Rect,
}

impl Image {
    async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Self> {
        capture(env, tab, name).await?;
        let path = env.artifact(&format!("material-fab-{name}.png"));
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
        if x >= self.frame.width as usize || y >= self.frame.height as usize {
            bail!("FAB sample outside the screenshot");
        }
        let index = y * self.frame.line_size + x * self.channels;
        Ok(self.pixels[index..index + 3].try_into()?)
    }

    fn count_ink(&self, [x, y, w, h]: Rect, ink: Rgb) -> Result<usize> {
        let mut count = 0;
        for row in 0..h.floor() as u32 {
            for col in 0..w.floor() as u32 {
                count += usize::from(
                    distance(
                        self.sample(x + f64::from(col) + 0.5, y + f64::from(row) + 0.5)?,
                        ink,
                    ) <= 24,
                );
            }
        }
        Ok(count)
    }
}

fn distance(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn rgb(value: &Value) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(value.clone()).context("RGBA token")?;
    Ok([r, g, b])
}

fn blend(under: Rgb, over: Rgb, fraction: f64) -> Rgb {
    std::array::from_fn(|i| {
        (f64::from(under[i]) * (1. - fraction) + f64::from(over[i]) * fraction).round() as u8
    })
}

fn ambient_blur(export: &Value) -> Result<f64> {
    find_node(export, FACE)?["box_shadows"][0]["blur"]
        .as_f64()
        .context("ambient FAB shadow blur")
}

async fn position(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    tab.mouse_move(p.x, p.y).await?;
    tab.settle().await
}

async fn clear_pointer(tab: &Tab) -> Result<()> {
    tab.mouse_move(4., 52.).await?;
    tab.settle().await
}

async fn hover_fab(tab: &Tab) -> Result<()> {
    position(tab, ACTION).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.hovered === {}",
        js(ACTION)
    ))
    .await?;
    tab.settle().await
}

async fn setup_section(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    viewport(tab, 1100, 880).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await
}

async fn open_fixture(tab: &Tab) -> Result<()> {
    click(tab, "fab-demo-show").await?;
    ready(tab, "d.open && d.visible").await
}

async fn geometry_and_paint(env: &Env, tab: &Tab) -> Result<Value> {
    step("M100","three square FAB sizes retain legacy sizing and paint six expressive palettes in both themes");
    setup_section(tab).await?;
    let legacy = bounds(tab, "fab-small").await?;
    let export = dom(tab).await?;
    let child = find_node(&export, "fab-small")?["first_child"]
        .as_u64()
        .context("legacy FAB face")?;
    let legacy_style = &export["dom"]["nodes"][child as usize]["style"];
    check!(
        near(legacy[2], 48.)
            && near(legacy[3], 48.)
            && legacy_style["width"]["Px"].as_f64() == Some(40.)
            && legacy_style["height"]["Px"].as_f64() == Some(40.),
        "legacy Small still wraps a 40 dp face in a 48 dp target"
    );
    open_fixture(tab).await?;
    ready(
        tab,
        "d.size === 'regular' && d.palette === 'tonal-primary' && !d.lowered",
    )
    .await?;
    let mut sizes = Vec::new();
    for (name, size, radius, glyph) in SIZES {
        choose(tab, "size", "size", name).await?;
        let face = bounds(tab, FACE).await?;
        let hit = bounds(tab, ACTION).await?;
        let icon = bounds(tab, ICON).await?;
        let export = dom(tab).await?;
        let corners = &find_node(&export, FACE)?["style"]["corners"];
        check!(
            near(face[2], size)
                && near(face[3], size)
                && contains(hit, face)
                && hit[2] >= 48.
                && hit[3] >= 48.,
            "{name} supplies a square face and a full target"
        );
        check!(
            near(icon[2], glyph)
                && near(icon[3], glyph)
                && contains(face, icon)
                && near(icon[0] + icon[2] / 2., face[0] + face[2] / 2.)
                && near(icon[1] + icon[3] / 2., face[1] + face[3] / 2.),
            "{name} has a centered {glyph} dp glyph"
        );
        for side in ["top_left", "top_right", "bottom_left", "bottom_right"] {
            check!(
                corners[side].as_f64().is_some_and(|r| near(r, radius)),
                "{name} radius is {radius} dp"
            );
        }
        sizes.push(json!({"name":name,"face":face,"icon":icon,"radius":radius,"hit":hit}));
    }
    choose(tab, "size", "size", "regular").await?;
    let mut palettes = Vec::new();
    for dark in [false, true] {
        theme(tab, dark).await?;
        let mut fills = Vec::new();
        for palette in PALETTES {
            choose(tab, "color", "palette", palette).await?;
            clear_pointer(tab).await?;
            tokio::time::sleep(Duration::from_millis(180)).await;
            let face = bounds(tab, FACE).await?;
            let icon = bounds(tab, ICON).await?;
            let export = dom(tab).await?;
            let fill = rgb(&find_node(&export, FACE)?["style"]["background"])?;
            let ink = rgb(&find_node(&export, ICON)?["fill_contour"]["color"])?;
            let image = Image::capture(
                env,
                tab,
                &format!("{palette}-{}", if dark { "dark" } else { "light" }),
            )
            .await?;
            let pixel = image.sample(face[0] + face[2] / 2., face[1] + 6.)?;
            let corner = image.sample(face[0] + 2., face[1] + 2.)?;
            let interior = image.sample(face[0] + 8., face[1] + 8.)?;
            let glyph_ink = image.count_ink(icon, ink)?;
            check!(
                distance(pixel, fill) <= 9
                    && distance(interior, fill) <= 9
                    && distance(corner, fill) > 14
                    && glyph_ink > 12,
                "{palette} paints both roles and its rounded contour"
            );
            check!(
                !fills.contains(&pixel),
                "all six fills are distinct for this theme"
            );
            fills.push(pixel);
            palettes.push(json!({"palette":palette,"dark":dark,"fill":pixel,"ink":ink,"ink_pixels":glyph_ink,"corner":corner}));
        }
    }
    theme(tab, false).await?;
    choose(tab, "color", "palette", "tonal-primary").await?;
    clear_pointer(tab).await?;
    let rest = ambient_blur(&dom(tab).await?)?;
    hover_fab(tab).await?;
    let hover = ambient_blur(&dom(tab).await?)?;
    select(tab, "lowered", "d.lowered").await?;
    clear_pointer(tab).await?;
    let lowered = ambient_blur(&dom(tab).await?)?;
    hover_fab(tab).await?;
    let raised_lowered = ambient_blur(&dom(tab).await?)?;
    check!(
        [rest, hover, lowered, raised_lowered]
            .into_iter()
            .zip([8., 10., 3., 6.])
            .all(|(a, b)| near(a, b)),
        "standard uses elevations 3 and 4 and lowered uses elevations 1 and 2"
    );
    capture(env, tab, "lowered-hover").await?;
    select(tab, "lowered", "!d.lowered").await?;
    clear_pointer(tab).await?;
    Ok(
        json!({"legacy_target":legacy,"sizes":sizes,"palettes":palettes,
        "ambient_blur":[rest,hover,lowered,raised_lowered]}),
    )
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    // Chrome still delivers real input and frames. Control only elapsed time.
    tab.eval("(() => { const start = Math.ceil(performance.now()); window.fabTime = 0;
        Object.defineProperty(performance, 'now', {configurable:true,value:() => start + window.fabTime}); })()").await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.fabTime = {ms}")).await?;
    tab.settle().await
}

async fn reduced_motion(tab: &Tab, reduced: bool) -> Result<()> {
    let value = if reduced { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":value}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {value})"))
    ))
    .await?;
    ready(tab, &format!("d.reduced_motion === {reduced}")).await
}

fn layer_alpha(export: &Value) -> Result<f64> {
    find_node(export, STATE_LAYER)?["style"]["background"][3]
        .as_f64()
        .context("FAB state layer alpha")
}

async fn ripple(env: &Env, tab: &Tab) -> Result<Value> {
    step("M101", "trusted off-center hold paints a clipped radial ripple and Reduce Motion paints a static press");
    clear_pointer(tab).await?;
    reduced_motion(tab, false).await?;
    let p = point(tab, ACTION).await?;
    let [x, y, w, h] = bounds(tab, FACE).await?;
    let source = dom(tab).await?;
    let fill = rgb(&find_node(&source, FACE)?["style"]["background"])?;
    let ink = rgb(&find_node(&source, ICON)?["fill_contour"]["color"])?;
    let tap = [12., 15.];
    let near_spot = [x + 13., y + 16.];
    let far_spot = [x + w - 9., y + h - 9.];
    let corner = [x + 2., y + 2.];
    let blank = [x + w / 2., y + 6.];
    let rest = Image::capture(env, tab, "ripple-rest").await?;
    pin_clock(tab).await?;
    tab.mouse_move(p.canvas[0] + x + tap[0], p.canvas[1] + y + tap[1])
        .await?;
    tab.settle().await?;
    let hovered = Image::capture(env, tab, "ripple-hover").await?;
    check!(
        near(layer_alpha(&dom(tab).await?)?, 20.)
            && distance(hovered.sample(blank[0], blank[1])?, blend(fill, ink, 0.08)) <= 10,
        "hover paints the 8 percent state layer"
    );
    tab.mouse_down().await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        js(ACTION)
    ))
    .await?;
    ready(tab, "d.ripple !== null && d.ripple.progress === 0").await?;
    advance(tab, 46).await?;
    ready(
        tab,
        "d.ripple?.progress > 0 && d.ripple?.progress < 1 && d.actions === 0",
    )
    .await?;
    let early_state = data(tab).await?["ripple"].clone();
    let center: [f64; 2] = serde_json::from_value(early_state["center"].clone())?;
    check!(
        (center[0] - tap[0] / w).abs() < 0.025 && (center[1] - tap[1] / h).abs() < 0.025,
        "ripple origin follows the pointer in the face"
    );
    let early = Image::capture(env, tab, "ripple-early").await?;
    let near_early = early.sample(near_spot[0], near_spot[1])?;
    let far_early = early.sample(far_spot[0], far_spot[1])?;
    check!(
        distance(near_early, rest.sample(near_spot[0], near_spot[1])?) >= 9
            && distance(far_early, fill) <= 7,
        "the initial pulse darkens the press point before the distant corner"
    );
    advance(tab, 225).await?;
    ready(
        tab,
        "d.ripple?.progress >= 0.999 && d.ripple?.opacity >= 0.999 && d.actions === 0",
    )
    .await?;
    let expanded = Image::capture(env, tab, "ripple-clipped").await?;
    let far_expanded = expanded.sample(far_spot[0], far_spot[1])?;
    let corner_early = early.sample(corner[0], corner[1])?;
    let corner_expanded = expanded.sample(corner[0], corner[1])?;
    check!(
        distance(far_early, far_expanded) >= 14
            && distance(far_expanded, blend(fill, ink, 0.10)) <= 10,
        "full ripple reaches the distant corner at 10 percent opacity"
    );
    check!(
        distance(corner_early, corner_expanded) <= 3 && distance(corner_expanded, fill) > 16,
        "the painted ripple stays inside the rounded contour"
    );
    let contour = dom(tab).await?;
    check!(
        find_node(&contour, RIPPLE)?["fill_contour"]["color"][3].as_u64() == Some(26),
        "full ripple emits the expected alpha"
    );
    tab.mouse_up().await?;
    ready(tab, "d.actions === 1").await?;
    advance(tab, 301).await?;
    ready(tab, "d.ripple?.opacity > 0 && d.ripple?.opacity < 1").await?;
    let released = data(tab).await?["ripple"].clone();
    advance(tab, 376).await?;
    ready(tab, "d.ripple === null").await?;
    reduced_motion(tab, true).await?;
    tab.mouse_down().await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        js(ACTION)
    ))
    .await?;
    let instant = Image::capture(env, tab, "ripple-reduced-press").await?;
    let reduced_before = instant.sample(far_spot[0], far_spot[1])?;
    check!(
        data(tab).await?["ripple"].is_null()
            && near(layer_alpha(&dom(tab).await?)?, 26.)
            && distance(reduced_before, blend(fill, ink, 0.10)) <= 10,
        "Reduce Motion paints a static 10 percent press immediately"
    );
    advance(tab, 601).await?;
    let later = Image::capture(env, tab, "ripple-reduced-held").await?;
    check!(
        later.sample(far_spot[0], far_spot[1])? == reduced_before
            && data(tab).await?["ripple"].is_null(),
        "the reduced-motion hold has no traveling ripple"
    );
    tab.mouse_up().await?;
    ready(tab, "d.actions === 2 && d.ripple === null").await?;
    tab.eval("delete performance.now").await?;
    reduced_motion(tab, false).await?;
    Ok(
        json!({"on_press":tap,"early":early_state,"early_near_pixel":near_early,"far_pixels":[far_early,far_expanded],
        "outer_corner_pixels":[corner_early,corner_expanded],"release":released,"reduced_press_pixel":reduced_before,"actions":2}),
    )
}

async fn named_ax(tab: &Tab, role: &str) -> Result<Vec<Value>> {
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    Ok(tree["nodes"]
        .as_array()
        .context("Chrome AX nodes")?
        .iter()
        .filter(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"] == NAME
        })
        .cloned()
        .collect())
}

async fn tab_to_action(tab: &Tab) -> Result<String> {
    let preceding = tab.eval(&format!("(() => {{ const stops = [...document.querySelectorAll('#surface-accessibility [data-surface-node]')];
        const index = stops.indexOf({}); return index > 0 ? stops[index-1].dataset.surfaceId : null; }})()",projected(ACTION))).await?;
    let preceding = preceding
        .as_str()
        .context("Tab stop before expressive FAB")?;
    focus(tab, preceding).await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    Ok(preceding.to_owned())
}

async fn touch(tab: &Tab, kind: &str, location: Option<(f64, f64)>) -> Result<()> {
    let points = location.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn touch_enabled(tab: &Tab, enabled: bool) -> Result<()> {
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":enabled,"maxTouchPoints":1}),
    )
    .await?;
    Ok(())
}

async fn ring(env: &Env, tab: &Tab, name: &str) -> Result<Value> {
    tab.settle().await?;
    let export = dom(tab).await?;
    let face = bounds(tab, FACE).await?;
    let ring = bounds(tab, RING).await?;
    let style = &find_node(&export, RING)?["style"];
    let ring_color = rgb(&style["border_color"])?;
    let [x, y, w, h] = face;
    check!(
        same_rect(ring, [x - 5., y - 5., w + 10., h + 10.])
            && style["border_width"].as_f64().is_some_and(|v| near(v, 3.)),
        "keyboard focus has a 3 dp exterior ring with a 2 dp gap"
    );
    check!(
        near(layer_alpha(&export)?, 26.),
        "keyboard focus paints the 10 percent state layer"
    );
    let image = Image::capture(env, tab, name).await?;
    let top = image.sample(x + w / 2., y - 3.5)?;
    let gap = image.sample(x + w / 2., y - 1.)?;
    check!(
        distance(top, ring_color) <= 9 && distance(gap, ring_color) > 14,
        "PNG paints the exterior ring and leaves its gap clear"
    );
    Ok(json!({"bounds":ring,"stroke":top,"gap":gap}))
}

async fn action(env: &Env, tab: &Tab) -> Result<Value> {
    step("M102", "one named button accepts trusted keyboard and short touch activation, then becomes inert when disabled");
    clear_pointer(tab).await?;
    let proxy = projected(ACTION);
    tab.wait_for(&format!(
        "{proxy}.getAttribute('aria-label') === {} && {proxy}.dataset.surfaceAction === 'press'",
        js(NAME)
    ))
    .await?;
    for id in [FACE, ICON, STATE_LAYER, RIPPLE, RING] {
        tab.wait_for(&format!("{} === null", projected(id))).await?;
    }
    let ax = named_ax(tab, "button").await?;
    check_eq!(
        ax.len(),
        1,
        "one rendered Chrome AX button has the FAB name"
    );
    let previous = tab_to_action(tab).await?;
    tab.press("Escape").await?;
    focused(tab, ACTION).await?;
    let focus_ring = ring(env, tab, "keyboard-focus").await?;
    tab.press("Enter").await?;
    ready(tab, "d.actions === 1").await?;
    focused(tab, ACTION).await?;
    tab.press("Space").await?;
    ready(tab, "d.actions === 2").await?;
    focused(tab, ACTION).await?;
    tab.eval(&format!("{proxy}.click()")).await?;
    ready(tab, "d.actions === 3").await?;
    focus(tab, "fab-demo-back").await?;
    touch_enabled(tab, true).await?;
    tab.eval("document.addEventListener('pointerdown', event => {
       if (event.target?.id === 'surface' && event.pointerType === 'touch') window.fabTrustedTouch = event.isTrusted;
    }, {capture:true,passive:true})").await?;
    let p = point(tab, ACTION).await?;
    touch(tab, "touchStart", Some((p.x, p.y))).await?;
    touch(tab, "touchEnd", None).await?;
    ready(tab, "d.actions === 4").await?;
    check_eq!(
        tab.eval("window.fabTrustedTouch").await?,
        json!(true),
        "Chrome delivers the actual touch tap"
    );
    touch_enabled(tab, false).await?;
    select(tab, "disable", "d.disabled").await?;
    clear_pointer(tab).await?;
    tab.wait_for(&format!("{proxy} === null || ({proxy}.getAttribute('aria-disabled') === 'true' && !{proxy}.dataset.surfaceAction)")).await?;
    let disabled_shadow = find_node(&dom(tab).await?, FACE)?["box_shadows"].clone();
    check!(
        disabled_shadow.as_array().is_none_or(|layers| layers
            .iter()
            .all(|layer| layer["color"][3].as_u64() == Some(0))),
        "disabled FAB has no elevation"
    );
    click(tab, ACTION).await?;
    ready(
        tab,
        "d.actions === 4 && d.ripple === null && !d.tooltip_shown",
    )
    .await?;
    focus(tab, &previous).await?;
    tab.press("Tab").await?;
    focused(tab, "fab-demo-back").await?;
    capture(env, tab, "disabled").await?;
    Ok(
        json!({"ax":ax,"focus":focus_ring,"previous_tab":previous,"disabled_next":"fab-demo-back","actions":4,"trusted_touch":true}),
    )
}

async fn tip_hidden(tab: &Tab) -> Result<()> {
    ready(tab, "!d.tooltip_shown").await?;
    tab.wait_for(&format!("{} === null", projected(TIP))).await
}

async fn tip_shown(tab: &Tab) -> Result<Rect> {
    ready(tab, "d.tooltip_shown").await?;
    let proxy = projected(TIP);
    tab.wait_for(&format!("{proxy}?.getAttribute('role') === 'tooltip' && ({proxy}.getAttribute('aria-label') || {proxy}.textContent) === {}",js(NAME))).await?;
    bounds(tab, TIP).await
}

async fn tooltip(env: &Env, tab: &Tab) -> Result<Value> {
    step("M103", "shared tooltip responds to delayed hover, keyboard focus, and trusted long touch without activating the FAB");
    clear_pointer(tab).await?;
    focus(tab, "fab-demo-back").await?;
    pin_clock(tab).await?;
    hover_fab(tab).await?;
    advance(tab, 499).await?;
    tip_hidden(tab).await?;
    advance(tab, 500).await?;
    let hover = tip_shown(tab).await?;
    until(tab, "s.focused === 'fab-demo-back'").await?;
    let tip_ax = named_ax(tab, "tooltip").await?;
    check_eq!(tip_ax.len(), 1, "the tooltip has one named Chrome AX role");
    let button_ax = named_ax(tab, "button").await?;
    check_eq!(
        button_ax.len(),
        1,
        "the FAB keeps its accessible name while the tooltip shows"
    );
    let proxy = projected(ACTION);
    check_eq!(
        tab.eval(&format!("{proxy}.getAttribute('aria-describedby')"))
            .await?,
        Value::Null,
        "the tooltip does not repeat the existing button name as a description"
    );
    let face = bounds(tab, FACE).await?;
    check!(
        hover[1] + hover[3] < face[1] && contains([0., 0., 1100., 880.], hover),
        "hover tooltip stays above its anchor and inside the viewport"
    );
    capture(env, tab, "tooltip-hover").await?;
    tab.press("Escape").await?;
    tip_hidden(tab).await?;
    clear_pointer(tab).await?;
    advance(tab, 1000).await?;
    let from = tab_to_action(tab).await?;
    let keyboard = tip_shown(tab).await?;
    let focus_occupied = bounds(tab, RING).await?;
    check!(
        near(hover[2], keyboard[2])
            && near(hover[3], keyboard[3])
            && near(focus_occupied[1] - (keyboard[1] + keyboard[3]), 4.),
        "keyboard focus shows the tooltip immediately with a 4 dp gap above the ring"
    );
    focused(tab, ACTION).await?;
    tab.press("Escape").await?;
    tip_hidden(tab).await?;
    focused(tab, ACTION).await?;
    focus(tab, &from).await?;
    touch_enabled(tab, true).await?;
    let p = point(tab, ACTION).await?;
    advance(tab, 2000).await?;
    touch(tab, "touchStart", Some((p.x, p.y))).await?;
    advance(tab, 2499).await?;
    tip_hidden(tab).await?;
    advance(tab, 2500).await?;
    let held = tip_shown(tab).await?;
    capture(env, tab, "tooltip-touch").await?;
    touch(tab, "touchEnd", None).await?;
    ready(tab, "d.actions === 0 && d.tooltip_shown").await?;
    advance(tab, 4000).await?;
    tip_hidden(tab).await?;
    touch_enabled(tab, false).await?;
    tab.eval("delete performance.now").await?;
    Ok(
        json!({"hover_delay_ms":500,"hover":hover,"keyboard":keyboard,"touch":held,"ax":tip_ax,"actions":0}),
    )
}

async fn wheel(tab: &Tab, dy: f64) -> Result<()> {
    position(tab, SCROLL).await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("Chrome device scale")?;
    tab.wheel(0., dy * scale).await?;
    tab.settle().await
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step("M104", "compact dark RTL at 200 percent anchors the icon and focus ring through a real notes scroll");
    theme(tab, true).await?;
    select(tab, "rtl", "d.rtl").await?;
    select(tab, "large-text", "d.text_scale === 2").await?;
    choose(tab, "size", "size", "large").await?;
    viewport(tab, 320, 720).await?;
    clear_pointer(tab).await?;
    let page = bounds(tab, PAGE).await?;
    let anchor = bounds(tab, ANCHOR).await?;
    let [x, y, w, h] = bounds(tab, FACE).await?;
    let geometry = [x, y, w, h];
    check!(
        contains(page, anchor)
            && contains(anchor, geometry)
            && near(x, page[0] + 16.)
            && near(y + h, page[1] + page[3] - 16.)
            && near(w, 96.)
            && near(h, 96.),
        "compact RTL moves the full size Large FAB to the logical trailing corner"
    );
    let icon = bounds(tab, ICON).await?;
    check!(
        same_rect(icon, [x + 30., y + 30., 36., 36.]),
        "200 percent text does not inflate or clip the decorative glyph"
    );
    focus(tab, ACTION).await?;
    let tip = tip_shown(tab).await?;
    let focus_occupied = bounds(tab, RING).await?;
    check!(
        tip[0] >= page[0]
            && tip[0] + tip[2] <= page[0] + page[2]
            && near(focus_occupied[1] - (tip[1] + tip[3]), 4.),
        "compact 200 percent tooltip fits the viewport with a 4 dp gap above the ring"
    );
    let tip_text_id = format!("{TIP}-text");
    check!(
        contains(tip, bounds(tab, &tip_text_id).await?),
        "compact tooltip contains the entire text box"
    );
    let tip_text = find_node(&dom(tab).await?, &tip_text_id)?["text"].clone();
    check_eq!(
        tip_text,
        json!(NAME),
        "compact tooltip keeps the full Chinese label"
    );
    capture(env, tab, "compact-dark-rtl-tooltip").await?;
    tab.press("Escape").await?;
    tip_hidden(tab).await?;
    let before = ring(env, tab, "compact-dark-rtl-focus").await?;
    let first_before = bounds(tab, "fab-demo-note-1").await?;
    wheel(tab, 128.).await?;
    ready(tab, "d.scroll_y > 100").await?;
    let first_after = bounds(tab, "fab-demo-note-1").await?;
    let offset = data(tab).await?["scroll_y"]
        .as_f64()
        .context("note list offset")?;
    check!(
        near(first_before[1] - first_after[1], offset)
            && same_rect(geometry, bounds(tab, FACE).await?),
        "trusted wheel moves the actual notes while the FAB stays fixed"
    );
    focused(tab, ACTION).await?;
    let after = ring(env, tab, "compact-dark-rtl-scrolled").await?;
    check_eq!(
        before["bounds"],
        after["bounds"],
        "the visible focus ring stays fixed during notes scroll"
    );
    tab.press("Enter").await?;
    ready(
        tab,
        "d.actions === 1 && d.visible && d.rtl && d.text_scale === 2",
    )
    .await?;
    focused(tab, ACTION).await?;
    Ok(
        json!({"page":page,"face":geometry,"icon":icon,"focus":after,"tooltip":tip,"scroll_y":offset,"actions":1}),
    )
}

async fn visibility(env: &Env, tab: &Tab) -> Result<Value> {
    step("M105", "focus returns to the caller while the hidden FAB exits, then the restored action becomes operable again");
    reduced_motion(tab, false).await?;
    select(tab, "hide-on-press", "d.hide_on_press").await?;
    clear_pointer(tab).await?;
    let face = bounds(tab, FACE).await?;
    let rail = bounds(tab, ANCHOR).await?;
    focus(tab, ACTION).await?;
    pin_clock(tab).await?;
    tab.press("Enter").await?;
    ready(tab, "!d.visible && d.actions === 1").await?;
    focused(tab, VISIBILITY).await?;
    let proxy = projected(ACTION);
    tab.wait_for(&format!(
        "{proxy} === null || !{proxy}.dataset.surfaceAction"
    ))
    .await?;
    advance(tab, 65).await?;
    ready(tab,"d.visibility_scale > 0.2 && d.visibility_scale < 1 && d.visibility_opacity > 0 && d.visibility_opacity < 1").await?;
    let mid = data(tab).await?;
    let exit = bounds(tab, FACE).await?;
    check!(
        contains(face, exit)
            && near(exit[0] + exit[2] / 2., face[0] + face[2] / 2.)
            && near(exit[1] + exit[3] / 2., face[1] + face[3] / 2.)
            && same_rect(rail, bounds(tab, ANCHOR).await?),
        "the painted exit shrinks around its center within the same anchor"
    );
    capture(env, tab, "visibility-leaving").await?;
    click(tab, ACTION).await?;
    ready(tab, "d.actions === 1").await?;
    advance(tab, 1500).await?;
    ready(
        tab,
        "d.visibility_scale <= 0.201 && d.visibility_opacity === 0",
    )
    .await?;
    check_eq!(
        named_ax(tab, "button").await?.len(),
        0,
        "the hidden action is absent from Chrome accessibility"
    );
    capture(env, tab, "visibility-hidden").await?;
    click(tab, VISIBILITY).await?;
    ready(tab, "d.visible && d.actions === 1").await?;
    advance(tab, 1550).await?;
    ready(tab,"d.visibility_scale > 0.2 && d.visibility_scale < 1 && d.visibility_opacity > 0 && d.visibility_opacity < 1").await?;
    let entering = data(tab).await?;
    advance(tab, 3000).await?;
    ready(
        tab,
        "d.visibility_scale === 1 && d.visibility_opacity === 1",
    )
    .await?;
    check!(
        same_rect(face, bounds(tab, FACE).await?),
        "show restores the exact FAB face"
    );
    check_eq!(
        named_ax(tab, "button").await?.len(),
        1,
        "the named action returns to Chrome accessibility"
    );
    select(tab, "hide-on-press", "!d.hide_on_press").await?;
    tab_to_action(tab).await?;
    tab.press("Space").await?;
    ready(tab, "d.actions === 2 && d.visible").await?;
    focused(tab, ACTION).await?;
    tab.eval("delete performance.now").await?;
    Ok(
        json!({"focus_return":VISIBILITY,"face":face,"exit":exit,"leaving":[mid["visibility_scale"],mid["visibility_opacity"]],
        "entering":[entering["visibility_scale"],entering["visibility_opacity"]],"actions":2}),
    )
}

async fn reopen(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    setup_section(tab).await?;
    open_fixture(tab).await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    let geometry = geometry_and_paint(env, tab).await?;
    reopen(tab, url).await?;
    let ripple = ripple(env, tab).await?;
    reopen(tab, url).await?;
    let action = action(env, tab).await?;
    reopen(tab, url).await?;
    let tooltip = tooltip(env, tab).await?;
    reopen(tab, url).await?;
    let compact = compact(env, tab).await?;
    reopen(tab, url).await?;
    let visibility = visibility(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "Expressive FAB page errors: {errors:?}");
    env.write_json("browser-material-fab.json",&json!({"browser":chrome.version,
        "geometry":geometry,"ripple":ripple,"action":action,"tooltip":tooltip,"compact":compact,"visibility":visibility,"errors":errors}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(1100, 924, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _=env.write_json("material-fab-failure.json",&json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}));
        let _ = tab
            .screenshot(&env.artifact("material-fab-failure.png"))
            .await;
    }
    result
}
