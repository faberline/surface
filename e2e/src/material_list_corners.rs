//! Interactive Material list contours measured in real Chrome WebGPU pixels.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    material_time_input::{activate, bounds, contains, focus_projected, focused, resize, Raster},
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

type Rect = [f64; 4];
type Rgb = [u8; 3];

const BEFORE: &str = "lists-demo-before";
const A: &str = "lists-a";
const B: &str = "lists-b";
const C: &str = "lists-c";
const D: &str = "lists-d";
const POSTER: &str = "list-media-poster";
const CLOCK: &str = "window.listCornersClock";
const DEPTH: f64 = 2.;

#[derive(Clone, Copy)]
pub(super) enum Scene {
    Lists,
    Media,
}

impl Scene {
    fn inspector(self) -> &'static str {
        match self {
            Self::Lists => "lists",
            Self::Media => "list_media",
        }
    }

    fn control(self, name: &str) -> String {
        let prefix = match self {
            Self::Lists => "lists-demo",
            Self::Media => "list-media-demo",
        };
        format!("{prefix}-{name}")
    }
}

pub(super) async fn ready(tab: &Tab, scene: Scene, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.{})", scene.inspector())).await?;
    tab.settle().await
}

pub(super) async fn reload(tab: &Tab, url: &str, scene: Scene) -> Result<()> {
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, &scene.control("show")).await?;
    ready(tab,scene,"d.visible && d.variant === 'segmented' && d.activations === 0 && !d.rtl && d.text_scale === 1").await?;
    resize(tab, 960, 820).await
}

pub(super) async fn preset(tab: &Tab, scene: Scene, name: &str) -> Result<()> {
    activate(tab, &scene.control(name)).await
}

pub(super) async fn palette(tab: &Tab, scene: Scene, key: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] =
        serde_json::from_value(state(tab).await?[scene.inspector()]["colors"][key].clone())
            .with_context(|| format!("Gallery color {key}"))?;
    check_eq!(a, 255, "row and gutter tokens are opaque");
    Ok([r, g, b])
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-list-corners-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

pub(super) async fn reduced_motion(tab: &Tab, on: bool) -> Result<()> {
    let value = if on { "reduce" } else { "no-preference" };
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

pub(super) async fn freeze(tab: &Tab) -> Result<()> {
    let set_clock = format!(
        "(() => {{
        const zero=performance.now();
        {CLOCK}=0;
        Object.defineProperty(performance,'now',{{configurable:true,value:()=>zero+{CLOCK}}});
    }})()"
    );
    tab.eval(&set_clock).await?;
    tab.settle().await?;
    // Finish transitions from fixture setup before timing an input.
    tick(tab, 240).await?;
    tab.eval(&set_clock).await?;
    tab.settle().await
}

pub(super) async fn tick(tab: &Tab, ms: u16) -> Result<()> {
    tab.eval(&format!("{CLOCK}={ms}")).await?;
    tab.settle().await
}

pub(super) async fn leave_rows(tab: &Tab) -> Result<()> {
    let [x, y, _, _] = tab.canvas_box().await?;
    tab.mouse_move(x + 1., y + 1.).await?;
    tab.settle().await
}

async fn hover_row(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    tab.mouse_move(p.x, p.y).await?;
    tab.settle().await
}

pub(super) fn distance(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

pub(super) fn pixel(image: &Raster, sample: Rect) -> Result<Rgb> {
    image
        .colors(sample)?
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .context("one pixel in the Chrome PNG")
}

fn stripe([x, y, w, _]: Rect, depth: f64) -> Rect {
    [x + w / 2. - 3., y + depth, 6., 0.5]
}

fn edge_pixel(image: &Raster, [x, y, w, _]: Rect, right: bool, n: u16) -> Result<Rgb> {
    let inset = f64::from(n) / 2.;
    let px = if right {
        x + w - inset - 0.5
    } else {
        x + inset
    };
    pixel(image, [px, y + DEPTH, 0.5, 0.5])
}

/// Horizontal distance to the outer contour at 2 dp from the top.
/// At DPR 2 each probe spans one device pixel.
fn edge(image: &Raster, row: Rect, gutter: Rgb, right: bool) -> Result<u16> {
    let face = pixel(image, stripe(row, DEPTH))?;
    check!(
        distance(face, gutter) >= 9,
        "the current painted state separates its face from its gutter: {face:?}, {gutter:?}"
    );
    for n in 0..36 {
        let sample = edge_pixel(image, row, right, n)?;
        if distance(sample, face) < distance(sample, gutter) {
            return Ok(n);
        }
    }
    anyhow::bail!("no visible outer contour within 18 dp of the row corner")
}

fn expect_radius(edge_pixel: u16, r: f64) -> Result<()> {
    // The sample pixel center is 0.25 dp inside the row and the scanned
    // index points to the first pixel on the visible side of the arc.
    let depth = DEPTH + 0.25;
    let x = if r <= depth {
        0.
    } else {
        r - (2. * r * depth - depth * depth).sqrt()
    };
    check!(
        (f64::from(edge_pixel) - 2. * x).abs() <= 1.5,
        "the actual edge {edge_pixel}px matches the {r} dp corner arc at depth {depth}"
    );
    Ok(())
}

fn same_pose(first: u16, second: u16, stage: &str) -> Result<()> {
    check!(
        first.abs_diff(second) <= 1,
        "{stage} starts from the displayed arc without a jump: {first} → {second} device pixels"
    );
    Ok(())
}

fn focus_alignment(image: &Raster, row: Rect, gutter: Rgb, ink: Rgb, right: bool) -> Result<u16> {
    let outer = edge(image, row, gutter, right)?;
    let centered = pixel(image, stripe(row, 2.))?;
    let inset = pixel(image, stripe(row, 3.5))?;
    check!(
        distance(centered, ink) <= 9 && distance(inset, ink) >= 24,
        "the inward focus line still measures 3 dp: center {centered:?}, inner fill {inset:?}"
    );
    let arc = edge_pixel(image, row, right, outer + 2)?;
    check!(
        distance(arc, ink) <= 12,
        "the visible focus line follows the same interpolated arc: {arc:?}, expected {ink:?}"
    );
    for n in 0..outer.saturating_sub(2) {
        let outside = edge_pixel(image, row, right, n)?;
        check!(
            distance(outside, gutter) <= 8,
            "row fill or state layer never sticks out ahead of the focused arc: {n}px {outside:?}"
        );
    }
    Ok(outer)
}

async fn hover(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M192",
        "locked-time hover takes a real intermediate arc and can reverse without jumping",
    );
    let scene = Scene::Lists;
    preset(tab, scene, "action").await?;
    ready(tab, scene, "d.mode === 'action'").await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await?;
    freeze(tab).await?;
    let row = bounds(tab, C).await?;
    let neighbor = bounds(tab, B).await?;
    let gutter = palette(tab, scene, "surface_container").await?;
    let rest = capture(env, tab, "hover-rest").await?;
    let rest_edge = edge(&rest, row, gutter, false)?;
    expect_radius(rest_edge, 4.)?;
    let neighbor_before = edge(&rest, neighbor, gutter, false)?;
    hover_row(tab, C).await?;
    let entering = capture(env, tab, "hover-first").await?;
    let first_edge = edge(&entering, row, gutter, false)?;
    same_pose(rest_edge, first_edge, "hover")?;
    tick(tab, 40).await?;
    let middle = capture(env, tab, "hover-midpoint").await?;
    let middle_edge = edge(&middle, row, gutter, false)?;
    check!(
        middle_edge > first_edge,
        "hover changes the physical arc by the first 40 ms"
    );
    leave_rows(tab).await?;
    let reverse = capture(env, tab, "hover-reverse-start").await?;
    let reverse_edge = edge(&reverse, row, gutter, false)?;
    same_pose(middle_edge, reverse_edge, "leaving mid-hover")?;
    tick(tab, 80).await?;
    let reverse_mid = capture(env, tab, "hover-reverse-midpoint").await?;
    let falling_edge = edge(&reverse_mid, row, gutter, false)?;
    check!(
        rest_edge < falling_edge && falling_edge < middle_edge,
        "reversed hover travels toward rest through a visible intermediate contour"
    );
    hover_row(tab, C).await?;
    let reentered = capture(env, tab, "hover-reenter").await?;
    let reentered_edge = edge(&reentered, row, gutter, false)?;
    same_pose(falling_edge, reentered_edge, "re-entering while leaving")?;
    tick(tab, 120).await?;
    let second_mid = capture(env, tab, "hover-second-midpoint").await?;
    let second_edge = edge(&second_mid, row, gutter, false)?;
    tick(tab, 280).await?;
    let done = capture(env, tab, "hover-settled").await?;
    let settled = edge(&done, row, gutter, false)?;
    expect_radius(settled, 12.)?;
    check!(
        reentered_edge < second_edge && second_edge < settled && middle_edge < settled,
        "the animated contour reaches the 12 dp hover endpoint through smaller arcs"
    );
    check_eq!(
        edge(&done, neighbor, gutter, false)?,
        neighbor_before,
        "the adjacent disabled row keeps its own corner"
    );
    check_eq!(
        bounds(tab, C).await?,
        row,
        "hover animation leaves the row's bounds fixed"
    );
    ready(tab, scene, "d.activations === 0").await?;
    Ok(json!({"row":row,"neighbor":neighbor,"edge_px":{
        "rest":rest_edge,"first":first_edge,"mid":middle_edge,"reverse":reverse_edge,
        "falling":falling_edge,"reentered":reentered_edge,"second_mid":second_edge,"settled":settled,
        "neighbor":neighbor_before},"time_ms":[0,40,80,120,280],"tokens_dp":[4,12]}))
}

const POINTER_TRACE: &str = r#"
    (() => {
        window.listCornerContacts = [];
        const canvas=document.getElementById('surface');
        for (const kind of ['pointerdown','pointerup']) {
            canvas.addEventListener(kind,event=>{
                if (event.isTrusted) window.listCornerContacts.push({kind,type:event.pointerType});
            },{capture:true,passive:true});
        }
    })()
"#;

async fn contact_trace(tab: &Tab, device: &str) -> Result<Value> {
    let contacts = tab.eval("window.listCornerContacts").await?;
    check_eq!(
        contacts,
        json!([{"kind":"pointerdown","type":device},{"kind":"pointerup","type":device}]),
        "actual input produced one trusted contact and release"
    );
    Ok(contacts)
}

async fn mouse_press(env: &Env, tab: &Tab, gutter: Rgb) -> Result<Value> {
    let scene = Scene::Lists;
    ready(
        tab,
        scene,
        "d.mode === 'select' && d.selected === 'lists-c'",
    )
    .await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await?;
    freeze(tab).await?;
    tab.eval(POINTER_TRACE).await?;
    let location = point(tab, D).await?;
    let row = bounds(tab, D).await?;
    hover_row(tab, D).await?;
    tick(tab, 200).await?;
    let hovering = capture(env, tab, "press-hover-settled").await?;
    let before = edge(&hovering, row, gutter, false)?;
    expect_radius(before, 12.)?;
    let blank = pixel(&hovering, [row[0] + 0.5, row[1] + 0.5, 0.5, 0.5])?;
    check!(
        distance(blank, gutter) <= 8,
        "the physical press starts in the rectangular corner outside the painted arc"
    );
    tab.mouse_move(
        location.canvas[0] + row[0] + 0.75,
        location.canvas[1] + row[1] + 0.75,
    )
    .await?;
    tab.mouse_down().await?;
    until(tab, &format!("s.focused === {}", js(D))).await?;
    ready(
        tab,
        scene,
        "d.activations === 0 && d.selected === 'lists-c'",
    )
    .await?;
    let first = capture(env, tab, "pressed-first").await?;
    let first = edge(&first, row, gutter, false)?;
    same_pose(before, first, "mouse press")?;
    tick(tab, 240).await?;
    let middle = capture(env, tab, "pressed-midpoint").await?;
    let mid = edge(&middle, row, gutter, false)?;
    tick(tab, 400).await?;
    let held = capture(env, tab, "pressed-settled").await?;
    let held = edge(&held, row, gutter, false)?;
    expect_radius(held, 16.)?;
    check!(
        first < mid && mid < held,
        "the held state traverses a separate physical midpoint"
    );
    ready(
        tab,
        scene,
        "d.activations === 0 && d.selected === 'lists-c'",
    )
    .await?;
    tab.mouse_up().await?;
    ready(
        tab,
        scene,
        "d.activations === 1 && d.last === 'lists-d' && d.selected === 'lists-d'",
    )
    .await?;
    let released = capture(env, tab, "press-selected").await?;
    let released_edge = edge(&released, row, gutter, false)?;
    same_pose(held, released_edge, "press to selected")?;
    check_eq!(
        bounds(tab, D).await?,
        row,
        "the actual rectangular target stays fixed during press and selection"
    );
    Ok(
        json!({"row":row,"edge_px":[before,first,mid,held,released_edge],"blank":blank,
        "activations":1,"events":contact_trace(tab,"mouse").await?,"selected":D}),
    )
}

async fn touch_press(env: &Env, tab: &Tab, gutter: Rgb) -> Result<Value> {
    let scene = Scene::Lists;
    leave_rows(tab).await?;
    freeze(tab).await?;
    tab.eval(POINTER_TRACE).await?;
    let at = point(tab, D).await?;
    let row = bounds(tab, D).await?;
    let rest = capture(env, tab, "touch-rest").await?;
    let rest = edge(&rest, row, gutter, false)?;
    expect_radius(rest, 4.)?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({
            "type":"touchStart","touchPoints":[{"x":at.x,"y":at.y,"id":1}]
        }),
    )
    .await?;
    tab.settle().await?;
    ready(
        tab,
        scene,
        "d.activations === 0 && d.selected === 'lists-c'",
    )
    .await?;
    let first = capture(env, tab, "touch-first").await?;
    let first = edge(&first, row, gutter, false)?;
    same_pose(rest, first, "touch press")?;
    tick(tab, 40).await?;
    let middle = capture(env, tab, "touch-midpoint").await?;
    let mid = edge(&middle, row, gutter, false)?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchEnd","touchPoints":[]}),
    )
    .await?;
    tab.settle().await?;
    ready(
        tab,
        scene,
        "d.activations === 1 && d.last === 'lists-d' && d.selected === 'lists-d'",
    )
    .await?;
    let released = capture(env, tab, "touch-early-selected").await?;
    let release_edge = edge(&released, row, gutter, false)?;
    same_pose(mid, release_edge, "early touch release to selected")?;
    tick(tab, 240).await?;
    let settled = capture(env, tab, "touch-selected-settled").await?;
    let end = edge(&settled, row, gutter, false)?;
    expect_radius(end, 16.)?;
    check!(
        first < mid && mid < end,
        "early selection continues toward the same 16 dp target"
    );
    check_eq!(
        bounds(tab, D).await?,
        row,
        "touch uses the same rectangular hit owner"
    );
    ready(tab, scene, "d.activations === 1").await?;
    Ok(
        json!({"row":row,"edge_px":[rest,first,mid,release_edge,end],
        "activations":1,"events":contact_trace(tab,"touch").await?,"selected":D}),
    )
}

async fn press(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M193",
        "mouse and touch use the rectangular target and the pressed-to-selected arc continues",
    );
    let gutter = palette(tab, Scene::Lists, "surface_container").await?;
    let mouse = mouse_press(env, tab, gutter).await?;
    reload(tab, url, Scene::Lists).await?;
    let touch = touch_press(env, tab, gutter).await?;
    Ok(json!({"mouse":mouse,"touch":touch,"tokens_dp":[4,12,16]}))
}

async fn keyboard_focus(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M194",
        "keyboard focus follows the animated row contour and Reduce Motion snaps the same target",
    );
    let scene = Scene::Lists;
    preset(tab, scene, "action").await?;
    ready(tab, scene, "d.mode === 'action'").await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await?;
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, A).await?;
    let row = bounds(tab, C).await?;
    let gutter = palette(tab, scene, "surface_container").await?;
    let ink = palette(tab, scene, "secondary").await?;
    freeze(tab).await?;
    let rest = capture(env, tab, "focus-rest").await?;
    let rest = edge(&rest, row, gutter, false)?;
    tab.press("ArrowDown").await?;
    focused(tab, C).await?;
    let first = capture(env, tab, "focus-first").await?;
    let first = focus_alignment(&first, row, gutter, ink, false)?;
    same_pose(rest, first, "keyboard arrow focus")?;
    tick(tab, 40).await?;
    let mid = capture(env, tab, "focus-midpoint").await?;
    let mid = focus_alignment(&mid, row, gutter, ink, false)?;
    reduced_motion(tab, true).await?;
    focused(tab, C).await?;
    let instant = capture(env, tab, "reduced").await?;
    let instant_edge = focus_alignment(&instant, row, gutter, ink, false)?;
    expect_radius(instant_edge, 16.)?;
    check!(
        first < mid && mid < instant_edge,
        "Reduce Motion skips the remaining corner movement"
    );
    check_eq!(
        tab.eval(CLOCK).await?,
        json!(40),
        "the reduced preference snapped the arc with time stopped"
    );
    check_eq!(
        bounds(tab, C).await?,
        row,
        "keyboard focus motion keeps the hit bounds"
    );
    ready(tab, scene, "d.activations === 0").await?;
    tab.press("Enter").await?;
    ready(tab, scene, "d.activations === 1 && d.last === 'lists-c'").await?;

    reload(tab, url, scene).await?;
    preset(tab, scene, "baseline").await?;
    preset(tab, scene, "action").await?;
    ready(
        tab,
        scene,
        "d.variant === 'baseline' && d.mode === 'action'",
    )
    .await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await?;
    freeze(tab).await?;
    let baseline = bounds(tab, C).await?;
    hover_row(tab, C).await?;
    tick(tab, 80).await?;
    let flat_hover = capture(env, tab, "baseline-hover").await?;
    let square_hover = edge(&flat_hover, baseline, gutter, false)?;
    expect_radius(square_hover, 0.)?;
    tab.mouse_down().await?;
    tick(tab, 160).await?;
    let flat_press = capture(env, tab, "baseline-pressed").await?;
    let square_press = edge(&flat_press, baseline, gutter, false)?;
    expect_radius(square_press, 0.)?;
    tab.mouse_up().await?;
    ready(tab, scene, "d.activations === 1 && d.last === 'lists-c'").await?;
    Ok(
        json!({"row":row,"edge_px":[rest,first,mid,instant_edge],"focus_ink":ink,"reduced_time_ms":40,
        "focused":C,"baseline":{"row":baseline,"edge_px":[square_hover,square_press]}}),
    )
}

async fn compact_media(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M195",
        "Standard media focus animates at 320 dp dark RTL without moving artwork and text",
    );
    let scene = Scene::Media;
    for setting in ["theme", "rtl", "large-text", "standard", "select"] {
        preset(tab, scene, setting).await?;
    }
    resize(tab, 320, 900).await?;
    ready(tab,scene,"d.variant === 'standard' && d.mode === 'select' && d.rtl && d.text_scale === 2 && d.selected.length === 0").await?;
    until(tab, "s.dark === true").await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await?;
    focus_projected(tab, "list-media-earth").await?;
    let gutter = palette(tab, scene, "surface_container").await?;
    let ink = palette(tab, Scene::Lists, "secondary").await?;
    freeze(tab).await?;
    tab.press("ArrowDown").await?;
    focused(tab, POSTER).await?;
    let row = bounds(tab, POSTER).await?;
    let artwork = bounds(tab, "list-media-poster-media").await?;
    let caption = bounds(tab, "list-media-poster-supporting").await?;
    let mark = bounds(tab, "list-media-poster-mark").await?;
    let clip = bounds(tab, "list-media-scroll").await?;
    check!(
        contains(clip, row)
            && [artwork, caption, mark]
                .into_iter()
                .all(|slot| contains(row, slot))
            && artwork[0] >= caption[0] + caption[2] - 0.75
            && mark[0] + mark[2] <= caption[0] + 0.75
            && caption[3] > 48.
            && row[2] >= 48.
            && row[3] >= 48.,
        "the fully visible media, large wrapped supporting text and mark keep their RTL slots"
    );
    let photo_sample = [
        artwork[0] + artwork[2] / 2.,
        artwork[1] + artwork[3] * 0.35,
        2.,
        2.,
    ];
    let first = capture(env, tab, "compact-media-focus-first").await?;
    let edge0 = focus_alignment(&first, row, gutter, ink, true)?;
    expect_radius(edge0, 4.)?;
    let image_rgb = pixel(&first, photo_sample)?;
    tick(tab, 40).await?;
    let midpoint = capture(env, tab, "compact-dark-rtl-media").await?;
    let mid = focus_alignment(&midpoint, row, gutter, ink, true)?;
    check_eq!(
        pixel(&midpoint, photo_sample)?,
        image_rgb,
        "the real raster is stable while the parent arc moves"
    );
    tick(tab, 200).await?;
    let done = capture(env, tab, "compact-media-focus-settled").await?;
    let last = focus_alignment(&done, row, gutter, ink, true)?;
    expect_radius(last, 16.)?;
    check!(
        edge0 < mid && mid < last,
        "Standard media moves through an intermediate on the RTL leading side"
    );
    check_eq!(
        pixel(&done, photo_sample)?,
        image_rgb,
        "the photo survives the final radius unchanged"
    );
    check_eq!(
        bounds(tab, POSTER).await?,
        row,
        "the compact hit target stays fixed"
    );
    focused(tab, POSTER).await?;
    ready(
        tab,
        scene,
        "d.selection_changes === 0 && d.selected.length === 0",
    )
    .await?;
    Ok(
        json!({"viewport":[320,900],"rtl":true,"text_scale":2,"variant":"standard",
        "row":row,"artwork":artwork,"caption":caption,"mark":mark,"clip":clip,
        "edge_px":[edge0,mid,last],"photo_rgb":image_rgb,"focused":POSTER}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url, Scene::Lists).await?;
    let hover = hover(env, tab).await?;
    reload(tab, url, Scene::Lists).await?;
    let press = press(env, tab, url).await?;
    reload(tab, url, Scene::Lists).await?;
    let focus = keyboard_focus(env, tab, url).await?;
    reload(tab, url, Scene::Media).await?;
    let compact = compact_media(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material list corner browser errors: {errors:?}"
    );
    env.write_json("browser-material-list-corners.json",&json!({
        "browser":chrome.version,"hover":hover,"press":press,"focus":focus,"compact":compact,"errors":errors,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json("material-list-corners-failure.json", &json!({
            "error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors(),
        }));
        let _ = tab
            .screenshot(&env.artifact("material-list-corners-failure.png"))
            .await;
    }
    result.context("Material 3 list interaction corners")
}
