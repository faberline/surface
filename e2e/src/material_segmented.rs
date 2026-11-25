//! Baseline segmented buttons in the stock headless WebGPU Gallery.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focused, near, resize, Raster,
};
use super::{state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "segmented-demo-show";
const BACK: &str = "segmented-demo-back";
const THEME: &str = "segmented-demo-theme";
const RTL: &str = "segmented-demo-rtl";
const TEXT: &str = "segmented-demo-large-text";
const AFTER: &str = "segmented-demo-after";
const COMPARE: &str = "segments-comparison";
const VIEW: &str = "segments-view";
const FILTERS: &str = "segments-filters";
const LEFT: &str = "segment-left-pane";
const RIGHT: &str = "segment-right-pane";
const LIST: &str = "segment-list-view";
const TABLE: &str = "segment-table-view";
const GRID: &str = "segment-grid-view";
const NEW: &str = "segment-new";
const SAVED: &str = "segment-saved";
const LOCAL: &str = "segment-local";
const SHARED: &str = "segment-shared";
const ARCHIVED: &str = "segment-archived";
const WIDE: &str = "material-segmented-wide.png";
const SELECTED: &str = "material-segmented-selection.png";
const PRESSED: &str = "material-segmented-pressed.png";
const COMPACT: &str = "material-segmented-rtl-200.png";
const SCROLLED: &str = "material-segmented-scroll-rtl.png";
pub(super) const STEPS: [&str; 4] = ["M320", "M321", "M322", "M323"];

type Rect = [f64; 4];
type Rgb = [u8; 3];
type Sample = (&'static str, &'static str, bool);

const COMPARE_ITEMS: [Sample; 2] = [(LEFT, "左側", false), (RIGHT, "右側", false)];
const VIEW_ITEMS: [Sample; 3] = [
    (LIST, "清單檢視", false),
    (TABLE, "表格檢視", true),
    (GRID, "格狀檢視", false),
];
const FILTER_ITEMS: [Sample; 5] = [
    (NEW, "最新", false),
    (SAVED, "已儲存", false),
    (LOCAL, "離線", true),
    (SHARED, "已分享", false),
    (ARCHIVED, "封存", false),
];

#[derive(Serialize)]
struct Segment {
    hit: Rect,
    face: Rect,
    focus: Rect,
}

fn centre([x, y, w, h]: Rect) -> [f64; 2] {
    [x + w / 2., y + h / 2.]
}

async fn part(tab: &Tab, id: &str, name: &str) -> Result<Rect> {
    bounds(tab, &format!("{id}-{name}")).await
}

async fn segment(tab: &Tab, id: &str) -> Result<Segment> {
    let g = Segment {
        hit: bounds(tab, id).await?,
        face: part(tab, id, "face").await?,
        focus: part(tab, id, "focus").await?,
    };
    check!(
        g.hit[2] >= 48. && g.hit[3] >= 48. && contains(g.hit, g.face),
        "{id} has an independent touch target"
    );
    check!(
        near(g.face[0] - g.focus[0], 5.)
            && near(g.face[1] - g.focus[1], 5.)
            && near(g.focus[2] - g.face[2], 10.)
            && near(g.focus[3] - g.face[3], 10.),
        "{id} keeps a 5 dp focus outset"
    );
    Ok(g)
}

async fn model(tab: &Tab, condition: &str) -> Result<Value> {
    until(tab, &format!("(d => {condition})(s.segmented_demo)")).await?;
    tab.settle().await?;
    Ok(state(tab).await?["segmented_demo"].clone())
}

fn paint_color(model: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(model["palette"][name].clone())
        .with_context(|| format!("segmented color {name}"))?;
    Ok([r, g, b])
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2200).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.segmented_demo.visible").await?;
    activate(tab, SHOW).await?;
    model(tab, "d.visible && !d.dark && !d.rtl && d.text_scale === 1").await?;
    resize(tab, 960, 880).await
}

fn states(tree: &[Value], items: &[Sample], role: &str, checked: &[&str]) -> Result<Value> {
    let mut exported = Vec::new();
    for (id, name, disabled) in items {
        let node = ax(tree, role, name)?;
        let expected = checked.contains(id);
        check_eq!(
            ax_property(node, "checked"),
            Some(&json!(if expected { "true" } else { "false" })),
            "{id} checked"
        );
        check_eq!(
            ax_property(node, "disabled") == Some(&json!(true)),
            *disabled,
            "{id} disabled"
        );
        exported
            .push(json!({"id":id,"name":name,"role":role,"checked":expected,"disabled":disabled}));
    }
    Ok(json!(exported))
}

async fn wide(tab: &Tab, env: &Env) -> Result<Value> {
    let initial = model(tab, "d.comparison === 'segment-right-pane' && d.view === 'segment-grid-view' && !d.filters.length").await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "radiogroup", "對照位置")?;
    ax(&tree, "radiogroup", "檢視方式")?;
    ax(&tree, "group", "篩選條件")?;
    let roles = json!({
        "comparison": states(&tree, &COMPARE_ITEMS, "radio", &[RIGHT])?,
        "view": states(&tree, &VIEW_ITEMS, "radio", &[GRID])?,
        "filters": states(&tree, &FILTER_ITEMS, "checkbox", &[])?,
    });

    let frame = photo(tab, env, WIDE).await?;
    let mut outline_samples = Vec::new();
    let mut rows = Vec::new();
    for (group, items) in [
        (COMPARE, COMPARE_ITEMS.as_slice()),
        (VIEW, VIEW_ITEMS.as_slice()),
        (FILTERS, FILTER_ITEMS.as_slice()),
    ] {
        let mut geometry = Vec::new();
        let outline = part(tab, group, "outline").await?;
        let first = segment(tab, items[0].0).await?;
        for (n, (id, _, _)) in items.iter().enumerate() {
            let g = segment(tab, id).await?;
            check!(
                near(g.hit[2], first.hit[2])
                    && near(g.face[3], 40.)
                    && near(g.face[1], first.face[1]),
                "equal-width 40 dp faces in one row"
            );
            check!(
                near(g.hit[0], first.hit[0] + n as f64 * first.hit[2]),
                "segments touch without gaps"
            );
            if n > 0 {
                let seam = part(tab, group, &format!("seam-{}", n - 1)).await?;
                let [x, y] = centre(seam);
                check!(
                    near(seam[2], 1.) && near(seam[3], g.face[3] - 2.),
                    "single 1 dp seam stops inside the outline, got {seam:?}"
                );
                let ink = frame.sample(x, y)?;
                check_eq!(
                    ink,
                    paint_color(&initial, "outline")?,
                    "seam uses the outline token"
                );
                outline_samples.push(json!({"rect":seam,"point":[x,y],"ink":ink}));
            }
            geometry.push(json!({"id":id,"rects":g}));
        }
        rows.push(json!({"id":group,"outline":outline,"segments":geometry}));
    }
    let grid = segment(tab, GRID).await?;
    let selected_point = [centre(grid.face)[0], grid.face[1] + 4.];
    check_eq!(
        frame.sample(selected_point[0], selected_point[1])?,
        paint_color(&initial, "selection")?,
        "checked icon-only option uses the selected color"
    );
    let check = part(tab, GRID, "check").await?;
    let icon = part(tab, GRID, "icon").await?;
    check!(
        check[2] > 0. && icon[2] > 0. && check[0] + check[2] < icon[0],
        "checked icon-only choice paints both cues"
    );

    Ok(
        json!({"initial":initial,"roles":roles,"rows":rows,"seams":outline_samples,
        "capture":WIDE,"selected_point":selected_point,"check":check,"icon":icon}),
    )
}

async fn keyboard(tab: &Tab, env: &Env) -> Result<Value> {
    activate(tab, TEXT).await?;
    activate(tab, TEXT).await?;
    model(tab, "d.text_scale === 1").await?;
    tab.press("Tab").await?;
    focused(tab, LEFT).await?;
    tab.press("ArrowRight").await?;
    focused(tab, RIGHT).await?;
    model(
        tab,
        "d.comparison === 'segment-right-pane' && d.activations === 0",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, LIST).await?;
    tab.press("ArrowRight").await?;
    focused(tab, GRID).await?;
    model(tab, "d.view === 'segment-grid-view' && d.activations === 0").await?;
    tab.press("ArrowLeft").await?;
    focused(tab, LIST).await?;
    tab.press("Space").await?;
    model(tab, "d.view === 'segment-list-view' && d.activations === 1").await?;
    tab.press("Enter").await?;
    model(tab, "d.view === 'segment-list-view' && d.activations === 1").await?;
    tab.press("Tab").await?;
    focused(tab, NEW).await?;
    tab.press("Space").await?;
    model(
        tab,
        "d.filters.includes('segment-new') && d.activations === 2",
    )
    .await?;
    tab.press("ArrowRight").await?;
    focused(tab, SAVED).await?;
    tab.press("ArrowRight").await?;
    focused(tab, SHARED).await?;
    model(tab, "d.filters.length === 1 && d.activations === 2").await?;
    for repeat in [false, true, true] {
        tab.cdp(
            "Input.dispatchKeyEvent",
            json!({"type":"keyDown","key":"Enter","code":"Enter","windowsVirtualKeyCode":13,"autoRepeat":repeat}),
        ).await?;
        model(
            tab,
            "d.filters.includes('segment-shared') && d.filters.length === 2 && d.activations === 3",
        )
        .await?;
    }
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({"type":"keyUp","key":"Enter","code":"Enter","windowsVirtualKeyCode":13}),
    )
    .await?;
    let selected = model(tab, "d.filters.length === 2").await?;
    let tree = ax_tree(tab).await?;
    let roles = json!({
        "view":states(&tree, &VIEW_ITEMS, "radio", &[LIST])?,
        "filters":states(&tree, &FILTER_ITEMS, "checkbox", &[NEW, SHARED])?,
    });
    let g = segment(tab, SHARED).await?;
    let frame = photo(tab, env, SELECTED).await?;
    let focus = paint_color(&selected, "focus")?;
    let focus_pixels = frame.colors(g.focus)?.get(&focus).copied().unwrap_or(0);
    check!(
        focus_pixels > 40,
        "keyboard focus is visible in the rendered frame"
    );
    tab.press("Space").await?;
    model(tab, "d.filters.length === 1 && d.activations === 4").await?;
    tab.press("ArrowLeft").await?;
    focused(tab, SAVED).await?;
    tab.press("ArrowLeft").await?;
    focused(tab, NEW).await?;
    tab.press("Enter").await?;
    let cleared = model(tab, "d.filters.length === 0 && d.activations === 5").await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    Ok(
        json!({"selected":selected,"cleared":cleared,"roles":roles,"focus":g.focus,
        "focus_pixels":focus_pixels,"capture":SELECTED}),
    )
}

async fn pressed(tab: &Tab, target: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        json!(target)
    ))
    .await?;
    tab.settle().await
}

async fn pointer(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    let canvas = tab.canvas_box().await?;
    let g = segment(tab, SAVED).await?;
    let x = centre(g.hit)[0];
    let y = g.hit[1] + 1.;
    check!(
        y < g.face[1],
        "test presses outside the artwork and inside the touch target"
    );
    let [cx, cy] = [canvas[0] + x, canvas[1] + y];
    tab.mouse_move(cx, cy).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(SAVED)).await?;
    let hold = model(tab, "d.filters.length === 0 && d.activations === 0").await?;
    photo(tab, env, PRESSED).await?;
    for (kind, buttons) in [("mouseMoved", 1), ("mouseReleased", 0)] {
        tab.cdp(
            "Input.dispatchMouseEvent",
            json!({
                "type":kind,"x":canvas[0] + 2.,"y":canvas[1] + 2.,
                "button":"left","buttons":buttons,"clickCount":1,
            }),
        )
        .await?;
    }
    pressed(tab, None).await?;
    model(tab, "d.filters.length === 0 && d.activations === 0").await?;

    let touch = json!({"x":cx,"y":cy,"id":1});
    for (end, requests) in [("touchCancel", 0), ("touchEnd", 1)] {
        tab.cdp(
            "Input.dispatchTouchEvent",
            json!({"type":"touchStart","touchPoints":[touch]}),
        )
        .await?;
        pressed(tab, Some(SAVED)).await?;
        tab.cdp(
            "Input.dispatchTouchEvent",
            json!({"type":end,"touchPoints":[]}),
        )
        .await?;
        pressed(tab, None).await?;
        model(tab, &format!("d.activations === {requests}")).await?;
    }
    let disabled = centre(bounds(tab, LOCAL).await?);
    tab.click(canvas[0] + disabled[0], canvas[1] + disabled[1])
        .await?;
    let final_state = model(
        tab,
        "d.filters.join() === 'segment-saved' && d.activations === 1",
    )
    .await?;
    Ok(json!({"hold":hold,"final":final_state,"point":[x,y],"capture":PRESSED}))
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, 320, 880).await?;
    for setting in [THEME, RTL, TEXT] {
        activate(tab, setting).await?;
    }
    let initial = model(
        tab,
        "d.dark && d.rtl && d.text_scale === 2 && d.width === 320",
    )
    .await?;
    for id in [LEFT, LIST, NEW] {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    let first = segment(tab, NEW).await?;
    let last = segment(tab, ARCHIVED).await?;
    check!(
        first.hit[0] > last.hit[0] && near(first.hit[1], last.hit[1]),
        "RTL is physically mirrored while Tab enters at the first logical choice"
    );
    check!(
        first.face[3] > 40.,
        "large labels grow the equal-height row"
    );
    let g = photo(tab, env, COMPACT).await?;
    let pixels = g
        .colors(first.focus)?
        .get(&paint_color(&initial, "focus")?)
        .copied()
        .unwrap_or(0);
    check!(pixels > 40, "dark compact keyboard ring is visible");

    resize(tab, 240, 880).await?;
    focused(tab, NEW).await?;
    let window = bounds(tab, FILTERS).await?;
    let start = part(tab, FILTERS, "track").await?;
    check!(
        start[2] > window[2] && contains(window, bounds(tab, NEW).await?),
        "the first touch target remains whole in the narrow scroll port"
    );
    for id in [SAVED, SHARED, ARCHIVED] {
        tab.press("ArrowLeft").await?;
        focused(tab, id).await?;
    }
    let end = part(tab, FILTERS, "track").await?;
    check!(
        end[0] > start[0] && contains(window, bounds(tab, ARCHIVED).await?),
        "logical movement reveals the other edge without reducing touch targets"
    );
    let archive = segment(tab, ARCHIVED).await?;
    tab.press("Space").await?;
    let final_state = model(
        tab,
        "d.width === 240 && d.filters.join() === 'segment-archived' && d.activations === 1",
    )
    .await?;
    let frame = photo(tab, env, SCROLLED).await?;
    let ring_pixels = frame
        .colors(archive.focus)?
        .get(&paint_color(&final_state, "focus")?)
        .copied()
        .unwrap_or(0);
    check!(
        ring_pixels > 40,
        "focused checkbox remains visible in the scrolled row"
    );
    resize(tab, 960, 2200).await?;
    activate(tab, BACK).await?;
    model(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"first":first,"last":last,"initial":initial,"final":final_state,"viewport":window,
        "track_before":start,"track_after":end,"compact_pixels":pixels,"focus_pixels":ring_pixels,
        "archive":archive,"returned_focus":SHOW,"captures":[COMPACT,SCROLLED]}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 924, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(
            STEPS[0],
            "Baseline segmented rows have equal faces, a single seam and distinct accessible roles",
        );
        route(&tab, url).await?;
        report["wide"] = wide(&tab, env).await?;
        step(
            STEPS[1],
            "trusted Tab and arrows move focus and explicit keys own selection",
        );
        report["keyboard"] = keyboard(&tab, env).await?;
        step(
            STEPS[2],
            "padded pointer or touch targets accept once and cancel cleanly",
        );
        report["pointer"] = pointer(&tab, env, url).await?;
        step(
            STEPS[3],
            "compact dark RTL grows the row and scrolls without reducing targets",
        );
        report["compact"] = compact(&tab, env, url).await?;
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-segmented-failure.png"))
            .await;
    }
    env.write_json("material-segmented.json", &report)?;
    result?;
    check!(tab.errors().is_empty(), "no segmented button page errors");
    Ok(())
}
