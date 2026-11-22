//! Split button rendering, input and Chrome accessibility through WebGPU.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader, time::Duration};

const TRACK: &str = "split-track";
const MAIN: &str = "split-track-leading";
const OPTIONS: &str = "split-track-trailing";
const MAIN_FACE: &str = "split-track-leading-face";
const OPTIONS_FACE: &str = "split-track-trailing-face";
const ARROW: &str = "split-track-chevron";
const LABEL: &str = "split-track-label";
const MENU: &str = "split-track-menu";
const MENU_ITEM: [&str; 3] = [
    "split-menu-item-0",
    "split-menu-item-1",
    "split-menu-item-2",
];
const SIZES: [(&str, f64); 5] = [
    ("xs", 32.),
    ("s", 40.),
    ("m", 56.),
    ("l", 96.),
    ("xl", 136.),
];
const APPEARANCES: [&str; 4] = ["elevated", "filled", "tonal", "outlined"];
type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["split_button"].clone())
}

async fn ready(tab: &Tab, expression: &str) -> Result<()> {
    until(tab, &format!("(d => {expression})(s.split_button)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, setting: &str, condition: &str) -> Result<()> {
    click(tab, &format!("split-{setting}")).await?;
    ready(tab, condition).await
}

async fn choice(tab: &Tab, field: &str, value: &str) -> Result<()> {
    select(
        tab,
        &format!("{field}-{value}"),
        &format!("d.{field} === {}", js(value)),
    )
    .await
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
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
        .context("four bounds")
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
        click(tab, "split-theme").await?;
    }
    until(tab, &format!("s.dark === {dark}")).await?;
    tab.settle().await
}

async fn save_png(env: &Env, tab: &Tab, label: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-split-button-{label}.png")))
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
        let path = env.artifact(&format!("material-split-button-{label}.png"));
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
            bail!("split button probe outside the screenshot");
        }
        let offset = y * self.frame.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }
}

fn distance(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn painted_color(value: &Value, background: Rgb) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(value.clone()).context("RGBA token")?;
    let opacity = f64::from(alpha) / 255.;
    Ok(std::array::from_fn(|i| {
        let channel = f64::from([r, g, b][i]) * opacity + f64::from(background[i]) * (1. - opacity);
        channel.round() as u8
    }))
}

async fn sizes_and_paint(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M89",
        "five split sizes preserve two full targets and four appearances paint in both themes",
    );
    ready(
        tab,
        "d.appearance === 'filled' && d.size === 'm' && d.leading === 'both'",
    )
    .await?;
    let mut sizes = Vec::new();
    for (size, height) in SIZES {
        choice(tab, "size", size).await?;
        let track = bounds(tab, TRACK).await?;
        let main = bounds(tab, MAIN).await?;
        let options = bounds(tab, OPTIONS).await?;
        let main_face = bounds(tab, MAIN_FACE).await?;
        let options_face = bounds(tab, OPTIONS_FACE).await?;
        for (hit, face) in [(main, main_face), (options, options_face)] {
            check!(
                contains(track, hit)
                    && contains(hit, face)
                    && hit[2] >= 47.25
                    && hit[3] >= 47.25
                    && near(face[3], height),
                "{size} paints a {height} dp face inside a full target"
            );
        }
        check!(
            main[0] + main[2] <= options[0] + 0.75 && near(main_face[1], options_face[1]),
            "{size} keeps the two hit targets separate on one row"
        );
        sizes.push(json!({"size":size,"main":main,"options":options,"main_face":main_face,"options_face":options_face}));
    }
    choice(tab, "size", "m").await?;

    choice(tab, "leading", "icon").await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(LABEL)
    ))
    .await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-label') === '匯出報告'",
        projected(MAIN)
    ))
    .await?;
    let icon_only = bounds(tab, MAIN).await?;
    choice(tab, "leading", "text").await?;
    let text_only = bounds(tab, LABEL).await?;
    choice(tab, "leading", "both").await?;
    let both = bounds(tab, MAIN).await?;
    check!(
        both[2] > icon_only[2] + 4. && contains(both, bounds(tab, LABEL).await?),
        "leading text, icon and combined content use their measured widths"
    );

    let mut samples = Vec::new();
    for dark in [false, true] {
        theme(tab, dark).await?;
        let mut shadow_pixels = Vec::new();
        for appearance in APPEARANCES {
            choice(tab, "appearance", appearance).await?;
            tab.mouse_move(4., 52.).await?;
            tokio::time::sleep(Duration::from_millis(200)).await;
            let export = dom(tab).await?;
            let [x, y, w, h] = bounds(tab, MAIN_FACE).await?;
            let image = Image::capture(
                env,
                tab,
                &format!("{appearance}-{}", if dark { "dark" } else { "light" }),
            )
            .await?;
            let background = image.sample(x + w / 2., y + h + 9.)?;
            let mut pair = Vec::new();
            for face in [MAIN_FACE, OPTIONS_FACE] {
                let node = find_node(&export, face)?;
                let style = &node["style"];
                let [left, top, _, height] = bounds(tab, face).await?;
                let fill = image.sample(left + 6., top + height / 2.)?;
                let expected = painted_color(&style["background"], background)?;
                check!(
                    distance(fill, expected) <= 12,
                    "{face} {appearance} paints {fill:?} near {expected:?}"
                );
                let border = style["border_width"].as_f64().context("outline width")?;
                check_eq!(
                    border > 0.,
                    appearance == "outlined",
                    "only outlined uses a stroke"
                );
                let stroke = if border > 0. {
                    let expected = painted_color(&style["border_color"], background)?;
                    let candidates = [0.25, 0.75, 1.25];
                    let mut best = fill;
                    for inset in candidates {
                        let pixel = image.sample(left + inset, top + height / 2.)?;
                        if distance(pixel, expected) < distance(best, expected) {
                            best = pixel;
                        }
                    }
                    check!(
                        distance(best, expected) <= 18 && distance(best, fill) > 16,
                        "outlined split button paints its edge stroke"
                    );
                    Some(best)
                } else {
                    None
                };
                pair.push(json!({"fill":fill,"stroke":stroke}));
            }
            let shadow = image.sample(x + w / 2., y + h + 0.75)?;
            shadow_pixels.push(shadow);
            samples.push(json!({"appearance":appearance,"dark":dark,"halves":pair,"below":shadow}));
        }
        check!(
            distance(shadow_pixels[0], shadow_pixels[1]) > 3
                && shadow_pixels[0]
                    .iter()
                    .zip(shadow_pixels[1])
                    .all(|(&a, b)| a <= b),
            "elevation darkens the real background beside the split face"
        );
    }
    theme(tab, false).await?;
    choice(tab, "appearance", "filled").await?;
    Ok(
        json!({"sizes":sizes,"palettes":samples,"icon_only":icon_only,"text_only_label":text_only,"combined":both}),
    )
}

async fn ax_node(tab: &Tab, role: &str, name: &str) -> Result<Value> {
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
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
        .with_context(|| format!("Chrome accessibility {role} named {name}"))
}

fn ax_property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|property| property["name"] == name)
        .map(|property| &property["value"]["value"])
}

async fn menu_hint(tab: &Tab, id: &str) -> Result<()> {
    let trigger = projected(id);
    tab.wait_for(&format!(
        "{trigger}?.getAttribute('aria-haspopup') === 'menu' && {trigger}.getAttribute('aria-expanded') === 'false'"
    )).await
}

async fn menu_focus(tab: &Tab, target: &str, follows_projection: bool) -> Result<()> {
    if follows_projection {
        focused(tab, target).await
    } else {
        until(tab, &format!("s.focused === {}", js(target))).await
    }
}

async fn revealed_menu(tab: &Tab, follows_projection: bool) -> Result<()> {
    ready(tab, "d.menu_open").await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('role') === 'menu'",
        projected(MENU)
    ))
    .await?;
    menu_focus(tab, MENU_ITEM[0], follows_projection).await
}

async fn closed_menu(tab: &Tab, target: &str, follows_projection: bool) -> Result<()> {
    ready(tab, "!d.menu_open").await?;
    tab.wait_for(&format!("{} === null", projected(MENU)))
        .await?;
    menu_focus(tab, target, follows_projection).await?;
    menu_hint(tab, OPTIONS).await
}

async fn pointer_and_menu(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M90",
        "the primary action stays independent and Chrome exposes a menu disclosure",
    );
    menu_hint(tab, OPTIONS).await?;
    let trigger_ax = ax_node(tab, "button", "匯出選項").await?;
    let main_ax = ax_node(tab, "button", "匯出報告").await?;
    check_eq!(
        ax_property(&trigger_ax, "hasPopup"),
        Some(&json!("menu")),
        "Chrome identifies the menu trigger"
    );
    check_eq!(
        ax_property(&trigger_ax, "expanded"),
        Some(&json!(false)),
        "Chrome sees the closed disclosure"
    );
    check!(
        ax_property(&main_ax, "hasPopup").is_none(),
        "the primary button has no popup hint"
    );
    let before = data(tab).await?;
    let count = before["primary_count"].as_u64().context("primary count")?;
    click(tab, MAIN).await?;
    ready(
        tab,
        &format!("d.primary_count === {} && !d.menu_open", count + 1),
    )
    .await?;
    let leading = bounds(tab, MAIN_FACE).await?;
    let trailing = bounds(tab, OPTIONS_FACE).await?;
    click(tab, OPTIONS).await?;
    revealed_menu(tab, false).await?;
    let export = dom(tab).await?;
    check_eq!(
        find_node(&export, OPTIONS)?["expanded"],
        json!(true),
        "logical menu trigger is expanded while open"
    );
    let menu = bounds(tab, MENU).await?;
    check!(contains([0.,0.,1100.,880.], menu)
        && near(menu[0], trailing[0])
        && near(menu[1], (leading[1] + leading[3]).max(trailing[1] + trailing[3]) + 4.),
        "wide popup meets the measured leading edge of the trailing face and leaves 4 dp below both faces");
    let menu_ax = ax_node(tab, "menu", "匯出格式").await?;
    let skipped_ax = ax_node(tab, "menuitem", "試算表").await?;
    check_eq!(
        ax_property(&skipped_ax, "disabled"),
        Some(&json!(true)),
        "Chrome exposes the disabled menu command"
    );
    check!(
        contains(menu, bounds(tab, MENU_ITEM[0]).await?)
            && contains(menu, bounds(tab, MENU_ITEM[2]).await?),
        "both enabled commands are inside the measured popup"
    );
    save_png(env, tab, "wide-menu-open").await?;
    click(tab, MAIN).await?;
    closed_menu(tab, OPTIONS, false).await?;
    let after = data(tab).await?;
    check_eq!(
        after["primary_count"],
        json!(count + 1),
        "outside click dismisses the menu without activating the main action"
    );
    check_eq!(
        &after["command_counts"],
        &before["command_counts"],
        "only choosing a command can invoke it"
    );
    Ok(
        json!({"trigger_ax":trigger_ax,"menu_ax":menu_ax,"disabled_ax":skipped_ax,
        "faces":[leading,trailing],"menu":menu,"primary_count":after["primary_count"]}),
    )
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M91",
        "keyboard menu navigation skips disabled items and exits to the correct tab stop",
    );
    focus(tab, "split-large-text").await?;
    tab.press("Tab").await?;
    focused(tab, MAIN).await?;
    tab.press("Tab").await?;
    focused(tab, OPTIONS).await?;
    let before = data(tab).await?;
    let activate_count = before["command_counts"][2]
        .as_u64()
        .context("text export count")?;
    tab.press("Enter").await?;
    revealed_menu(tab, true).await?;
    tab.press("ArrowDown").await?;
    focused(tab, MENU_ITEM[2]).await?;
    tab.press("ArrowUp").await?;
    focused(tab, MENU_ITEM[0]).await?;
    tab.press("ArrowDown").await?;
    focused(tab, MENU_ITEM[2]).await?;
    save_png(env, tab, "keyboard-menu-focus").await?;
    tab.press("Enter").await?;
    closed_menu(tab, OPTIONS, true).await?;
    ready(
        tab,
        &format!("d.command_counts[2] === {}", activate_count + 1),
    )
    .await?;

    for (open, exit, target) in [
        ("Space", "Escape", OPTIONS),
        ("Enter", "Tab", "split-disable-main"),
        ("Enter", "Shift+Tab", MAIN),
    ] {
        focus(tab, OPTIONS).await?;
        tab.press(open).await?;
        revealed_menu(tab, true).await?;
        tab.press(exit).await?;
        closed_menu(tab, target, true).await?;
    }
    let after = data(tab).await?;
    check_eq!(
        &after["primary_count"],
        &before["primary_count"],
        "opening and closing options never invokes the primary action"
    );
    check_eq!(
        &after["command_counts"][0],
        &before["command_counts"][0],
        "closing with Escape or Tab invokes no focused command"
    );
    check_eq!(
        &after["command_counts"][1],
        &before["command_counts"][1],
        "disabled command is never invoked"
    );
    check_eq!(
        after["command_counts"][2],
        json!(activate_count + 1),
        "Enter invokes the chosen command once"
    );
    Ok(
        json!({"counts":after["command_counts"],"escape":OPTIONS,"tab":"split-disable-main","shift_tab":MAIN}),
    )
}

struct Chevron {
    frame: Rect,
    points: Vec<[f64; 2]>,
    ink: Rgb,
}

async fn chevron(tab: &Tab) -> Result<Chevron> {
    let export = dom(tab).await?;
    let contour = &find_node(&export, ARROW)?["fill_contour"];
    let points =
        serde_json::from_value(contour["points"].clone()).context("visible chevron points")?;
    Ok(Chevron {
        frame: bounds(tab, ARROW).await?,
        points,
        ink: painted_color(&contour["color"], [0; 3])?,
    })
}

fn check_rotation(before: &Chevron, after: &Chevron, degrees: f64) -> Result<()> {
    check_eq!(
        before.points.len(),
        after.points.len(),
        "rotating preserves the chevron vertex count"
    );
    check!(
        before.points.len() >= 3
            && near(before.frame[2], after.frame[2])
            && near(before.frame[3], after.frame[3]),
        "the two chevrons share a stable icon viewport"
    );
    let [cx, cy] = [before.frame[2] / 2., before.frame[3] / 2.];
    let (sin, cos) = degrees.to_radians().sin_cos();
    for (&[x, y], &[u, v]) in before.points.iter().zip(&after.points) {
        let px = cx + (x - cx) * cos - (y - cy) * sin;
        let py = cy + (x - cx) * sin + (y - cy) * cos;
        check!(
            near(px, u) && near(py, v),
            "rendered chevron contour follows physical angle {degrees}°"
        );
    }
    Ok(())
}

fn ink_mask(image: &Image, arrow: &Chevron) -> Result<Vec<bool>> {
    let [x, y, width, height] = arrow.frame;
    let mut pixels = Vec::new();
    for row in 0..height.round() as u32 {
        for col in 0..width.round() as u32 {
            let color = image.sample(x + f64::from(col) + 0.5, y + f64::from(row) + 0.5)?;
            pixels.push(distance(color, arrow.ink) <= 23);
        }
    }
    Ok(pixels)
}

async fn advance(tab: &Tab, time: u32) -> Result<()> {
    tab.eval(&format!("window.splitClock = {time}")).await?;
    tab.settle().await
}

async fn set_reduced_motion(tab: &Tab, enabled: bool) -> Result<()> {
    let preference = if enabled { "reduce" } else { "no-preference" };
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
    ready(tab, &format!("d.reduced_motion === {enabled}")).await
}

async fn rotation(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M92",
        "RTL mirrors the painted 180 degree chevron turn and Reduce Motion reaches both endpoints",
    );
    select(tab, "rtl", "d.rtl").await?;
    set_reduced_motion(tab, false).await?;
    ready(tab, "Math.abs(d.angle) < 0.1").await?;
    focus(tab, OPTIONS).await?;
    let closed = chevron(tab).await?;
    let closed_image = Image::capture(env, tab, "rtl-closed-chevron").await?;
    tab.eval("(() => { const now = performance.now(); window.splitClock = 0;
        Object.defineProperty(performance,'now',{configurable:true,value:() => now + window.splitClock}); })()").await?;
    tab.press("Enter").await?;
    revealed_menu(tab, true).await?;
    advance(tab, 35).await?;
    ready(
        tab,
        "d.angle > 10 && d.angle < 170 && d.physical_angle === -d.angle",
    )
    .await?;
    let mid = data(tab).await?["physical_angle"]
        .as_f64()
        .context("physical intermediate angle")?;
    let partially_open = chevron(tab).await?;
    check_rotation(&closed, &partially_open, mid)?;
    save_png(env, tab, "rtl-turning-chevron").await?;
    advance(tab, 260).await?;
    ready(tab, "Math.abs(d.physical_angle + 180) < 0.1").await?;
    let opened = chevron(tab).await?;
    check_rotation(&closed, &opened, -180.)?;
    let open_image = Image::capture(env, tab, "rtl-open-chevron").await?;
    let before = ink_mask(&closed_image, &closed)?;
    let after = ink_mask(&open_image, &opened)?;
    check_eq!(
        before.len(),
        after.len(),
        "same chevron raster area at the endpoints"
    );
    let ink_before = before.iter().filter(|&&pixel| pixel).count();
    let ink_after = after.iter().filter(|&&pixel| pixel).count();
    let unrotated = before.iter().zip(&after).filter(|&(a, b)| *a && *b).count();
    let rotated = before
        .iter()
        .rev()
        .zip(&after)
        .filter(|&(a, b)| *a && *b)
        .count();
    check!(ink_before >= 8 && ink_after >= 8 && rotated * 4 >= ink_after * 3 && rotated > unrotated + 4,
        "WebGPU PNGs show the chevron actually turning, with {rotated} rotated and {unrotated} unrotated ink pixels in common");

    tab.press("Escape").await?;
    closed_menu(tab, OPTIONS, true).await?;
    advance(tab, 520).await?;
    ready(tab, "Math.abs(d.angle) < 0.1").await?;
    set_reduced_motion(tab, true).await?;
    tab.press("Enter").await?;
    revealed_menu(tab, true).await?;
    ready(tab, "d.angle === 180 && d.physical_angle === -180").await?;
    check_rotation(&closed, &chevron(tab).await?, -180.)?;
    tab.press("Escape").await?;
    closed_menu(tab, OPTIONS, true).await?;
    ready(tab, "d.angle === 0 && d.physical_angle === 0").await?;
    check_rotation(&closed, &chevron(tab).await?, 0.)?;
    tab.eval("delete performance.now").await?;
    set_reduced_motion(tab, false).await?;
    Ok(
        json!({"physical_intermediate_angle":mid,"closed_ink":ink_before,"open_ink":ink_after,
        "rotated_overlap":rotated,"unrotated_overlap":unrotated,"reduced_motion_endpoints":[-180,0]}),
    )
}

async fn disabled_halves(tab: &Tab) -> Result<Value> {
    select(tab, "rtl", "!d.rtl").await?;
    let before = data(tab).await?;
    let count = before["primary_count"].as_u64().context("primary count")?;
    select(tab, "disable-main", "d.leading_disabled").await?;
    let main = projected(MAIN);
    tab.wait_for(&format!(
        "{main}?.getAttribute('aria-disabled') === 'true' && {main}.tabIndex < 0"
    ))
    .await?;
    focus(tab, "split-large-text").await?;
    tab.press("Tab").await?;
    focused(tab, OPTIONS).await?;
    click(tab, MAIN).await?;
    ready(tab, &format!("d.primary_count === {count} && !d.menu_open")).await?;
    click(tab, OPTIONS).await?;
    revealed_menu(tab, false).await?;
    tab.press("Escape").await?;
    closed_menu(tab, OPTIONS, false).await?;

    select(tab, "disable-main", "!d.leading_disabled").await?;
    select(tab, "disable-options", "d.trailing_disabled").await?;
    let options = projected(OPTIONS);
    tab.wait_for(&format!(
        "{options}?.getAttribute('aria-disabled') === 'true' && {options}.tabIndex < 0"
    ))
    .await?;
    click(tab, OPTIONS).await?;
    ready(tab, &format!("d.primary_count === {count} && !d.menu_open")).await?;
    focus(tab, "split-large-text").await?;
    tab.press("Tab").await?;
    focused(tab, MAIN).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        &format!("d.primary_count === {} && !d.menu_open", count + 1),
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, "split-disable-main").await?;
    let after = data(tab).await?;
    check_eq!(
        &after["command_counts"],
        &before["command_counts"],
        "disabling one half never invokes commands"
    );
    Ok(
        json!({"main_action_before":count,"main_action_after":count+1,"both_tab_paths_checked":true}),
    )
}

fn resolved_corners(style: &Value, face: Rect) -> Result<[f64; 4]> {
    let limit = face[2].min(face[3]) / 2.;
    let mut result = [0.; 4];
    for (radius, key) in
        result
            .iter_mut()
            .zip(["top_left", "top_right", "bottom_right", "bottom_left"])
    {
        *radius = style["corners"][key]
            .as_f64()
            .with_context(|| format!("split face {key} radius"))?
            .clamp(0., limit);
    }
    Ok(result)
}

fn clearance([left, top, width, height]: Rect, radii: [f64; 4], x: f64, y: f64) -> f64 {
    let [l, t, r, b] = [x - left, y - top, left + width - x, top + height - y];
    let mut distance = l.min(t).min(r).min(b);
    for ((horizontal, vertical), radius) in [(l, t), (r, t), (r, b), (l, b)].into_iter().zip(radii)
    {
        if horizontal < radius && vertical < radius {
            distance = distance.min(radius - (radius - horizontal).hypot(radius - vertical));
        }
    }
    distance
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    choice(tab, "size", "xl").await?;
    choice(tab, "leading", "both").await?;
    select(tab, "rtl", "d.rtl").await?;
    select(tab, "large-text", "d.text_scale === 2").await?;
    select(tab, "anchor-bottom", "d.bottom").await?;
    theme(tab, true).await?;
    resize(tab, 320, 620).await?;
    let viewport = [0., 0., 320., 620.];
    let track = bounds(tab, TRACK).await?;
    let main = bounds(tab, MAIN_FACE).await?;
    let trailing = bounds(tab, OPTIONS_FACE).await?;
    let label = bounds(tab, LABEL).await?;
    check!(
        contains(viewport, track)
            && contains(track, main)
            && contains(track, trailing)
            && near(main[1], trailing[1])
            && near(main[3], trailing[3])
            && trailing[0] + trailing[2] <= main[0] + 0.75,
        "both compact XL faces share a row and mirrored order"
    );
    for id in [MAIN, OPTIONS] {
        let hit = bounds(tab, id).await?;
        check!(
            contains(viewport, hit) && hit[2] >= 47.25 && hit[3] >= 47.25,
            "the compact split keeps two full, visible touch targets"
        );
    }
    let export = dom(tab).await?;
    let label_node = find_node(&export, LABEL)?;
    check_eq!(
        label_node["text"],
        json!("匯出報告"),
        "the four Chinese characters stay in the primary label"
    );
    let style = &label_node["style"];
    let char_width = style["font_size"].as_f64().context("label font size")? * 2.;
    let line_height = style["line_height"].as_f64().context("label line height")? * 2.;
    let per_line = ((label[2] + 0.75) / char_width).floor() as usize;
    check!(
        per_line == 2 && near(label[3], line_height * 2.) && contains(main, label),
        "all four scaled characters fit as two glyphs on each of two complete lines"
    );
    let ink = painted_color(&style["foreground"], [0; 3])?;
    let image = Image::capture(env, tab, "compact-dark-rtl-xl").await?;
    let mut ink_counts = [[0; 2]; 2];
    let radii = resolved_corners(&find_node(&export, MAIN_FACE)?["style"], main)?;
    let mut min_clearance = f64::INFINITY;
    for (line, glyphs) in ink_counts.iter_mut().enumerate() {
        let top = label[1] + line_height * line as f64;
        for (glyph, painted) in glyphs.iter_mut().enumerate() {
            let left = label[0] + char_width * glyph as f64;
            for row in 0..line_height.floor() as u32 {
                for col in 0..char_width.floor() as u32 {
                    let x = left + f64::from(col) + 0.5;
                    let y = top + f64::from(row) + 0.5;
                    if distance(image.sample(x, y)?, ink) <= 24 {
                        *painted += 1;
                        min_clearance = min_clearance.min(clearance(main, radii, x, y));
                    }
                }
            }
            check!(
                *painted > 20,
                "line {}, column {} paints a full-scale glyph",
                line + 1,
                glyph + 1
            );
        }
    }
    check!(
        min_clearance >= 0.75,
        "all painted glyphs clear the rounded face edge, minimum {min_clearance:.2} dp"
    );

    click(tab, OPTIONS).await?;
    revealed_menu(tab, false).await?;
    let popup = bounds(tab, MENU).await?;
    let rtl_alignment = trailing[0] + trailing[2] - popup[2];
    let expected_x = if rtl_alignment >= 0. {
        rtl_alignment
    } else {
        trailing[0]
    };
    check!(
        contains(viewport, popup)
            && near(popup[1] + popup[3] + 4., main[1])
            && near(popup[0], expected_x),
        "the 320 dp popup flips above the measured faces and stays inside the RTL viewport"
    );
    for id in [MENU_ITEM[0], MENU_ITEM[2]] {
        check!(
            contains(popup, bounds(tab, id).await?),
            "large-text menu choice remains fully visible"
        );
    }
    save_png(env, tab, "compact-dark-rtl-menu-above").await?;
    tab.press("Escape").await?;
    closed_menu(tab, OPTIONS, false).await?;
    Ok(
        json!({"track":track,"main":main,"trailing":trailing,"label":label,
            "label_ink_per_glyph":ink_counts,"min_label_ink_clearance":min_clearance,"popup":popup}),
    )
}

async fn prior_button_group(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M94",
        "Button Groups keep the shared menu hint and leave the row on Tab in either direction",
    );
    open_section(tab, "button-group-demo-show").await?;
    click(tab, "button-group-commands").await?;
    click(tab, "button-group-size-xl").await?;
    click(tab, "button-group-large-text").await?;
    resize(tab, 320, 620).await?;
    let trigger = "button-group-track-overflow";
    let menu = "button-group-track-menu";
    menu_hint(tab, trigger).await?;
    let closed_ax = ax_node(tab, "button", "更多工具").await?;
    check_eq!(
        ax_property(&closed_ax, "hasPopup"),
        Some(&json!("menu")),
        "Chrome exposes the group overflow as a menu trigger"
    );
    let before = state(tab).await?["button_group"]["activations"].clone();
    for (key, destination) in [
        ("Tab", "button-group-standard"),
        ("Shift+Tab", "button-group-large-text"),
    ] {
        focus(tab, trigger).await?;
        tab.press("Enter").await?;
        until(tab, "s.button_group.overflow_open").await?;
        tab.wait_for(&format!(
            "{}?.getAttribute('role') === 'menu'",
            projected(menu)
        ))
        .await?;
        save_png(
            env,
            tab,
            if key == "Tab" {
                "group-overflow-tab"
            } else {
                "group-overflow-shift-tab"
            },
        )
        .await?;
        tab.press(key).await?;
        until(tab, "!s.button_group.overflow_open").await?;
        focused(tab, destination).await?;
        menu_hint(tab, trigger).await?;
    }
    check_eq!(
        &state(tab).await?["button_group"]["activations"],
        &before,
        "leaving the group menu invokes no command"
    );
    Ok(
        json!({"trigger_ax":closed_ax,"tab":"button-group-standard","shift_tab":"button-group-large-text","activations":before}),
    )
}

async fn open_section(tab: &Tab, show: &str) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 1100, 880).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, show).await?;
    tab.settle().await
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    open_section(tab, "split-demo-show").await?;
    ready(tab, "d.open").await?;
    let sizes = sizes_and_paint(env, tab).await?;
    let pointer = pointer_and_menu(env, tab).await?;
    let keyboard = keyboard(env, tab).await?;
    let rotation = rotation(env, tab).await?;
    step(
        "M93",
        "disabled halves stay independent and compact XL text and popup fit at 320 dp dark RTL",
    );
    let disabled = disabled_halves(tab).await?;
    tab.goto(url).await?;
    open_section(tab, "split-demo-show").await?;
    ready(tab, "d.open").await?;
    let compact = compact(env, tab).await?;
    tab.goto(url).await?;
    let group = prior_button_group(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "split button page errors: {errors:?}");
    env.write_json(
        "browser-material-split-button.json",
        &json!({"browser":chrome.version,
        "sizes":sizes,"pointer":pointer,"keyboard":keyboard,"rotation":rotation,
        "disabled":disabled,"compact":compact,"group_menu_retrofit":group,"errors":errors}),
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
        let _ = env.write_json("material-split-button-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-split-button-failure.png"))
            .await;
    }
    result
}
