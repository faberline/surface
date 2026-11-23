//! Finite numeric Material Scroll Fields in Chrome and WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_list_corners::{distance, freeze, leave_rows, reduced_motion, tick},
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const OPEN: &str = "scroll-field-demo-show";
const ADJUSTABLE: &str = "scroll-field-demo-value-adjustable";
const FACE: &str = "scroll-field-demo-value-face";
const ABOVE: &str = "scroll-field-demo-value-above";
const CENTER: &str = "scroll-field-demo-value-center";
const BELOW: &str = "scroll-field-demo-value-below";
const ABOVE_TARGET: &str = "scroll-field-demo-value-above-target";
const BELOW_TARGET: &str = "scroll-field-demo-value-below-target";
const EDITOR: &str = "scroll-field-demo-value-editor";
const ERROR: &str = "scroll-field-demo-value-error";
const FOCUS: &str = "scroll-field-demo-value-focus";
const USE_SCROLL: &str = "scroll-field-demo-value-use-scroll";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";

type Rect = [f64; 4];
type Rgb = [u8; 3];
type Position = [f64; 2];

fn center([x, y, w, h]: Rect) -> Position {
    [x + w / 2., y + h / 2.]
}

async fn read(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["scroll_field"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.scroll_field)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-0").await?;
    until(tab, "s.section === 0").await?;
    activate(tab, OPEN).await?;
    ready(
        tab,
        "d.visible && d.mode === 'scroll' && d.committed === 14 && d.commits === 0",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn choose(tab: &Tab, control: &str) -> Result<()> {
    activate(tab, &format!("scroll-field-demo-{control}")).await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-scroll-fields-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn color(tab: &Tab, key: &str) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] =
        serde_json::from_value(read(tab).await?["palette"][key].clone())
            .with_context(|| format!("Scroll Field token {key}"))?;
    check_eq!(alpha, 255, "the gallery token is opaque");
    Ok([r, g, b])
}

async fn screen(tab: &Tab, [x, y]: Position) -> Result<Position> {
    let [left, top, _, _] = tab.canvas_box().await?;
    Ok([left + x, top + y])
}

async fn touch(tab: &Tab, kind: &str, position: Option<Position>) -> Result<()> {
    let points = position.map_or_else(|| json!([]), |[x, y]| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn mouse(tab: &Tab, kind: &str, [x, y]: Position, held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,"button":"left","buttons":u8::from(held),"clickCount":1,"pointerType":"mouse"}),
    )
    .await?;
    tab.settle().await
}

const TRACE: &str = r#"
    (() => {
        window.scrollFieldContacts = [];
        const canvas = document.getElementById('surface');
        for (const kind of ['pointerdown','pointerup','pointercancel','gotpointercapture','lostpointercapture','wheel']) {
            canvas.addEventListener(kind,event=>{
                if(event.isTrusted) window.scrollFieldContacts.push({
                    kind, id:event.pointerId ?? null, device:event.pointerType ?? null,
                    x:event.clientX,y:event.clientY,buttons:event.buttons,
                });
            },{capture:true,passive:true});
        }
    })()
"#;

async fn spin(tab: &Tab, value: i32, min: i32, max: i32) -> Result<Value> {
    let proxy = projected(ADJUSTABLE);
    let dom = tab
        .eval(&format!(
            r#"(() => {{
        const node={proxy};
        if (!node) return null;
        return {{
            role:node.getAttribute('role'), label:node.getAttribute('aria-label'),
            minimum:node.getAttribute('aria-valuemin'), maximum:node.getAttribute('aria-valuemax'),
            value:node.getAttribute('aria-valuenow'), text:node.getAttribute('aria-valuetext'),
            stops:node.querySelectorAll('[tabindex], [data-surface-action]').length,
        }};
    }})()"#
        ))
        .await?;
    let number = |name: &str| dom[name].as_str().and_then(|n| n.parse::<i32>().ok());
    let value_text = dom["text"]
        .as_str()
        .context("localized adjustable value text")?;
    check!(
        dom["role"] == "spinbutton"
            && dom["label"] == "數量"
            && number("minimum") == Some(min)
            && number("maximum") == Some(max)
            && number("value") == Some(value)
            && dom["stops"] == 0
            && value_text.replace(',', "") == format!("{value} 件"),
        "one named adjustable exposes the accepted number and localized unit: {dom}"
    );
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "spinbutton", "數量")?;
    let property = |key| ax_property(node, key).and_then(Value::as_f64);
    check!(
        ax_property(node, "focusable") == Some(&json!(true))
            && property("valuemin") == Some(f64::from(min))
            && property("valuemax") == Some(f64::from(max))
            && node["value"]["value"].as_f64() == Some(f64::from(value)),
        "actual Chrome accessibility carries the same focusable bounded numeric value: {node}"
    );
    Ok(
        json!({"dom":dom,"chrome_value":node["value"]["value"],"backend_id":node["backendDOMNodeId"]}),
    )
}

async fn editor_focus(tab: &Tab) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(EDITOR))).await?;
    tab.wait_for(&format!(
        "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
    ))
    .await?;
    tab.settle().await
}

fn composing() -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(EDITOR)
    )
}

async fn replace(tab: &Tab, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    ready(tab, &format!("d.raw === {} && d.mode === 'input'", js(raw))).await
}

async fn explain_error(tab: &Tab) -> Result<String> {
    let helper = projected(ERROR);
    tab.wait_for(&format!(
        r#"(() => {{
        const id={TEXTAREA}?.getAttribute('aria-describedby');
        return id && document.getElementById(id) === {helper};
    }})()"#
    ))
    .await?;
    let spoken = ax_tree(tab).await?;
    let textbox = ax(&spoken, "textbox", "數量")?;
    let projected_invalid = tab
        .eval(&format!("{TEXTAREA}.getAttribute('aria-invalid')"))
        .await?;
    let chrome_invalid = ax_property(textbox, "invalid");
    check!(
        projected_invalid == "true"
            && (chrome_invalid == Some(&json!("true")) || chrome_invalid == Some(&json!(true))),
        "the real textarea and Chrome accessibility both expose invalid input"
    );
    let desc = textbox["description"]["value"]
        .as_str()
        .context("Chrome's textbox error description")?;
    let visible = tab.eval_str(&format!("{helper}.textContent")).await?;
    check!(
        !visible.trim().is_empty() && desc.contains(visible.trim()),
        "Chrome associates the visible localized error with the direct editor"
    );
    let error = bounds(tab, ERROR).await?;
    let editor = bounds(tab, EDITOR).await?;
    check!(
        error[1] >= editor[1] + editor[3] - 0.5 && error[2] > 0. && error[3] > 0.,
        "the error is visibly below the editor: {editor:?}/{error:?}"
    );
    Ok(desc.to_owned())
}

async fn sized_face(tab: &Tab, size: &str, width: f64, height: f64) -> Result<Rect> {
    choose(tab, size).await?;
    ready(
        tab,
        &format!("d.size === {size:?} && d.fits && d.mode === 'scroll'"),
    )
    .await?;
    let face = bounds(tab, FACE).await?;
    check!(
        near(face[2], width) && near(face[3], height),
        "{size} face uses the {width} by {height} dp token for two-digit values: {face:?}"
    );
    Ok(face)
}

async fn tokens(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M205",
        "Small, Medium and Large paint the scroll colors and keep focused geometry explicit",
    );
    let mut sizes = Vec::new();
    for (choice, width) in [("small", 100.), ("medium", 120.), ("large", 186.)] {
        let face = sized_face(tab, choice, width, 200.).await?;
        sizes.push(json!({"choice":choice,"face":face}));
    }
    let face @ [x, y, w, h] = sized_face(tab, "medium", 120., 200.).await?;
    leave_rows(tab).await?;
    let plain = capture(env, tab, "tokens").await?;
    let face_ink = color(tab, "face").await?;
    let surrounding = color(tab, "surface").await?;
    let center_ink = color(tab, "center").await?;
    let neighbor_ink = color(tab, "neighbor").await?;
    let ring_ink = color(tab, "focus").await?;
    check_eq!(
        plain.sample(x + 10., y + h / 2.)?,
        face_ink,
        "actual face uses Surface Container Lowest"
    );
    check_eq!(
        plain.sample(x + 1., y + 1.)?,
        surrounding,
        "the 16 dp radius leaves the first painted corner clear"
    );
    let center_bounds = bounds(tab, CENTER).await?;
    let above = bounds(tab, ABOVE).await?;
    let below = bounds(tab, BELOW).await?;
    let ink_stats = [
        plain
            .colors(center_bounds)?
            .get(&center_ink)
            .copied()
            .unwrap_or(0),
        plain
            .colors(above)?
            .get(&neighbor_ink)
            .copied()
            .unwrap_or(0),
        plain
            .colors(below)?
            .get(&neighbor_ink)
            .copied()
            .unwrap_or(0),
    ];
    check!(
        ink_stats.into_iter().all(|pixels| pixels >= 8),
        "exact On Surface center and Outline neighbors appear in the PNG: {ink_stats:?}"
    );

    let [mx, my] = screen(tab, center(face)).await?;
    tab.mouse_move(mx, my).await?;
    tab.settle().await?;
    let hovered = capture(env, tab, "hover").await?;
    let shaded = hovered.sample(x + 10., y + h / 2.)?;
    check!(
        distance(shaded, face_ink) > 20
            && distance(shaded, center_ink) < distance(face_ink, center_ink),
        "hover adds visible On Surface tint without changing the layout"
    );
    leave_rows(tab).await?;
    focus_projected(tab, ADJUSTABLE).await?;
    let ring_bounds = bounds(tab, FOCUS).await?;
    let ring = capture(env, tab, "focus").await?;
    check!(
        contains(ring_bounds, face)
            && near(ring_bounds[2], w + 10.)
            && near(ring_bounds[3], h + 10.),
        "3 dp focus ink and 2 dp gap are outside the face"
    );
    let ring_pixel = ring.sample(x + w / 2., y - 3.)?;
    let gap = ring.sample(x + w / 2., y - 1.)?;
    check_eq!(ring_pixel, ring_ink, "the real outer stroke is Secondary");
    check_eq!(
        gap,
        surrounding,
        "there is a real clear gap between the face and focus ring"
    );
    let semantics = spin(tab, 14, 0, 30).await?;
    Ok(
        json!({"sizes":sizes,"face_color":face_ink,"corner":surrounding,"ink_counts":ink_stats,
        "hover":shaded,"focus":{"bounds":ring_bounds,"ink":ring_pixel,"gap":gap},"ax":semantics}),
    )
}

fn outside_zone(zone: Rect, face: Rect, above: bool) -> Result<Position> {
    let [x, y, w, h] = zone;
    let face_edge = if above { face[1] } else { face[1] + face[3] };
    let outer = if above { y } else { y + h };
    let inner = face_edge.clamp(y, y + h);
    check!(
        h >= 48. && (inner - outer).abs() >= 4.,
        "the 48 dp action extends beyond the painted face: {zone:?}/{face:?}"
    );
    Ok([x + w / 2., (inner + outer) / 2.])
}

async fn settled(tab: &Tab, value: i32, commits: u8) -> Result<()> {
    ready(
        tab,
        &format!("d.committed === {value} && d.commits === {commits} && !d.requests_frame"),
    )
    .await
}

async fn taps(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M206",
        "trusted mouse and touch tap the 48 dp cropped zones and wheel remains page scrolling",
    );
    choose(tab, "compact").await?;
    let face = bounds(tab, FACE).await?;
    check!(near(face[3], 120.), "the compact face is painted at 120 dp");
    let zones = [
        bounds(tab, ABOVE_TARGET).await?,
        bounds(tab, BELOW_TARGET).await?,
    ];
    let upper = outside_zone(zones[0], face, true)?;
    let lower = outside_zone(zones[1], face, false)?;
    check!(
        upper[1] < face[1] && lower[1] > face[1] + face[3],
        "both selection points are fully outside the painted face"
    );
    tab.eval(TRACE).await?;
    let [x, y] = screen(tab, upper).await?;
    tab.click(x, y).await?;
    settled(tab, 16, 1).await?;
    touch(tab, "touchStart", Some(screen(tab, lower).await?)).await?;
    touch(tab, "touchEnd", None).await?;
    settled(tab, 14, 2).await?;
    capture(env, tab, "cropped-zones").await?;
    click(tab, ADJUSTABLE).await?;
    editor_focus(tab).await?;
    ready(
        tab,
        "d.mode === 'input' && d.committed === 14 && d.commits === 2",
    )
    .await?;
    tab.press("Escape").await?;
    ready(
        tab,
        "d.mode === 'scroll' && d.committed === 14 && d.commits === 2",
    )
    .await?;

    let center = point(tab, ADJUSTABLE).await?;
    let before = read(tab).await?["scroll_top"]
        .as_f64()
        .context("ancestor scroll offset")?;
    tab.mouse_move(center.x, center.y).await?;
    tab.wheel(0., 96.).await?;
    ready(
        tab,
        &format!("d.scroll_top > {before} && d.committed === 14 && d.commits === 2"),
    )
    .await?;
    let after = read(tab).await?["scroll_top"]
        .as_f64()
        .context("scrolled ancestor offset")?;
    choose(tab, "disabled").await?;
    ready(tab, "d.disabled && d.committed === 14").await?;
    let disabled = tab
        .eval(&format!(
            "{}?.getAttribute('aria-disabled')",
            projected(ADJUSTABLE)
        ))
        .await?;
    check_eq!(
        disabled,
        json!("true"),
        "Chrome marks the named adjustable disabled"
    );
    let inert = point(tab, FACE).await?;
    tab.click(inert.x, inert.y).await?;
    ready(
        tab,
        "d.mode === 'scroll' && d.committed === 14 && d.commits === 2",
    )
    .await?;
    capture(env, tab, "disabled").await?;
    let disabled_before = read(tab).await?["scroll_top"]
        .as_f64()
        .context("disabled scroll offset")?;
    tab.mouse_move(inert.x, inert.y).await?;
    tab.wheel(0., 96.).await?;
    ready(
        tab,
        &format!("d.scroll_top > {disabled_before} && d.committed === 14 && d.commits === 2"),
    )
    .await?;
    let disabled_after = read(tab).await?["scroll_top"]
        .as_f64()
        .context("ancestor scrolled while disabled")?;
    let events = tab.eval("window.scrollFieldContacts").await?;
    let trace = events.as_array().context("trusted Chrome pointer trace")?;
    for device in ["mouse", "touch"] {
        check!(
            trace
                .iter()
                .any(|e| e["kind"] == "pointerdown" && e["device"] == device)
                && trace
                    .iter()
                    .any(|e| e["kind"] == "pointerup" && e["device"] == device),
            "trusted {device} events reach the actual canvas"
        );
    }
    check!(
        trace.iter().any(|e| e["kind"] == "wheel"),
        "actual trusted wheel reaches the canvas over the field"
    );
    Ok(
        json!({"face":face,"zones":zones,"outside_points":[upper,lower],"commits":[16,14],"ancestor_scroll":[before,after],
        "disabled_scroll":[disabled_before,disabled_after],"center_opens_editor":true,
        "disabled_inert":true,"trusted_contacts":events}),
    )
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M207",
        "trusted keys move one adjustable value; the direct editor rejects off-step and out-of-range input",
    );
    focus_projected(tab, "scroll-field-demo-wide-range").await?;
    tab.press("Tab").await?;
    focused(tab, ADJUSTABLE).await?;
    let before = spin(tab, 14, 0, 30).await?;
    tab.press("ArrowUp").await?;
    settled(tab, 16, 1).await?;
    let up = spin(tab, 16, 0, 30).await?;
    tab.press("ArrowDown").await?;
    settled(tab, 14, 2).await?;
    tab.press("Space").await?;
    ready(
        tab,
        "d.mode === 'input' && d.committed === 14 && d.raw === '14'",
    )
    .await?;
    editor_focus(tab).await?;
    let editing_notice = tab
        .eval(&format!("{TEXTAREA}.getAttribute('aria-describedby')"))
        .await?;
    check!(
        editing_notice.as_str().is_some_and(|id| !id.is_empty()),
        "the direct editor has its own range/step helper"
    );
    let mut errors = Vec::new();
    for raw in ["15", "999", "abc"] {
        replace(tab, raw).await?;
        tab.press("Enter").await?;
        ready(
            tab,
            "d.error !== null && d.mode === 'input' && d.committed === 14 && d.commits === 2",
        )
        .await?;
        editor_focus(tab).await?;
        errors.push(json!({"raw":raw,"kind":read(tab).await?["error"],"description":explain_error(tab).await?}));
    }
    check!(
        errors[0]["kind"] != errors[1]["kind"] && errors[1]["kind"] != errors[2]["kind"],
        "off-step, outside the range and invalid syntax are different errors"
    );
    let error_image = capture(env, tab, "editor-error").await?;
    let [ex, ey, ew, _] = bounds(tab, FACE).await?;
    let error_ink = error_image.sample(ex + ew / 2., ey + 0.75)?;
    check_eq!(
        error_ink,
        color(tab, "error").await?,
        "the invalid face has its real Error outline"
    );
    replace(tab, "14").await?;
    ready(
        tab,
        "d.error === null && d.committed === 14 && d.commits === 2",
    )
    .await?;
    tab.wait_for(&format!(
        "{TEXTAREA}?.getAttribute('aria-invalid') !== 'true'"
    ))
    .await?;
    tab.press("Escape").await?;
    ready(
        tab,
        "d.mode === 'scroll' && d.committed === 14 && d.commits === 2 && d.error === null",
    )
    .await?;
    focused(tab, ADJUSTABLE).await?;
    tab.press("Enter").await?;
    editor_focus(tab).await?;
    replace(tab, "22").await?;
    let active = capture(env, tab, "editor-active").await?;
    let [px, py, pw, _] = bounds(tab, FACE).await?;
    let editing_ink = active.sample(px + pw / 2., py + 0.75)?;
    check_eq!(
        editing_ink,
        color(tab, "editing").await?,
        "valid editing restores the Primary outline"
    );
    tab.press("Enter").await?;
    settled(tab, 22, 3).await?;
    focused(tab, ADJUSTABLE).await?;
    let final_ax = spin(tab, 22, 0, 30).await?;
    Ok(
        json!({"values":[14,16,14,22],"commits":3,"ax":[before,up,final_ax],"errors":errors,
        "escape_restored":14,"editor_focus_restored":true,"aria_invalid_cleared_on_edit":true,
        "outlines":[error_ink,editing_ink]}),
    )
}

async fn ime(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M208",
        "CDP-simulated composition consumes the first Enter and only a second Enter commits",
    );
    focus_projected(tab, ADJUSTABLE).await?;
    tab.press("Enter").await?;
    editor_focus(tab).await?;
    tab.shortcut("a").await?;
    tab.ime_compose("22", 2, 2).await?;
    let composing = composing();
    tab.wait_for(&composing).await?;
    click(tab, USE_SCROLL).await?;
    tab.wait_for(&composing).await?;
    editor_focus(tab).await?;
    ready(
        tab,
        "d.mode === 'input' && d.committed === 14 && d.commits === 0",
    )
    .await?;
    capture(env, tab, "ime-preedit").await?;

    tab.press("Enter").await?;
    ready(
        tab,
        "d.mode === 'input' && d.committed === 14 && d.commits === 0",
    )
    .await?;
    if read(tab).await?["raw"] != "22" {
        tab.insert_text("22").await?;
    }
    ready(
        tab,
        "d.raw === '22' && d.committed === 14 && d.commits === 0",
    )
    .await?;
    tab.wait_for(&format!("!({composing})")).await?;
    editor_focus(tab).await?;
    tab.press("Enter").await?;
    settled(tab, 22, 1).await?;
    focused(tab, ADJUSTABLE).await?;
    Ok(
        json!({"input_method":"CDP synthetic composition","first_enter_commits":0,"mode_press_preserved_preedit":true,
        "explicit_confirm_commits":1,"value":22,"ax":spin(tab,22,0,30).await?}),
    )
}

async fn motion_offset(tab: &Tab) -> Result<f64> {
    read(tab).await?["offset"]
        .as_f64()
        .context("fractional scroll field offset")
}

fn painted_glyph_y(image: &Raster, face: Rect, ink: Rgb) -> Result<f64> {
    let [x, y, width, height] = face;
    let mut rows = 0.;
    let mut samples = 0;
    for row in 3..height as u32 - 3 {
        for column in 3..width as u32 - 3 {
            if image.sample(x + f64::from(column), y + f64::from(row))? == ink {
                rows += f64::from(row);
                samples += 1;
            }
        }
    }
    check!(samples > 20, "the active glyph has real visible ink");
    Ok(y + rows / f64::from(samples))
}

async fn gestures(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M209",
        "trusted swipe previews without changing the value, cancels cleanly and springs after acceptance",
    );
    reduced_motion(tab, false).await?;
    tab.eval(TRACE).await?;
    freeze(tab).await?;
    let field = point(tab, ADJUSTABLE).await?;
    let origin = [field.x, field.y];
    let face = bounds(tab, FACE).await?;
    let center_ink = color(tab, "center").await?;
    let pitch = center(bounds(tab, CENTER).await?)[1] - center(bounds(tab, ABOVE).await?)[1];
    check!(
        pitch >= 48.,
        "the gesture sample traverses one measured visual row"
    );
    let dragged = [origin[0], origin[1] + pitch * 1.23];
    tab.mouse_move(origin[0], origin[1]).await?;
    mouse(tab, "mousePressed", origin, true).await?;
    mouse(tab, "mouseMoved", dragged, true).await?;
    ready(
        tab,
        "d.dragging && d.preview === 16 && d.committed === 14 && d.commits === 0",
    )
    .await?;
    let held = motion_offset(tab).await?;
    spin(tab, 14, 0, 30).await?;
    let held_frame = capture(env, tab, "gesture-held").await?;
    let held_glyph = painted_glyph_y(&held_frame, face, center_ink)?;
    tab.wait_for(
        "window.scrollFieldContacts.some(e=>e.kind==='gotpointercapture' && e.device==='mouse')",
    )
    .await?;
    let owner = tab
        .eval("window.scrollFieldContacts.find(e=>e.kind==='gotpointercapture' && e.device==='mouse').id")
        .await?
        .as_i64()
        .context("trusted captured pointer")?;
    tab.wait_for(&format!(
        "document.getElementById('surface').hasPointerCapture({owner})"
    ))
    .await?;
    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    // A held move processes capture loss before the terminal button event.
    let after_loss = [dragged[0] + 2., dragged[1]];
    mouse(tab, "mouseMoved", after_loss, true).await?;
    ready(tab, "!d.dragging && d.committed === 14 && d.commits === 0").await?;
    mouse(tab, "mouseReleased", after_loss, false).await?;
    tick(tab, 700).await?;
    settled(tab, 14, 0).await?;
    check!(
        (motion_offset(tab).await? - 7.).abs() < 0.001,
        "rejected release rolls back to the accepted position"
    );

    touch(tab, "touchStart", Some(origin)).await?;
    touch(tab, "touchMove", Some(dragged)).await?;
    ready(
        tab,
        "d.dragging && d.preview === 16 && d.committed === 14 && d.commits === 0",
    )
    .await?;
    touch(tab, "touchCancel", None).await?;
    tick(tab, 1400).await?;
    settled(tab, 14, 0).await?;

    touch(tab, "touchStart", Some(origin)).await?;
    touch(tab, "touchMove", Some(dragged)).await?;
    ready(
        tab,
        "d.dragging && d.preview === 16 && d.committed === 14 && d.commits === 0",
    )
    .await?;
    let pre_release = motion_offset(tab).await?;
    touch(tab, "touchEnd", None).await?;
    ready(
        tab,
        "!d.dragging && d.committed === 16 && d.commits === 1 && d.requests_frame",
    )
    .await?;
    let at_release = motion_offset(tab).await?;
    check!(
        (at_release - pre_release).abs() < 0.1 && (at_release - 8.).abs() > 0.06,
        "acceptance keeps an intermediate start position for the spring"
    );
    tick(tab, 1438).await?;
    let middle = motion_offset(tab).await?;
    check!(
        (middle - 8.).abs() > 0.009 && (middle - 8.).abs() < (at_release - 8.).abs(),
        "the real next frame approaches the accepted index from the held position"
    );
    let spring_frame = capture(env, tab, "gesture-spring").await?;
    let spring_glyph = painted_glyph_y(&spring_frame, face, center_ink)?;
    tick(tab, 2100).await?;
    settled(tab, 16, 1).await?;
    check!(
        (motion_offset(tab).await? - 8.).abs() < 0.001,
        "the value snaps exactly to the accepted index"
    );
    let geometry = spin(tab, 16, 0, 30).await?;
    let rest_frame = capture(env, tab, "gesture-settled").await?;
    let rest_glyph = painted_glyph_y(&rest_frame, face, center_ink)?;
    check!(
        held_glyph > spring_glyph + 1. && spring_glyph > rest_glyph + 1.,
        "the exact center-color glyph moves through an intermediate position in the actual PNG"
    );

    reduced_motion(tab, true).await?;
    ready(tab, "d.reduced_motion").await?;
    touch(tab, "touchStart", Some(origin)).await?;
    touch(tab, "touchMove", Some(dragged)).await?;
    ready(
        tab,
        "d.dragging && d.preview === 18 && d.committed === 16 && d.commits === 1",
    )
    .await?;
    touch(tab, "touchEnd", None).await?;
    settled(tab, 18, 2).await?;
    let snapped = motion_offset(tab).await?;
    check!(
        (snapped - 9.).abs() < 0.001,
        "Reduce Motion accepts and snaps without a delayed frame"
    );
    capture(env, tab, "gesture-reduced").await?;
    let events = tab.eval("window.scrollFieldContacts").await?;
    let trace = events.as_array().context("trusted capture trace")?;
    let index = |kind| {
        trace
            .iter()
            .position(|e| e["kind"] == kind && e["device"] == "mouse" && e["id"] == owner)
    };
    let loss = index("lostpointercapture").context("actual mouse capture loss")?;
    let up = index("pointerup").context("late actual mouse up")?;
    check!(
        loss < up
            && trace[loss]["buttons"] == 1
            && trace
                .iter()
                .any(|e| e["kind"] == "pointercancel" && e["device"] == "touch"),
        "trusted capture loss and touch cancellation both reach the canvas"
    );
    Ok(
        json!({"pitch":pitch,"held_offset":held,"canceled_to":14,"accepted":16,"spring":[at_release,middle,8.],
        "painted_glyph_y":[held_glyph,spring_glyph,rest_glyph],
        "reduced_value":18,"reduced_offset":snapped,"commits":2,"ax":geometry,"trusted_contacts":events}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M210",
        "dark RTL at 320 dp and 200% text uses the real editor when the measured scroll field cannot fit",
    );
    for option in ["theme", "rtl", "large-text"] {
        choose(tab, option).await?;
    }
    resize(tab, 320, 820).await?;
    until(tab, "s.dark === true").await?;
    ready(
        tab,
        "d.rtl && d.text_scale === 2 && d.fits && d.mode === 'scroll' && d.committed === 14",
    )
    .await?;
    let compact_scroll = bounds(tab, FACE).await?;
    check!(
        contains([0., 0., 320., 820.], compact_scroll) && compact_scroll[3] >= 224.,
        "the normal numeric range grows to contain actual 200 percent text at 320 dp"
    );
    capture(env, tab, "compact-regular").await?;
    choose(tab, "wide-range").await?;
    ready(
        tab,
        "d.rtl && d.text_scale === 2 && !d.fits && d.mode === 'input' && d.committed === 1200",
    )
    .await?;
    click(tab, EDITOR).await?;
    editor_focus(tab).await?;
    let editor = bounds(tab, EDITOR).await?;
    check!(
        contains([0., 0., 320., 820.], editor) && editor[2] >= 48. && editor[3] >= 48.,
        "the fallback direct editor remains fully visible and usable at its true text scale"
    );
    capture(env, tab, "compact-dark-rtl").await?;
    tab.shortcut("a").await?;
    tab.ime_compose("1300", 4, 4).await?;
    let composing = composing();
    tab.wait_for(&composing).await?;
    ready(
        tab,
        "d.raw === '1200' && d.committed === 1200 && d.commits === 0",
    )
    .await?;
    resize(tab, 960, 820).await?;
    ready(
        tab,
        "d.fits && d.mode === 'input' && d.committed === 1200 && d.raw === '1200'",
    )
    .await?;
    tab.wait_for(&composing).await?;
    editor_focus(tab).await?;
    capture(env, tab, "resized-ime-preedit").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.mode === 'input' && d.committed === 1200 && d.commits === 0",
    )
    .await?;
    if read(tab).await?["raw"] != "1300" {
        tab.insert_text("1300").await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    ready(
        tab,
        "d.raw === '1300' && d.committed === 1200 && d.commits === 0",
    )
    .await?;
    resize(tab, 320, 820).await?;
    ready(
        tab,
        "!d.fits && d.mode === 'input' && d.committed === 1200 && d.raw === '1300'",
    )
    .await?;
    editor_focus(tab).await?;
    resize(tab, 960, 820).await?;
    ready(
        tab,
        "d.fits && d.mode === 'input' && d.committed === 1200 && d.raw === '1300'",
    )
    .await?;
    editor_focus(tab).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.committed === 1300 && d.commits === 1 && d.mode === 'scroll' && d.fits",
    )
    .await?;
    until(tab, &format!("s.focused === {}", js(ADJUSTABLE))).await?;
    tab.wait_for("document.activeElement === document.getElementById('surface')")
        .await?;
    let canvas_focus = tab.eval_str("document.activeElement.tagName").await?;
    let accepted = spin(tab, 1300, -1800, 1800).await?;
    capture(env, tab, "wide-rtl-value").await?;
    resize(tab, 320, 820).await?;
    ready(
        tab,
        "!d.fits && d.mode === 'input' && d.committed === 1300 && d.commits === 1 && d.raw === '1300'",
    )
    .await?;
    editor_focus(tab).await?;
    Ok(
        json!({"viewport":[320,820],"rtl":true,"text_scale":2,"fallback_editor":editor,
        "short_range_scroll":compact_scroll,"preedit_survived_resize":true,
        "draft_preserved":"1300","accepted":1300,"commits":1,
        "pointer_focus":{"runtime":ADJUSTABLE,"browser":canvas_focus},
        "roomy_ax":accepted,"resize_restores_editor":true}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let tokens = tokens(env, tab).await?;
    reload(tab, url).await?;
    let taps = taps(env, tab).await?;
    reload(tab, url).await?;
    let editor = keyboard(env, tab).await?;
    reload(tab, url).await?;
    let ime = ime(env, tab).await?;
    reload(tab, url).await?;
    let gestures = gestures(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Numeric Scroll Field page errors: {errors:?}"
    );
    env.write_json("browser-material-scroll-fields.json",&json!({"browser":chrome.version,
        "tokens":tokens,"taps":taps,"editor":editor,"ime":ime,"gestures":gestures,"compact":compact,"errors":errors}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-scroll-fields-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
                "contacts":tab.eval("window.scrollFieldContacts ?? []").await.ok()
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-scroll-fields-failure.png"))
            .await;
    }
    result.context("Material 3 numeric Scroll Fields")
}
