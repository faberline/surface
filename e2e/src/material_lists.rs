//! Material 3 Lists on the real Chrome WebGPU host.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const GROUP: &str = "lists-items";
const SCROLL: &str = "lists-scroll";
const BEFORE: &str = "lists-demo-before";
const AFTER: &str = "lists-demo-after";
const A: &str = "lists-a";
const B: &str = "lists-b";
const C: &str = "lists-c";
const D: &str = "lists-d";
const F: &str = "lists-f";

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn read(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["lists"].take())
}

async fn ready(tab: &Tab, expression: &str) -> Result<()> {
    until(tab, &format!("(d => {expression})(s.lists)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1644).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1600).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "lists-demo-show").await?;
    ready(
        tab,
        "d.visible && d.variant === 'segmented' && d.mode === 'select' && d.selected === 'lists-c' &&
         d.activations === 0 && !d.rtl && d.text_scale === 1",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("lists-demo-{name}")).await
}

async fn node(tab: &Tab, id: &str) -> Result<Value> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})",
        js(id)
    ))
    .await
}

async fn corners(tab: &Tab, id: &str) -> Result<[f64; 4]> {
    let n = node(tab, id).await?;
    let contour = &n["style"]["corners"];
    if contour.is_null() {
        let radius = n["style"]["radius"].as_f64().context("row radius")?;
        return Ok([radius; 4]);
    }
    let values = ["top_left", "top_right", "bottom_right", "bottom_left"];
    let mut output = [0.; 4];
    for (slot, name) in output.iter_mut().zip(values) {
        *slot = contour[name].as_f64().context("row corner radius")?;
    }
    Ok(output)
}

async fn color(tab: &Tab, key: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(read(tab).await?["colors"][key].clone())
        .with_context(|| format!("Gallery palette role {key}"))?;
    check_eq!(a, 255, format!("{key} is an opaque Gallery token"));
    Ok([r, g, b])
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-lists-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn distance(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn pixel(image: &Raster, sample: Rect) -> Result<Rgb> {
    image
        .colors(sample)?
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(rgb, _)| rgb)
        .context("actual browser PNG color")
}

fn expect_pixel(image: &Raster, sample: Rect, expected: Rgb, label: &str) -> Result<Rgb> {
    let actual = pixel(image, sample)?;
    check!(
        distance(actual, expected) <= 9,
        "{label} at {sample:?}: actual {actual:?}, expected {expected:?}"
    );
    Ok(actual)
}

fn center_patch([x, y, w, _]: Rect, inset: f64) -> Rect {
    [x + w * 0.5 - 6., y + inset, 12., 0.5]
}

async fn variants(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M161",
        "the three list styles keep exact row heights, gaps, and resting corner tokens",
    );
    preset(tab, "passive").await?;
    ready(tab, "d.mode === 'passive'").await?;
    let surface = color(tab, "surface").await?;
    let gutter = color(tab, "surface_container").await?;
    let mut variants = Vec::new();
    for (style, gap, expected) in [
        (
            "segmented",
            2.,
            [[16., 16., 4., 4.], [4.; 4], [4., 4., 16., 16.]],
        ),
        ("standard", 0., [[4.; 4]; 3]),
        ("baseline", 0., [[0.; 4]; 3]),
    ] {
        preset(tab, style).await?;
        ready(tab, &format!("d.variant === {}", js(style))).await?;
        let a = bounds(tab, A).await?;
        let b = bounds(tab, B).await?;
        let c = bounds(tab, C).await?;
        let d = bounds(tab, D).await?;
        check!(
            near(a[3], 56.) && near(c[3], 72.) && near(d[3], 88.),
            "{style} uses the three compact row heights: {a:?} {c:?} {d:?}"
        );
        check!(
            near(b[1] - a[1] - a[3], gap)
                && near(c[1] - b[1] - b[3], gap)
                && near(d[1] - c[1] - c[3], gap),
            "{style} has the physical {gap} dp gap between complete row targets"
        );
        let authored = [
            corners(tab, A).await?,
            corners(tab, B).await?,
            corners(tab, F).await?,
        ];
        check_eq!(authored, expected, format!("{style} resting corner radii"));
        let image = capture(env, tab, &format!("{style}-passive")).await?;
        if style == "segmented" {
            expect_pixel(&image, center_patch(a, 4.), surface, "segmented row fill")?;
            expect_pixel(
                &image,
                center_patch(a, a[3] + 0.5),
                gutter,
                "physical 2 dp gap",
            )?;
            expect_pixel(
                &image,
                [a[0] + 2., a[1] + 2., 0.5, 0.5],
                gutter,
                "16 dp outer corner exposes the gutter",
            )?;
            expect_pixel(
                &image,
                [b[0] + 2.5, b[1] + 2.5, 0.5, 0.5],
                surface,
                "4 dp inner corner keeps the row's Surface paint",
            )?;
        }
        variants.push(json!({"style":style,"gap":gap,"corners":authored,
            "heights":[a[3],c[3],d[3]]}));
    }
    Ok(json!(variants))
}

async fn attributes(tab: &Tab, id: &str) -> Result<Value> {
    let target = projected(id);
    tab.eval(&format!(
        "(() => {{
          const element = {target};
          return element && {{
            role: element.getAttribute('role'), name: element.getAttribute('aria-label'),
            selected: element.getAttribute('aria-selected'), checked: element.getAttribute('aria-checked'),
            disabled: element.getAttribute('aria-disabled'), tabindex: element.getAttribute('tabindex'),
            nested_controls: element.querySelectorAll('[tabindex],button,[role=button]').length,
          }};
        }})()"
    ))
    .await
}

async fn passive_and_action(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M162",
        "passive and command rows expose different accessible targets and command activation is momentary",
    );
    preset(tab, "passive").await?;
    ready(tab, "d.mode === 'passive'").await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "list", "書籤")?;
    ax(&tree, "StaticText", "山")?;
    check_eq!(
        attributes(tab, GROUP).await?["role"],
        json!("list"),
        "passive rows have a semantic list owner"
    );
    check_eq!(
        attributes(tab, A).await?["tabindex"],
        Value::Null,
        "passive row has no Tab stop"
    );
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    click(tab, A).await?;
    ready(tab, "d.activations === 0").await?;

    preset(tab, "action").await?;
    ready(tab, "d.mode === 'action'").await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "list", "書籤")?;
    let row = ax(&tree, "button", "山")?;
    check_eq!(
        ax_property(row, "focusable"),
        Some(&json!(true)),
        "action row is focusable"
    );
    let row = attributes(tab, A).await?;
    check!(
        row["role"] == "button"
            && row["selected"].is_null()
            && row["checked"].is_null()
            && row["nested_controls"] == 0,
        "one command row owns its label and all decorative slots: {row}"
    );
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, A).await?;
    tab.press("ArrowDown").await?;
    focused(tab, C).await?;
    ready(tab, "d.activations === 0").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.activations === 1 && d.last === 'lists-c' && d.selected === null",
    )
    .await?;
    let activated = attributes(tab, C).await?;
    check!(
        activated["selected"].is_null() && activated["checked"].is_null(),
        "activating a command exposes no persistent selection: {activated}"
    );
    capture(env, tab, "action-keyboard").await?;
    Ok(json!({"passive_skipped":true,"command":"lists-c","activations":1,"aria":activated}))
}

async fn option(tab: &Tab, id: &str, label: &str, selected: bool, disabled: bool) -> Result<Value> {
    let dom = attributes(tab, id).await?;
    let tree = ax_tree(tab).await?;
    let exposed = ax(&tree, "option", label)?;
    let selection = if selected { "true" } else { "false" };
    check!(
        dom["role"] == "option"
            && dom["name"] == label
            && dom["selected"] == selection
            && dom["checked"].is_null()
            && dom["nested_controls"] == 0,
        "one named DOM option owns its selection and passive children: {dom}"
    );
    check_eq!(
        ax_property(exposed, "selected"),
        Some(&json!(selected)),
        format!("Chrome exposes the selected state of {label}")
    );
    if disabled {
        check!(
            dom["disabled"] == "true"
                && dom["tabindex"].is_null()
                && ax_property(exposed, "disabled") == Some(&json!(true)),
            "disabled option is named without becoming a focus target"
        );
    }
    Ok(json!({"dom":dom,"chrome_name":exposed["name"]["value"],
        "chrome_selected":ax_property(exposed,"selected")}))
}

async fn enter_selected(tab: &Tab) -> Result<()> {
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, C).await
}

async fn focus_paint(env: &Env, tab: &Tab, row: Rect) -> Result<Value> {
    let image = capture(env, tab, "focused-segmented").await?;
    let ink = color(tab, "secondary").await?;
    let first = expect_pixel(
        &image,
        center_patch(row, 0.5),
        ink,
        "physical focus begins at row edge",
    )?;
    let third = expect_pixel(
        &image,
        center_patch(row, 2.0),
        ink,
        "focus covers the third dp",
    )?;
    let inside = pixel(&image, center_patch(row, 3.5))?;
    check!(
        distance(inside, ink) >= 24,
        "the focused fill starts immediately after the 3 dp inner line: {inside:?} versus {ink:?}"
    );
    let authored = node(tab, A).await?;
    check_eq!(
        authored["focus_outline_width"].as_f64(),
        Some(3.),
        "the opted-in inner stroke is 3 dp"
    );
    Ok(json!({"row":row,"first":first,"third":third,"inside":inside}))
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M163",
        "the named listbox wraps, skips disabled rows, and reveals focus before manual selection",
    );
    let tree = ax_tree(tab).await?;
    ax(&tree, "listbox", "書籤")?;
    check_eq!(
        attributes(tab, GROUP).await?["role"],
        json!("listbox"),
        "the options have a named listbox owner"
    );
    let before = bounds(tab, F).await?;
    let scroll = bounds(tab, SCROLL).await?;
    check!(
        before[1] >= scroll[1] + scroll[3],
        "the far row starts fully clipped: {before:?} / {scroll:?}"
    );
    check_eq!(
        attributes(tab, F).await?,
        Value::Null,
        "the fully clipped row has no projected target"
    );
    let initial_c = option(tab, C, "河", true, false).await?;
    option(tab, B, "海", false, true).await?;
    option(tab, A, "山", false, false).await?;
    let river = ax(&tree, "option", "河")?;
    check!(
        river["description"]["value"]
            .as_str()
            .is_some_and(|value| value.contains("河岸的步道")),
        "supporting text labels the option without another focus stop: {river}"
    );
    enter_selected(tab).await?;
    tab.press("ArrowUp").await?;
    focused(tab, A).await?;
    let stroke = focus_paint(env, tab, bounds(tab, A).await?).await?;
    tab.press("ArrowUp").await?;
    focused(tab, F).await?;
    let revealed = bounds(tab, F).await?;
    check!(
        contains(bounds(tab, SCROLL).await?, revealed) && revealed[1] < before[1],
        "reverse wrap scrolls the last option completely into view: {revealed:?}"
    );
    ready(tab, "d.selected === 'lists-c' && d.activations === 0").await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, C).await?;
    tab.press("Home").await?;
    focused(tab, A).await?;
    tab.press("End").await?;
    focused(tab, F).await?;
    tab.press("ArrowDown").await?;
    focused(tab, A).await?;
    ready(tab, "d.selected === 'lists-c' && d.activations === 0").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.selected === 'lists-a' && d.activations === 1 && d.last === 'lists-a'",
    )
    .await?;
    let committed = option(tab, A, "山", true, false).await?;
    option(tab, C, "河", false, false).await?;
    let a_mark = node(tab, "lists-a-mark").await?;
    let c_mark = node(tab, "lists-c-mark").await?;
    check!(
        a_mark["fill_contour"]["points"].is_array()
            && c_mark["fill_contour"].is_null()
            && attributes(tab, "lists-a-mark").await?["tabindex"].is_null()
            && corners(tab, A).await? == [16.; 4],
        "the committed row keeps a passive painted mark and 16 dp corners"
    );
    tab.press("End").await?;
    focused(tab, F).await?;
    tab.press("Space").await?;
    ready(
        tab,
        "d.selected === 'lists-f' && d.activations === 2 && d.last === 'lists-f'",
    )
    .await?;
    option(tab, F, "森林", true, false).await?;
    Ok(
        json!({"initial_c":initial_c,"focus_stroke":stroke,"pre_scroll":before,
        "revealed":revealed,"enter":committed,"space":"lists-f","activations":2}),
    )
}

async fn touch(tab: &Tab, id: &str) -> Result<()> {
    let where_to = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":where_to.x,"y":where_to.y,"id":1}]}),
    )
    .await?;
    tab.settle().await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchEnd","touchPoints":[]}),
    )
    .await?;
    tab.settle().await
}

async fn pointer_selection(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M164",
        "trusted touch and pointer use the row target, with inert gaps and disabled rows",
    );
    tab.eval(
        "(() => {
          window.listsContacts = [];
          for (const kind of ['pointerdown','pointerup']) {
            document.querySelector('canvas').addEventListener(kind, event => {
              if (event.isTrusted) window.listsContacts.push({kind,type:event.pointerType});
            }, {capture:true,passive:true});
          }
        })()",
    )
    .await?;
    let a = point(tab, A).await?;
    tab.click(a.x, a.canvas[1] + a.b[1] + a.b[3] + 1.).await?;
    ready(tab, "d.activations === 0 && d.selected === 'lists-c'").await?;
    click(tab, B).await?;
    ready(tab, "d.activations === 0 && d.selected === 'lists-c'").await?;
    touch(tab, A).await?;
    ready(
        tab,
        "d.activations === 1 && d.selected === 'lists-a' && d.last === 'lists-a'",
    )
    .await?;
    option(tab, A, "山", true, false).await?;
    option(tab, C, "河", false, false).await?;
    let c = point(tab, C).await?;
    tab.click(c.canvas[0] + c.b[0] + 0.75, c.canvas[1] + c.b[1] + 0.75)
        .await?;
    ready(
        tab,
        "d.activations === 2 && d.selected === 'lists-c' && d.last === 'lists-c'",
    )
    .await?;
    let contacts = tab.eval("window.listsContacts").await?;
    let trusted = contacts.as_array().context("trusted canvas pointer log")?;
    for (device, expected) in [("mouse", 6), ("touch", 2)] {
        check_eq!(
            trusted
                .iter()
                .filter(|event| event["type"] == device)
                .count(),
            expected,
            format!("both real press phases reach the canvas for {device}")
        );
    }
    capture(env, tab, "pointer-selection").await?;

    preset(tab, "reorder").await?;
    ready(
        tab,
        "d.items[0].id === 'lists-f' && d.selected === 'lists-c' && d.activations === 2",
    )
    .await?;
    enter_selected(tab).await?;
    focus_projected(tab, AFTER).await?;
    tab.press("Enter").await?;
    ready(tab, "d.selected === null && d.activations === 2").await?;
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, F).await?;
    option(tab, F, "森林", false, false).await?;
    Ok(
        json!({"gap_and_disabled_inert":true,"touch":"lists-a","corner":"lists-c",
        "pointer_activations":2,"selected_retained_on_reorder":true,"entry_after_clear":"lists-f",
        "contacts":contacts}),
    )
}

async fn freeze_time(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            const start = performance.now();
            window.listsClock = 0;
            Object.defineProperty(performance, 'now', {
                configurable: true, value: () => start + window.listsClock,
            });
        })()",
    )
    .await?;
    tab.settle().await
}

async fn advance_time(tab: &Tab, ms: u16) -> Result<()> {
    tab.eval(&format!("window.listsClock = {ms}")).await?;
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
    tab.settle().await
}

fn corner_edge(image: &Raster, row: Rect, gutter: Rgb) -> Result<usize> {
    let fill = pixel(image, center_patch(row, 3.))?;
    check!(
        distance(fill, gutter) > 0,
        "row fill separates from its gutter at the sampled frame"
    );
    for pixel_index in 0..28 {
        let offset = pixel_index as f64 * 0.5;
        let color = pixel(image, [row[0] + offset, row[1] + 3., 0.5, 0.5])?;
        if distance(color, fill) < distance(color, gutter) {
            return Ok(pixel_index);
        }
    }
    anyhow::bail!("no row foreground in 14 dp alongside its corner")
}

async fn select_from_first(tab: &Tab) -> Result<()> {
    enter_selected(tab).await?;
    tab.press("ArrowUp").await?;
    focused(tab, A).await
}

async fn motion(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M165",
        "the neighboring selection changes shape and color over time or snaps with Reduce Motion",
    );
    reduced_motion(tab, false).await?;
    select_from_first(tab).await?;
    freeze_time(tab).await?;
    let row = bounds(tab, C).await?;
    let surface = color(tab, "surface").await?;
    let selected = color(tab, "secondary_container").await?;
    let gutter = color(tab, "surface_container").await?;
    let initial = capture(env, tab, "motion-selected").await?;
    let first_edge = corner_edge(&initial, row, gutter)?;
    expect_pixel(
        &initial,
        center_patch(row, 4.),
        selected,
        "former selected row starts filled",
    )?;

    tab.press("Enter").await?;
    ready(tab, "d.activations === 1 && d.selected === 'lists-a'").await?;
    option(tab, C, "河", false, false).await?;
    option(tab, A, "山", true, false).await?;
    let start = capture(env, tab, "motion-first-frame").await?;
    check_eq!(
        corner_edge(&start, row, gutter)?,
        first_edge,
        "the first frame retains the old radius"
    );
    expect_pixel(
        &start,
        center_patch(row, 4.),
        selected,
        "the first frame retains the old color",
    )?;
    advance_time(tab, 40).await?;
    let middle = capture(env, tab, "motion-midpoint").await?;
    let mid_edge = corner_edge(&middle, row, gutter)?;
    let mid_color = pixel(&middle, center_patch(row, 4.))?;
    check!(
        distance(mid_color, surface) >= 12 && distance(mid_color, selected) >= 12,
        "WebGPU paints an intermediate selection fill: {selected:?} → {mid_color:?} → {surface:?}"
    );
    advance_time(tab, 320).await?;
    let end = capture(env, tab, "motion-settled").await?;
    let end_edge = corner_edge(&end, row, gutter)?;
    check!(
        first_edge > mid_edge && mid_edge > end_edge,
        "the physical row corner takes a distinct intermediate shape: {first_edge} → {mid_edge} → {end_edge}"
    );
    expect_pixel(
        &end,
        center_patch(row, 4.),
        surface,
        "the settled former selection paints Surface",
    )?;
    check_eq!(
        bounds(tab, C).await?,
        row,
        "selection paint never moves the row target"
    );

    reload(tab, url).await?;
    reduced_motion(tab, true).await?;
    select_from_first(tab).await?;
    freeze_time(tab).await?;
    tab.press("Enter").await?;
    ready(tab, "d.activations === 1 && d.selected === 'lists-a'").await?;
    let instant_row = bounds(tab, C).await?;
    let instant = capture(env, tab, "motion-reduced").await?;
    check_eq!(
        corner_edge(&instant, instant_row, gutter)?,
        end_edge,
        "Reduce Motion reaches the same corner without clock advance"
    );
    expect_pixel(
        &instant,
        center_patch(instant_row, 4.),
        surface,
        "Reduce Motion reaches Surface without clock advance",
    )?;
    check_eq!(
        tab.eval("window.listsClock").await?,
        json!(0),
        "the reduced-motion clock stayed paused"
    );
    Ok(
        json!({"edge_device_pixels":[first_edge,mid_edge,end_edge],"fill":[selected,mid_color,surface],
        "elapsed_ms":[0,40,320],"reduced_motion_snap":true}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M166",
        "dark RTL at 200 percent wraps long labels and keeps their full rows reachable at 320 dp",
    );
    for setting in ["theme", "rtl", "large-text"] {
        preset(tab, setting).await?;
    }
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2").await?;
    resize(tab, 320, 900).await?;
    select_from_first(tab).await?;
    let first = bounds(tab, A).await?;
    let label = bounds(tab, "lists-a-headline").await?;
    let leading = bounds(tab, "lists-a-leading").await?;
    let trailing = bounds(tab, "lists-a-trailing").await?;
    check!(
        [label, leading, trailing].into_iter().all(|slot| contains(first, slot))
            && leading[0] >= label[0] + label[2] - 0.75
            && trailing[0] + trailing[2] <= label[0] + 0.75,
        "RTL keeps an independent leading icon, headline and trailing label: {first:?} {leading:?} {label:?} {trailing:?}"
    );
    capture(env, tab, "compact-rtl-leading-trailing").await?;
    tab.press("ArrowDown").await?;
    focused(tab, C).await?;
    tab.press("ArrowDown").await?;
    focused(tab, D).await?;
    let long = bounds(tab, D).await?;
    let title = bounds(tab, "lists-d-headline").await?;
    let supporting = bounds(tab, "lists-d-supporting").await?;
    let overline = bounds(tab, "lists-d-overline").await?;
    check!(
        contains([0., 0., 320., 900.], long) && contains(bounds(tab, SCROLL).await?, long)
            && [title, supporting, overline].into_iter().all(|slot| contains(long, slot))
            && title[3] > 55.
            && title[1] >= overline[1] + overline[3] - 0.75
            && supporting[1] >= title[1] + title[3] - 0.75,
        "the large three-slot label wraps and is fully visible after focus: {long:?} {overline:?} {title:?} {supporting:?}"
    );
    let text = "長文字會在窄畫面自動換行";
    check_eq!(
        node(tab, "lists-d-headline").await?["text"],
        json!(text),
        "the paint bounds belong to the complete long headline"
    );
    option(tab, D, text, false, false).await?;
    tab.press("Space").await?;
    ready(tab, "d.selected === 'lists-d' && d.activations === 1").await?;
    let committed = option(tab, D, text, true, false).await?;
    let scene = bounds(tab, "lists-demo").await?;
    let status = bounds(tab, "lists-demo-status").await?;
    let status_text = format!("已選取：{text}");
    check!(
        status[0] >= scene[0] - 0.75
            && status[0] + status[2] <= scene[0] + scene[2] + 0.75
            && status[3] > 48.
            && node(tab, "lists-demo-status").await?["text"] == status_text.as_str(),
        "the long committed name wraps without horizontal clipping in the live status: {status:?}"
    );
    capture(env, tab, "compact-dark-rtl-large-text").await?;

    tab.press("ArrowUp").await?;
    focused(tab, C).await?;
    tab.press("Space").await?;
    ready(tab, "d.selected === 'lists-c' && d.activations === 2").await?;
    focus_projected(tab, BEFORE).await?;
    resize(tab, 320, 240).await?;
    tab.press("Tab").await?;
    focused(tab, C).await?;
    let short = read(tab).await?;
    let visible = short["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == C))
        .context("short selected row and actual clip")?;
    let rect: Rect = serde_json::from_value(visible["bounds"].clone())?;
    let clip: Rect = serde_json::from_value(visible["clip"].clone())?;
    let top = rect[1].max(clip[1]).max(0.);
    let bottom = (rect[1] + rect[3]).min(clip[1] + clip[3]).min(240.);
    check!(
        bottom - top >= 47.5 && rect[0] >= 0. && rect[0] + rect[2] <= 320.,
        "the short scene reveals a complete usable strip of the selected row: {rect:?} / {clip:?}"
    );
    tab.press("Enter").await?;
    ready(
        tab,
        "d.selected === 'lists-c' && d.last === 'lists-c' && d.activations === 3",
    )
    .await?;
    capture(env, tab, "short-dark-rtl-selected").await?;
    Ok(
        json!({"row":long,"headline":title,"supporting":supporting,"overline":overline,
        "rtl_slots":[leading,label,trailing],"selection":committed,"wrapped_status":status,
        "short":{"row":rect,"clip":clip,"visible_height":bottom-top,"activated":"lists-c"}}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let styles = variants(env, tab).await?;
    reload(tab, url).await?;
    let semantics = passive_and_action(env, tab).await?;
    reload(tab, url).await?;
    let keyboard = keyboard(env, tab).await?;
    reload(tab, url).await?;
    let pointer = pointer_selection(env, tab).await?;
    reload(tab, url).await?;
    let motion = motion(env, tab, url).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material 3 Lists page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-lists.json",
        &json!({"browser":chrome.version,"styles":styles,"semantics":semantics,"keyboard":keyboard,
            "pointer":pointer,"motion":motion,"compact":compact,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1644, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-lists-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-lists-failure.png"))
            .await;
    }
    result.context("Material 3 Lists")
}
