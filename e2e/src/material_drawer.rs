//! Shared-space and modal navigation drawers in the WebGPU Gallery.

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

const PANEL: &str = "drawer-demo";
const SCRIM: &str = "drawer-demo-layer";
const EDGE: &str = "drawer-demo-swipe-edge";
const OPEN: &str = "drawer-demo-open";
const CLOSE: &str = "drawer-demo-close";
const HEADER: &str = "drawer-demo-header";
const NAVIGATION: &str = "drawer-demo-navigation";
const BACKGROUND: &str = "drawer-demo-background";
const HOME: &str = "drawer-demo-home";
const ARCHIVE: &str = "drawer-demo-archive";
const INBOX: &str = "drawer-demo-inbox";
const REPORTS: &str = "drawer-demo-reports";
const STANDARD: &str = "drawer-standard";
const LAYOUT: &str = "drawer-standard-layout";
const CONTENT: &str = "drawer-standard-content";
const TOGGLE: &str = "drawer-demo-standard-toggle";
const SHARED: &str = "drawer-demo-shared";

async fn drawer(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["navigation_drawer"].clone())
}

async fn ready(tab: &Tab, predicate: &str) -> Result<()> {
    until(tab, &format!("(d => {predicate})(s.navigation_drawer)")).await?;
    tab.settle().await
}

async fn endpoint(tab: &Tab, open: bool) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.open === {open} && !d.settling && d.progress === {}",
            u8::from(open)
        ),
    )
    .await
}

async fn visible(tab: &Tab, id: &str) -> Result<()> {
    tab.wait_for(&format!(
        "(() => {{
            const [x,y,w,h] = window.gallery.control_bounds({});
            const c = document.querySelector('canvas').getBoundingClientRect();
            return w > 0 && h > 0 && x >= -1 && y >= -1 &&
                x+w <= c.width+1 && y+h <= c.height+1;
        }})()",
        js(id)
    ))
    .await
}

async fn visible_click(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    if p.b[1] < 8. || p.b[1] + p.b[3] > p.canvas[3] - 8. {
        let scale = tab.eval("devicePixelRatio").await?;
        // The page margin is outside nested scroll containers.
        tab.mouse_move(p.canvas[0] + 10., p.canvas[1] + 26.).await?;
        tab.wheel(0., (p.b[1] - 32.) * scale.as_f64().context("pixel ratio")?)
            .await?;
    }
    visible(tab, id).await?;
    click(tab, id).await
}

async fn reveal_in_scroll(tab: &Tab, id: &str, viewport: &str) -> Result<()> {
    let region = point(tab, viewport).await?;
    let target = point(tab, id).await?;
    if target.b[1] < region.b[1] || target.b[1] + target.b[3] > region.b[1] + region.b[3] {
        let scale = tab.eval("devicePixelRatio").await?;
        tab.mouse_move(region.x, region.y).await?;
        tab.wheel(
            0.,
            (target.y - region.y) * scale.as_f64().context("pixel ratio")?,
        )
        .await?;
    }
    tab.wait_for(&format!(
        "(() => {{
            const [,y,,h] = window.gallery.control_bounds({});
            const [,top,,height] = window.gallery.control_bounds({});
            return h > 0 && y >= top-1 && y+h <= top+height+1;
        }})()",
        js(id),
        js(viewport)
    ))
    .await
}

async fn click_preview_control(tab: &Tab, id: &str) -> Result<()> {
    reveal_in_scroll(tab, id, "drawer-demo-controls").await?;
    click(tab, id).await
}

fn element(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn browser_focus(tab: &Tab, id: &str) -> Result<()> {
    runtime_focus(tab, id).await?;
    tab.wait_for(&format!("document.activeElement === {}", element(id)))
        .await
}

async fn project_focus(tab: &Tab, id: &str) -> Result<()> {
    let target = element(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    browser_focus(tab, id).await
}

async fn viewport(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const c = document.querySelector('canvas');
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return c.width === {pixel_width} && c.height === {pixel_height} &&
                   v?.width === {width} && v.height === {height};
        }})()",
        pixel_width = width * 2,
        pixel_height = height * 2,
    ))
    .await?;
    tab.settle().await
}

async fn export(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

fn ax_node<'a>(tree: &'a Value, role: &str, name: &str) -> Result<&'a Value> {
    tree["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
                node["ignored"] == false
                    && node["role"]["value"] == role
                    && node["name"]["value"] == name
            })
        })
        .with_context(|| format!("missing Chrome {role} named {name}"))
}

fn ax_property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|item| item["name"] == name)
        .map(|item| &item["value"]["value"])
}

fn ax_contains(tree: &Value, ancestor: &Value, child: &Value) -> bool {
    let Some(nodes) = tree["nodes"].as_array() else {
        return false;
    };
    let mut parent = &child["parentId"];
    while let Some(node) = nodes.iter().find(|node| node["nodeId"] == *parent) {
        if node["nodeId"] == ancestor["nodeId"] {
            return true;
        }
        parent = &node["parentId"];
    }
    false
}

struct Screenshot {
    bytes: Vec<u8>,
    frame: png::OutputInfo,
    channels: usize,
    scale: f64,
}

impl Screenshot {
    async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Self> {
        tab.settle().await?;
        let path = env.artifact(&format!("material-navigation-drawer-{name}.png"));
        tab.screenshot(&path).await?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![0; reader.output_buffer_size().context("PNG output size")?];
        let frame = reader.next_frame(&mut bytes)?;
        let channels = match frame.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG type {other:?}"),
        };
        let width = tab.eval("window.innerWidth").await?;
        let scale = f64::from(frame.width) / width.as_f64().context("browser page width")?;
        Ok(Self {
            bytes,
            frame,
            channels,
            scale,
        })
    }

    fn rgb(&self, x: f64, y: f64) -> Result<[u8; 3]> {
        let col = (x * self.scale).floor() as usize;
        let row = (y * self.scale).floor() as usize;
        if col >= self.frame.width as usize || row >= self.frame.height as usize {
            bail!("sample ({x}, {y}) is outside Chrome's screenshot");
        }
        let offset = row * self.frame.line_size + col * self.channels;
        Ok(self.bytes[offset..offset + 3].try_into()?)
    }
}

fn color(node: &Value, key: &str) -> Result<[u8; 4]> {
    serde_json::from_value(node["style"][key].clone()).with_context(|| format!("style {key}"))
}

fn rgb([red, green, blue, _]: [u8; 4]) -> [u8; 3] {
    [red, green, blue]
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn blend(color: [u8; 3], [red, green, blue, alpha]: [u8; 4]) -> [u8; 3] {
    let opacity = f64::from(alpha) / 255.;
    std::array::from_fn(|channel| {
        (f64::from(color[channel]) * (1. - opacity)
            + f64::from([red, green, blue][channel]) * opacity)
            .round() as u8
    })
}

fn next(before: &Value, field: &str) -> Result<u64> {
    Ok(before[field]
        .as_u64()
        .with_context(|| format!("missing {field}"))?
        + 1)
}

async fn current_page(tab: &Tab, id: &str, old: &str) -> Result<()> {
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-current') === 'page' && !{}?.hasAttribute('aria-current')",
        element(id),
        element(old)
    ))
    .await
}

async fn standard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M36",
        "permanent and dismissible drawers share space and preserve routing",
    );
    ready(tab, "d.standard_mode === 'permanent' && d.standard_open").await?;
    let pane = point(tab, STANDARD).await?;
    let layout = point(tab, LAYOUT).await?;
    let page = point(tab, CONTENT).await?;
    check_eq!(
        pane.b[2],
        360.,
        "wide standard drawer uses the 360 dp width"
    );
    check!(
        (layout.b[0] - pane.b[0]).abs() <= 1.
            && (layout.b[3] - pane.b[3]).abs() <= 1.
            && pane.b[0] + pane.b[2] <= page.b[0] + 1.
            && page.b[0] + page.b[2] <= layout.b[0] + layout.b[2] + 1.,
        "drawer and content share the allocated width: pane {:?}, content {:?}, layout {:?}",
        pane.b,
        page.b,
        layout.b
    );
    check_eq!(
        point(tab, EDGE).await?.b[2],
        24.,
        "the separate modal sample exposes a leading gesture edge"
    );
    let home = "drawer-standard-home";
    let archive = "drawer-standard-archive";
    let inbox = "drawer-standard-inbox";
    let before = drawer(tab).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-current') === 'page' && {}?.getAttribute('aria-disabled') === 'true' &&
        !document.querySelector('#surface-accessibility [aria-modal=true]')",
        element(home), element(archive)
    )).await?;
    click(tab, archive).await?;
    tab.settle().await?;
    check_eq!(
        drawer(tab).await?["standard_actions"],
        before["standard_actions"],
        "disabled pointer target does not route"
    );
    project_focus(tab, home).await?;
    tab.press("ArrowDown").await?;
    browser_focus(tab, inbox).await?;
    check_eq!(
        drawer(tab).await?["standard_selected"],
        before["standard_selected"],
        "moving focus does not route"
    );
    tab.press("Enter").await?;
    ready(
        tab,
        &format!(
            "d.standard_selected === 'inbox' && d.standard_actions === {}",
            next(&before, "standard_actions")?
        ),
    )
    .await?;
    current_page(tab, inbox, home).await?;

    visible_click(tab, SHARED).await?;
    ready(
        tab,
        &format!(
            "d.standard_open && d.shared_clicks === {}",
            next(&before, "shared_clicks")?
        ),
    )
    .await?;
    Screenshot::capture(env, tab, "standard-permanent").await?;

    visible_click(tab, "drawer-demo-dismissible").await?;
    ready(tab, "d.standard_mode === 'dismissible'").await?;
    if drawer(tab).await?["standard_open"] == false {
        visible_click(tab, TOGGLE).await?;
        ready(tab, "d.standard_open").await?;
    }
    let expanded = point(tab, CONTENT).await?;
    check!(
        (expanded.b[2] - page.b[2]).abs() <= 1.,
        "open dismissible drawer reserves the same space as permanent"
    );
    current_page(tab, inbox, home).await?;
    project_focus(tab, TOGGLE).await?;
    tab.press("Enter").await?;
    ready(tab, "!d.standard_open").await?;
    browser_focus(tab, TOGGLE).await?;
    tab.wait_for(&format!("{} === null", element(inbox)))
        .await?;
    let full = point(tab, CONTENT).await?;
    check!(
        full.b[2] >= expanded.b[2] + 350.,
        "dismissed drawer gives the content its space back: {:?} → {:?}",
        expanded.b,
        full.b
    );
    Screenshot::capture(env, tab, "standard-dismissed").await?;
    tab.press("Space").await?;
    ready(tab, "d.standard_open && d.standard_selected === 'inbox'").await?;
    current_page(tab, inbox, home).await?;
    visible_click(tab, "drawer-demo-permanent").await?;
    ready(tab, "d.standard_mode === 'permanent'").await?;
    Ok(json!({
        "panel":pane.b,"layout":layout.b,"expanded_content":expanded.b,
        "dismissed_content":full.b,"selection":drawer(tab).await?,
    }))
}

async fn modal(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M37",
        "modal drawer isolates its destinations and restores focus",
    );
    visible_click(tab, BACKGROUND).await?;
    let behind = point(tab, BACKGROUND).await?;
    tab.mouse_move(behind.x, behind.y).await?;
    let before = drawer(tab).await?;
    let base = Screenshot::capture(env, tab, "modal-background").await?;
    project_focus(tab, OPEN).await?;
    tab.press("Enter").await?;
    endpoint(tab, true).await?;
    browser_focus(tab, CLOSE).await?;
    let panel = point(tab, PANEL).await?;
    check_eq!(
        panel.b,
        [0., 0., 360., panel.canvas[3]],
        "wide LTR modal bounds"
    );
    check!(
        behind.x >= panel.canvas[0] + panel.b[2],
        "background activation probe is under the scrim and outside the drawer"
    );
    tab.wait_for(&format!(
        "{} === null && {} === null &&
         {}?.getAttribute('aria-current') === 'page' &&
         {}?.getAttribute('aria-disabled') === 'true'",
        element(OPEN),
        element(BACKGROUND),
        element(HOME),
        element(ARCHIVE)
    ))
    .await?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let dialog = ax_node(&tree, "dialog", "導覽抽屜")?;
    let nav = ax_node(&tree, "navigation", "工作區")?;
    let archive = ax_node(&tree, "button", "封存")?;
    check_eq!(
        ax_property(dialog, "modal"),
        Some(&json!(true)),
        "Chrome modal flag"
    );
    check_eq!(
        ax_property(archive, "disabled"),
        Some(&json!(true)),
        "Chrome disabled flag"
    );
    check!(
        ax_contains(&tree, dialog, nav) && ax_contains(&tree, nav, archive),
        "Chrome exposes the dialog, navigation and disabled destination in order"
    );
    let dimmed = Screenshot::capture(env, tab, "modal-open").await?;
    let authored = export(tab).await?;
    let scrim = color(find_node(&authored, SCRIM)?, "background")?;
    check_eq!(scrim[3], 82, "Material drawer scrim opacity");
    let probe = (panel.canvas[0] + panel.canvas[2] - 6., panel.canvas[1] + 6.);
    let plain = base.rgb(probe.0, probe.1)?;
    let shaded = dimmed.rgb(probe.0, probe.1)?;
    let expected = blend(plain, scrim);
    check!(
        distance(shaded, expected) <= 8 && distance(shaded, plain) > 10,
        "WebGPU scrim dims real content: {shaded:?}, expected {expected:?}, before {plain:?}"
    );

    tab.press("Shift+Tab").await?;
    browser_focus(tab, HOME).await?;
    tab.press("Tab").await?;
    browser_focus(tab, CLOSE).await?;
    tab.press("Tab").await?;
    browser_focus(tab, HOME).await?;
    tab.press("ArrowDown").await?;
    browser_focus(tab, INBOX).await?;
    check_eq!(
        drawer(tab).await?["selected"],
        before["selected"],
        "arrow keys move focus"
    );
    tab.press("Escape").await?;
    endpoint(tab, false).await?;
    browser_focus(tab, OPEN).await?;
    ready(
        tab,
        &format!("d.dismissals === {}", next(&before, "dismissals")?),
    )
    .await?;

    visible_click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    runtime_focus(tab, CLOSE).await?;
    let outside = drawer(tab).await?;
    tab.click(behind.x, behind.y).await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, OPEN).await?;
    ready(
        tab,
        &format!("d.dismissals === {}", next(&outside, "dismissals")?),
    )
    .await?;
    check_eq!(
        drawer(tab).await?["background_actions"],
        before["background_actions"],
        "clicking outside the panel does not activate the covered button"
    );

    visible_click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    let action = drawer(tab).await?;
    click(tab, INBOX).await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, OPEN).await?;
    ready(
        tab,
        &format!(
            "d.selected === 'inbox' && d.actions === {}",
            next(&action, "actions")?
        ),
    )
    .await?;
    visible_click(tab, OPEN).await?;
    endpoint(tab, true).await?;
    current_page(tab, INBOX, HOME).await?;
    click(tab, CLOSE).await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, OPEN).await?;
    Ok(json!({
        "panel":panel.b,"dialog_ax":dialog,"navigation_ax":nav,"disabled_ax":archive,
        "scrim_before":plain,"scrim_after":shaded,"state":drawer(tab).await?,
    }))
}

fn rect(value: &Value) -> Result<[f64; 4]> {
    let get = |field: &str| {
        value[field]
            .as_f64()
            .with_context(|| format!("missing bounds {field}"))
    };
    Ok([get("x")?, get("y")?, get("width")?, get("height")?])
}

fn report_label(dom: &Value) -> Result<(&Value, &Value)> {
    let nodes = dom["dom"]["nodes"].as_array().context("authored nodes")?;
    let parent = nodes
        .iter()
        .position(|n| n["stable_id"] == REPORTS)
        .context("authored Reports destination")?;
    let (index, label) = nodes
        .iter()
        .enumerate()
        .find(|(_, node)| node["parent"] == parent && node["text"].is_string())
        .context("Chinese Reports label")?;
    let layout = dom["layout"]["nodes"]
        .as_array()
        .and_then(|entries| entries.iter().find(|entry| entry["id"] == index))
        .context("Reports label layout")?;
    Ok((label, layout))
}

fn wrapped_ink(dom: &Value, image: &Screenshot, canvas: [f64; 4]) -> Result<Vec<usize>> {
    let (label, layout) = report_label(dom)?;
    let [x, y, width, height] = rect(&layout["bounds"])?;
    let [clip_x, clip_y, clip_width, clip_height] = rect(&layout["clip"])?;
    check!(
        x >= clip_x - 1.
            && y >= clip_y - 1.
            && x + width <= clip_x + clip_width + 1.
            && y + height <= clip_y + clip_height + 1.,
        "Reports wraps within the visible scrolling area"
    );
    let line_height = label["style"]["line_height"]
        .as_f64()
        .context("Reports line height")?
        * 2.;
    let lines = (height / line_height).round() as usize;
    check!(
        lines >= 2 && (height - lines as f64 * line_height).abs() <= 1.,
        "200% Reports text wraps at 320 dp across {lines} whole lines"
    );
    let color = rgb(color(label, "foreground")?);
    let x = ((canvas[0] + x) * image.scale).ceil() as usize;
    let width = (width * image.scale).floor() as usize;
    let mut ink = Vec::with_capacity(lines);
    for line in 0..lines {
        let top = ((canvas[1] + y + line as f64 * line_height) * image.scale).ceil() as usize;
        let bottom = top + (line_height * image.scale).floor() as usize;
        let mut count = 0;
        for row in top..bottom.min(image.frame.height as usize) {
            for col in x..(x + width).min(image.frame.width as usize) {
                let offset = row * image.frame.line_size + col * image.channels;
                let pixel = image.bytes[offset..offset + 3].try_into()?;
                if distance(pixel, color) <= 8 {
                    count += 1;
                }
            }
        }
        ink.push(count);
    }
    check!(
        ink.iter().all(|count| *count > 20),
        "each Chinese line paints text: {ink:?}"
    );
    Ok(ink)
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M38",
        "320 dp RTL at 200% keeps shared controls visible and reveals wrapped destinations",
    );
    visible_click(tab, "drawer-demo-rtl").await?;
    ready(tab, "d.rtl").await?;
    visible_click(tab, "drawer-demo-scale").await?;
    ready(tab, "d.text_scale === 2").await?;
    visible_click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    viewport(tab, 320, 560).await?;

    let layout = point(tab, LAYOUT).await?;
    let standard = point(tab, STANDARD).await?;
    let content = point(tab, CONTENT).await?;
    check!(
        standard.b[2] > 0.
            && standard.b[2] <= layout.b[2] * 0.6 + 1.
            && layout.b[3] >= 112.
            && (standard.b[3] - layout.b[3]).abs() <= 1.
            && (standard.b[0] + standard.b[2] - layout.b[0] - layout.b[2]).abs() <= 1.
            && (content.b[0] + content.b[2] - standard.b[0]).abs() <= 1.,
        "RTL standard drawer stays at the right and leaves room for content: {:?} / {:?}",
        standard.b,
        content.b
    );
    reveal_in_scroll(tab, "drawer-standard-inbox", STANDARD).await?;
    project_focus(tab, "drawer-standard-inbox").await?;
    visible(tab, "drawer-standard-inbox").await?;
    let shared = drawer(tab).await?;
    visible_click(tab, SHARED).await?;
    ready(
        tab,
        &format!(
            "d.shared_clicks === {} && d.standard_selected === 'inbox'",
            next(&shared, "shared_clicks")?
        ),
    )
    .await?;
    Screenshot::capture(env, tab, "standard-rtl-320-200").await?;

    click_preview_control(tab, OPEN).await?;
    endpoint(tab, true).await?;
    viewport(tab, 320, 240).await?;
    let panel = point(tab, PANEL).await?;
    check_eq!(
        panel.b,
        [0., 0., 320., 240.],
        "compact modal spans the canvas"
    );
    let header = point(tab, HEADER).await?.b;
    let before = drawer(tab).await?;
    tab.press("Tab").await?;
    runtime_focus(tab, INBOX).await?;
    tab.press("ArrowDown").await?;
    runtime_focus(tab, REPORTS).await?;
    ready(
        tab,
        &format!(
            "d.scroll > {}",
            before["scroll"].as_f64().context("initial drawer scroll")?
        ),
    )
    .await?;
    visible(tab, REPORTS).await?;
    let report = point(tab, REPORTS).await?;
    let nav = point(tab, NAVIGATION).await?;
    check_eq!(
        point(tab, HEADER).await?.b,
        header,
        "scroll leaves the header in place"
    );
    check!(
        nav.b[1] >= header[1] + header[3] && report.b[1] >= nav.b[1] - 1. && report.b[3] >= 56.,
        "focused row stays inside the scroll area"
    );
    let authored = export(tab).await?;
    let image = Screenshot::capture(env, tab, "modal-rtl-320-200-wrapped").await?;
    let fill = image.rgb(panel.canvas[0] + 5., panel.canvas[1] + 5.)?;
    check!(
        distance(
            fill,
            rgb(color(find_node(&authored, PANEL)?, "background")?)
        ) <= 8,
        "compact drawer paints the dark theme surface"
    );
    let ink = wrapped_ink(&authored, &image, panel.canvas)?;
    let scrolled = drawer(tab).await?;
    tab.press("Enter").await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, OPEN).await?;
    ready(
        tab,
        &format!(
            "d.selected === 'reports' && d.actions === {}",
            next(&before, "actions")?
        ),
    )
    .await?;
    Ok(json!({
        "canvas":panel.canvas,"layout":layout.b,"standard":standard.b,"page":content.b,
        "header":header,"navigation":nav.b,"report":report.b,"wrapped_ink":ink,
        "initial_scroll":before["scroll"],"focused_scroll":scrolled["scroll"],"state":drawer(tab).await?,
    }))
}

#[derive(Clone, Copy)]
enum Input {
    Mouse,
    Touch,
}

impl Input {
    fn name(self) -> &'static str {
        match self {
            Self::Mouse => "mouse",
            Self::Touch => "touch",
        }
    }

    async fn configure(self, tab: &Tab, enabled: bool) -> Result<()> {
        if matches!(self, Self::Touch) {
            tab.cdp(
                "Emulation.setTouchEmulationEnabled",
                json!({"enabled":enabled, "maxTouchPoints":2}),
            )
            .await?;
        }
        Ok(())
    }

    async fn start(self, tab: &Tab, (x, y): (f64, f64)) -> Result<()> {
        match self {
            Self::Mouse => {
                tab.mouse_move(x, y).await?;
                tab.mouse_down().await
            }
            Self::Touch => touch(tab, "touchStart", Some((x, y))).await,
        }
    }

    async fn drag(self, tab: &Tab, (x, y): (f64, f64)) -> Result<()> {
        match self {
            Self::Mouse => mouse(tab, "mouseMoved", x, y, 1).await,
            Self::Touch => touch(tab, "touchMove", Some((x, y))).await,
        }
    }

    async fn release(self, tab: &Tab, (x, y): (f64, f64)) -> Result<()> {
        // Let the velocity sample expire so distance determines the anchor.
        tokio::time::sleep(Duration::from_millis(140)).await;
        match self {
            Self::Mouse => mouse(tab, "mouseReleased", x, y, 0).await,
            Self::Touch => touch(tab, "touchEnd", None).await,
        }
    }
}

async fn mouse(tab: &Tab, phase: &str, x: f64, y: f64, buttons: u8) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":phase,"x":x,"y":y,"buttons":buttons,
            "button":"left","clickCount":1}),
    )
    .await?;
    Ok(())
}

async fn touch(tab: &Tab, phase: &str, point: Option<(f64, f64)>) -> Result<()> {
    let touches: Vec<_> = point
        .into_iter()
        .map(|(x, y)| json!({"x":x, "y":y, "id":1}))
        .collect();
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":phase, "touchPoints":touches}),
    )
    .await?;
    Ok(())
}

fn along((x, y): (f64, f64), distance: f64) -> (f64, f64) {
    (x + distance, y)
}

async fn audit_input(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            window.drawerEvents = [];
            if (window.drawerListening) return;
            window.drawerListening = true;
            for (const kind of ['pointerdown','pointermove','pointerup','pointercancel']) {
                document.addEventListener(kind, e => {
                    if (e.target?.id === 'surface') {
                        window.drawerEvents.push({kind, device:e.pointerType, trusted:e.isTrusted});
                    }
                }, {capture:true, passive:true});
            }
        })()",
    )
    .await?;
    Ok(())
}

async fn leading_edge(tab: &Tab, rtl: bool) -> Result<Point> {
    let edge = point(tab, EDGE).await?;
    let x = if rtl { edge.canvas[2] - 24. } else { 0. };
    check_eq!(
        edge.b,
        [x, 0., 24., edge.canvas[3]],
        "24 dp logical leading edge"
    );
    Ok(edge)
}

async fn held_preview(
    env: &Env,
    tab: &Tab,
    rtl: bool,
    label: &str,
    base: &Screenshot,
) -> Result<Value> {
    let panel = point(tab, PANEL).await?;
    let [left, top, width, height] = panel.canvas;
    let progress = drawer(tab).await?["progress"]
        .as_f64()
        .context("drawer progress")?;
    let panel_width = width.min(360.);
    let hidden = (1. - progress) * panel_width;
    let x = if rtl {
        width - panel_width + hidden
    } else {
        -hidden
    };
    check!(
        (panel.b[0] - x).abs() <= 1.
            && panel.b[1] == 0.
            && panel.b[2] == panel_width
            && panel.b[3] == height,
        "panel follows the held drag: {:?}, expected x {x}",
        panel.b
    );
    let authored = export(tab).await?;
    let scrim = color(find_node(&authored, SCRIM)?, "background")?;
    check!(
        (f64::from(scrim[3]) - 82. * progress).abs() <= 1.,
        "scrim opacity follows progress {progress}: {scrim:?}"
    );
    let image = Screenshot::capture(env, tab, label).await?;
    let outside = if rtl { 6. } else { width - 6. };
    let before = base.rgb(left + outside, top + height / 2.)?;
    let after = image.rgb(left + outside, top + height / 2.)?;
    let expected = blend(before, scrim);
    let error = distance(after, expected);
    check!(
        error <= 8 && error < distance(before, expected),
        "held scrim paints the probed pixel: {after:?}, expected {expected:?}, before {before:?}"
    );
    let face = image.rgb(left + width - outside, top + 6.)?;
    let expected_face = rgb(color(find_node(&authored, PANEL)?, "background")?);
    check!(
        distance(face, expected_face) <= 8,
        "held panel paints the exposed face: {face:?}, expected {expected_face:?}"
    );
    Ok(
        json!({"progress":progress,"panel":panel.b,"scrim":scrim,"previous_pixel":before,
        "scrim_pixel":after,"expected_scrim_pixel":expected,"panel_pixel":face}),
    )
}

async fn close_drag_origin(tab: &Tab, rtl: bool) -> Result<(f64, f64)> {
    let panel = point(tab, PANEL).await?;
    let header = point(tab, HEADER).await?;
    let fraction = if rtl { 0.2 } else { 0.8 };
    Ok((
        panel.canvas[0] + panel.b[0] + panel.b[2] * fraction,
        header.y,
    ))
}

async fn swipe(env: &Env, tab: &Tab, input: Input, rtl: bool) -> Result<Value> {
    click_preview_control(tab, BACKGROUND).await?;
    runtime_focus(tab, BACKGROUND).await?;
    let edge = leading_edge(tab, rtl).await?;
    let direction = if rtl { -1. } else { 1. };
    let span = direction * edge.canvas[2].min(360.);
    let origin = (edge.x, edge.y);
    let tag = format!("{}-{}", input.name(), if rtl { "rtl" } else { "ltr" });
    let before = drawer(tab).await?;
    audit_input(tab).await?;
    let base = Screenshot::capture(env, tab, &format!("{tag}-before")).await?;
    input.configure(tab, true).await?;

    input.start(tab, origin).await?;
    let short = along(origin, span * 0.22);
    input.drag(tab, short).await?;
    ready(
        tab,
        "!d.open && !d.settling && d.progress > 0.15 && d.progress < 0.3",
    )
    .await?;
    runtime_focus(tab, BACKGROUND).await?;
    tab.wait_for(&format!(
        "{} === null && {} !== null",
        element(PANEL),
        element(OPEN)
    ))
    .await?;
    input.release(tab, short).await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, BACKGROUND).await?;

    if matches!(input, Input::Touch) {
        input.start(tab, origin).await?;
        input.drag(tab, along(origin, span * 0.68)).await?;
        ready(tab, "!d.open && d.progress > 0.6").await?;
        touch(tab, "touchCancel", None).await?;
        endpoint(tab, false).await?;
        runtime_focus(tab, BACKGROUND).await?;
    }

    let far = along(origin, span * 0.68);
    input.start(tab, origin).await?;
    input.drag(tab, far).await?;
    ready(
        tab,
        "!d.open && !d.settling && d.progress > 0.6 && d.progress < 0.75",
    )
    .await?;
    runtime_focus(tab, BACKGROUND).await?;
    let opening = held_preview(env, tab, rtl, &format!("{tag}-opening-held"), &base).await?;
    tab.wait_for(&format!(
        "{} === null && {} !== null",
        element(PANEL),
        element(OPEN)
    ))
    .await?;
    input.release(tab, far).await?;
    endpoint(tab, true).await?;
    runtime_focus(tab, CLOSE).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-modal') === 'true' && {} === null",
        element(PANEL),
        element(BACKGROUND)
    ))
    .await?;

    let origin = close_drag_origin(tab, rtl).await?;
    let far = along(origin, -span * 0.63);
    input.start(tab, origin).await?;
    input.drag(tab, far).await?;
    ready(
        tab,
        "d.open && !d.settling && d.progress > 0.3 && d.progress < 0.45",
    )
    .await?;
    runtime_focus(tab, CLOSE).await?;
    tab.wait_for(&format!("{} === null", element(BACKGROUND)))
        .await?;
    let closing = held_preview(env, tab, rtl, &format!("{tag}-closing-held"), &base).await?;
    input.release(tab, far).await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, BACKGROUND).await?;
    input.configure(tab, false).await?;

    let after = drawer(tab).await?;
    for field in ["selected", "actions", "background_actions"] {
        check_eq!(
            after[field],
            before[field],
            format!("{tag} preserves {field}")
        );
    }
    check_eq!(
        after["dismissals"],
        next(&before, "dismissals")?,
        "only committed close dismisses"
    );
    let events = tab.eval("window.drawerEvents").await?;
    let events = events.as_array().context("trusted pointer events")?;
    let seen = |kind: &str| {
        events.iter().any(|event| {
            event["kind"] == kind && event["device"] == input.name() && event["trusted"] == true
        })
    };
    for kind in ["pointerdown", "pointermove", "pointerup"] {
        check!(seen(kind), "{tag} used trusted {kind} events");
    }
    if matches!(input, Input::Touch) {
        check!(
            seen("pointercancel"),
            "Chrome delivered a trusted touch cancellation"
        );
    }
    Ok(json!({"before":before,"opening":opening,"closing":closing,"after":after,"events":events}))
}

async fn reduced_motion(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M40",
        "open drawer reflows on resize and Reduce Motion retires the closing layer",
    );
    click_preview_control(tab, OPEN).await?;
    endpoint(tab, true).await?;
    runtime_focus(tab, CLOSE).await?;
    viewport(tab, 960, 760).await?;
    let panel = point(tab, PANEL).await?;
    check_eq!(
        panel.b,
        [600., 0., 360., 760.],
        "open RTL panel reflows at logical start"
    );
    runtime_focus(tab, CLOSE).await?;
    current_page(tab, REPORTS, INBOX).await?;
    let before = drawer(tab).await?;
    let origin = close_drag_origin(tab, true).await?;
    let far = along(origin, panel.b[2] * 0.63);
    Input::Mouse.start(tab, origin).await?;
    Input::Mouse.drag(tab, far).await?;
    ready(tab, "d.open && d.progress > 0.3 && d.progress < 0.45").await?;

    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    tab.wait_for("matchMedia('(prefers-reduced-motion: reduce)').matches")
        .await?;
    tab.settle().await?;
    // A fixed clock makes an unintended animated close impossible to hide
    // on a slow runner. Real input, rendering and the media query still run.
    tab.eval(
        "(() => {
        const now = performance.now();
        Object.defineProperty(performance, 'now', {configurable:true, value:() => now});
    })()",
    )
    .await?;
    Input::Mouse.release(tab, far).await?;
    endpoint(tab, false).await?;
    runtime_focus(tab, OPEN).await?;
    tab.eval("delete performance.now").await?;
    let dom = export(tab).await?;
    check!(
        find_node(&dom, PANEL).is_err() && find_node(&dom, SCRIM).is_err(),
        "Reduce Motion removes panel and scrim without clock advancement"
    );
    let after = drawer(tab).await?;
    check_eq!(
        after["dismissals"],
        next(&before, "dismissals")?,
        "one reduced-motion dismissal"
    );
    Screenshot::capture(env, tab, "reduced-motion-closed").await?;
    Ok(json!({"resized_panel":panel.b,"restored_focus":OPEN,"before":before,"after":after}))
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    viewport(tab, 960, 760).await?;
    visible_click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    visible_click(tab, "drawer-demo-show").await?;
    endpoint(tab, false).await?;
    let adapter = tab.eval(
        "(async () => { const a = await navigator.gpu.requestAdapter(); if (!a) return null;
          const {vendor,architecture,isFallbackAdapter}=a.info;return {vendor,architecture,isFallbackAdapter}; })()"
    ).await?;
    check!(!adapter.is_null(), "WebGPU adapter is available");
    let standard = standard(env, tab).await?;
    let modal = modal(env, tab).await?;
    let compact = compact(env, tab).await?;

    step(
        "M39",
        "trusted mouse and touch drag the logical edge, panel, and scrim",
    );
    let rtl = swipe(env, tab, Input::Touch, true).await?;
    viewport(tab, 960, 760).await?;
    visible_click(tab, "drawer-demo-rtl").await?;
    ready(tab, "!d.rtl").await?;
    visible_click(tab, "theme").await?;
    until(tab, "!s.dark").await?;
    let ltr = swipe(env, tab, Input::Mouse, false).await?;
    visible_click(tab, "drawer-demo-rtl").await?;
    ready(tab, "d.rtl").await?;
    viewport(tab, 320, 240).await?;
    let reduced = reduced_motion(env, tab).await?;
    check!(
        tab.errors().is_empty(),
        "drawer page errors: {:?}",
        tab.errors()
    );
    env.write_json("browser-material-navigation-drawer.json", &json!({
        "browser":chrome.version,"adapter":adapter,"standard":standard,"modal":modal,"compact":compact,
        "swipe_ltr_mouse":ltr,"swipe_rtl_touch":rtl,"reduced_motion":reduced
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, &tab).await;
    if let Err(error) = &result {
        let dump = json!({
            "error":format!("{error:#}"),
            "state":state(&tab).await.ok(),
            "dom":export(&tab).await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "page_errors":tab.errors()
        });
        let _ = env.write_json("material-navigation-drawer-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-navigation-drawer-failure.png"))
            .await;
    }
    result
}
