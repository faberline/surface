//! Material Badge paint, hit testing and accessibility in the WebGPU Gallery.

use super::{click, point, state, until, Point};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, path::Path};

const INBOX: &str = "badge-inbox";
const PREVIEW: &str = "badge-anchor-surface";
const NAVIGATION: &str = "badge-navigation";
const NAV_INBOX: &str = "badge-nav-1";
const SCROLL: &str = "badge-scroll";
const SCROLL_OWNER: &str = "badge-scroll-owner";

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(badge => {condition})(s.badges)")).await?;
    tab.settle().await
}

fn selector(id: &str) -> String {
    format!("#surface-accessibility [data-surface-id=\"{id}\"]")
}

fn element(id: &str) -> String {
    format!("document.querySelector({})", js(selector(id)))
}

async fn reveal(tab: &Tab, id: &str) -> Result<()> {
    let target = point(tab, id).await?;
    if target.b[1] < 8. || target.b[1] + target.b[3] > target.canvas[3] - 8. {
        let scale = tab.eval("devicePixelRatio").await?;
        // The page margin is outside the nested scrolling sample.
        tab.mouse_move(target.canvas[0] + 10., target.canvas[1] + 26.)
            .await?;
        tab.wheel(
            0.,
            (target.b[1] - 32.) * scale.as_f64().context("pixel ratio")?,
        )
        .await?;
    }
    tab.wait_for(&format!(
        "(() => {{
            const [x,y,w,h] = window.gallery.control_bounds({});
            const canvas = document.querySelector('canvas').getBoundingClientRect();
            return w > 0 && h > 0 && x >= -1 && y >= -1 &&
                x+w <= canvas.width+1 && y+h <= canvas.height+1;
        }})()",
        js(id)
    ))
    .await
}

async fn visible_click(tab: &Tab, id: &str) -> Result<()> {
    reveal(tab, id).await?;
    click(tab, id).await
}

async fn kind(tab: &Tab, kind: &str) -> Result<()> {
    visible_click(tab, &format!("badge-show-{kind}")).await?;
    ready(tab, &format!("badge.kind === {}", js(kind))).await
}

/// A rectangle in page CSS coordinates, including the canvas offset.
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn from_point(point: &Point) -> Self {
        Self {
            x: point.canvas[0] + point.b[0],
            y: point.canvas[1] + point.b[1],
            width: point.b[2],
            height: point.b[3],
        }
    }

    fn right(self) -> f64 {
        self.x + self.width
    }

    fn bottom(self) -> f64 {
        self.y + self.height
    }

    fn contains(self, x: f64, y: f64, inset: f64) -> bool {
        x >= self.x + inset
            && x < self.right() - inset
            && y >= self.y + inset
            && y < self.bottom() - inset
    }

    fn relative(self, anchor: Self) -> [f64; 4] {
        [
            self.x - anchor.x,
            self.y - anchor.y,
            self.width,
            self.height,
        ]
    }
}

#[derive(Default)]
struct Paint {
    bounds: Option<Rect>,
    inside: Option<(f64, f64)>,
    outside: Option<(f64, f64)>,
    pixels: usize,
}

struct Screenshot {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
    channels: usize,
    scale: f64,
}

impl Screenshot {
    fn read(path: &Path, page_width: f64) -> Result<Self> {
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().context("PNG size")?];
        let frame = reader.next_frame(&mut pixels)?;
        let channels = match frame.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG type {other:?}"),
        };
        Ok(Self {
            pixels,
            width: frame.width as usize,
            height: frame.height as usize,
            channels,
            scale: f64::from(frame.width) / page_width,
        })
    }

    fn scan(&self, rect: Rect, color: [u8; 3], owner: Option<Rect>) -> Paint {
        let start_x = (rect.x * self.scale).ceil() as usize;
        let start_y = (rect.y * self.scale).ceil() as usize;
        let end_x = ((rect.right() * self.scale).floor() as usize).min(self.width);
        let end_y = ((rect.bottom() * self.scale).floor() as usize).min(self.height);
        let mut extents = [usize::MAX, usize::MAX, 0, 0];
        let mut paint = Paint::default();
        for y in start_y..end_y {
            for x in start_x..end_x {
                let offset = (y * self.width + x) * self.channels;
                let distance = self.pixels[offset..offset + 3]
                    .iter()
                    .zip(color)
                    .map(|(actual, expected)| u16::from(actual.abs_diff(expected)))
                    .sum::<u16>();
                if distance > 8 {
                    continue;
                }
                paint.pixels += 1;
                extents[0] = extents[0].min(x);
                extents[1] = extents[1].min(y);
                extents[2] = extents[2].max(x + 1);
                extents[3] = extents[3].max(y + 1);
                if let Some(owner) = owner {
                    let at = ((x as f64 + 0.5) / self.scale, (y as f64 + 0.5) / self.scale);
                    if owner.contains(at.0, at.1, 1.) {
                        paint.inside.get_or_insert(at);
                    } else if !owner.contains(at.0, at.1, -1.) {
                        paint.outside.get_or_insert(at);
                    }
                }
            }
        }
        if paint.pixels != 0 {
            let [x, y, right, bottom] = extents;
            paint.bounds = Some(Rect {
                x: x as f64 / self.scale,
                y: y as f64 / self.scale,
                width: (right - x) as f64 / self.scale,
                height: (bottom - y) as f64 / self.scale,
            });
        }
        paint
    }
}

async fn screenshot(env: &Env, tab: &Tab, name: &str) -> Result<Screenshot> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    let width = tab.eval("window.innerWidth").await?;
    Screenshot::read(&path, width.as_f64().context("browser page width")?)
}

async fn colors(tab: &Tab) -> Result<([u8; 3], [u8; 3])> {
    let state = state(tab).await?;
    Ok((
        opaque_rgb(&state["badges"]["error"])?,
        opaque_rgb(&state["badges"]["on_error"])?,
    ))
}

fn opaque_rgb(value: &Value) -> Result<[u8; 3]> {
    let [red, green, blue, alpha]: [u8; 4] = serde_json::from_value(value.clone())?;
    check_eq!(alpha, 255, "opaque paint token");
    Ok([red, green, blue])
}

fn dot_size(paint: Rect) -> Result<()> {
    check!(
        (paint.width - 6.).abs() <= 1. && (paint.height - 6.).abs() <= 1.,
        "dot paints 6 dp, got {} by {}",
        paint.width,
        paint.height
    );
    Ok(())
}

fn full_badge(image: &Screenshot, fill: Rect, ink: [u8; 3]) -> Result<()> {
    check!(fill.height >= 15., "content badge paints at least 16 dp");
    check!(
        image.scan(fill, ink, None).pixels >= 4,
        "badge contains visible on-error text pixels"
    );
    Ok(())
}

async fn light(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M32",
        "Badge paints both sizes without changing the anchor geometry",
    );
    visible_click(tab, "section-3").await?;
    until(tab, "s.section === 3 && !s.dark").await?;
    kind(tab, "hidden").await?;
    reveal(tab, PREVIEW).await?;
    let preview = Rect::from_point(&point(tab, PREVIEW).await?);
    let owner = Rect::from_point(&point(tab, INBOX).await?);
    let baseline = owner.relative(preview);
    check_eq!(baseline[2..], [48., 48.], "the primary anchor is 48 dp");
    let (fill, ink) = colors(tab).await?;
    let image = screenshot(env, tab, "material-badge-light-hidden.png").await?;
    check_eq!(
        image.scan(preview, fill, None).pixels,
        0,
        "hidden decoration paints no error-colored pixels"
    );

    kind(tab, "dot").await?;
    reveal(tab, PREVIEW).await?;
    let preview = Rect::from_point(&point(tab, PREVIEW).await?);
    let owner = Rect::from_point(&point(tab, INBOX).await?);
    check_eq!(
        owner.relative(preview),
        baseline,
        "dot keeps anchor geometry"
    );
    let image = screenshot(env, tab, "material-badge-light-ltr-dot.png").await?;
    let dot = image
        .scan(preview, fill, Some(owner))
        .bounds
        .context("anchored dot is missing from the WebGPU PNG")?;
    dot_size(dot)?;
    check!(
        dot.x > owner.x + owner.width / 2.,
        "LTR dot is at logical end"
    );

    reveal(tab, "badge-standalone-dot").await?;
    reveal(tab, "badge-standalone-label").await?;
    let solo_dot = Rect::from_point(&point(tab, "badge-standalone-dot").await?);
    let solo_label = Rect::from_point(&point(tab, "badge-standalone-label").await?);
    check_eq!(
        [solo_dot.width, solo_dot.height],
        [6., 6.],
        "standalone dot layout"
    );
    check_eq!(
        solo_label.height,
        16.,
        "default content badge layout height"
    );
    let image = screenshot(env, tab, "material-badge-light-standalone.png").await?;
    dot_size(
        image
            .scan(solo_dot, fill, None)
            .bounds
            .context("standalone dot")?,
    )?;
    let pill = image
        .scan(solo_label, fill, None)
        .bounds
        .context("standalone label")?;
    full_badge(&image, pill, ink)?;
    let dot_node = element("badge-standalone-dot");
    let label_node = element("badge-standalone-label");
    tab.wait_for(&format!(
        "{dot_node}?.textContent === '有新訊息' && !{dot_node}.hasAttribute('tabindex') &&
         {label_node}?.textContent === '有新內容' && !{label_node}.hasAttribute('tabindex')"
    ))
    .await?;
    Ok(json!({"anchor":baseline, "dot":dot.relative(owner),
        "standalone_label":pill.relative(solo_label), "color":fill}))
}

async fn visible_text(tab: &Tab, text: &str, preview: Rect) -> Result<()> {
    let [offset_x, offset_y, _, _] = tab.canvas_box().await?;
    tab.wait_for(&format!(
        "(() => {{
            const snapshot = JSON.parse(window.gallery.export_dom());
            return snapshot.dom.nodes.some((node, index) => {{
                if (node.text !== {}) return false;
                const rect = snapshot.layout.nodes[index]?.bounds;
                return rect?.x + {offset_x} >= {} - 1 && rect?.y + {offset_y} >= {} - 1 &&
                    rect?.x + {offset_x} + rect.width <= {} + 1 &&
                    rect?.y + {offset_y} + rect.height <= {} + 1;
            }});
        }})()",
        js(text),
        preview.x,
        preview.y,
        preview.right(),
        preview.bottom()
    ))
    .await
}

async fn accessible_anchor(tab: &Tab, count: u64, selected: bool) -> Result<()> {
    let name = format!("收件匣，{count} 則未讀");
    let anchor = element(INBOX);
    tab.wait_for(&format!(
        "{anchor}?.getAttribute('role') === 'button' &&
         {anchor}.getAttribute('aria-label') === {} &&
         {anchor}.getAttribute('aria-pressed') === {}",
        js(&name),
        js(selected.to_string())
    ))
    .await?;
    let document = tab.cdp("DOM.getDocument", json!({"depth":0})).await?;
    let owner = tab
        .cdp(
            "DOM.querySelector",
            json!({"nodeId":document["root"]["nodeId"],"selector":selector(INBOX)}),
        )
        .await?;
    let described = tab
        .cdp("DOM.describeNode", json!({"nodeId":owner["nodeId"]}))
        .await?;
    let backend = described["node"]["backendNodeId"]
        .as_u64()
        .context("Inbox backend DOM ID")?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let nodes = tree["nodes"].as_array().context("Chrome AX nodes")?;
    let matching: Vec<_> = nodes
        .iter()
        .filter(|node| {
            node["ignored"] == false && node["backendDOMNodeId"].as_u64() == Some(backend)
        })
        .collect();
    check_eq!(
        matching.len(),
        1,
        "the primary Inbox has one accessible owner"
    );
    let node = matching[0];
    check_eq!(
        node["role"]["value"],
        "button",
        "the owner retains its button role"
    );
    check_eq!(
        node["name"]["value"],
        name,
        "the owner keeps the uncapped count"
    );
    let properties = node["properties"]
        .as_array()
        .context("Inbox AX properties")?;
    check!(
        properties
            .iter()
            .any(|prop| prop["name"] == "focusable" && prop["value"]["value"] == true),
        "the owner stays keyboard focusable"
    );
    check!(
        !nodes
            .iter()
            .any(|node| node["ignored"] == false && node["name"]["value"] == "999+"),
        "visual count abbreviation adds no accessible node"
    );
    Ok(())
}

async fn interaction(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M33",
        "Badge updates the accessible count without adding a hit or tab target",
    );
    kind(tab, "count").await?;
    reveal(tab, PREVIEW).await?;
    let (fill, ink) = colors(tab).await?;
    let preview = Rect::from_point(&point(tab, PREVIEW).await?);
    let owner = Rect::from_point(&point(tab, INBOX).await?);
    accessible_anchor(tab, 999, false).await?;
    visible_text(tab, "999", preview).await?;
    let image = screenshot(env, tab, "material-badge-light-ltr-count.png").await?;
    let paint = image.scan(preview, fill, Some(owner));
    let painted = paint.bounds.context("label pixels are missing")?;
    full_badge(&image, painted, ink)?;
    let inside = paint
        .inside
        .context("badge does not overlap the Inbox target")?;
    tab.click(inside.0, inside.1).await?;
    ready(
        tab,
        "badge.count === 1000 && badge.presses === 1 && badge.selected",
    )
    .await?;
    until(tab, "s.focused === 'badge-inbox'").await?;
    accessible_anchor(tab, 1000, true).await?;
    visible_text(tab, "999+", preview).await?;

    let image = screenshot(env, tab, "material-badge-light-overflow.png").await?;
    let overflow = image.scan(preview, fill, Some(owner));
    full_badge(&image, overflow.bounds.context("overflow pill")?, ink)?;
    let outside = overflow
        .outside
        .context("overflow does not protrude beyond the owner")?;
    tab.click(outside.0, outside.1).await?;
    tab.settle().await?;
    let after_outside = state(tab).await?;
    check_eq!(
        after_outside["badges"]["presses"],
        1,
        "paint outside is not a hit target"
    );
    check_eq!(
        after_outside["badges"]["count"],
        1000,
        "outside press leaves count"
    );

    let anchor = element(INBOX);
    tab.eval(&format!("{anchor}.focus()")).await?;
    until(tab, "s.focused === 'badge-inbox'").await?;
    tab.wait_for(&format!("document.activeElement === {anchor}"))
        .await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "badge.count === 1001 && badge.presses === 2 && !badge.selected",
    )
    .await?;
    until(tab, "s.focused === 'badge-inbox'").await?;
    tab.wait_for(&format!("document.activeElement === {anchor}"))
        .await?;
    accessible_anchor(tab, 1001, false).await?;
    tab.press("Tab").await?;
    until(tab, "s.focused !== 'badge-inbox' && s.focused !== null").await?;
    tab.press("Shift+Tab").await?;
    until(tab, "s.focused === 'badge-inbox'").await?;
    check_eq!(
        state(tab).await?["badges"]["presses"],
        2,
        "keyboard focus does not activate the anchor"
    );
    Ok(json!({"count":1001,"painted":"999+","presses":2,"inside":inside,"outside":outside}))
}

async fn compact(env: &Env, tab: &Tab, light: &Value) -> Result<Value> {
    step(
        "M34",
        "Badge mirrors and keeps CJK glyphs clear at 320 dp and 200% text",
    );
    visible_click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    visible_click(tab, "badge-direction").await?;
    ready(tab, "badge.rtl").await?;
    visible_click(tab, "badge-scale").await?;
    ready(tab, "badge.text_scale === 2").await?;
    kind(tab, "dot").await?;
    tab.viewport(320, 564).await?;
    tab.wait_for(
        "(() => {
        const canvas = document.querySelector('canvas');
        const layout = JSON.parse(window.gallery.export_dom()).layout.viewport;
        return layout.width === 320 && canvas.width === 320 * devicePixelRatio;
    })()",
    )
    .await?;
    reveal(tab, PREVIEW).await?;
    let preview = Rect::from_point(&point(tab, PREVIEW).await?);
    let owner = Rect::from_point(&point(tab, INBOX).await?);
    let (fill, ink) = colors(tab).await?;
    let image = screenshot(env, tab, "material-badge-dark-rtl-dot-200.png").await?;
    let dot = image
        .scan(preview, fill, Some(owner))
        .bounds
        .context("dark RTL dot")?;
    dot_size(dot)?;
    let relative = dot.relative(owner);
    let ltr: [f64; 4] = serde_json::from_value(light["dot"].clone())?;
    check!(
        (relative[0] + relative[2] + ltr[0] - owner.width).abs() <= 1.,
        "RTL mirrors dot pixels across the anchor: {relative:?}"
    );

    kind(tab, "label").await?;
    reveal(tab, PREVIEW).await?;
    let preview = Rect::from_point(&point(tab, PREVIEW).await?);
    let owner = Rect::from_point(&point(tab, INBOX).await?);
    visible_text(tab, "新", preview).await?;
    let image = screenshot(env, tab, "material-badge-dark-rtl-label-200.png").await?;
    let label = image
        .scan(preview, fill, Some(owner))
        .bounds
        .context("dark RTL label")?;
    full_badge(&image, label, ink)?;
    check!(label.height > 16., "content badge grows at 200% text");
    check!(
        label.x >= preview.x
            && label.right() <= preview.right()
            && label.y >= preview.y
            && label.bottom() <= preview.bottom(),
        "label clears the preview edges"
    );
    check_eq!(owner.width, 48., "large-text anchor width");
    check_eq!(owner.height, 48., "large-text anchor height");
    Ok(json!({"dot":relative,"label":label.relative(owner),"fill":fill,"ink":ink}))
}

async fn navigation_artwork(env: &Env, tab: &Tab) -> Result<Value> {
    reveal(tab, NAVIGATION).await?;
    let container = Rect::from_point(&point(tab, NAVIGATION).await?);
    let owner = Rect::from_point(&point(tab, NAV_INBOX).await?);
    visible_text(tab, "999+", container).await?;
    let artwork = tab.eval(&format!(
        "(() => {{
            const snapshot = JSON.parse(window.gallery.export_dom());
            const nodes = snapshot.dom.nodes;
            const owner = nodes.findIndex(node => node.stable_id === {});
            const index = nodes.findIndex(node => {{
                if (!node.fill_contour) return false;
                for (let parent = node.parent; Number.isInteger(parent); parent = nodes[parent].parent) {{
                    if (parent === owner) return true;
                }}
                return false;
            }});
            if (index < 0) return null;
            const contour = nodes[index].fill_contour;
            const bounds = snapshot.layout.nodes[index].bounds;
            // Rasterize the same even-odd geometry independently as the
            // unobstructed ink reference for the actual WebGPU PNG.
            const scale = devicePixelRatio;
            const reference = new OffscreenCanvas(
                Math.round(bounds.width * scale), Math.round(bounds.height * scale)
            );
            const context = reference.getContext('2d');
            context.scale(scale, scale);
            const path = new Path2D();
            path.moveTo(...contour.points[0]);
            for (const point of contour.points.slice(1)) path.lineTo(...point);
            path.closePath();
            context.fill(path, 'evenodd');
            const rgba = context.getImageData(0, 0, reference.width, reference.height).data;
            let referencePixels = 0;
            for (let i = 3; i < rgba.length; i += 4) {{
                if (rgba[i] >= 247) referencePixels++;
            }}
            return {{bounds, color: contour.color, referencePixels}};
        }})()",
        js(NAV_INBOX)
    )).await?;
    let bounds = &artwork["bounds"];
    let component = |key: &str| {
        bounds[key]
            .as_f64()
            .with_context(|| format!("Inbox artwork {key}"))
    };
    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    let icon = Rect {
        x: canvas_x + component("x")?,
        y: canvas_y + component("y")?,
        width: component("width")?,
        height: component("height")?,
    };
    check_eq!(
        [icon.width, icon.height],
        [24., 24.],
        "destination preserves its vector slot"
    );
    let image = screenshot(env, tab, "material-badge-dark-rtl-nav-overflow-200.png").await?;
    let (fill, glyph) = colors(tab).await?;
    let badge = image
        .scan(owner, fill, None)
        .bounds
        .context("overflow badge paint inside Inbox")?;
    full_badge(&image, badge, glyph)?;
    let overlap = (badge.bottom() - icon.y).max(0.);
    check!(
        overlap <= 4.5,
        "the enlarged badge overlaps the navigation icon by at most 4 dp (0.5 dp raster tolerance), got {overlap} dp"
    );
    let visible_top = icon.y.max(badge.bottom());
    let exposed_artwork = Rect {
        y: visible_top,
        height: (icon.bottom() - visible_top).max(0.),
        ..icon
    };
    let pixels = image
        .scan(exposed_artwork, opaque_rgb(&artwork["color"])?, None)
        .pixels;
    let reference = artwork["referencePixels"]
        .as_u64()
        .context("unobstructed Inbox reference ink")? as usize;
    check!(
        reference > 32 && pixels * 5 >= reference * 4,
        "the enlarged count leaves at least 80% of the Inbox ink below it, got {pixels}/{reference} pixels"
    );
    Ok(
        json!({"visible_ink_pixels":pixels,"reference_ink_pixels":reference,"badge_overlap_dp":overlap}),
    )
}

async fn navigation_and_scroll(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M35",
        "Navigation and clipped anchors preserve selection and paint scope",
    );
    kind(tab, "count").await?;
    let home = element("badge-nav-0");
    let inbox = element(NAV_INBOX);
    let archive = element("badge-nav-2");
    reveal(tab, NAV_INBOX).await?;
    tab.wait_for(&format!("{home}?.getAttribute('aria-current') === 'page' &&
         {inbox}?.getAttribute('role') === 'button' && {archive}?.getAttribute('aria-disabled') === 'true'"))
        .await?;
    visible_click(tab, NAV_INBOX).await?;
    ready(tab, "badge.nav === 1").await?;
    until(tab, "s.focused === 'badge-nav-1'").await?;
    tab.wait_for(&format!(
        "{inbox}?.getAttribute('aria-current') === 'page' &&
        !{home}?.hasAttribute('aria-current') && !{archive}?.hasAttribute('aria-current')"
    ))
    .await?;
    let nav_artwork = navigation_artwork(env, tab).await?;
    kind(tab, "dot").await?;
    ready(tab, "badge.nav === 1").await?;
    reveal(tab, NAV_INBOX).await?;
    tab.wait_for(&format!("{inbox}?.getAttribute('aria-current') === 'page'"))
        .await?;

    reveal(tab, SCROLL).await?;
    let region = point(tab, SCROLL).await?;
    let viewport = Rect::from_point(&region);
    let owner = Rect::from_point(&point(tab, SCROLL_OWNER).await?);
    let (fill, _) = colors(tab).await?;
    let image = screenshot(env, tab, "material-badge-scroll-visible.png").await?;
    let before = image
        .scan(viewport, fill, Some(owner))
        .bounds
        .context("visible scroll badge")?;
    check!(
        before.x >= viewport.x && before.right() <= viewport.right(),
        "fitted badge stays inside the scrolling region"
    );
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("pixel ratio")?;
    tab.mouse_move(region.x, region.y).await?;
    tab.wheel(0., 96. * scale).await?;
    tab.wait_for(&format!(
        "(() => {{
            const owner = window.gallery.control_bounds({});
            const clip = window.gallery.control_bounds({});
            return owner[1] + owner[3] <= clip[1];
        }})()",
        js(SCROLL_OWNER),
        js(SCROLL)
    ))
    .await?;
    let image = screenshot(env, tab, "material-badge-scroll-hidden.png").await?;
    let viewport = Rect::from_point(&point(tab, SCROLL).await?);
    check_eq!(
        image.scan(viewport, fill, None).pixels,
        0,
        "clipped owner leaves no badge paint"
    );
    tab.wheel(0., -96. * scale).await?;
    tab.wait_for(&format!(
        "(() => {{
            const owner = window.gallery.control_bounds({});
            const clip = window.gallery.control_bounds({});
            return Math.abs(owner[1] - clip[1]) <= 1;
        }})()",
        js(SCROLL_OWNER),
        js(SCROLL)
    ))
    .await?;
    let image = screenshot(env, tab, "material-badge-scroll-restored.png").await?;
    let viewport = Rect::from_point(&point(tab, SCROLL).await?);
    let after = image
        .scan(viewport, fill, None)
        .bounds
        .context("restored scroll badge")?;
    check!(
        (after.width - before.width).abs() <= 1. && (after.height - before.height).abs() <= 1.,
        "returning the owner restores the fitted badge size"
    );
    ready(tab, "badge.nav === 1").await?;
    Ok(json!({"nav":1,"nav_artwork":nav_artwork,
        "badge_initially_visible":true,"clipped_paint":false,"restored":true}))
}

async fn journey(env: &Env, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    let adapter = tab
        .eval(
            "(async () => {
            const adapter = await navigator.gpu.requestAdapter();
            if (!adapter) return null;
            const {vendor, architecture, isFallbackAdapter} = adapter.info;
            return {vendor, architecture, isFallbackAdapter};
        })()",
        )
        .await?;
    check!(!adapter.is_null(), "WebGPU adapter is available");
    let light = light(env, tab).await?;
    let interaction = interaction(env, tab).await?;
    let compact = compact(env, tab, &light).await?;
    let navigation_and_scroll = navigation_and_scroll(env, tab).await?;
    check!(
        tab.errors().is_empty(),
        "Badge page errors: {:?}",
        tab.errors()
    );
    env.write_json(
        "browser-material-badge.json",
        &json!({"adapter":adapter,"light":light,"interaction":interaction,"compact":compact,
            "navigation_and_scroll":navigation_and_scroll,"state":state(tab).await?["badges"]}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, &tab).await;
    if result.is_err() {
        let dump = tab.eval("({state: JSON.parse(window.gallery.inspect()),
            dom: JSON.parse(window.gallery.export_dom()), focus: document.activeElement?.outerHTML})")
            .await.unwrap_or_else(|error| json!({"error": error.to_string()}));
        let _ = env.write_json("material-badge-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-badge-failure.png"))
            .await;
    }
    result
}
