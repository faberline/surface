//! Material toolbars through Chrome WebGPU and trusted keyboard, pointer and wheel input.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader;

const BAR: &str = "toolbar-demo-bar";
const LAYOUT: &str = "toolbar-demo-bar-layout";
const WORKSPACE: &str = "toolbar-demo-workspace";
const LEAD: &str = "toolbar-demo-lead";
const CORE: &str = "toolbar-demo-core";
const DISABLED: &str = "toolbar-demo-disabled";
const EXTRA: &str = "toolbar-demo-extra";
const OVERFLOW: &str = "toolbar-demo-overflow";
const MENU: &str = "toolbar-demo-overflow-menu";
const MENU_ACTION: &str = "toolbar-demo-overflow-action";
const FAB: &str = "toolbar-demo-action-fab";
const SCROLL: &str = "toolbar-demo-scroll";
const FIRST_ROW: &str = "toolbar-demo-row-0";
const BODY_ACTION: &str = "toolbar-demo-body-action";
const PREVIEW: &str = "toolbar-demo-preview";
const KEEP_VISIBLE: &str = "toolbar-demo-keep-visible";

type Rect = [f64; 4];

async fn toolbar(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["toolbar"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.toolbar)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, id: &str, condition: &str) -> Result<()> {
    click(tab, id).await?;
    ready(tab, condition).await
}

async fn choice(tab: &Tab, property: &str, value: &str) -> Result<()> {
    select(
        tab,
        &format!("toolbar-demo-{value}"),
        &format!("d.{property} === {}", js(value)),
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

async fn model_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    model_focus(tab, id).await?;
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
        .context("four toolbar control bounds")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn contains([x, y, w, h]: Rect, [cx, cy, cw, ch]: Rect) -> bool {
    cx >= x - 0.75 && cy >= y - 0.75 && cx + cw <= x + w + 0.75 && cy + ch <= y + h + 0.75
}

fn same_rect(a: Rect, b: Rect) -> bool {
    a.into_iter().zip(b).all(|(a, b)| near(a, b))
}

async fn absent(tab: &Tab, id: &str) -> Result<()> {
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(id)
    ))
    .await
}

struct Image {
    pixels: Vec<u8>,
    info: png::OutputInfo,
    channels: usize,
    scale: f64,
    canvas: Rect,
}

impl Image {
    async fn capture(env: &Env, tab: &Tab, label: &str) -> Result<Self> {
        tab.settle().await?;
        let path = env.artifact(&format!("material-toolbar-{label}.png"));
        tab.screenshot(&path).await?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let info = reader.next_frame(&mut pixels)?;
        let channels = match info.color_type {
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
            scale: f64::from(info.width) / width,
            info,
            canvas: tab.canvas_box().await?,
        })
    }

    fn pixel(&self, x: f64, y: f64) -> Result<[u8; 3]> {
        let x = ((x + self.canvas[0]) * self.scale).floor() as usize;
        let y = ((y + self.canvas[1]) * self.scale).floor() as usize;
        if x >= self.info.width as usize || y >= self.info.height as usize {
            bail!("toolbar probe is outside the screenshot");
        }
        let offset = y * self.info.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }
}

fn color(value: &Value) -> Result<[u8; 3]> {
    let [r, g, b, _]: [u8; 4] =
        serde_json::from_value(value.clone()).context("toolbar color token")?;
    Ok([r, g, b])
}

fn similar(a: [u8; 3], b: [u8; 3]) -> bool {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum::<u16>()
        <= 8
}

async fn paint(env: &Env, tab: &Tab, tag: &str) -> Result<[u8; 3]> {
    let [x, y, w, _] = bounds(tab, BAR).await?;
    let expected = color(&toolbar(tab).await?["container"])?;
    let image = Image::capture(env, tab, tag).await?;
    let pixel = image.pixel(x + w / 2., y + 2.)?;
    check!(
        similar(pixel, expected),
        "WebGPU toolbar surface is {pixel:?}, expected {expected:?}"
    );
    Ok(pixel)
}

async fn surface_bounds(tab: &Tab, kind: &str) -> Result<Rect> {
    let bar = bounds(tab, BAR).await?;
    let layout = bounds(tab, LAYOUT).await?;
    let workspace = bounds(tab, WORKSPACE).await?;
    let [x, y, w, h] = workspace;
    check!(
        contains(workspace, layout) && contains(layout, bar),
        "toolbar fits its layout in the workspace"
    );
    let preferences = toolbar(tab).await?;
    if kind == "docked" {
        check!(
            near(bar[0], x) && near(bar[2], w),
            "docked toolbar spans the workspace"
        );
        let edges_align = if preferences["edge"] == "top" {
            near(bar[1], y)
        } else {
            near(bar[1] + bar[3], y + h)
        };
        check!(edges_align, "docked bar touches the chosen workspace edge");
    } else if kind == "horizontal" {
        check!(
            bar[3] >= 64. && near(bar[0] + bar[2] / 2., x + w / 2.),
            "floating horizontal toolbar has 64 dp cross size and centered position"
        );
        let inset = if preferences["edge"] == "top" {
            bar[1] - y
        } else {
            y + h - bar[1] - bar[3]
        };
        check!(near(inset, 16.), "floating horizontal inset is {inset} dp");
    } else {
        check!(
            bar[2] >= 64. && near(bar[1] + bar[3] / 2., y + h / 2.),
            "floating vertical toolbar has 64 dp cross size and centered position"
        );
        let rtl = preferences["rtl"] == true;
        let right = (preferences["edge"] == "end") ^ rtl;
        let inset = if right {
            x + w - bar[0] - bar[2]
        } else {
            bar[0] - x
        };
        check!(
            near(inset, 16.),
            "logical edge maps to a 16 dp inset, got {inset}"
        );
    }
    Ok(bar)
}

async fn targets(tab: &Tab, bar: Rect, vertical: bool) -> Result<()> {
    let axis = usize::from(vertical);
    let rtl = toolbar(tab).await?["rtl"] == true;
    let mut previous: Option<Rect> = None;
    for id in [LEAD, CORE, DISABLED, EXTRA, OVERFLOW] {
        let target = bounds(tab, id).await?;
        check!(
            contains(bar, target) && target[2] >= 48. && target[3] >= 48.,
            "{id} keeps its 48 dp target inside the toolbar"
        );
        if let Some(before) = previous {
            let gap = if rtl && !vertical {
                before[axis] - target[axis] - target[axis + 2]
            } else {
                target[axis] - before[axis] - before[axis + 2]
            };
            check!(near(gap, 4.), "{id} is 4 dp from its neighbor, got {gap}");
        }
        previous = Some(target);
    }
    Ok(())
}

async fn variants(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M63",
        "docked and floating toolbar layouts paint both Material palettes",
    );
    ready(
        tab,
        "d.kind === 'docked' && d.palette === 'standard' && d.edge === 'bottom' && !d.fab",
    )
    .await?;
    let mut result = Vec::new();
    for kind in ["docked", "horizontal", "vertical"] {
        choice(tab, "kind", kind).await?;
        choice(
            tab,
            "edge",
            if kind == "vertical" { "end" } else { "bottom" },
        )
        .await?;
        let mut colors = Vec::new();
        for palette in ["standard", "vibrant"] {
            choice(tab, "palette", palette).await?;
            let bar = surface_bounds(tab, kind).await?;
            targets(tab, bar, kind == "vertical").await?;
            let sample = paint(env, tab, &format!("{kind}-{palette}")).await?;
            colors.push(sample);
            result.push(json!({"kind":kind,"palette":palette,"bounds":bar,"surface":sample}));
        }
        check!(
            !similar(colors[0], colors[1]),
            "{kind} paints distinct toolbar palettes"
        );
        let first_body = bounds(tab, SCROLL).await?;
        choice(
            tab,
            "edge",
            if kind == "vertical" { "start" } else { "top" },
        )
        .await?;
        surface_bounds(tab, kind).await?;
        if kind != "docked" {
            check!(
                same_rect(first_body, bounds(tab, SCROLL).await?),
                "floating placement preserves the body viewport"
            );
        }
    }
    Ok(json!(result))
}

async fn navigation(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M64",
        "arrow navigation mirrors horizontal RTL and skips the disabled action",
    );
    choice(tab, "kind", "horizontal").await?;
    choice(tab, "edge", "bottom").await?;
    choice(tab, "palette", "standard").await?;
    let mut paths = Vec::new();
    for (kind, rtl, forward, backward) in [
        ("horizontal", false, "ArrowRight", "ArrowLeft"),
        ("horizontal", true, "ArrowLeft", "ArrowRight"),
        ("vertical", true, "ArrowDown", "ArrowUp"),
    ] {
        choice(tab, "kind", kind).await?;
        choice(
            tab,
            "edge",
            if kind == "vertical" { "end" } else { "bottom" },
        )
        .await?;
        if toolbar(tab).await?["rtl"] != rtl {
            select(tab, "toolbar-demo-rtl", &format!("d.rtl === {rtl}")).await?;
        }
        let bar = surface_bounds(tab, kind).await?;
        targets(tab, bar, kind == "vertical").await?;
        tab.wait_for(&format!(
            "{}?.getAttribute('aria-disabled') === 'true'",
            projected(DISABLED)
        ))
        .await?;
        focus(tab, LEAD).await?;
        for id in [CORE, EXTRA, OVERFLOW, LEAD] {
            tab.press(forward).await?;
            focused(tab, id).await?;
        }
        tab.press(backward).await?;
        focused(tab, OVERFLOW).await?;
        tab.press("Home").await?;
        focused(tab, LEAD).await?;
        tab.press("End").await?;
        focused(tab, OVERFLOW).await?;
        let before = toolbar(tab).await?["actions"].clone();
        click(tab, DISABLED).await?;
        tab.settle().await?;
        check_eq!(
            toolbar(tab).await?["actions"],
            before,
            "disabled toolbar action ignores pointer input"
        );
        paths.push(json!({"kind":kind,"rtl":rtl,"next":forward,"previous":backward,"bounds":bar}));
    }
    focus(tab, CORE).await?;
    let initial = toolbar(tab).await?["selected"]
        .as_bool()
        .context("pin state")?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.selected === {}", !initial)).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-pressed') === '{}'",
        projected(CORE),
        !initial
    ))
    .await?;
    if initial {
        tab.press("Enter").await?;
        ready(tab, "d.selected").await?;
    }
    let [x, y, w, h] = bounds(tab, CORE).await?;
    let image = Image::capture(env, tab, "selected").await?;
    let pixel = image.pixel(x + w / 2., y + h / 2. + 14.)?;
    let selected = color(&toolbar(tab).await?["selected_container"])?;
    check!(
        similar(pixel, selected),
        "selected button paints its token: {pixel:?}, expected {selected:?}"
    );
    tab.press("Space").await?;
    ready(tab, "!d.selected").await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-pressed') === 'false'",
        projected(CORE)
    ))
    .await?;
    select(tab, "toolbar-demo-rtl", "!d.rtl").await?;
    Ok(json!({"paths":paths,"selected_surface":pixel}))
}

async fn open_menu(tab: &Tab) -> Result<()> {
    focus(tab, OVERFLOW).await?;
    tab.press("Enter").await?;
    ready(tab, "d.overflow_open").await?;
    let export = dom(tab).await?;
    check_eq!(
        find_node(&export, MENU)?["name"],
        json!("其他動作"),
        "overflow menu label"
    );
    check_eq!(
        find_node(&export, MENU_ACTION)?["name"],
        json!("重新整理"),
        "first overflow action label"
    );
    focused(tab, MENU_ACTION).await
}

async fn paired(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M65",
        "overflow restores focus and optional FAB has its own target and collapsed state",
    );
    choice(tab, "kind", "horizontal").await?;
    choice(tab, "edge", "bottom").await?;
    open_menu(tab).await?;
    Image::capture(env, tab, "overflow").await?;
    tab.press("Escape").await?;
    ready(tab, "!d.overflow_open").await?;
    focused(tab, OVERFLOW).await?;
    open_menu(tab).await?;
    let before = toolbar(tab).await?["actions"]
        .as_u64()
        .context("action count")?;
    tab.press("Enter").await?;
    ready(
        tab,
        &format!("!d.overflow_open && d.actions === {}", before + 1),
    )
    .await?;
    focused(tab, OVERFLOW).await?;
    select(tab, "toolbar-demo-fab", "d.fab").await?;
    let bar = bounds(tab, BAR).await?;
    let fab = bounds(tab, FAB).await?;
    check!(
        near(fab[2], 56.) && near(fab[3], 56.),
        "paired FAB is 56 dp"
    );
    check!(
        near(fab[0] - bar[0] - bar[2], 8.) && near(fab[1] + fab[3] / 2., bar[1] + bar[3] / 2.),
        "FAB attaches to the trailing side with an 8 dp gap"
    );
    focus(tab, LEAD).await?;
    tab.press("Tab").await?;
    focused(tab, FAB).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.actions === {}", before + 2)).await?;
    Image::capture(env, tab, "fab-expanded").await?;

    select(tab, PREVIEW, "d.visibility === 'collapsed'").await?;
    absent(tab, BAR).await?;
    let collapsed_fab = bounds(tab, FAB).await?;
    check!(
        near(collapsed_fab[2], 56.) && near(collapsed_fab[3], 56.),
        "paired collapse keeps only the FAB"
    );
    Image::capture(env, tab, "fab-collapsed").await?;
    select(tab, PREVIEW, "d.visibility === 'hidden'").await?;
    absent(tab, FAB).await?;
    select(tab, PREVIEW, "d.visibility === 'expanded'").await?;
    bounds(tab, BAR).await?;
    select(tab, "toolbar-demo-fab", "!d.fab").await?;
    absent(tab, FAB).await?;
    Ok(json!({"bar":bar,"fab":fab,"collapsed_fab":collapsed_fab}))
}

async fn wheel(tab: &Tab, dy: f64) -> Result<()> {
    let pane = point(tab, SCROLL).await?;
    tab.mouse_move(pane.x, pane.y).await?;
    // CDP wheel sends physical units at the emulated device scale.
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("device scale")?;
    tab.wheel(0., dy * scale).await?;
    tab.settle().await
}

async fn reveal_body_control(tab: &Tab, id: &str) -> Result<()> {
    let control = bounds(tab, id).await?;
    let clip = bounds(tab, SCROLL).await?;
    if !contains(clip, control) {
        let control_center = control[1] + control[3] / 2.;
        let clip_center = clip[1] + clip[3] / 2.;
        wheel(tab, control_center - clip_center).await?;
    }
    check!(
        contains(bounds(tab, SCROLL).await?, bounds(tab, id).await?),
        "scrolling in the compact page reveals {id}"
    );
    Ok(())
}

async fn row_offset(tab: &Tab) -> Result<f64> {
    Ok(bounds(tab, FIRST_ROW).await?[1] - bounds(tab, SCROLL).await?[1])
}

async fn expect_offset(tab: &Tab, start: f64, offset: f64) -> Result<()> {
    ready(
        tab,
        &format!("Math.abs(d.scrolled_offset - {offset}) < 0.75"),
    )
    .await?;
    let model = dom(tab).await?;
    let y = find_node(&model, SCROLL)?["style"]["scroll_y"]
        .as_f64()
        .context("DOM scroll offset")?;
    check!(
        near(y, offset) && near(start - row_offset(tab).await?, offset),
        "toolbar leaves the complete {offset} dp motion to the rendered body"
    );
    Ok(())
}

async fn visibility(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M66",
        "real wheel travels 40 dp before hiding or collapsing; focus keeps actions visible",
    );
    choice(tab, "kind", "horizontal").await?;
    choice(tab, "edge", "bottom").await?;
    wheel(tab, -900.).await?;
    choice(tab, "behavior", "hide-on-scroll").await?;
    if toolbar(tab).await?["keep_visible"] == true {
        select(tab, KEEP_VISIBLE, "!d.keep_visible").await?;
    }
    ready(tab, "d.visibility === 'expanded'").await?;
    let start = row_offset(tab).await?;
    wheel(tab, 24.).await?;
    expect_offset(tab, start, 24.).await?;
    ready(tab, "d.visibility === 'expanded'").await?;
    wheel(tab, 15.).await?;
    expect_offset(tab, start, 39.).await?;
    ready(tab, "d.visibility === 'expanded'").await?;
    wheel(tab, 1.).await?;
    expect_offset(tab, start, 40.).await?;
    ready(tab, "d.visibility === 'hidden'").await?;
    absent(tab, BAR).await?;
    Image::capture(env, tab, "wheel-hidden").await?;
    wheel(tab, -1.).await?;
    expect_offset(tab, start, 39.).await?;
    ready(tab, "d.visibility === 'expanded'").await?;
    bounds(tab, BAR).await?;

    wheel(tab, -900.).await?;
    choice(tab, "behavior", "collapse-on-scroll").await?;
    let start = row_offset(tab).await?;
    wheel(tab, 48.).await?;
    expect_offset(tab, start, 48.).await?;
    ready(tab, "d.visibility === 'collapsed'").await?;
    for id in [LEAD, EXTRA, OVERFLOW] {
        absent(tab, id).await?;
    }
    let collapsed = bounds(tab, BAR).await?;
    check!(
        contains(collapsed, bounds(tab, CORE).await?)
            && contains(collapsed, bounds(tab, DISABLED).await?),
        "unpaired collapse retains core and disabled actions"
    );
    Image::capture(env, tab, "wheel-collapsed").await?;
    wheel(tab, -1.).await?;
    expect_offset(tab, start, 47.).await?;
    ready(tab, "d.visibility === 'expanded'").await?;

    wheel(tab, -900.).await?;
    choice(tab, "behavior", "hide-on-scroll").await?;
    focus(tab, CORE).await?;
    let start = row_offset(tab).await?;
    wheel(tab, 80.).await?;
    expect_offset(tab, start, 80.).await?;
    ready(tab, "d.visibility === 'expanded'").await?;
    focused(tab, CORE).await?;
    wheel(tab, -900.).await?;
    select(tab, KEEP_VISIBLE, "d.keep_visible").await?;
    wheel(tab, 80.).await?;
    expect_offset(tab, start, 80.).await?;
    ready(tab, "d.visibility === 'expanded'").await?;
    bounds(tab, BAR).await?;
    wheel(tab, -900.).await?;
    select(tab, KEEP_VISIBLE, "!d.keep_visible").await?;
    choice(tab, "behavior", "pinned").await?;
    Ok(
        json!({"hide_offsets":[24,39,40,39],"collapse_offsets":[48,47],"collapsed":collapsed,
        "focused_scroll":80,"explicit_keep_visible_scroll":80}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M67",
        "320 dp dark RTL keeps 200% controls and toolbar actions inside the viewport",
    );
    choice(tab, "kind", "vertical").await?;
    choice(tab, "edge", "end").await?;
    choice(tab, "palette", "vibrant").await?;
    select(tab, "toolbar-demo-rtl", "d.rtl").await?;
    select(tab, "toolbar-demo-scale", "d.text_scale === 2").await?;
    click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    resize(tab, 320, 580).await?;
    let bar = surface_bounds(tab, "vertical").await?;
    targets(tab, bar, true).await?;
    let body_viewport = bounds(tab, SCROLL).await?;
    check!(
        bar[0] + bar[2] <= body_viewport[0] - 7.25,
        "compact settings and content clear the RTL floating toolbar"
    );
    let sample = paint(env, tab, "compact-dark-rtl").await?;
    let export = dom(tab).await?;
    check_eq!(
        find_node(&export, CORE)?["name"],
        json!("釘選"),
        "scaled compact core name"
    );
    check_eq!(
        find_node(&export, DISABLED)?["name"],
        json!("尚未開放"),
        "scaled compact disabled name"
    );
    open_menu(tab).await?;
    let view = bounds(tab, MENU).await?;
    let viewport = tab.canvas_box().await?;
    check!(
        contains([0., 0., viewport[2], viewport[3]], view),
        "scaled overflow fits the canvas"
    );
    check!(
        contains(view, bounds(tab, MENU_ACTION).await?),
        "scaled menu action stays inside its menu"
    );
    Image::capture(env, tab, "compact-dark-overflow").await?;
    tab.press("Escape").await?;
    ready(tab, "!d.overflow_open").await?;
    focused(tab, OVERFLOW).await?;
    reveal_body_control(tab, BODY_ACTION).await?;
    let before = toolbar(tab).await?["body_actions"]
        .as_u64()
        .context("body action count")?;
    focus(tab, BODY_ACTION).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.body_actions === {}", before + 1)).await?;
    Image::capture(env, tab, "compact-dark-scrolled").await?;
    Ok(
        json!({"bar":bar,"body_viewport":body_viewport,"overflow":view,
        "surface":sample,"body_actions":before+1}),
    )
}

async fn compact_pair(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M68",
        "320 dp horizontal toolbar and FAB fit inside the margins at 200% RTL",
    );
    reveal_body_control(tab, "toolbar-demo-horizontal").await?;
    choice(tab, "kind", "horizontal").await?;
    reveal_body_control(tab, "toolbar-demo-fab").await?;
    select(tab, "toolbar-demo-fab", "d.fab").await?;
    let [x, y, width, height] = bounds(tab, WORKSPACE).await?;
    let layout = bounds(tab, LAYOUT).await?;
    check!(
        contains([x + 16., y + 16., width - 32., height - 32.], layout)
            && near(layout[1] + layout[3], y + height - 16.),
        "compact toolbar and FAB stay inside the floating margins"
    );
    let bar = bounds(tab, BAR).await?;
    let fab = bounds(tab, FAB).await?;
    check!(
        contains(layout, bar) && contains(layout, fab) && near(fab[2], 56.) && near(fab[3], 56.),
        "compact floating layout contains a full 56 dp FAB"
    );
    let inline_gap = (bar[0] - fab[0] - fab[2]).max(fab[0] - bar[0] - bar[2]);
    let wrapped_gap = (bar[1] - fab[1] - fab[3]).max(fab[1] - bar[1] - bar[3]);
    check!(
        near(inline_gap, 8.) || near(wrapped_gap, 8.),
        "compact toolbar stays 8 dp away from the paired FAB"
    );
    for id in [LEAD, CORE, DISABLED, EXTRA, OVERFLOW] {
        let target = bounds(tab, id).await?;
        check!(
            contains(bar, target) && target[2] >= 48. && target[3] >= 48.,
            "{id} has its full target after adapting the compact toolbar"
        );
    }
    let body = bounds(tab, SCROLL).await?;
    check!(
        body[1] + body[3] <= layout[1] - 7.25,
        "paired toolbar clears the compact body"
    );
    let before = toolbar(tab).await?["actions"]
        .as_u64()
        .context("action count")?;
    focus(tab, FAB).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.actions === {}", before + 1)).await?;
    Image::capture(env, tab, "compact-dark-horizontal-fab").await?;
    Ok(json!({"bar":bar,"fab":fab,"layout":layout,"body_viewport":body}))
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 760).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, "toolbar-demo-show").await?;
    let variants = variants(env, tab).await?;
    let navigation = navigation(env, tab).await?;
    let paired = paired(env, tab).await?;
    let visibility = visibility(env, tab).await?;
    let compact = compact(env, tab).await?;
    let compact_pair = compact_pair(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "toolbar page errors: {errors:?}");
    env.write_json(
        "browser-material-toolbar.json",
        &json!({
            "browser":chrome.version,"variants":variants,"navigation":navigation,
            "paired":paired,"visibility":visibility,"compact":compact,"compact_pair":compact_pair,
            "errors":errors
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-toolbar-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-toolbar-failure.png"))
            .await;
    }
    result
}
