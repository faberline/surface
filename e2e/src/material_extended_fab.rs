//! Extended floating action button through Chrome WebGPU and accessibility.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, time::Duration};

const ACTION: &str = "extended-fab-action";
const FACE: &str = "extended-fab-action-face";
const ICON: &str = "extended-fab-action-icon";
const LABEL: &str = "extended-fab-action-label";
const STATE_LAYER: &str = "extended-fab-action-state-layer";
const FOCUS_RING: &str = "extended-fab-action-focus-ring";
const NAME: &str = "新增專案筆記";
const PAGE: &str = "extended-fab-page";
const ANCHOR: &str = "extended-fab-anchor";
const SCROLL: &str = "extended-fab-scroll";
const VISIBILITY: &str = "extended-fab-toggle-visibility";
const SIZES: [(&str, f64, f64, f64); 3] = [
    ("small", 56., 16., 24.),
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
    Ok(state(tab).await?["extended_fab"].clone())
}

async fn ready(tab: &Tab, expression: &str) -> Result<()> {
    until(tab, &format!("(d => {expression})(s.extended_fab)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, control: &str, expression: &str) -> Result<()> {
    click(tab, &format!("extended-fab-{control}")).await?;
    ready(tab, expression).await
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
        .context("four Extended FAB bounds")
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
    let element = projected(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    focused(tab, id).await
}

async fn theme(tab: &Tab, dark: bool) -> Result<()> {
    if (state(tab).await?["dark"] == true) != dark {
        click(tab, "extended-fab-theme").await?;
    }
    until(tab, &format!("s.dark === {dark}")).await?;
    tab.settle().await
}

async fn save_png(env: &Env, tab: &Tab, label: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-extended-fab-{label}.png")))
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
    async fn capture(env: &Env, tab: &Tab, label: &str) -> Result<Self> {
        save_png(env, tab, label).await?;
        let path = env.artifact(&format!("material-extended-fab-{label}.png"));
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
            bail!("Extended FAB sample outside the screenshot");
        }
        let offset = y * self.frame.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }

    fn face_fill(&self, [x, y, width, _]: Rect) -> Result<Rgb> {
        self.sample(x + width / 2., y + 6.)
    }
}

fn distance(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn rgb(value: &Value) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(value.clone()).context("RGBA role")?;
    Ok([r, g, b])
}

fn blend(under: Rgb, overlay: Rgb, opacity: f64) -> Rgb {
    std::array::from_fn(|index| {
        (f64::from(under[index]) * (1. - opacity) + f64::from(overlay[index]) * opacity).round()
            as u8
    })
}

fn face_style(export: &Value) -> Result<&Value> {
    Ok(&find_node(export, FACE)?["style"])
}

fn corners(style: &Value) -> Result<[f64; 4]> {
    let values = &style["corners"];
    if values.is_null() {
        return Ok([style["radius"].as_f64().context("face radius")?; 4]);
    }
    let mut result = [0.; 4];
    for (radius, key) in
        result
            .iter_mut()
            .zip(["top_left", "top_right", "bottom_right", "bottom_left"])
    {
        *radius = values[key]
            .as_f64()
            .with_context(|| format!("face {key} radius"))?;
    }
    Ok(result)
}

fn ambient_blur(export: &Value) -> Result<f64> {
    find_node(export, FACE)?["box_shadows"][0]["blur"]
        .as_f64()
        .context("ambient Extended FAB shadow blur")
}

async fn check_anchor(tab: &Tab, face: Rect, rtl: bool, center: bool) -> Result<Rect> {
    let anchor = bounds(tab, ANCHOR).await?;
    let page = bounds(tab, PAGE).await?;
    let [x, y, w, h] = face;
    check!(
        contains(page, face) && near(y + h, page[1] + page[3] - 16.),
        "the face keeps its 16 dp bottom margin while its width changes"
    );
    let aligned = if center {
        near(x + w / 2., page[0] + page[2] / 2.)
    } else if rtl {
        near(x, page[0] + 16.)
    } else {
        near(x + w, page[0] + page[2] - 16.)
    };
    check!(
        aligned,
        "the physical face honors its logical horizontal anchor"
    );
    Ok(anchor)
}

async fn dimensions_and_palettes(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M95",
        "three Extended FAB sizes and six Material palettes paint their contours in both themes",
    );
    ready(tab, "d.size === 'small' && d.palette === 'tonal-primary' && d.expanded && d.expansion >= 0.999 && d.visibility_scale >= 0.999").await?;
    let mut sizes = Vec::new();
    for (label, height, radius, icon_size) in SIZES {
        choose(tab, "size", "size", label).await?;
        let face = bounds(tab, FACE).await?;
        let icon = bounds(tab, ICON).await?;
        let text = bounds(tab, LABEL).await?;
        let hit = bounds(tab, ACTION).await?;
        let export = dom(tab).await?;
        check!(
            near(face[3], height)
                && near(icon[2], icon_size)
                && near(icon[3], icon_size)
                && face[2] > face[3]
                && contains(hit, face)
                && contains(face, icon)
                && contains(face, text),
            "expanded {label} has the expected height, icon size and complete hit target"
        );
        check!(
            corners(face_style(&export)?)?
                .into_iter()
                .all(|corner| near(corner, radius)),
            "expanded {label} uses its Material corner radius"
        );
        let anchor = check_anchor(tab, face, false, false).await?;
        select(
            tab,
            "toggle-expand",
            "!d.requested_expanded && d.expansion <= 0.001",
        )
        .await?;
        let collapsed = bounds(tab, FACE).await?;
        check!(
            near(collapsed[2], height) && near(collapsed[3], height),
            "collapsed {label} is square at its Material size"
        );
        check_anchor(tab, collapsed, false, false).await?;
        select(
            tab,
            "toggle-expand",
            "d.requested_expanded && d.expansion >= 0.999",
        )
        .await?;
        sizes.push(json!({"size":label,"expanded":face,"collapsed":collapsed,"icon":icon,"text":text,"anchor":anchor}));
    }
    choose(tab, "size", "size", "small").await?;
    select(tab, "center", "d.center").await?;
    check_anchor(tab, bounds(tab, FACE).await?, false, true).await?;
    select(tab, "center", "!d.center").await?;

    let mut palettes = Vec::new();
    for dark in [false, true] {
        theme(tab, dark).await?;
        let mut fills = Vec::new();
        for palette in PALETTES {
            choose(tab, "color", "palette", palette).await?;
            tab.mouse_move(4., 58.).await?;
            tokio::time::sleep(Duration::from_millis(180)).await;
            let face = bounds(tab, FACE).await?;
            let [x, y, _, _] = face;
            let export = dom(tab).await?;
            let style = face_style(&export)?;
            let token = rgb(&style["background"])?;
            let ink = rgb(&find_node(&export, LABEL)?["style"]["foreground"])?;
            let icon_ink = rgb(&find_node(&export, ICON)?["fill_contour"]["color"])?;
            check_eq!(
                ink,
                icon_ink,
                "label and glyph share the palette's foreground role"
            );
            let image = Image::capture(
                env,
                tab,
                &format!("{palette}-{}", if dark { "dark" } else { "light" }),
            )
            .await?;
            let fill = image.face_fill(face)?;
            let corner = image.sample(x + 2., y + 2.)?;
            let inside = image.sample(x + 8., y + 8.)?;
            check!(
                distance(fill, token) <= 9
                    && distance(inside, token) <= 9
                    && distance(corner, token) > 14,
                "{palette} paints its fill inside the actual rounded contour"
            );
            check!(
                !fills.contains(&fill),
                "all six paints are distinct in this theme"
            );
            fills.push(fill);
            palettes
                .push(json!({"palette":palette,"dark":dark,"fill":fill,"ink":ink,"corner":corner}));
        }
    }
    theme(tab, false).await?;
    choose(tab, "color", "palette", "tonal-primary").await?;
    tab.mouse_move(4., 58.).await?;
    tab.settle().await?;
    let standard = ambient_blur(&dom(tab).await?)?;
    select(tab, "lowered", "d.lowered").await?;
    tab.mouse_move(4., 58.).await?;
    tab.settle().await?;
    let lowered = ambient_blur(&dom(tab).await?)?;
    let p = point(tab, ACTION).await?;
    tab.mouse_move(p.x, p.y).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.hovered === {}",
        js(ACTION)
    ))
    .await?;
    tab.settle().await?;
    let lowered_hover = ambient_blur(&dom(tab).await?)?;
    check!(
        near(standard, 8.) && near(lowered, 3.) && near(lowered_hover, 6.),
        "standard rest and lowered rest/hover use Material elevations 3, 1 and 2"
    );
    save_png(env, tab, "lowered-hover").await?;
    select(tab, "lowered", "!d.lowered").await?;
    tab.mouse_move(4., 58.).await?;
    tab.settle().await?;
    Ok(
        json!({"sizes":sizes,"palettes":palettes,"ambient_blur_standard_and_lowered":[standard,lowered,lowered_hover]}),
    )
}

async fn action_semantics(tab: &Tab) -> Result<Value> {
    let action = projected(ACTION);
    tab.wait_for(&format!(
        "{action}?.getAttribute('role') === 'button' && {action}.getAttribute('aria-label') === {} && {action}.hasAttribute('data-surface-node') && {action}.getAttribute('data-surface-action') === 'press'",
        js(NAME)
    )).await?;
    for id in [FACE, ICON, LABEL] {
        tab.wait_for(&format!("{} === null", projected(id))).await?;
    }
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let matches: Vec<_> = tree["nodes"]
        .as_array()
        .context("Chrome AX nodes")?
        .iter()
        .filter(|node| {
            node["ignored"] == false
                && node["role"]["value"] == "button"
                && node["name"]["value"] == NAME
        })
        .collect();
    check_eq!(
        matches.len(),
        1,
        "Chrome exposes exactly one named Extended FAB button"
    );
    Ok(matches[0].clone())
}

async fn tab_to_action(tab: &Tab) -> Result<String> {
    let candidate = tab.eval(&format!(
        "(() => {{ const tabStops = [...document.querySelectorAll('#surface-accessibility [data-surface-node]')];
            const index = tabStops.indexOf({});
            return index > 0 ? tabStops[index - 1].dataset.surfaceId : null; }})()",
        projected(ACTION)
    )).await?;
    let previous = candidate
        .as_str()
        .context("visible Tab stop before the Extended FAB")?;
    focus(tab, previous).await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    Ok(previous.to_string())
}

fn layer_opacity(export: &Value) -> Result<f64> {
    find_node(export, STATE_LAYER)?["style"]["background"][3]
        .as_f64()
        .map(|alpha| alpha / 255.)
        .context("state layer opacity")
}

async fn touch_tap(tab: &Tab, id: &str) -> Result<()> {
    let location = point(tab, id).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":location.x,"y":location.y,"id":1}]}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchEnd","touchPoints":[]}),
    )
    .await?;
    tab.settle().await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;
    Ok(())
}

async fn interaction(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M96",
        "real pointer and touch, keyboard and accessibility share one action with Material states",
    );
    let ax = action_semantics(tab).await?;
    let before = data(tab).await?["actions"]
        .as_u64()
        .context("action count")?;
    let face = bounds(tab, FACE).await?;
    let export = dom(tab).await?;
    let fill = rgb(&face_style(&export)?["background"])?;
    let ink = rgb(&find_node(&export, LABEL)?["style"]["foreground"])?;
    let rest = Image::capture(env, tab, "rest").await?;
    let target = point(tab, ACTION).await?;
    tab.mouse_move(target.x, target.y).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.hovered === {}",
        js(ACTION)
    ))
    .await?;
    tab.settle().await?;
    let hover = Image::capture(env, tab, "hover").await?;
    let hover_alpha = layer_opacity(&dom(tab).await?)?;
    let hover_fill = hover.face_fill(face)?;
    check!(
        distance(hover_fill, blend(fill, ink, 0.08)) <= 9 && (hover_alpha - 0.08).abs() <= 0.004,
        "the actual hover paint uses the 8% state layer"
    );
    let mut shadow = Vec::new();
    for gap in [4., 6., 8.] {
        let x = face[0] + face[2] / 2.;
        let y = face[1] + face[3] + gap;
        shadow.push((rest.sample(x, y)?, hover.sample(x, y)?));
    }
    check!(
        shadow
            .iter()
            .any(|&(rest, raised)| distance(rest, raised) > 3
                && rest
                    .iter()
                    .zip(raised)
                    .all(|(&before, after)| after <= before)),
        "the actual hover shadow darkens the area beyond rest elevation"
    );
    tab.mouse_down().await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        js(ACTION)
    ))
    .await?;
    ready(tab, &format!("d.actions === {before}")).await?;
    let pressed = Image::capture(env, tab, "pressed").await?;
    let pressed_alpha = layer_opacity(&dom(tab).await?)?;
    let pressed_fill = pressed.face_fill(face)?;
    check!(
        distance(pressed_fill, blend(fill, ink, 0.10)) <= 9
            && (pressed_alpha - 0.10).abs() <= 0.004,
        "the actual held press uses the 10% state layer and waits for release"
    );
    tab.mouse_up().await?;
    ready(tab, &format!("d.actions === {}", before + 1)).await?;
    tab.mouse_move(4., 58.).await?;

    let previous = tab_to_action(tab).await?;
    let ring = bounds(tab, FOCUS_RING).await?;
    let focused_export = dom(tab).await?;
    let focused_alpha = layer_opacity(&focused_export)?;
    let ring_style = &find_node(&focused_export, FOCUS_RING)?["style"];
    let ring_color = rgb(&ring_style["border_color"])?;
    check!(
        near(ring[0], face[0] - 5.)
            && near(ring[1], face[1] - 5.)
            && near(ring[2], face[2] + 10.)
            && near(ring[3], face[3] + 10.)
            && ring_style["border_width"] == 3.,
        "the keyboard focus ring has a 2 dp gap and 3 dp stroke"
    );
    let image = Image::capture(env, tab, "keyboard-focus").await?;
    let x = face[0] + face[2] / 2.;
    let ring_fill = image.sample(x, face[1] - 3.5)?;
    let gap_fill = image.sample(x, face[1] - 1.)?;
    let focus_fill = image.face_fill(face)?;
    check!(
        distance(ring_fill, ring_color) <= 12 && distance(gap_fill, ring_color) > 12,
        "actual paint distinguishes the external indicator from its gap"
    );
    check!(
        distance(focus_fill, blend(fill, ink, 0.10)) <= 9 && (focused_alpha - 0.10).abs() <= 0.004,
        "keyboard focus paints the 10% state layer"
    );

    for (index, key) in ["Enter", "Space"].into_iter().enumerate() {
        tab.press(key).await?;
        ready(tab, &format!("d.actions === {}", before + 2 + index as u64)).await?;
        focused(tab, ACTION).await?;
    }
    tab.eval(&format!("{}.click()", projected(ACTION))).await?;
    ready(tab, &format!("d.actions === {}", before + 4)).await?;
    tab.eval("(() => { window.fabTrustedTouch = false; document.addEventListener('pointerdown',
        e => { if (e.target?.id === 'surface' && e.pointerType === 'touch' && e.isTrusted) window.fabTrustedTouch = true; },
        {capture:true,passive:true}); })()").await?;
    touch_tap(tab, ACTION).await?;
    ready(tab, &format!("d.actions === {}", before + 5)).await?;
    tab.wait_for("window.fabTrustedTouch").await?;
    Ok(
        json!({"ax":ax,"tab_previous":previous,"actions":before+5,"hover_alpha":hover_alpha,
        "pressed_alpha":pressed_alpha,"focused_alpha":focused_alpha,"hover_fill":hover_fill,
        "pressed_fill":pressed_fill,"focused_fill":focus_fill,"shadow":shadow,"focus_ring":ring,"focus_ink":ring_fill}),
    )
}

async fn reduce_motion(tab: &Tab, reduced: bool) -> Result<()> {
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

async fn pin_clock(tab: &Tab) -> Result<()> {
    tab.eval("(() => { const start = performance.now(); window.extendedFabTime = 0;
        Object.defineProperty(performance,'now',{configurable:true,value:() => start + window.extendedFabTime}); })()").await?;
    Ok(())
}

async fn advance(tab: &Tab, time: u32) -> Result<()> {
    tab.eval(&format!("window.extendedFabTime = {time}"))
        .await?;
    tab.settle().await
}

async fn restore_clock(tab: &Tab) -> Result<()> {
    tab.eval("delete performance.now").await?;
    tab.settle().await
}

async fn wheel(tab: &Tab, dy: f64) -> Result<()> {
    let area = point(tab, SCROLL).await?;
    tab.mouse_move(area.x, area.y).await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("wheel scale")?;
    tab.wheel(0., dy * scale).await?;
    tab.settle().await
}

async fn expect_scroll(tab: &Tab, offset: f64) -> Result<()> {
    ready(tab, &format!("Math.abs(d.scroll_y - {offset}) <= 0.75")).await?;
    let export = dom(tab).await?;
    let actual = find_node(&export, SCROLL)?["style"]["scroll_y"]
        .as_f64()
        .context("body scroll offset")?;
    check!(
        near(actual, offset),
        "the body consumes the actual wheel travel"
    );
    Ok(())
}

async fn scroll_morph(env: &Env, tab: &Tab) -> Result<Value> {
    step("M97", "real wheel moves the body, morphing reverses smoothly, and Reduce Motion keeps one anchored action");
    reduce_motion(tab, false).await?;
    focus(tab, ACTION).await?;
    ready(
        tab,
        "d.requested_expanded && d.expansion >= 0.999 && d.label_opacity >= 0.999",
    )
    .await?;
    let expanded = bounds(tab, FACE).await?;
    let note_before = bounds(tab, "extended-fab-note-0").await?;
    expect_scroll(tab, 0.).await?;
    pin_clock(tab).await?;
    wheel(tab, 20.).await?;
    expect_scroll(tab, 20.).await?;
    ready(tab, "d.requested_expanded && d.expansion >= 0.999").await?;
    wheel(tab, 20.).await?;
    expect_scroll(tab, 40.).await?;
    ready(tab, "!d.requested_expanded && d.expansion >= 0.999").await?;
    check!(
        same_rect(expanded, bounds(tab, FACE).await?),
        "the morph begins at the same position and width"
    );
    check!(
        near(
            note_before[1] - bounds(tab, "extended-fab-note-0").await?[1],
            40.
        ),
        "painted note geometry moved by all 40 dp of scroll demand"
    );

    advance(tab, 80).await?;
    ready(
        tab,
        "d.expansion > 0.02 && d.expansion < 0.98 && d.label_opacity > 0 && d.label_opacity < 0.99",
    )
    .await?;
    let partial = bounds(tab, FACE).await?;
    let partial_state = data(tab).await?;
    check!(
        partial[2] < expanded[2] - 3. && partial[2] > 58.,
        "the intermediate width is painted between endpoints"
    );
    check_anchor(tab, partial, false, false).await?;
    focused(tab, ACTION).await?;
    action_semantics(tab).await?;
    save_png(env, tab, "wheel-morph-middle").await?;

    wheel(tab, -6.).await?;
    expect_scroll(tab, 34.).await?;
    ready(tab, "d.requested_expanded").await?;
    check!(
        same_rect(partial, bounds(tab, FACE).await?),
        "wheel reversal preserves the current anchored geometry at the same clock time"
    );
    advance(tab, 160).await?;
    let growing = bounds(tab, FACE).await?;
    check!(
        growing[2] > partial[2] + 1.0 && growing[2] < expanded[2],
        "the reversed morph grows toward its original width"
    );
    check_anchor(tab, growing, false, false).await?;
    advance(tab, 700).await?;
    ready(tab, "d.expansion >= 0.999 && d.label_opacity >= 0.999").await?;
    check!(
        same_rect(expanded, bounds(tab, FACE).await?),
        "the reversed morph settles at the initial face"
    );
    focused(tab, ACTION).await?;

    reduce_motion(tab, true).await?;
    wheel(tab, 60.).await?;
    expect_scroll(tab, 94.).await?;
    ready(
        tab,
        "!d.requested_expanded && d.expansion === 0 && d.label_opacity === 0",
    )
    .await?;
    let collapsed = bounds(tab, FACE).await?;
    check!(
        near(collapsed[2], 56.) && near(collapsed[3], 56.),
        "Reduce Motion reaches the compact square at the frozen frame time"
    );
    check_anchor(tab, collapsed, false, false).await?;
    focused(tab, ACTION).await?;
    action_semantics(tab).await?;
    let actions = data(tab).await?["actions"]
        .as_u64()
        .context("action count")?;
    tab.press("Space").await?;
    ready(tab, &format!("d.actions === {}", actions + 1)).await?;
    save_png(env, tab, "wheel-reduced-collapsed").await?;
    wheel(tab, -60.).await?;
    expect_scroll(tab, 34.).await?;
    ready(
        tab,
        "d.requested_expanded && d.expansion === 1 && d.label_opacity === 1",
    )
    .await?;
    check!(
        same_rect(expanded, bounds(tab, FACE).await?),
        "Reduce Motion restores the full face at the same frozen frame time"
    );
    restore_clock(tab).await?;
    reduce_motion(tab, false).await?;
    Ok(
        json!({"expanded":expanded,"partial":partial,"partial_state":partial_state,
        "reversing":growing,"collapsed":collapsed,"body_offsets":[0,20,40,34,94,34],"compact_keyboard_action":actions+1}),
    )
}

fn clearance([left, top, width, height]: Rect, radii: [f64; 4], x: f64, y: f64) -> f64 {
    let [l, t, r, b] = [x - left, y - top, left + width - x, top + height - y];
    let mut edge = l.min(t).min(r).min(b);
    for ((horizontal, vertical), radius) in [(l, t), (r, t), (r, b), (l, b)].into_iter().zip(radii)
    {
        let radius = radius.min(width.min(height) / 2.);
        if horizontal < radius && vertical < radius {
            edge = edge.min(radius - (radius - horizontal).hypot(radius - vertical));
        }
    }
    edge
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step("M98", "the 320 dp dark RTL Extended FAB mirrors its icon and paints all six 200% glyphs inside the rounded face");
    choose(tab, "size", "size", "large").await?;
    theme(tab, true).await?;
    select(tab, "rtl", "d.rtl").await?;
    select(tab, "large-text", "d.text_scale === 2").await?;
    viewport(tab, 320, 720).await?;
    let face = bounds(tab, FACE).await?;
    let label = bounds(tab, LABEL).await?;
    let icon = bounds(tab, ICON).await?;
    let hit = bounds(tab, ACTION).await?;
    check!(
        contains([0., 0., 320., 720.], hit)
            && contains(hit, face)
            && contains(face, label)
            && contains(face, icon)
            && hit[2] >= 48.
            && hit[3] >= 48.,
        "the compact face, icon and label fit inside a complete pointer target"
    );
    check!(
        icon[0] >= label[0] + label[2] + 1.
            && near(icon[1] + icon[3] / 2., label[1] + label[3] / 2.),
        "the leading icon moves to the right of the Chinese label in RTL"
    );
    check_anchor(tab, face, true, false).await?;
    let export = dom(tab).await?;
    let radii = corners(face_style(&export)?)?;
    let node = find_node(&export, LABEL)?;
    check_eq!(
        node["text"],
        json!(NAME),
        "the compact action keeps the complete Chinese label"
    );
    let label_style = &node["style"];
    let glyph_width = label_style["font_size"].as_f64().context("glyph width")? * 2.;
    let line_height = label_style["line_height"]
        .as_f64()
        .context("glyph line height")?
        * 2.;
    let columns = ((label[2] + 0.75) / glyph_width).floor() as usize;
    check!(
        columns == 3 && near(label[3], line_height * 2.),
        "all six glyphs fit on two complete lines of three"
    );
    let ink = rgb(&label_style["foreground"])?;
    let image = Image::capture(env, tab, "compact-dark-rtl-200").await?;
    let mut glyph_ink = [[0; 3]; 2];
    let mut minimum = f64::INFINITY;
    for (line, cells) in glyph_ink.iter_mut().enumerate() {
        for (column, count) in cells.iter_mut().enumerate() {
            let x0 = label[0] + glyph_width * column as f64;
            let y0 = label[1] + line_height * line as f64;
            for y in 0..line_height.floor() as u32 {
                for x in 0..glyph_width.floor() as u32 {
                    let x = x0 + f64::from(x) + 0.5;
                    let y = y0 + f64::from(y) + 0.5;
                    if distance(image.sample(x, y)?, ink) <= 24 {
                        *count += 1;
                        minimum = minimum.min(clearance(face, radii, x, y));
                    }
                }
            }
            check!(
                *count > 20,
                "compact line {}, column {} paints a full-scale CJK glyph",
                line + 1,
                column + 1
            );
        }
    }
    check!(
        minimum >= 0.75,
        "all compact glyph ink stays inside the rounded face, clearance {minimum:.2} dp"
    );
    action_semantics(tab).await?;
    let before = data(tab).await?["actions"]
        .as_u64()
        .context("action count")?;
    click(tab, ACTION).await?;
    ready(tab, &format!("d.actions === {}", before + 1)).await?;
    Ok(
        json!({"face":face,"label":label,"icon":icon,"hit":hit,"glyph_ink":glyph_ink,
        "minimum_ink_clearance":minimum,"actions":before+1}),
    )
}

async fn unavailable(tab: &Tab) -> Result<()> {
    let control = projected(ACTION);
    tab.wait_for(&format!("!{control} || ({control}.getAttribute('aria-disabled') === 'true' && !{control}.hasAttribute('data-surface-action'))")).await
}

async fn variants_and_visibility(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M99",
        "text-only stays legible, and disabled or exiting actions yield focus and block activation",
    );
    select(tab, "text-only", "d.text_only && d.expanded").await?;
    let text_face = bounds(tab, FACE).await?;
    select(tab, "toggle-expand", "!d.requested_expanded && d.expanded").await?;
    check!(
        same_rect(text_face, bounds(tab, FACE).await?)
            && contains(text_face, bounds(tab, LABEL).await?),
        "the collapsed request keeps text-only visible at full width"
    );
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(ICON)
    ))
    .await?;
    action_semantics(tab).await?;
    save_png(env, tab, "text-only").await?;
    select(tab, "toggle-expand", "d.requested_expanded").await?;
    select(tab, "text-only", "!d.text_only").await?;
    ready(tab, "d.expansion >= 0.999").await?;

    let actions = data(tab).await?["actions"]
        .as_u64()
        .context("initial action count")?;
    let preceding = tab_to_action(tab).await?;
    select(tab, "disable", "d.disabled").await?;
    unavailable(tab).await?;
    focus(tab, &preceding).await?;
    tab.press("Tab").await?;
    focused(tab, "extended-fab-back").await?;
    click(tab, ACTION).await?;
    ready(tab, &format!("d.disabled && d.actions === {actions}")).await?;
    save_png(env, tab, "disabled").await?;
    select(tab, "disable", "!d.disabled").await?;
    action_semantics(tab).await?;

    select(tab, "hide-on-press", "d.hide_on_press").await?;
    reduce_motion(tab, false).await?;
    focus(tab, ACTION).await?;
    let shown = bounds(tab, FACE).await?;
    pin_clock(tab).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("!d.visible && d.actions === {}", actions + 1)).await?;
    focused(tab, VISIBILITY).await?;
    unavailable(tab).await?;
    advance(tab, 100).await?;
    ready(tab,"d.visibility_scale > 0.4 && d.visibility_scale < 0.99 && d.visibility_opacity > 0 && d.visibility_opacity < 0.99").await?;
    let exiting = bounds(tab, FACE).await?;
    let exit_state = data(tab).await?;
    let scale = exit_state["visibility_scale"]
        .as_f64()
        .context("exit scale")?;
    check!(
        exiting[2] < shown[2] - 1.
            && exiting[3] < shown[3] - 1.
            && near(exiting[2], shown[2] * scale)
            && near(exiting[3], shown[3] * scale),
        "the exiting face visibly shrinks by the sampled motion scale"
    );
    let exit_slot = bounds(tab, ACTION).await?;
    check!(
        contains(exit_slot, exiting),
        "the exit paint stays within the anchored allocation"
    );
    check_anchor(tab, exit_slot, false, false).await?;
    save_png(env, tab, "visibility-exit").await?;
    click(tab, FACE).await?;
    ready(tab, &format!("!d.visible && d.actions === {}", actions + 1)).await?;
    advance(tab, 320).await?;
    ready(
        tab,
        "d.visibility_opacity === 0 && d.visibility_scale <= 0.401",
    )
    .await?;
    unavailable(tab).await?;

    click(tab, VISIBILITY).await?;
    ready(tab, "d.visible").await?;
    advance(tab, 550).await?;
    ready(tab,"d.visibility_scale > 0.4 && d.visibility_scale < 0.999 && d.visibility_opacity > 0 && d.visibility_opacity < 0.999").await?;
    let entering = bounds(tab, FACE).await?;
    let entry = data(tab).await?;
    save_png(env, tab, "visibility-entry").await?;
    advance(tab, 900).await?;
    ready(
        tab,
        "d.visibility_scale >= 0.999 && d.visibility_opacity >= 0.999",
    )
    .await?;
    check!(
        same_rect(shown, bounds(tab, FACE).await?),
        "the shown action returns to the same anchored face"
    );
    restore_clock(tab).await?;
    select(tab, "hide-on-press", "!d.hide_on_press").await?;
    action_semantics(tab).await?;
    tab_to_action(tab).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.visible && d.actions === {}", actions + 2)).await?;
    Ok(
        json!({"text_face":text_face,"shown":shown,"exit_face":exiting,"exit_state":exit_state,
        "entry_face":entering,"entry_state":entry,"hide_focus":VISIBILITY,"actions":actions+2}),
    )
}

async fn open(tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    viewport(tab, 1100, 880).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, "extended-fab-demo-show").await?;
    ready(tab, "d.open && d.visibility_scale >= 0.999").await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    open(tab).await?;
    let appearance = dimensions_and_palettes(env, tab).await?;
    let state = interaction(env, tab).await?;
    tab.goto(url).await?;
    open(tab).await?;
    let motion = scroll_morph(env, tab).await?;
    tab.goto(url).await?;
    open(tab).await?;
    let compact = compact(env, tab).await?;
    tab.goto(url).await?;
    open(tab).await?;
    let visibility = variants_and_visibility(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "Extended FAB page errors: {errors:?}");
    env.write_json(
        "browser-material-extended-fab.json",
        &json!({"browser":chrome.version,
        "appearance":appearance,"interaction":state,"movement":motion,"compact":compact,
        "visibility":visibility,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(1100, 924, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-extended-fab-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-extended-fab-failure.png"))
            .await;
    }
    result
}
