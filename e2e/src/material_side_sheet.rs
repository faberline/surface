//! Material side sheets through Chrome input, layout, AX and WebGPU paint.

use super::{click, point, state, until, Point};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

const PANEL: &str = "sheet-demo-panel";
const LAYOUT: &str = "sheet-demo-panel-layout";
const BODY: &str = "sheet-demo-panel-body";
const SCRIM: &str = "sheet-demo-panel-layer";
const CONTENT: &str = "sheet-demo-panel-content";
const HEADER: &str = "sheet-demo-panel-header";
const TITLE: &str = "sheet-demo-panel-title";
const CLOSE: &str = "sheet-demo-panel-close";
const OPEN: &str = "sheet-demo-open";
const MAIN: &str = "sheet-demo-main-action";
const ACTION: &str = "sheet-demo-action";
const DISABLED: &str = "sheet-demo-disabled";
const LONG: &str = "sheet-demo-long";
const LAST: &str = "sheet-demo-last";

async fn sheet(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["side_sheet"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.side_sheet)")).await?;
    tab.settle().await
}

async fn endpoint(tab: &Tab, open: bool) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.open === {open} && !d.settling && !d.dragging && d.reveal === {}",
            u8::from(open)
        ),
    )
    .await
}

async fn viewport(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const c = document.querySelector('canvas');
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return c.width === {backing_width} && c.height === {backing_height}
                && v?.width === {width} && v.height === {height};
        }})()",
        backing_width = width * 2,
        backing_height = height * 2,
    ))
    .await?;
    tab.settle().await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Point> {
    let p = point(tab, id).await?;
    let [x, y, width, height] = rect(&p)?;
    check!(
        width > 0.
            && height > 0.
            && x >= -0.75
            && y >= -0.75
            && x + width <= p.canvas[2] + 0.75
            && y + height <= p.canvas[3] + 0.75,
        "{id} fits the visible canvas: {:?} in {:?}",
        p.b,
        p.canvas
    );
    Ok(p)
}

async fn visible_click(tab: &Tab, id: &str) -> Result<()> {
    let p = bounds(tab, id).await?;
    tab.click(p.x, p.y).await
}

async fn select(tab: &Tab, control: &str, condition: &str) -> Result<()> {
    visible_click(tab, control).await?;
    ready(tab, condition).await
}

fn element(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn projected_focus(tab: &Tab, id: &str) -> Result<()> {
    focused(tab, id).await?;
    tab.wait_for(&format!("document.activeElement === {}", element(id)))
        .await
}

async fn focus_element(tab: &Tab, id: &str) -> Result<()> {
    let target = element(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    projected_focus(tab, id).await
}

async fn tab_to_last(tab: &Tab) -> Result<()> {
    focus_element(tab, CLOSE).await?;
    for id in [ACTION, LAST] {
        tab.press("Tab").await?;
        projected_focus(tab, id).await?;
    }
    Ok(())
}

async fn export(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

fn rect(p: &Point) -> Result<[f64; 4]> {
    p.b.as_slice().try_into().context("four control bounds")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn same_rect(a: [f64; 4], b: [f64; 4]) -> bool {
    a.into_iter().zip(b).all(|(a, b)| near(a, b))
}

fn in_rect(x: f64, y: f64, [left, top, width, height]: [f64; 4]) -> bool {
    x >= left && y >= top && x < left + width && y < top + height
}

fn next_count(before: &Value, field: &str) -> Result<u64> {
    Ok(before[field]
        .as_u64()
        .with_context(|| format!("missing {field}"))?
        + 1)
}

async fn geometry(tab: &Tab, detached: bool, right: bool) -> Result<[f64; 4]> {
    let layout = bounds(tab, LAYOUT).await?;
    let [x, y, width, height] = rect(&layout)?;
    let margin = if detached { 16. } else { 0. };
    let panel_width = 256_f64.min(width - 2. * margin);
    let expected = [
        if right {
            x + width - panel_width - margin
        } else {
            x + margin
        },
        y + margin,
        panel_width,
        height - 2. * margin,
    ];
    let actual = rect(&bounds(tab, PANEL).await?)?;
    check!(
        same_rect(actual, expected),
        "sheet uses the selected edge and inset: {actual:?}, expected {expected:?}"
    );
    Ok(actual)
}

struct Image {
    bytes: Vec<u8>,
    frame: png::OutputInfo,
    channels: usize,
    scale: f64,
}

impl Image {
    async fn capture(env: &Env, tab: &Tab, label: &str) -> Result<Self> {
        tab.settle().await?;
        let path = env.artifact(&format!("material-side-sheet-{label}.png"));
        tab.screenshot(&path).await?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![0; reader.output_buffer_size().context("PNG size")?];
        let frame = reader.next_frame(&mut bytes)?;
        let channels = match frame.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            original => bail!("unexpected Chrome PNG type {original:?}"),
        };
        let browser_width = tab.eval("innerWidth").await?;
        let scale = f64::from(frame.width) / browser_width.as_f64().context("page width")?;
        Ok(Self {
            bytes,
            frame,
            channels,
            scale,
        })
    }

    fn pixel(&self, x: f64, y: f64) -> Result<[u8; 3]> {
        let column = (x * self.scale).floor() as usize;
        let row = (y * self.scale).floor() as usize;
        if x < 0.
            || y < 0.
            || column >= self.frame.width as usize
            || row >= self.frame.height as usize
        {
            bail!("sample ({x}, {y}) is outside Chrome's screenshot");
        }
        let offset = row * self.frame.line_size + column * self.channels;
        Ok(self.bytes[offset..offset + 3].try_into()?)
    }
}

fn color(value: &Value) -> Result<[u8; 4]> {
    serde_json::from_value(value.clone()).context("RGBA color")
}

fn rgb([r, g, b, _]: [u8; 4]) -> [u8; 3] {
    [r, g, b]
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn blend(base: [u8; 3], layer: [u8; 4]) -> [u8; 3] {
    let opacity = f64::from(layer[3]) / 255.;
    std::array::from_fn(|i| {
        (f64::from(base[i]) * (1. - opacity) + f64::from(layer[i]) * opacity).round() as u8
    })
}

async fn surface_pixel(tab: &Tab, image: &Image, field: &str) -> Result<[u8; 3]> {
    let panel = point(tab, PANEL).await?;
    let [x, y, width, _] = rect(&panel)?;
    let painted = image.pixel(panel.canvas[0] + x + width / 2., panel.canvas[1] + y + 5.)?;
    let expected = rgb(color(&sheet(tab).await?[field])?);
    check!(
        distance(painted, expected) <= 8,
        "WebGPU paints the expected sheet surface: {painted:?}, expected {expected:?}"
    );
    Ok(painted)
}

async fn standard(env: &Env, tab: &Tab, mode: &str) -> Result<Value> {
    if mode == "overlay" {
        step(
            "M45",
            "standard overlay keeps body bounds and outside actions",
        );
    } else {
        step(
            "M46",
            "standard coplanar shares space on both logical edges",
        );
    }
    select(
        tab,
        &format!("sheet-demo-{mode}"),
        &format!("d.effective_presentation === {}", js(mode)),
    )
    .await?;
    let original = rect(&bounds(tab, BODY).await?)?;
    let bare = Image::capture(env, tab, &format!("{mode}-before")).await?;
    let mut cases = Vec::new();
    for rtl in [false, true] {
        if sheet(tab).await?["rtl"] != rtl {
            select(tab, "sheet-demo-rtl", &format!("d.rtl === {rtl}")).await?;
        }
        for detached in [false, true] {
            let style = if detached { "detached" } else { "docked" };
            select(
                tab,
                &format!("sheet-demo-{style}"),
                &format!("d.style === {}", js(style)),
            )
            .await?;
            for edge in ["start", "end"] {
                select(
                    tab,
                    &format!("sheet-demo-edge-{edge}"),
                    &format!("d.edge === {}", js(edge)),
                )
                .await?;
                visible_click(tab, OPEN).await?;
                endpoint(tab, true).await?;
                let right = (edge == "end") ^ rtl;
                let panel = geometry(tab, detached, right).await?;
                let body = rect(&bounds(tab, BODY).await?)?;
                if mode == "coplanar" {
                    let width_removed = original[2] - body[2];
                    let separated = if right {
                        near(original[0], body[0]) && body[0] + body[2] <= panel[0] + 0.75
                    } else {
                        near(original[0] + original[2], body[0] + body[2])
                            && panel[0] + panel[2] <= body[0] + 0.75
                    };
                    check!(
                        separated
                            && width_removed >= panel[2] - 0.75
                            && width_removed <= panel[2] + 32.75
                            && near(original[3], body[3]),
                        "coplanar reserves the sheet width: body {body:?}, panel {panel:?}"
                    );
                } else {
                    check!(same_rect(original, body), "overlay preserves main bounds");
                }
                tab.wait_for(&format!(
                    "{} !== null && {} !== null && !document.querySelector('#surface-accessibility [aria-modal=true]')",
                    element(MAIN), element(ACTION)
                )).await?;
                let dom = export(tab).await?;
                let layer = color(&find_node(&dom, SCRIM)?["style"]["background"])?;
                check_eq!(layer[3], 0, "standard layer has no visible scrim");
                let before = sheet(tab).await?;
                let main = bounds(tab, MAIN).await?;
                check!(
                    !in_rect(
                        main.b[0] + main.b[2] / 2.,
                        main.b[1] + main.b[3] / 2.,
                        panel
                    ),
                    "outside probe stays outside the standard sheet"
                );
                tab.click(main.x, main.y).await?;
                ready(
                    tab,
                    &format!(
                        "d.open && d.main_actions === {}",
                        next_count(&before, "main_actions")?
                    ),
                )
                .await?;
                check_eq!(
                    sheet(tab).await?["dismissals"],
                    before["dismissals"],
                    "outside click keeps standard sheet open"
                );
                if rtl == detached && edge == "end" {
                    let tag = format!("{mode}-{style}-{}", if rtl { "rtl" } else { "ltr" });
                    let image = Image::capture(env, tab, &tag).await?;
                    surface_pixel(tab, &image, "standard_surface").await?;
                    let x = main.canvas[0] + if right { 8. } else { main.canvas[2] - 8. };
                    let y = main.canvas[1] + 8.;
                    let base = bare.pixel(x, y)?;
                    let painted = image.pixel(x, y)?;
                    check!(
                        distance(base, painted) <= 8,
                        "standard leaves the outside pixels unchanged"
                    );
                    tab_to_last(tab).await?;
                    tab.press("Tab").await?;
                    projected_focus(tab, "sheet-demo-back").await?;
                    tab.press("Escape").await?;
                    endpoint(tab, true).await?;
                    focus_element(tab, ACTION).await?;
                    tab.press("Escape").await?;
                } else {
                    visible_click(tab, CLOSE).await?;
                }
                endpoint(tab, false).await?;
                focused(tab, OPEN).await?;
                let restored = rect(&bounds(tab, BODY).await?)?;
                check!(
                    same_rect(original, restored),
                    "closing restores the whole body"
                );
                cases.push(json!({"presentation":mode,"style":style,"edge":edge,"rtl":rtl,"panel":panel,"body":body}));
            }
        }
    }
    Ok(json!(cases))
}

fn ax_node(tree: &Value, role: &str, name: &str) -> Result<Value> {
    tree["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
                node["ignored"] == false
                    && node["role"]["value"] == role
                    && node["name"]["value"] == name
            })
        })
        .cloned()
        .with_context(|| format!("missing Chrome {role} named {name}"))
}

fn ax_flag(node: &Value, name: &str) -> bool {
    node["properties"].as_array().is_some_and(|properties| {
        properties
            .iter()
            .any(|p| p["name"] == name && p["value"]["value"] == true)
    })
}

async fn modal(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M47",
        "modal isolates background and restores focus on Escape, scrim and Close",
    );
    select(
        tab,
        "sheet-demo-modal",
        "d.effective_presentation === 'modal'",
    )
    .await?;
    select(tab, "sheet-demo-docked", "d.style === 'docked'").await?;
    if sheet(tab).await?["rtl"] == true {
        select(tab, "sheet-demo-rtl", "!d.rtl").await?;
    }
    let main = bounds(tab, MAIN).await?;
    let probe = (main.canvas[0] + 8., main.canvas[1] + 8.);
    let bare = Image::capture(env, tab, "modal-before").await?;
    focus_element(tab, OPEN).await?;
    let before = sheet(tab).await?;
    tab.press("Enter").await?;
    endpoint(tab, true).await?;
    projected_focus(tab, CLOSE).await?;
    let docked_bounds = geometry(tab, false, true).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-modal') === 'true' && {} === null && {} === null && {}?.getAttribute('aria-disabled') === 'true'",
        element(PANEL), element(MAIN), element(OPEN), element(DISABLED)
    )).await?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let dialog = ax_node(&tree, "dialog", "工作區詳細資料")?;
    let dom = export(tab).await?;
    let disabled_name = find_node(&dom, DISABLED)?["name"]
        .as_str()
        .context("disabled label")?;
    let disabled = ax_node(&tree, "button", disabled_name)?;
    check!(
        ax_flag(&dialog, "modal") && ax_flag(&disabled, "disabled"),
        "Chrome exposes modal and disabled state"
    );
    let image = Image::capture(env, tab, "modal-docked-light").await?;
    surface_pixel(tab, &image, "active_surface").await?;
    let scrim = color(&find_node(&dom, SCRIM)?["style"]["background"])?;
    let base_pixel = bare.pixel(probe.0, probe.1)?;
    let painted_pixel = image.pixel(probe.0, probe.1)?;
    let expected = blend(base_pixel, scrim);
    check!(scrim[3] > 0 && distance(painted_pixel, expected) <= 8 && distance(base_pixel, painted_pixel) > 10,
        "WebGPU paints the dimming scrim: {base_pixel:?} → {painted_pixel:?}, expected {expected:?}");
    visible_click(tab, DISABLED).await?;
    check_eq!(
        sheet(tab).await?["panel_actions"],
        before["panel_actions"],
        "disabled button ignores real pointer"
    );

    focus_element(tab, CLOSE).await?;
    // Tab through the entire scope in both directions. Disabled controls
    // and the page must stay out of the cycle.
    let mut visited = Vec::new();
    for key in ["Tab"; 5].into_iter().chain(["Shift+Tab"; 5]) {
        tab.press(key).await?;
        tab.settle().await?;
        let s = state(tab).await?;
        let focus = s["focused"].as_str().context("modal focus")?;
        check!(
            [CLOSE, ACTION, LAST].contains(&focus),
            "modal kept {key} in scope: {focus}"
        );
        visited.push(focus.to_owned());
    }
    check!(
        [CLOSE, ACTION, LAST]
            .iter()
            .all(|id| visited.iter().any(|focus| focus == id)),
        "keyboard visits each enabled sheet control"
    );
    focus_element(tab, ACTION).await?;
    tab.press("Space").await?;
    ready(
        tab,
        &format!(
            "d.open && d.panel_actions === {}",
            next_count(&before, "panel_actions")?
        ),
    )
    .await?;
    tab.press("Escape").await?;
    endpoint(tab, false).await?;
    projected_focus(tab, OPEN).await?;

    visible_click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    let panel = point(tab, PANEL).await?;
    check!(
        !in_rect(
            main.x - panel.canvas[0],
            main.y - panel.canvas[1],
            rect(&panel)?
        ),
        "scrim probe is over the outside main action"
    );
    tab.click(main.x, main.y).await?;
    endpoint(tab, false).await?;
    focused(tab, OPEN).await?;
    let after_outside = sheet(tab).await?;
    check_eq!(
        after_outside["main_actions"],
        before["main_actions"],
        "scrim consumes outside activation"
    );
    check_eq!(
        after_outside["dismissals"],
        json!(next_count(&before, "dismissals")? + 1),
        "Escape and outside click each dismiss once"
    );

    // Both styles, both logical edges, and both document directions keep
    // modal placement and the close affordance accessible.
    let mut variants = Vec::new();
    for rtl in [false, true] {
        if sheet(tab).await?["rtl"] != rtl {
            select(tab, "sheet-demo-rtl", &format!("d.rtl === {rtl}")).await?;
        }
        for detached in [false, true] {
            let style = if detached { "detached" } else { "docked" };
            select(
                tab,
                &format!("sheet-demo-{style}"),
                &format!("d.style === {}", js(style)),
            )
            .await?;
            for edge in ["start", "end"] {
                select(
                    tab,
                    &format!("sheet-demo-edge-{edge}"),
                    &format!("d.edge === {}", js(edge)),
                )
                .await?;
                visible_click(tab, OPEN).await?;
                endpoint(tab, true).await?;
                focused(tab, CLOSE).await?;
                let rect = geometry(tab, detached, (edge == "end") ^ rtl).await?;
                visible_click(tab, CLOSE).await?;
                endpoint(tab, false).await?;
                focused(tab, OPEN).await?;
                variants.push(json!({"style":style,"edge":edge,"rtl":rtl,"bounds":rect}));
            }
        }
    }
    Ok(
        json!({"panel":docked_bounds,"dialog_ax":dialog,"disabled_ax":disabled,"focus_path":visited,
        "scrim_base":base_pixel,"scrim_painted":painted_pixel,"after_outside":after_outside,"variants":variants}),
    )
}

async fn wrapped_ink(tab: &Tab, image: &Image, id: &str, viewport_id: &str) -> Result<Vec<usize>> {
    let dom = export(tab).await?;
    let node = find_node(&dom, id)?;
    let glyphs = node["text"].as_str().context("authored Chinese text")?;
    check!(
        glyphs.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}')),
        "{id} contains Chinese text"
    );
    let line_height = node["style"]["line_height"]
        .as_f64()
        .context("authored line height")?
        * 2.;
    let p = point(tab, id).await?;
    let [x, y, width, height] = rect(&p)?;
    let [clip_x, clip_y, clip_width, clip_height] = rect(&point(tab, viewport_id).await?)?;
    check!(
        height >= 2. * line_height - 0.75
            && x >= clip_x - 0.75
            && x + width <= clip_x + clip_width + 0.75,
        "{id} wraps inside {viewport_id}"
    );
    let expected = rgb(color(&node["style"]["foreground"])?);
    let mut ink = Vec::new();
    for line in 0..2 {
        let top = y + f64::from(line) * line_height;
        let bottom = top + line_height;
        check!(
            top >= clip_y - 0.75 && bottom <= clip_y + clip_height + 0.75,
            "{id} line {} is visible",
            line + 1
        );
        let mut count = 0;
        for row in top.ceil() as usize..bottom.floor() as usize {
            for column in x.ceil() as usize..(x + width).floor() as usize {
                if distance(
                    image.pixel(p.canvas[0] + column as f64, p.canvas[1] + row as f64)?,
                    expected,
                ) <= 8
                {
                    count += 1;
                }
            }
        }
        check!(
            count >= 10,
            "{id} line {} paints actual glyphs: {count} pixels",
            line + 1
        );
        ink.push(count);
    }
    Ok(ink)
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M48",
        "320 dp uses modal fallback, wraps 200% Chinese, and preserves state on resize",
    );
    select(tab, "sheet-demo-coplanar", "d.presentation === 'coplanar'").await?;
    select(tab, "sheet-demo-scale", "d.text_scale === 2").await?;
    visible_click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    visible_click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    let wide = geometry(tab, true, false).await?;
    focus_element(tab, ACTION).await?;
    let before = sheet(tab).await?;
    viewport(tab, 320, 580).await?;
    endpoint(tab, true).await?;
    ready(tab, "d.presentation === 'coplanar' && d.effective_presentation === 'modal' && d.text_scale === 2 && d.rtl").await?;
    projected_focus(tab, CLOSE).await?;
    let narrow = geometry(tab, true, false).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-modal') === 'true' && {} === null",
        element(PANEL),
        element(MAIN)
    ))
    .await?;
    let header = rect(&bounds(tab, HEADER).await?)?;
    let area = point(tab, CONTENT).await?;
    let image = Image::capture(env, tab, "compact-detached-dark-rtl").await?;
    surface_pixel(tab, &image, "active_surface").await?;
    let title_ink = wrapped_ink(tab, &image, TITLE, HEADER).await?;
    let text_ink = wrapped_ink(tab, &image, LONG, CONTENT).await?;
    let original_last = point(tab, LAST).await?.b[1];
    tab.mouse_move(area.x, area.y).await?;
    tab.wheel(0., 260.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({})[1] < {}",
        js(LAST),
        original_last - 16.
    ))
    .await?;
    let scrolled_last = point(tab, LAST).await?.b[1];
    tab_to_last(tab).await?;
    let last = rect(&bounds(tab, LAST).await?)?;
    let content = rect(&bounds(tab, CONTENT).await?)?;
    check!(
        last[1] >= content[1] - 0.75
            && last[1] + last[3] <= content[1] + content[3] + 0.75
            && same_rect(header, rect(&bounds(tab, HEADER).await?)?),
        "wheel and keyboard reveal the last action; header stays fixed"
    );
    Image::capture(env, tab, "compact-detached-dark-rtl-scrolled").await?;
    viewport(tab, 960, 760).await?;
    endpoint(tab, true).await?;
    ready(
        tab,
        "d.effective_presentation === 'coplanar' && d.style === 'detached'",
    )
    .await?;
    projected_focus(tab, ACTION).await?;
    check_eq!(
        geometry(tab, true, false).await?,
        wide,
        "wide variant returns to its original bounds"
    );
    for field in ["main_actions", "panel_actions", "dismissals"] {
        check_eq!(
            sheet(tab).await?[field],
            before[field],
            format!("reflow preserves {field}")
        );
    }
    visible_click(tab, CLOSE).await?;
    endpoint(tab, false).await?;
    focused(tab, OPEN).await?;
    Ok(
        json!({"wide":wide,"narrow":narrow,"title_ink":title_ink,"text_ink":text_ink,
        "last_before_wheel":original_last,"last_after_wheel":scrolled_last,"last_after_focus":last,
        "header":header,"restored_standard_focus":ACTION}),
    )
}

async fn media(tab: &Tab, preference: &str) -> Result<()> {
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":preference}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {preference})"))
    ))
    .await?;
    tab.settle().await
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    // Keep real rAF, input and rendering. Only freeze the animation clock.
    tab.eval(
        "(() => {
        const start = performance.now();
        window.sideSheetTestElapsed = 0;
        Object.defineProperty(performance, 'now', {
            configurable:true, value:() => start + window.sideSheetTestElapsed,
        });
    })()",
    )
    .await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.sideSheetTestElapsed = {ms}"))
        .await?;
    tab.settle().await
}

async fn animate_open(env: &Env, tab: &Tab) -> Result<Value> {
    pin_clock(tab).await?;
    visible_click(tab, OPEN).await?;
    ready(tab, "d.open && d.settling && d.reveal === 0").await?;
    advance(tab, 100).await?;
    ready(tab, "d.open && d.settling && d.reveal > 0 && d.reveal < 1").await?;
    let mid = sheet(tab).await?;
    let midpoint = point(tab, PANEL).await?;
    let [x, _, width, _] = rect(&midpoint)?;
    check!(
        x > midpoint.canvas[2] - width && x < midpoint.canvas[2],
        "intermediate WebGPU frame is between the open and hidden edges"
    );
    Image::capture(env, tab, "modal-enter-100ms").await?;
    advance(tab, 280).await?;
    endpoint(tab, true).await?;
    tab.eval("delete performance.now").await?;
    Ok(json!({"elapsed_ms":100,"bounds":midpoint.b,"reveal":mid["reveal"]}))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pointer {
    Mouse,
    Touch,
}

impl Pointer {
    fn kind(self) -> &'static str {
        match self {
            Self::Mouse => "mouse",
            Self::Touch => "touch",
        }
    }

    async fn phase(self, tab: &Tab, phase: &str, (x, y): (f64, f64)) -> Result<()> {
        if self == Self::Touch {
            let touches = if matches!(phase, "End" | "Cancel") {
                json!([])
            } else {
                json!([{"x":x,"y":y,"id":1}])
            };
            tab.cdp(
                "Input.dispatchTouchEvent",
                json!({"type":format!("touch{phase}"),"touchPoints":touches}),
            )
            .await?;
        } else {
            match phase {
                "Start" => {
                    tab.mouse_move(x, y).await?;
                    tab.mouse_down().await?;
                }
                "Move" | "End" => {
                    let (kind, buttons) = if phase == "Move" {
                        ("mouseMoved", 1)
                    } else {
                        ("mouseReleased", 0)
                    };
                    tab.cdp("Input.dispatchMouseEvent", json!({"type":kind,"x":x,"y":y,"buttons":buttons,"button":"left","clickCount":1})).await?;
                }
                _ => bail!("unsupported mouse phase {phase}"),
            }
        }
        Ok(())
    }
}

async fn swipe(env: &Env, tab: &Tab, pointer: Pointer, rtl: bool) -> Result<Value> {
    if pointer == Pointer::Touch {
        tab.cdp(
            "Emulation.setTouchEmulationEnabled",
            json!({"enabled":true,"maxTouchPoints":1}),
        )
        .await?;
    }
    tab.eval(
        "(() => {
        window.sheetEvents = [];
        if (window.sheetListening) return;
        window.sheetListening = true;
        for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
            document.addEventListener(kind, e => {
                if (e.target?.id === 'surface') {
                    window.sheetEvents.push({kind,device:e.pointerType,trusted:e.isTrusted});
                }
            }, {capture:true,passive:true});
        }
    })()",
    )
    .await?;
    let panel = bounds(tab, PANEL).await?;
    let [left, _, width, _] = rect(&panel)?;
    let button = bounds(tab, ACTION).await?;
    let [button_x, _, button_width, _] = rect(&button)?;
    let origin_x = if rtl {
        button_x + button_width - 12.
    } else {
        button_x + 12.
    };
    let origin = (panel.canvas[0] + origin_x, button.y);
    let sign = if rtl { -1. } else { 1. };
    let at = |fraction: f64| (origin.0 + sign * width * fraction, origin.1);
    let far = at(0.65);
    check!(
        far.0 >= panel.canvas[0] && far.0 < panel.canvas[0] + panel.canvas[2],
        "the swipe crosses the midpoint while staying inside Chrome"
    );
    let before = sheet(tab).await?;
    if pointer == Pointer::Touch {
        pointer.phase(tab, "Start", origin).await?;
        pointer.phase(tab, "Move", far).await?;
        ready(
            tab,
            "d.open && d.dragging && d.reveal > 0.27 && d.reveal < 0.43",
        )
        .await?;
        pointer.phase(tab, "Cancel", far).await?;
        endpoint(tab, true).await?;
        check_eq!(
            sheet(tab).await?["dismissals"],
            before["dismissals"],
            "touch cancellation returns to the open anchor"
        );
    }

    pointer.phase(tab, "Start", origin).await?;
    pointer.phase(tab, "Move", at(0.25)).await?;
    ready(
        tab,
        "d.open && d.dragging && d.reveal > 0.68 && d.reveal < 0.82",
    )
    .await?;
    // Let the velocity sample expire so the midpoint decides the release.
    tokio::time::sleep(Duration::from_millis(140)).await;
    pointer.phase(tab, "End", at(0.25)).await?;
    endpoint(tab, true).await?;
    check_eq!(
        sheet(tab).await?["dismissals"],
        before["dismissals"],
        "short swipe rebounds"
    );

    pointer.phase(tab, "Start", origin).await?;
    pointer.phase(tab, "Move", far).await?;
    ready(
        tab,
        "d.open && d.dragging && d.reveal > 0.27 && d.reveal < 0.43",
    )
    .await?;
    let held = sheet(tab).await?;
    let preview = rect(&point(tab, PANEL).await?)?;
    check!(
        near(preview[0], left + sign * width * 0.65),
        "sheet follows the held pointer"
    );
    tab.wait_for(&format!("{} === null", element(MAIN))).await?;
    Image::capture(env, tab, &format!("{}-held", pointer.kind())).await?;
    tokio::time::sleep(Duration::from_millis(140)).await;
    pointer.phase(tab, "End", far).await?;
    endpoint(tab, false).await?;
    focused(tab, OPEN).await?;
    let after = sheet(tab).await?;
    check_eq!(
        after["dismissals"],
        json!(next_count(&before, "dismissals")?),
        "committed swipe closes once"
    );
    for field in ["panel_actions", "main_actions"] {
        check_eq!(after[field], before[field], format!("swipe keeps {field}"));
    }

    let events = tab.eval("window.sheetEvents").await?;
    let down_move_up = ["pointerdown", "pointermove", "pointerup"];
    let kinds = down_move_up
        .into_iter()
        .chain((pointer == Pointer::Touch).then_some("pointercancel"));
    let observed = events.as_array().context("Chrome pointer events")?;
    for kind in kinds {
        check!(
            observed.iter().any(|event| event["kind"] == kind
                && event["device"] == pointer.kind()
                && event["trusted"] == true),
            "{} delivered trusted {kind}",
            pointer.kind()
        );
    }
    if pointer == Pointer::Touch {
        tab.cdp(
            "Emulation.setTouchEmulationEnabled",
            json!({"enabled":false}),
        )
        .await?;
    }
    Ok(
        json!({"open_panel":panel.b,"held_panel":preview,"reveal":held["reveal"],"events":events,"after":after}),
    )
}

async fn motion(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M49",
        "WebGPU frames follow trusted mouse and touch; Reduce Motion snaps",
    );
    select(
        tab,
        "sheet-demo-modal",
        "d.effective_presentation === 'modal'",
    )
    .await?;
    select(tab, "sheet-demo-docked", "d.style === 'docked'").await?;
    select(tab, "sheet-demo-scale", "d.text_scale === 1").await?;
    select(tab, "sheet-demo-rtl", "!d.rtl").await?;
    visible_click(tab, "theme").await?;
    until(tab, "!s.dark").await?;
    media(tab, "no-preference").await?;
    let entering = animate_open(env, tab).await?;
    let mouse = swipe(env, tab, Pointer::Mouse, false).await?;
    select(tab, "sheet-demo-rtl", "d.rtl").await?;
    visible_click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    let touch = swipe(env, tab, Pointer::Touch, true).await?;

    media(tab, "reduce").await?;
    pin_clock(tab).await?;
    focus_element(tab, OPEN).await?;
    tab.press("Enter").await?;
    endpoint(tab, true).await?;
    ready(tab, "d.reduced_motion").await?;
    let open = sheet(tab).await?;
    projected_focus(tab, CLOSE).await?;
    tab.press("Escape").await?;
    endpoint(tab, false).await?;
    projected_focus(tab, OPEN).await?;
    let dom = export(tab).await?;
    check!(
        find_node(&dom, PANEL).is_err() && find_node(&dom, SCRIM).is_err(),
        "Reduce Motion closes panel and scrim without clock advancement"
    );
    let clock = tab.eval("window.sideSheetTestElapsed").await?;
    check_eq!(clock, json!(0), "the motion probe kept the clock fixed");
    tab.eval("delete performance.now").await?;
    Image::capture(env, tab, "reduced-motion-closed").await?;
    Ok(
        json!({"enter":entering,"mouse":mouse,"touch":touch,"reduced_motion_open":open,
        "reduced_motion_closed":sheet(tab).await?}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    viewport(tab, 960, 760).await?;
    media(tab, "no-preference").await?;
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    visible_click(tab, "side-sheet-demo-show").await?;
    endpoint(tab, false).await?;
    let adapter = tab
        .eval(
            "(async () => {
            const a = await navigator.gpu.requestAdapter();
            if (!a) return null;
            const {vendor, architecture, isFallbackAdapter} = a.info;
            return {vendor, architecture, isFallbackAdapter};
        })()",
        )
        .await?;
    check!(!adapter.is_null(), "Chrome has a WebGPU adapter");
    let overlay = standard(env, tab, "overlay").await?;
    let coplanar = standard(env, tab, "coplanar").await?;
    let modal = modal(env, tab).await?;
    let compact = compact(env, tab).await?;
    let motion = motion(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "side sheet page errors: {errors:?}");
    env.write_json(
        "browser-material-side-sheet.json",
        &json!({
            "browser":chrome.version,"adapter":adapter,"overlay":overlay,"coplanar":coplanar,
            "modal":modal,"compact":compact,"motion":motion,"errors":errors
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":export(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-side-sheet-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-side-sheet-failure.png"))
            .await;
    }
    result
}
