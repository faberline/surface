//! Trusted browser input, accessibility and WebGPU proof for Material chips.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "chip-demo-show";
const BACK: &str = "chip-demo-back";
const THEME: &str = "chip-demo-theme";
const RTL: &str = "chip-demo-rtl";
const LARGE: &str = "chip-demo-large";
const SINGLE: &str = "chip-demo-single";
const ASSIST: &str = "chip-demo-assist";
const ASSIST_RAISED: &str = "chip-demo-assist-elevated";
const SUGGEST: &str = "chip-demo-suggest";
const SUGGEST_RAISED: &str = "chip-demo-suggest-elevated";
const DISABLED: &str = "chip-demo-disabled";
const NEWS: &str = "chip-demo-news";
const IMAGES: &str = "chip-demo-images";
const TYPE: &str = "chip-demo-filter-type";
const TYPE_MENU: &str = "chip-demo-filter-type-menu";
const CHEN: &str = "chip-demo-chen";
const CHEN_REMOVE: &str = "chip-demo-chen-remove";
const MEI: &str = "chip-demo-mei";
const LIN: &str = "chip-demo-lin";
const LIN_REMOVE: &str = "chip-demo-lin-remove";
const DOCUMENTS: &str = "chip-demo-documents";
const LIN_LABEL: &str = "林小姐的工作用電子郵件";
const ACTIONS: [&str; 4] = [ASSIST, ASSIST_RAISED, SUGGEST, SUGGEST_RAISED];
const WIDE: (u32, u32) = (960, 800);
const COMPACT: (u32, u32) = (320, 1020);

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Adapter {
    vendor: String,
    architecture: String,
    is_fallback_adapter: bool,
}

#[derive(Deserialize, Serialize)]
struct Palette {
    surface: [u8; 4],
    surface_container_low: [u8; 4],
    outline_variant: [u8; 4],
    secondary_container: [u8; 4],
    secondary: [u8; 4],
}

#[derive(Deserialize, Serialize)]
struct Model {
    visible: bool,
    rtl: bool,
    text_scale: f64,
    multi: bool,
    compact: bool,
    menu_open: bool,
    selected: [bool; 3],
    removed: [bool; 3],
    actions: u32,
    edits: u32,
    removals: u32,
    last: Option<String>,
    palette: Palette,
}

#[derive(Serialize)]
struct Geometry {
    id: &'static str,
    kind: &'static str,
    hit: Rect,
    face: Rect,
    chip: Rect,
    label: Rect,
}

#[derive(Serialize)]
struct AxAction {
    id: &'static str,
    name: &'static str,
    role: &'static str,
    grid: &'static str,
    ancestry: Vec<String>,
    checked: Option<String>,
    disabled: bool,
}

#[derive(Serialize)]
struct Wide {
    canvas: [u32; 2],
    palette: Palette,
    variants: Vec<Geometry>,
    artwork: [[f64; 2]; 4],
    ax: Vec<AxAction>,
    surface_pixel: Rgb,
    elevated_fill_pixels: [Rgb; 3],
    selected_fill_pixel: Rgb,
    outline_pixel: Rgb,
    shadow_samples: [usize; 4],
    capture: &'static str,
}

#[derive(Serialize)]
struct Keyboard {
    stops: Vec<&'static str>,
    multi: [bool; 3],
    single: [bool; 3],
    wide_menu_trigger: &'static str,
    focus: &'static str,
    focus_rect: Rect,
    focus_pixels: usize,
    capture: &'static str,
}

#[derive(Serialize)]
struct Pointer {
    before: Rgb,
    held: Rgb,
    body_target: Rect,
    remove_target: Rect,
    body_point: [f64; 2],
    remove_point: [f64; 2],
    after_body: Model,
    after_canceled_touch: Model,
    after_remove: Model,
    restored_focus: [&'static str; 2],
    after_delete: Model,
    after_backspace: Model,
    capture: &'static str,
}

#[derive(Serialize)]
struct Compact {
    canvas: [u32; 2],
    rtl: bool,
    text_scale: f64,
    first_row: Rect,
    next_in_row: Rect,
    following_row: Rect,
    focus_path: [&'static str; 4],
    long_label: Rect,
    removal: Rect,
    label_ink: Rgb,
    label_line_ink: Vec<usize>,
    menu_body: &'static str,
    menu_ax: AxAction,
    selection_after_menu: [bool; 3],
    back_focus: &'static str,
    capture: &'static str,
}

#[derive(Serialize)]
struct Report {
    browser: String,
    adapter: Option<Adapter>,
    wide: Option<Wide>,
    keyboard: Option<Keyboard>,
    pointer: Option<Pointer>,
    compact: Option<Compact>,
    page_errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_at_failure: Option<Value>,
}

fn key(id: &str, part: &str) -> String {
    format!("{id}-{part}")
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

fn center(rect: Rect) -> [f64; 2] {
    [rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.]
}

fn rgb(color: [u8; 4]) -> Rgb {
    [color[0], color[1], color[2]]
}

async fn chip_state(tab: &Tab) -> Result<Model> {
    Ok(serde_json::from_value(
        state(tab).await?["chip_demo"].clone(),
    )?)
}

async fn ready(tab: &Tab, predicate: &str) -> Result<Model> {
    until(tab, &format!("(d => {predicate})(s.chip_demo)")).await?;
    tab.settle().await?;
    chip_state(tab).await
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.chip_demo.visible").await?;
    activate(tab, SHOW).await?;
    ready(
        tab,
        "d.visible && !d.rtl && d.text_scale === 1 && d.removals === 0",
    )
    .await?;
    focused(tab, BACK).await?;
    resize(tab, WIDE.0, WIDE.1).await?;
    tab.settle().await
}

async fn photo(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn geometry(tab: &Tab, id: &'static str, kind: &'static str) -> Result<Geometry> {
    Ok(Geometry {
        id,
        kind,
        hit: bounds(tab, id).await?,
        face: bounds(tab, &key(id, "face")).await?,
        chip: bounds(tab, &key(id, "chip")).await?,
        label: bounds(tab, &key(id, "label")).await?,
    })
}

fn ax_action(
    tree: &[Value],
    id: &'static str,
    name: &'static str,
    role: &'static str,
    grid: &'static str,
) -> Result<AxAction> {
    let node = ax(tree, role, name)?;
    let mut current = node;
    let mut ancestry = Vec::new();
    while let Some(parent) = current["parentId"].as_str() {
        let Some(target) = tree.iter().find(|item| item["nodeId"] == parent) else {
            break;
        };
        current = target;
        let role = current["role"]["value"].as_str().unwrap_or_default();
        if matches!(role, "gridcell" | "row" | "grid") {
            ancestry.push(role.to_owned());
        }
        if role == "grid" {
            break;
        }
    }
    check_eq!(
        ancestry,
        ["gridcell", "row", "grid"],
        format!("{id} AX grid cell ancestry")
    );
    check_eq!(
        current["name"]["value"],
        grid,
        format!("{id} owns the expected grid")
    );
    Ok(AxAction {
        id,
        name,
        role,
        grid,
        ancestry,
        checked: ax_property(node, "checked")
            .and_then(Value::as_str)
            .map(str::to_owned),
        disabled: ax_property(node, "disabled") == Some(&json!(true)),
    })
}

async fn wide(env: &Env, tab: &Tab) -> Result<Wide> {
    let model = ready(
        tab,
        "!d.compact && d.multi && d.selected.join() === 'true,false,false'",
    )
    .await?;
    let variants = [
        (ASSIST, "assist-flat"),
        (ASSIST_RAISED, "assist-elevated"),
        (NEWS, "filter-flat"),
        (IMAGES, "filter-elevated"),
        (CHEN, "input"),
        (SUGGEST, "suggestion-flat"),
        (SUGGEST_RAISED, "suggestion-elevated"),
    ];
    let mut measured = Vec::new();
    let viewport = bounds(tab, "chip-demo-preview").await?;
    for (id, kind) in variants {
        let g = geometry(tab, id, kind).await?;
        check!(
            near(g.face[3], 32.) && near(g.face[1] - g.hit[1], 8.),
            format!("{id} has 32 dp art inside the action")
        );
        check!(
            g.hit[2] >= 48. && g.hit[3] >= 48. && contains(g.hit, g.label),
            format!("{id} has a full 48 dp target and visible label")
        );
        check!(
            contains(g.chip, g.hit) && contains(viewport, g.chip),
            format!("{id} is fully in the clear preview")
        );
        measured.push(g);
    }
    let mut artwork = [[0.; 2]; 4];
    for (index, (name, size)) in [
        (key(ASSIST, "leading"), 18.),
        (key(ASSIST_RAISED, "leading"), 18.),
        (key(NEWS, "leading"), 18.),
        (key(CHEN, "leading"), 24.),
    ]
    .into_iter()
    .enumerate()
    {
        let art = bounds(tab, &name).await?;
        check!(
            near(art[2], size) && near(art[3], size),
            format!("{name} artwork measurement")
        );
        artwork[index] = [art[2], art[3]];
    }

    let tree = ax_tree(tab).await?;
    let mut actions = Vec::new();
    for (id, name, role, grid) in [
        (ASSIST, "排程", "button", "動作與建議"),
        (ASSIST_RAISED, "查看行程", "button", "動作與建議"),
        (DISABLED, "停用", "button", "動作與建議"),
        (NEWS, "新聞", "checkbox", "搜尋篩選"),
        (IMAGES, "圖片", "checkbox", "搜尋篩選"),
        (TYPE, "依檔案類型篩選", "checkbox", "搜尋篩選"),
        (TYPE_MENU, "開啟檔案類型選單", "button", "搜尋篩選"),
        (CHEN, "小陳", "button", "收件人"),
        (CHEN_REMOVE, "移除小陳", "button", "收件人"),
        (MEI, "移除小美", "button", "收件人"),
        (LIN, LIN_LABEL, "button", "收件人"),
        (LIN_REMOVE, "移除林小姐", "button", "收件人"),
    ] {
        let action = ax_action(&tree, id, name, role, grid)?;
        check_eq!(
            action.disabled,
            id == DISABLED,
            "disabled action in actual Chrome AX"
        );
        if role == "checkbox" {
            check_eq!(
                action.checked.as_deref(),
                Some(if id == NEWS { "true" } else { "false" }),
                "checkbox state in actual Chrome AX"
            );
        }
        actions.push(action);
    }

    let image = photo(env, tab, "material-chips-wide-light.png").await?;
    let fill = |face: Rect| image.sample(face[0] + 3., center(face)[1]);
    let raised = [
        fill(measured[1].face)?,
        fill(measured[3].face)?,
        fill(measured[6].face)?,
    ];
    let surface_pixel = fill(measured[0].face)?;
    let selected = fill(measured[2].face)?;
    let border = image.sample(measured[0].face[0] + 0.5, center(measured[0].face)[1])?;
    check_eq!(
        raised,
        [rgb(model.palette.surface_container_low); 3],
        "all elevated faces paint SurfaceContainerLow"
    );
    check_eq!(
        surface_pixel,
        rgb(model.palette.surface),
        "flat unselected face leaves the Surface color"
    );
    check_eq!(
        selected,
        rgb(model.palette.secondary_container),
        "selected filter face paints SecondaryContainer"
    );
    check_eq!(
        border,
        rgb(model.palette.outline_variant),
        "flat Assist uses the exact outline token"
    );
    let mut shadows = [0; 4];
    for (index, g) in [&measured[0], &measured[1], &measured[3], &measured[6]]
        .into_iter()
        .enumerate()
    {
        shadows[index] = image
            .colors([g.face[0] + 12., bottom(g.face) + 0.5, g.face[2] - 24., 4.])?
            .into_iter()
            .filter(|(color, _)| *color != surface_pixel)
            .map(|(_, count)| count)
            .sum();
    }
    check!(
        shadows[0] == 0 && shadows[1..].iter().all(|&count| count > 0),
        "only elevated chips paint short ambient shadows below the face"
    );
    Ok(Wide {
        canvas: [WIDE.0, WIDE.1],
        palette: model.palette,
        variants: measured,
        artwork,
        ax: actions,
        surface_pixel,
        elevated_fill_pixels: raised,
        selected_fill_pixel: selected,
        outline_pixel: border,
        shadow_samples: shadows,
        capture: "material-chips-wide-light.png",
    })
}

async fn menu_trigger(tab: &Tab, id: &str) -> Result<()> {
    for (attr, expected) in [("aria-haspopup", "menu"), ("aria-expanded", "false")] {
        let value = tab
            .eval(&format!("{}?.getAttribute({})", projected(id), js(attr)))
            .await?;
        check_eq!(value, json!(expected), format!("{id} {attr}"));
    }
    Ok(())
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Keyboard> {
    let mut stops = Vec::new();
    focus_projected(tab, SINGLE).await?;
    for (press, id) in [
        ("Tab", ASSIST),
        ("ArrowRight", ASSIST_RAISED),
        ("End", SUGGEST_RAISED),
        ("Tab", NEWS),
        ("Shift+Tab", ASSIST),
        ("Tab", NEWS),
        ("ArrowRight", IMAGES),
    ] {
        tab.press(press).await?;
        focused(tab, id).await?;
        stops.push(id);
    }
    ready(
        tab,
        "d.actions === 0 && d.edits === 0 && d.selected.join() === 'true,false,false'",
    )
    .await?;
    tab.press("Space").await?;
    let multi = ready(tab, "d.multi && d.selected.join() === 'true,true,false'")
        .await?
        .selected;
    activate(tab, SINGLE).await?;
    ready(tab, "!d.multi && d.selected.join() === 'true,false,false'").await?;
    activate(tab, IMAGES).await?;
    let single = ready(tab, "!d.multi && d.selected.join() === 'false,true,false'")
        .await?
        .selected;
    activate(tab, SINGLE).await?;
    ready(tab, "d.multi").await?;

    menu_trigger(tab, TYPE_MENU).await?;
    let whole = bounds(tab, &key(TYPE, "chip")).await?;
    let action = bounds(tab, TYPE).await?;
    let menu = bounds(tab, TYPE_MENU).await?;
    check!(
        contains(whole, action) && contains(whole, menu) && right(action) <= menu[0] + 0.1,
        "wide Filter keeps separate body and menu hit areas"
    );
    check!(
        menu[2] >= 48. && menu[3] >= 48.,
        "wide menu has a full action target"
    );
    activate(tab, TYPE_MENU).await?;
    ready(
        tab,
        "d.menu_open && d.selected.join() === 'false,true,false'",
    )
    .await?;
    ax(&ax_tree(tab).await?, "menuitem", "文件")?;
    tab.press("Escape").await?;
    ready(tab, "!d.menu_open && !d.selected[2]").await?;
    focused(tab, TYPE_MENU).await?;

    focus_projected(tab, NEWS).await?;
    tab.press("Tab").await?;
    focused(tab, CHEN).await?;
    tab.press("ArrowRight").await?;
    focused(tab, CHEN_REMOVE).await?;
    stops.extend([CHEN, CHEN_REMOVE]);
    let focus_rect = bounds(tab, &key(CHEN_REMOVE, "focus")).await?;
    check!(
        near(focus_rect[2], 28.) && near(focus_rect[3], 28.),
        "remove icon has an independent outer focus ring"
    );
    let pressed = chip_state(tab).await?;
    check_eq!(
        (pressed.actions, pressed.edits, pressed.removals),
        (0, 0, 0),
        "directional focus never activates a chip"
    );
    let image = photo(env, tab, "material-chips-wide-focus.png").await?;
    let focus_pixels = image
        .colors(focus_rect)?
        .get(&rgb(pressed.palette.secondary))
        .copied()
        .unwrap_or(0);
    check!(
        focus_pixels > 20,
        "the actual remove focus outline uses Secondary"
    );
    Ok(Keyboard {
        stops,
        multi,
        single,
        wide_menu_trigger: TYPE_MENU,
        focus: CHEN_REMOVE,
        focus_rect,
        focus_pixels,
        capture: "material-chips-wide-focus.png",
    })
}

async fn browser_point(tab: &Tab, at: [f64; 2]) -> Result<[f64; 2]> {
    let canvas = tab.canvas_box().await?;
    Ok([canvas[0] + at[0], canvas[1] + at[1]])
}

async fn touch(tab: &Tab, phase: &str, contact: Option<[f64; 2]>) -> Result<()> {
    let points = contact.map_or_else(Vec::new, |[x, y]| vec![json!({"x":x,"y":y,"id":1})]);
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":phase,"touchPoints":points}),
    )
    .await?;
    Ok(())
}

async fn pointer(env: &Env, tab: &Tab) -> Result<Pointer> {
    let assist = bounds(tab, ASSIST).await?;
    let face = bounds(tab, &key(ASSIST, "face")).await?;
    let at = [face[0] + 3., center(face)[1]];
    let before = Raster::read(tab, env.artifact("material-chips-wide-light.png"))
        .await?
        .sample(at[0], at[1])?;
    let [x, y] = browser_point(tab, center(assist)).await?;
    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    ready(tab, "d.actions === 0").await?;
    let held = photo(env, tab, "material-chips-wide-pressed.png")
        .await?
        .sample(at[0], at[1])?;
    check!(
        held != before,
        "accepted held pointer paints inside the visible Assist face"
    );
    let [outside_x, outside_y] =
        browser_point(tab, [WIDE.0 as f64 - 9., WIDE.1 as f64 - 9.]).await?;
    tab.mouse_move(outside_x, outside_y).await?;
    tab.mouse_up().await?;
    ready(tab, "d.actions === 0 && d.edits === 0 && d.removals === 0").await?;
    tab.click(x, y).await?;
    ready(tab, "d.actions === 1 && d.last === 'chip-demo-assist'").await?;

    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    let disabled = browser_point(tab, center(bounds(tab, DISABLED).await?)).await?;
    touch(tab, "touchStart", Some(disabled)).await?;
    touch(tab, "touchEnd", None).await?;
    ready(tab, "d.actions === 1 && d.edits === 0 && d.removals === 0").await?;
    check!(
        state(tab).await?["focused"] != DISABLED,
        "disabled touch cannot focus"
    );
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;

    let body = bounds(tab, CHEN).await?;
    let remove = bounds(tab, CHEN_REMOVE).await?;
    let icon = bounds(tab, &key(CHEN_REMOVE, "icon")).await?;
    check!(
        right(body) <= remove[0] + 0.1 && remove[2] >= 48. && remove[3] >= 48.,
        "split recipient has independent nonoverlapping body and remove targets"
    );
    let body_point = [body[0] + 2., body[1] + 4.];
    let remove_point = [right(remove) - 2., remove[1] + 4.];
    check!(
        !contains(icon, [remove_point[0], remove_point[1], 1., 1.]),
        "remove padding is outside its small glyph"
    );
    let [x, y] = browser_point(tab, body_point).await?;
    tab.click(x, y).await?;
    let after_body = ready(
        tab,
        "d.edits === 1 && d.removals === 0 && d.last === 'chip-demo-chen'",
    )
    .await?;
    focus(tab, CHEN, "surface").await?;
    let touch_point = browser_point(tab, remove_point).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    touch(tab, "touchStart", Some(touch_point)).await?;
    ready(tab, "d.edits === 1 && d.removals === 0").await?;
    touch(tab, "touchCancel", None).await?;
    let after_canceled_touch = ready(
        tab,
        "d.edits === 1 && d.removals === 0 && !d.removed.some(Boolean)",
    )
    .await?;
    touch(tab, "touchStart", Some(touch_point)).await?;
    ready(tab, "d.edits === 1 && d.removals === 0").await?;
    touch(tab, "touchEnd", None).await?;
    let after_remove = ready(
        tab,
        "d.edits === 1 && d.removals === 1 && d.removed.join() === 'true,false,false'",
    )
    .await?;
    until(tab, &format!("s.focused === {}", js(MEI))).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;

    focus_projected(tab, MEI).await?;
    tab.press("Delete").await?;
    let after_delete = ready(
        tab,
        "d.removals === 2 && d.removed.join() === 'true,true,false'",
    )
    .await?;
    focused(tab, LIN).await?;
    tab.press("Backspace").await?;
    let after_backspace = ready(
        tab,
        "d.removals === 3 && d.removed.every(Boolean) && d.edits === 1",
    )
    .await?;
    let tree = ax_tree(tab).await?;
    check!(
        tree.iter().all(|node| node["ignored"] != false
            || !["小陳", "移除小陳", "移除小美", LIN_LABEL, "移除林小姐"]
                .iter()
                .any(|name| node["name"]["value"] == *name)),
        "removed recipients leave the real AX tree"
    );
    Ok(Pointer {
        before,
        held,
        body_target: body,
        remove_target: remove,
        body_point,
        remove_point,
        after_body,
        after_canceled_touch,
        after_remove,
        restored_focus: [MEI, LIN],
        after_delete,
        after_backspace,
        capture: "material-chips-wide-pressed.png",
    })
}

async fn compact(env: &Env, tab: &Tab, url: &str) -> Result<Compact> {
    route(tab, url).await?;
    for toggle in [THEME, RTL, LARGE] {
        activate(tab, toggle).await?;
    }
    resize(tab, COMPACT.0, COMPACT.1).await?;
    until(tab, "s.dark").await?;
    let model = ready(tab, "d.visible && d.rtl && d.compact && d.text_scale === 2").await?;
    let mut placed = Vec::new();
    for id in ACTIONS {
        placed.push((id, bounds(tab, id).await?));
    }
    let first = placed[0];
    let [first_x, first_y] = center(first.1);
    let left = placed
        .iter()
        .copied()
        .filter(|(_, rect)| {
            let [x, y] = center(*rect);
            near(y, first_y) && x + 0.5 < first_x
        })
        .max_by(|(_, a), (_, b)| center(*a)[0].total_cmp(&center(*b)[0]))
        .context("320 dp has a real second action in the first RTL row")?;
    let next_y = placed
        .iter()
        .map(|(_, rect)| center(*rect)[1])
        .filter(|&y| y > first_y + 1.)
        .min_by(f64::total_cmp)
        .context("320 dp actions wrap to another measured row")?;
    let left_x = center(left.1)[0];
    let down = placed
        .iter()
        .copied()
        .filter(|(_, rect)| near(center(*rect)[1], next_y))
        .min_by(|(_, a), (_, b)| {
            (center(*a)[0] - left_x)
                .abs()
                .total_cmp(&(center(*b)[0] - left_x).abs())
        })
        .context("next rendered row has an eligible action")?;
    let down_x = center(down.1)[0];
    let up = placed
        .iter()
        .copied()
        .filter(|(_, rect)| near(center(*rect)[1], first_y))
        .min_by(|(_, a), (_, b)| {
            (center(*a)[0] - down_x)
                .abs()
                .total_cmp(&(center(*b)[0] - down_x).abs())
        })
        .context("previous rendered row has an eligible action")?;
    check!(
        down.1[1] >= bottom(left.1) - 1.,
        "target is on a distinct wrapped row"
    );
    focus_projected(tab, SINGLE).await?;
    for (press, id) in [
        ("Tab", first.0),
        ("ArrowLeft", left.0),
        ("ArrowDown", down.0),
        ("ArrowUp", up.0),
    ] {
        tab.press(press).await?;
        focused(tab, id)
            .await
            .with_context(|| format!("{press} should focus the measured target {id}"))?;
    }
    ready(tab, "d.actions === 0 && d.edits === 0 && d.removals === 0").await?;

    menu_trigger(tab, TYPE).await?;
    check!(
        tab.eval(&format!("{} === null", projected(TYPE_MENU)))
            .await?
            == true,
        "compact Filter does not expose a second menu trigger"
    );
    let compact_ax = ax_action(
        &ax_tree(tab).await?,
        TYPE,
        "依檔案類型篩選",
        "button",
        "搜尋篩選",
    )?;
    let body = bounds(tab, TYPE).await?;
    let glyph = bounds(tab, &key(TYPE, "trailing")).await?;
    let label_probe = [right(body) - 4., center(body)[1]];
    check!(
        !contains(glyph, [label_probe[0], label_probe[1], 1., 1.]),
        "the compact menu press lands inside the body away from its trailing glyph"
    );
    let [x, y] = browser_point(tab, label_probe).await?;
    tab.click(x, y).await?;
    ready(tab, "d.menu_open && !d.selected[2]").await?;
    ax(&ax_tree(tab).await?, "menuitem", "文件")?;
    focus_projected(tab, DOCUMENTS).await?;
    tab.press("Enter").await?;
    let accepted = ready(
        tab,
        "!d.menu_open && d.selected.join() === 'true,false,true'",
    )
    .await?;
    until(tab, &format!("s.focused === {}", js(TYPE))).await?;

    focus_projected(tab, TYPE).await?;
    tab.press("Tab").await?;
    focused(tab, CHEN).await?;
    for _ in 0..2 {
        if state(tab).await?["focused"] == LIN {
            break;
        }
        tab.press("ArrowDown").await?;
        tab.settle().await?;
    }
    focused(tab, LIN).await?;
    let long_label = bounds(tab, &key(LIN, "label")).await?;
    let removal = bounds(tab, LIN_REMOVE).await?;
    let recipient = bounds(tab, LIN).await?;
    let preview = bounds(tab, "chip-demo-preview").await?;
    check!(
        contains(preview, long_label)
            && contains(preview, removal)
            && right(removal) <= recipient[0] + 0.1
            && removal[2] >= 48.
            && removal[3] >= 48.,
        "RTL keeps the separate remove hit and all of the wrapped 200% label inside the preview"
    );
    ax_action(&ax_tree(tab).await?, LIN, LIN_LABEL, "button", "收件人")?;
    let label_rgba: [u8; 4] = serde_json::from_value(
        tab.eval(&format!(
            "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {}).style.foreground",
            js(key(LIN, "label"))
        ))
        .await?,
    )?;
    let label_ink = rgb(label_rgba);
    check!(
        label_ink != rgb(model.palette.surface),
        "dark label has a distinct foreground ink"
    );
    let image = photo(env, tab, "material-chips-compact-dark-rtl-200.png").await?;
    let line_height = 40.;
    let lines = (long_label[3] / line_height).round() as usize;
    check!(lines >= 2, "the 200 percent recipient actually wraps");
    let mut ink_counts = Vec::new();
    for line in 0..lines {
        let y = long_label[1] + line as f64 * line_height;
        let height = line_height.min(bottom(long_label) - y);
        let amount = image
            .colors([long_label[0], y, long_label[2], height])?
            .get(&label_ink)
            .copied()
            .unwrap_or(0);
        check!(
            amount > 20,
            format!("wrapped label line {line} has actual ink")
        );
        ink_counts.push(amount);
    }
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(Compact {
        canvas: [COMPACT.0, COMPACT.1],
        rtl: model.rtl,
        text_scale: model.text_scale,
        first_row: first.1,
        next_in_row: left.1,
        following_row: down.1,
        focus_path: [first.0, left.0, down.0, up.0],
        long_label,
        removal,
        label_ink,
        label_line_ink: ink_counts,
        menu_body: TYPE,
        menu_ax: compact_ax,
        selection_after_menu: accepted.selected,
        back_focus: SHOW,
        capture: "material-chips-compact-dark-rtl-200.png",
    })
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    let mut report = Report {
        browser: chrome.version.clone(),
        adapter: None,
        wide: None,
        keyboard: None,
        pointer: None,
        compact: None,
        page_errors: Vec::new(),
        failure: None,
        state_at_failure: None,
    };
    let result: Result<()> = async {
        step("M290", "all seven Material chip surfaces, targets, artwork and actual Chrome grid semantics");
        route(&tab, url).await?;
        report.adapter = Some(serde_json::from_value(tab.eval(
            "(async () => {
                const a = await navigator.gpu?.requestAdapter();
                if (!a) return null;
                const {vendor, architecture, isFallbackAdapter} = a.info;
                return {vendor, architecture, isFallbackAdapter};
            })()"
        ).await?).context("actual Chrome WebGPU adapter")?);
        report.wide = Some(wide(env, &tab).await.context("M290 wide variant and AX proof")?);
        step("M291", "keyboard grid traversal, controlled single/multiple filters, independent menu and focus paint");
        report.keyboard = Some(keyboard(env, &tab).await.context("M291 keyboard and menu proof")?);
        step("M292", "trusted pointer, touch cancellation, split recipient, deletion and retained focus");
        report.pointer = Some(pointer(env, &tab).await.context("M292 input and removal proof")?);
        step("M293", "320 dp dark RTL 200 percent measured rows, full label and whole-chip menu");
        report.compact = Some(compact(env, &tab, url).await.context("M293 compact and RTL proof")?);
        check!(tab.errors().is_empty(), "Chips journey has no Chrome page errors");
        Ok(())
    }.await;
    report.page_errors = tab.errors();
    if let Err(error) = &result {
        report.failure = Some(format!("{error:#}"));
        report.state_at_failure = state(&tab).await.ok();
        let _ = tab
            .screenshot(&env.artifact("material-chips-failure.png"))
            .await;
    }
    env.write_json("material-chips.json", &serde_json::to_value(&report)?)?;
    result
}
