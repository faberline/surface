//! Material Icon Buttons through trusted browser input and actual WebGPU paint.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::material_list_corners::{freeze, reduced_motion, tick};
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{focus, state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "icon-button-demo-show";
const BACK: &str = "icon-button-demo-back";
const THEME: &str = "icon-button-demo-theme";
const RTL: &str = "icon-button-demo-rtl";
const LARGE_TEXT: &str = "icon-button-demo-large-text";
const SCROLL: &str = "icon-button-demo-scroll";
const COMMAND: &str = "icon-button-demo-command";
const TOGGLE: &str = "icon-button-demo-toggle";
const DISABLED: &str = "icon-button-demo-disabled";
const MEDIUM: &str = "icon-button-demo-medium";
const LARGE: &str = "icon-button-demo-large";
const XL: &str = "icon-button-demo-extra-large";
const TOOLTIP: &str = "icon-button-demo-tooltip";
const DESCRIPTION: &str = "儲存目前對話";
const LIGHT: &str = "material-icon-buttons-wide-light.png";
const SELECTED: &str = "material-icon-buttons-selected-focus.png";
const HELD: &str = "material-icon-buttons-pressed.png";
const COMPACT: &str = "material-icon-buttons-compact-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

struct Specimen {
    id: &'static str,
    name: &'static str,
    face: [f64; 2],
    icon: f64,
}

const SPECIMENS: [Specimen; 6] = [
    Specimen {
        id: COMMAND,
        name: "儲存",
        face: [28., 32.],
        icon: 20.,
    },
    Specimen {
        id: TOGGLE,
        name: "加星號",
        face: [52., 40.],
        icon: 24.,
    },
    Specimen {
        id: DISABLED,
        name: "鎖定",
        face: [48., 56.],
        icon: 24.,
    },
    Specimen {
        id: MEDIUM,
        name: "搜尋",
        face: [56., 56.],
        icon: 24.,
    },
    Specimen {
        id: LARGE,
        name: "更多選項",
        face: [64., 96.],
        icon: 32.,
    },
    Specimen {
        id: XL,
        name: "收件匣",
        face: [184., 136.],
        icon: 40.,
    },
];

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Adapter {
    vendor: String,
    architecture: String,
    is_fallback_adapter: bool,
}

#[derive(Deserialize, Serialize)]
struct Palette {
    background: [u8; 4],
    primary: [u8; 4],
    on_primary: [u8; 4],
    secondary: [u8; 4],
    on_secondary: [u8; 4],
    secondary_container: [u8; 4],
    on_secondary_container: [u8; 4],
    outline_variant: [u8; 4],
}

#[derive(Deserialize, Serialize)]
struct Model {
    visible: bool,
    rtl: bool,
    text_scale: f64,
    checked: bool,
    requests: u32,
    last: Option<String>,
    width: f64,
    height: f64,
    reduced_motion: bool,
    tooltip_shown: bool,
    palette: Palette,
}

#[derive(Serialize)]
struct Geometry {
    id: &'static str,
    hit: Rect,
    face: Rect,
    icon: Rect,
    focus: Rect,
}

#[derive(Serialize)]
struct AxButton {
    id: &'static str,
    name: &'static str,
    role: String,
    disabled: bool,
    pressed: Option<String>,
}

#[derive(Serialize)]
struct Wide {
    viewport: [u32; 2],
    geometry: Vec<Geometry>,
    ax: Vec<AxButton>,
    palette: Palette,
    command_fill: Rgb,
    toggle_off_fill: Rgb,
    toggle_off_corner: Rgb,
    outline: Rgb,
    command_icon_pixels: usize,
    off_ink: Vec<[u8; 2]>,
    capture: &'static str,
}

#[derive(Serialize)]
struct Keyboard {
    focus_path: Vec<&'static str>,
    requests: [u32; 3],
    repeated_spaces: usize,
    checked_ax: AxButton,
    unchecked_ax: AxButton,
    selected_fill: Rgb,
    toggle_on_corner: Rgb,
    on_ink: Vec<[u8; 2]>,
    focus_pixels: usize,
    capture: &'static str,
}

#[derive(Serialize)]
struct Pointer {
    slack_point: [f64; 2],
    outside_point: [f64; 2],
    face: Rect,
    corner_patch: Rect,
    normal_times_ms: [u16; 4],
    corner_pixels: [usize; 4],
    reduced_corner_pixels: usize,
    requests_after_cancel: u32,
    requests_after_hit: u32,
    capture: &'static str,
}

#[derive(Serialize)]
struct Compact {
    viewport: [u32; 2],
    rtl: bool,
    text_scale: f64,
    scroll: Rect,
    geometry: Vec<Geometry>,
    tooltip: Rect,
    description: &'static str,
    command_fill: Rgb,
    command_icon_pixels: usize,
    palette: Palette,
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

fn part(id: &str, part: &str) -> String {
    format!("{id}-{part}")
}

fn rgb([r, g, b, _]: [u8; 4]) -> Rgb {
    [r, g, b]
}

fn focus_fill(pixel: Rgb, base: Rgb, ink: Rgb) -> bool {
    pixel
        .into_iter()
        .zip(base)
        .zip(ink)
        .all(|((actual, base), ink)| {
            let expected = (u16::from(base) * 229 + u16::from(ink) * 26) / 255;
            u16::from(actual).abs_diff(expected) <= 2
        })
}

fn middle(rect: Rect) -> [f64; 2] {
    [rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.]
}

async fn ready(tab: &Tab, condition: &str) -> Result<Model> {
    until(tab, &format!("(d => {condition})(s.icon_button_demo)")).await?;
    tab.settle().await?;
    Ok(serde_json::from_value(
        state(tab).await?["icon_button_demo"].clone(),
    )?)
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.icon_button_demo.visible").await?;
    activate(tab, SHOW).await?;
    ready(tab, "d.visible && !d.checked && d.requests === 0").await?;
    resize(tab, 960, 800).await
}

async fn image(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    let bytes = Raster::read(tab, path.clone()).await;
    if name == "material-icon-buttons-probe.png" {
        std::fs::remove_file(path)?;
    }
    bytes
}

async fn geom(tab: &Tab, specimen: &Specimen) -> Result<Geometry> {
    let id = specimen.id;
    let hit = bounds(tab, id).await?;
    let face = bounds(tab, &part(id, "face")).await?;
    let icon = bounds(tab, &part(id, "icon")).await?;
    let focus = bounds(tab, &part(id, "focus")).await?;
    for (i, dimension) in specimen.face.into_iter().enumerate() {
        check!(
            near(face[i + 2], dimension) && near(hit[i + 2], dimension.max(48.)),
            format!("{id} face uses its token extent and the target is at least 48 dp")
        );
        check!(
            near(icon[i + 2], specimen.icon) && near(focus[i + 2], dimension + 10.),
            format!("{id} keeps token icon size and separate focus outset")
        );
        check!(
            near(middle(face)[i], middle(hit)[i]) && near(middle(icon)[i], middle(hit)[i]),
            format!("{id} centers both art and face inside the hit area")
        );
    }
    Ok(Geometry {
        id,
        hit,
        face,
        icon,
        focus,
    })
}

fn ax_action(tree: &[Value], specimen: &Specimen, expected: Option<bool>) -> Result<AxButton> {
    let matching: Vec<_> = tree
        .iter()
        .filter(|node| {
            node["ignored"] == false
                && node["name"]["value"] == specimen.name
                && matches!(
                    node["role"]["value"].as_str(),
                    Some("button" | "toggleButton")
                )
        })
        .collect();
    check_eq!(
        matching.len(),
        1,
        format!("{} has one named Chrome action", specimen.id)
    );
    let node = matching[0];
    let role = node["role"]["value"].as_str().context("AX button role")?;
    let disabled = ax_property(node, "disabled") == Some(&json!(true));
    let pressed = ax_property(node, "pressed").and_then(Value::as_str);
    check_eq!(
        disabled,
        specimen.id == DISABLED,
        "actual AX disabled state"
    );
    check_eq!(
        pressed,
        expected.map(|on| if on { "true" } else { "false" }),
        "actual AX pressed state"
    );
    check!(
        expected.is_some() || role == "button",
        "commands remain ordinary buttons"
    );
    Ok(AxButton {
        id: specimen.id,
        name: specimen.name,
        role: role.to_owned(),
        disabled,
        pressed: pressed.map(str::to_owned),
    })
}

fn ink(image: &Raster, at: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(at)?.get(&color).copied().unwrap_or(0))
}

fn mask(image: &Raster, icon: Rect, color: Rgb) -> Result<Vec<[u8; 2]>> {
    let mut points = Vec::new();
    for y in 0..24 {
        for x in 0..24 {
            if image.sample(icon[0] + f64::from(x) + 0.5, icon[1] + f64::from(y) + 0.5)? == color {
                points.push([x, y]);
            }
        }
    }
    Ok(points)
}

fn fill(image: &Raster, face: Rect) -> Result<Rgb> {
    image.sample(middle(face)[0], face[1] + 3.)
}

async fn wide(env: &Env, tab: &Tab) -> Result<Wide> {
    let model = ready(
        tab,
        "d.visible && !d.rtl && d.text_scale === 1 && d.requests === 0",
    )
    .await?;
    let scroll = bounds(tab, SCROLL).await?;
    let mut geometry = Vec::new();
    for specimen in &SPECIMENS {
        let g = geom(tab, specimen).await?;
        check!(
            contains(scroll, g.hit),
            format!("{} is fully in the wide scroller", specimen.id)
        );
        geometry.push(g);
    }
    let tree = ax_tree(tab).await?;
    let ax = SPECIMENS
        .iter()
        .map(|specimen| ax_action(&tree, specimen, (specimen.id == TOGGLE).then_some(false)))
        .collect::<Result<_>>()?;
    let image = image(env, tab, LIGHT).await?;
    let command_fill = fill(&image, geometry[0].face)?;
    let toggle_off_fill = fill(&image, geometry[1].face)?;
    let toggle_face = geometry[1].face;
    let toggle_off_corner = image.sample(toggle_face[0] + 2., toggle_face[1] + 8.)?;
    let outline = image.sample(geometry[3].face[0] + 0.5, middle(geometry[3].face)[1])?;
    check_eq!(
        command_fill,
        rgb(model.palette.primary),
        "Filled command token paint"
    );
    check_eq!(
        toggle_off_fill,
        rgb(model.palette.secondary_container),
        "unchecked Tonal token paint"
    );
    check_eq!(
        toggle_off_corner,
        rgb(model.palette.background),
        "the unchecked round corner leaves the background clear"
    );
    check_eq!(
        outline,
        rgb(model.palette.outline_variant),
        "Outlined square border token paint"
    );
    let command_icon_pixels = ink(&image, geometry[0].icon, rgb(model.palette.on_primary))?;
    let off_ink = mask(
        &image,
        geometry[1].icon,
        rgb(model.palette.on_secondary_container),
    )?;
    check!(
        command_icon_pixels > 40 && off_ink.len() > 12,
        "the two actual vector shapes paint their ink tokens"
    );
    Ok(Wide {
        viewport: [960, 800],
        geometry,
        ax,
        palette: model.palette,
        command_fill,
        toggle_off_fill,
        toggle_off_corner,
        outline,
        command_icon_pixels,
        off_ink,
        capture: LIGHT,
    })
}

async fn keyboard(env: &Env, tab: &Tab, wide: &Wide) -> Result<Keyboard> {
    focus_projected(tab, LARGE_TEXT).await?;
    tab.press("Tab").await?;
    focused(tab, COMMAND).await?;
    tab.press("Enter").await?;
    let command = ready(
        tab,
        "d.requests === 1 && !d.checked && d.last === 'icon-button-demo-command'",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, TOGGLE).await?;
    tab.eval("window.iconTestKeys=[]; document.addEventListener('keydown', event => {
        if(event.code==='Space') window.iconTestKeys.push({repeat:event.repeat,trusted:event.isTrusted,id:event.target.dataset?.surfaceId});
    },true)").await?;
    for repeat in [false, true, true] {
        tab.cdp(
            "Input.dispatchKeyEvent",
            json!({"type":"keyDown","key":" ","code":"Space",
            "windowsVirtualKeyCode":32,"text":" ","unmodifiedText":" ","autoRepeat":repeat}),
        )
        .await?;
        ready(
            tab,
            "d.requests === 2 && d.checked && d.last === 'icon-button-demo-toggle'",
        )
        .await?;
    }
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({"type":"keyUp","key":" ","code":"Space","windowsVirtualKeyCode":32}),
    )
    .await?;
    let events = tab.eval("window.iconTestKeys").await?;
    check_eq!(
        events,
        json!([{ "repeat":false,"trusted":true,"id":TOGGLE },
        { "repeat":true,"trusted":true,"id":TOGGLE }, { "repeat":true,"trusted":true,"id":TOGGLE }]),
        "repeat keys actually reach the focused Chrome control without activation"
    );
    let checked_ax = ax_action(&ax_tree(tab).await?, &SPECIMENS[1], Some(true))?;
    let g = &wide.geometry[1];
    let image = image(env, tab, SELECTED).await?;
    let selected_fill = fill(&image, g.face)?;
    let toggle_on_corner = image.sample(g.face[0] + 2., g.face[1] + 8.)?;
    let on_ink = mask(&image, g.icon, rgb(wide.palette.on_secondary))?;
    let focus_pixels = ink(&image, g.focus, rgb(wide.palette.secondary))?.saturating_sub(ink(
        &image,
        g.face,
        rgb(wide.palette.secondary),
    )?);
    check!(
        focus_fill(
            selected_fill,
            rgb(wide.palette.secondary),
            rgb(wide.palette.on_secondary)
        ) && toggle_on_corner == selected_fill
            && toggle_on_corner != wide.toggle_off_corner,
        "selected Tonal fill, focus ink blend, and square corners paint together"
    );
    check!(
        on_ink.len() > 12 && on_ink != wide.off_ink,
        "independent checked artwork changes its rendered mask"
    );
    check!(
        focus_pixels > 40,
        "keyboard focus paints outside the selected face"
    );
    tab.press("Tab").await?;
    focused(tab, MEDIUM).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, TOGGLE).await?;
    tab.press("Enter").await?;
    let released = ready(
        tab,
        "d.requests === 3 && !d.checked && d.last === 'icon-button-demo-toggle'",
    )
    .await?;
    let unchecked_ax = ax_action(&ax_tree(tab).await?, &SPECIMENS[1], Some(false))?;
    Ok(Keyboard {
        focus_path: vec![COMMAND, TOGGLE, MEDIUM, TOGGLE],
        requests: [command.requests, 2, released.requests],
        repeated_spaces: 2,
        checked_ax,
        unchecked_ax,
        selected_fill,
        toggle_on_corner,
        on_ink,
        focus_pixels,
        capture: SELECTED,
    })
}

async fn point(tab: &Tab, at: [f64; 2]) -> Result<[f64; 2]> {
    let canvas = tab.canvas_box().await?;
    Ok([canvas[0] + at[0], canvas[1] + at[1]])
}

async fn pressed(tab: &Tab, expected: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        json!(expected)
    ))
    .await?;
    tab.settle().await
}

fn corner(image: &Raster, region: Rect, background: Rgb) -> Result<usize> {
    Ok(image
        .colors(region)?
        .iter()
        .filter(|(color, _)| **color != background)
        .map(|(_, count)| count)
        .sum())
}

async fn pointer(env: &Env, tab: &Tab, url: &str) -> Result<Pointer> {
    route(tab, url).await?;
    reduced_motion(tab, false).await?;
    let model = ready(tab, "!d.reduced_motion && !d.tooltip_shown").await?;
    let g = geom(tab, &SPECIMENS[0]).await?;
    let disabled = bounds(tab, DISABLED).await?;
    let [x, y] = point(tab, middle(disabled)).await?;
    tab.click(x, y).await?;
    ready(tab, "d.requests === 0").await?;
    check!(
        state(tab).await?["focused"] != DISABLED,
        "disabled pointer activation and focus stay inert"
    );
    let slack_point = [g.hit[0] + 3., middle(g.hit)[1]];
    let outside_point = [7., 790.];
    check!(
        slack_point[0] < g.face[0] && contains(g.hit, [slack_point[0], slack_point[1], 1., 1.]),
        "physical press is inside the 48 dp target and in the slack outside its face"
    );
    let [x, y] = point(tab, slack_point).await?;
    freeze(tab).await?;
    tab.mouse_move(x, y).await?;
    tab.settle().await?;
    let region = [g.face[0] + 0.5, g.face[1] + 3., 6., 8.];
    let background = rgb(model.palette.background);
    let probe = "material-icon-buttons-probe.png";
    let rest = corner(&image(env, tab, probe).await?, region, background)?;
    tab.mouse_down().await?;
    pressed(tab, Some(COMMAND)).await?;
    let start = corner(&image(env, tab, probe).await?, region, background)?;
    tick(tab, 28).await?;
    let early = corner(&image(env, tab, probe).await?, region, background)?;
    tick(tab, 240).await?;
    let held = corner(&image(env, tab, HELD).await?, region, background)?;
    check!(start == rest && rest < early && early < held, "pressed corners start from the rest pose, curve through an intermediate and reach a stronger silhouette");
    ready(tab, "d.requests === 0").await?;
    let [out_x, out_y] = point(tab, outside_point).await?;
    for (kind, buttons) in [("mouseMoved", 1), ("mouseReleased", 0)] {
        tab.cdp(
            "Input.dispatchMouseEvent",
            json!({"type":kind,"x":out_x,"y":out_y,"button":"left",
            "buttons":buttons,"clickCount":1}),
        )
        .await?;
    }
    pressed(tab, None).await?;
    tick(tab, 480).await?;
    let released = corner(&image(env, tab, probe).await?, region, background)?;
    let canceled = ready(tab, "d.requests === 0").await?;
    check_eq!(
        released,
        rest,
        "release outside restores the command contour without activating"
    );

    reduced_motion(tab, true).await?;
    ready(tab, "d.reduced_motion").await?;
    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(COMMAND)).await?;
    let snapped = corner(&image(env, tab, probe).await?, region, background)?;
    check!(
        snapped > early && snapped.abs_diff(held) <= 8,
        "Reduce Motion renders the pressed silhouette without advancing a frame clock"
    );
    ready(tab, "d.requests === 0").await?;
    tab.mouse_up().await?;
    let accepted = ready(
        tab,
        "d.requests === 1 && d.last === 'icon-button-demo-command'",
    )
    .await?;
    focus(tab, COMMAND, "surface").await?;
    Ok(Pointer {
        slack_point,
        outside_point,
        face: g.face,
        corner_patch: region,
        normal_times_ms: [0, 28, 240, 480],
        corner_pixels: [rest, early, held, released],
        reduced_corner_pixels: snapped,
        requests_after_cancel: canceled.requests,
        requests_after_hit: accepted.requests,
        capture: HELD,
    })
}

async fn compact(env: &Env, tab: &Tab, url: &str) -> Result<Compact> {
    route(tab, url).await?;
    for id in [THEME, RTL, LARGE_TEXT] {
        activate(tab, id).await?;
    }
    resize(tab, 320, 760).await?;
    until(tab, "s.dark").await?;
    focus_projected(tab, LARGE_TEXT).await?;
    tab.press("Tab").await?;
    focused(tab, COMMAND).await?;
    let model = ready(
        tab,
        "d.rtl && d.text_scale === 2 && d.tooltip_shown && d.requests === 0",
    )
    .await?;
    check!(
        near(model.width, 320.) && near(model.height, 760.),
        "compact width and height agree with the actual canvas"
    );
    let tip = projected(TOOLTIP);
    let target = projected(COMMAND);
    tab.wait_for(&format!(
        "(() => {{ const tip={tip}; const target={target};
        return tip?.getAttribute('role')==='tooltip' && !!tip.id
            && target?.getAttribute('aria-describedby')?.split(' ').includes(tip.id); }})()"
    ))
    .await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "tooltip", DESCRIPTION)?;
    let action = ax(&tree, "button", SPECIMENS[0].name)?;
    check_eq!(
        action["description"]["value"],
        DESCRIPTION,
        "Chrome exposes the separate tooltip as an action description"
    );
    let tooltip = bounds(tab, TOOLTIP).await?;
    let scroll = bounds(tab, SCROLL).await?;
    let viewport = [0., 0., 320., 760.];
    let mut geometry = Vec::new();
    for specimen in &SPECIMENS {
        let g = geom(tab, specimen).await?;
        check!(
            contains(viewport, g.hit) && contains(scroll, g.hit),
            format!("{} remains fully reachable in compact", g.id)
        );
        geometry.push(g);
    }
    check!(
        geometry[0].hit[0] > geometry[1].hit[0]
            && near(middle(geometry[0].hit)[1], middle(geometry[1].hit)[1]),
        "fixture mirrors the first two authored destinations visually without changing focus order"
    );
    check!(
        contains(viewport, tooltip) && tooltip[1] + tooltip[3] <= geometry[0].hit[1] + 0.5,
        format!("tooltip {tooltip:?} must fit the compact viewport above hit {:?}, face {:?} and focus {:?}",
            geometry[0].hit, geometry[0].face, geometry[0].focus)
    );
    let presented = image(env, tab, COMPACT).await?;
    let command_fill = fill(&presented, geometry[0].face)?;
    let command_icon_pixels = ink(&presented, geometry[0].icon, rgb(model.palette.on_primary))?;
    check!(
        focus_fill(
            command_fill,
            rgb(model.palette.primary),
            rgb(model.palette.on_primary)
        ) && command_icon_pixels > 40,
        "the actual dark icon remains readable with its token palette"
    );
    tab.press("Escape").await?;
    ready(tab, "!d.tooltip_shown && d.requests === 0").await?;
    focused(tab, COMMAND).await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(Compact {
        viewport: [320, 760],
        rtl: model.rtl,
        text_scale: model.text_scale,
        scroll,
        geometry,
        tooltip,
        description: DESCRIPTION,
        command_fill,
        command_icon_pixels,
        palette: model.palette,
        capture: COMPACT,
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
        page_errors: vec![],
        failure: None,
        state_at_failure: None,
    };
    let result: Result<()> = async {
        step("M294", "Icon Button visual tokens, independent targets, and actual Chrome action states");
        route(&tab, url).await?;
        let value = tab.eval("(async()=>{ const a = await navigator.gpu.requestAdapter();
            const {vendor,architecture,isFallbackAdapter} = a.info; return {vendor,architecture,isFallbackAdapter}; })()").await?;
        report.adapter = Some(serde_json::from_value(value)?);
        report.wide = Some(wide(env, &tab).await.context("M294 wide Icon Button fixtures")?);
        step("M295", "Icon Button keyboard activation, held repeat, checked artwork, and focus outline");
        report.keyboard = Some(keyboard(env, &tab, report.wide.as_ref().expect("wide proof")).await.context("M295 keyboard Icon Buttons")?);
        step("M296", "Icon Button pointer hit slack, animated press, cancel, and Reduce Motion");
        report.pointer = Some(pointer(env, &tab, url).await.context("M296 pointer Icon Buttons")?);
        step("M297", "Icon Button compact dark RTL at 200 percent, tooltip description, and focus restore");
        report.compact = Some(compact(env, &tab, url).await.context("M297 compact Icon Buttons")?);
        Ok(())
    }.await;
    report.page_errors = tab.errors();
    if let Err(error) = &result {
        report.failure = Some(format!("{error:#}"));
        report.state_at_failure = state(&tab).await.ok();
        let _ = tab
            .screenshot(&env.artifact("material-icon-buttons-failure.png"))
            .await;
    }
    env.write_json(
        "material-icon-buttons.json",
        &serde_json::to_value(&report)?,
    )?;
    result?;
    check!(
        report.page_errors.is_empty(),
        "Icon Button Chrome page has no errors"
    );
    Ok(())
}
