//! Finite text choices in a Material Scroll Field, using Chrome and WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_list_corners::{freeze, leave_rows, reduced_motion, tick},
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const OPEN: &str = "scroll-choice-demo-show";
const ADJUSTABLE: &str = "scroll-choice-demo-value-adjustable";
const FACE: &str = "scroll-choice-demo-value-face";
const ABOVE: &str = "scroll-choice-demo-value-above";
const CENTER: &str = "scroll-choice-demo-value-center";
const BELOW: &str = "scroll-choice-demo-value-below";
const ABOVE_TARGET: &str = "scroll-choice-demo-value-above-target";
const BELOW_TARGET: &str = "scroll-choice-demo-value-below-target";
const EDITOR: &str = "scroll-choice-demo-value-editor";
const USE_SCROLL: &str = "scroll-choice-demo-value-use-scroll";
const ERROR: &str = "scroll-choice-demo-value-error";
const SUGGESTION: &str = "scroll-choice-demo-value-suggestion";
const REORDER: &str = "scroll-choice-demo-reorder";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const CANVAS_FOCUS: &str = "document.activeElement === document.getElementById('surface')";
const LABEL: &str = "地點";
const ORDER: [&str; 5] = [
    "banqiao",
    "taipei-north",
    "taipei-station",
    "taoyuan",
    "hsinchu",
];
const REVERSED: [&str; 5] = [
    "hsinchu",
    "taoyuan",
    "taipei-station",
    "taipei-north",
    "banqiao",
];

type Rect = [f64; 4];
type Position = [f64; 2];
type Rgb = [u8; 3];

async fn read(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["scroll_choice"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.scroll_choice)")).await?;
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
        "d.visible && d.committed === 'banqiao' && d.commits === 0",
    )
    .await?;
    resize(tab, 960, 820).await?;
    ready(tab, "d.fits && d.mode === 'scroll' && d.ordinal === 1").await
}

async fn choose(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("scroll-choice-demo-{name}")).await
}

async fn snapshot(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-scroll-choice-fields-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn color(data: &Value, key: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(data["palette"][key].clone())
        .with_context(|| format!("Scroll Choice palette token {key}"))?;
    check_eq!(a, 255, "the paint color is opaque");
    Ok([r, g, b])
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn editor_focus(tab: &Tab) -> Result<()> {
    runtime_focus(tab, EDITOR).await?;
    tab.wait_for(&format!(
        "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
    ))
    .await?;
    tab.settle().await
}

async fn edit(tab: &Tab) -> Result<()> {
    focus_projected(tab, ADJUSTABLE).await?;
    tab.press("Enter").await?;
    editor_focus(tab).await
}

async fn type_query(tab: &Tab, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    if raw.is_empty() {
        tab.press("Backspace").await?;
    } else {
        tab.insert_text(raw).await?;
    }
    ready(tab, &format!("d.mode === 'input' && d.raw === {}", js(raw))).await
}

async fn settled(tab: &Tab, id: &str, ordinal: u8, commits: u8) -> Result<()> {
    ready(tab, &format!(
        "d.committed === {id} && d.ordinal === {ordinal} && d.commits === {commits} && !d.requests_frame && !d.dragging",
        id = js(id),
    )).await
}

async fn spin(tab: &Tab, ordinal: u8, words: &[&str]) -> Result<Value> {
    let dom = tab.eval(&format!(r#"(() => {{
        const node = {};
        if (!node) return null;
        return {{
            role: node.getAttribute('role'), name: node.getAttribute('aria-label'),
            min: Number(node.getAttribute('aria-valuemin')), max: Number(node.getAttribute('aria-valuemax')),
            now: Number(node.getAttribute('aria-valuenow')), text: node.getAttribute('aria-valuetext'),
            descendants: node.querySelectorAll('[tabindex], [data-surface-action]').length,
            wheel_count: document.querySelectorAll('#surface-accessibility [role="spinbutton"]').length,
        }};
    }})()"#, projected(ADJUSTABLE))).await?;
    let matches = |value: &Value| {
        value
            .as_str()
            .is_some_and(|text| words.iter().all(|word| text.contains(word)))
    };
    check!(
        dom["role"] == "spinbutton"
            && dom["name"] == LABEL
            && dom["min"] == 1
            && dom["max"] == 5
            && dom["now"] == ordinal
            && dom["descendants"] == 0
            && dom["wheel_count"] == 1
            && matches(&dom["text"]),
        "one named spinbutton keeps the finite ordinal and complete localized choice: {dom}"
    );
    let tree = ax_tree(tab).await?;
    let item = ax(&tree, "spinbutton", LABEL)?;
    let property = |key| ax_property(item, key).and_then(Value::as_f64);
    let value = &item["value"]["value"];
    let description = &item["description"]["value"];
    check!(
        ax_property(item, "focusable") == Some(&json!(true))
            && property("valuemin") == Some(1.)
            && property("valuemax") == Some(5.)
            && description
                .as_str()
                .is_some_and(|text| text.contains("可輸入部分地名"))
            && (matches(value)
                || (value.as_f64() == Some(f64::from(ordinal)) && matches(description))),
        "Chrome accessibility keeps the localized value and the authored help: {item}"
    );
    Ok(json!({"dom":dom,"chrome_value":value,"chrome_description":description}))
}

async fn helper(tab: &Tab, id: &str, text: &str, invalid: bool) -> Result<Value> {
    let related = tab
        .eval(&format!(
            r#"(() => {{
        const input = {TEXTAREA};
        const helper = {};
        const links = input?.getAttribute('aria-describedby')?.split(/\s+/) ?? [];
        return {{
            linked: links.some(id => document.getElementById(id) === helper),
            invalid: input?.getAttribute('aria-invalid') === 'true',
            visible: helper?.textContent,
        }};
    }})()"#,
            projected(id)
        ))
        .await?;
    check!(
        related["linked"] == true
            && related["invalid"] == invalid
            && related["visible"]
                .as_str()
                .is_some_and(|value| value.contains(text)),
        "the live editor references the visible localized copy: {related}"
    );
    let tree = ax_tree(tab).await?;
    let item = ax(&tree, "textbox", LABEL)?;
    let description = &item["description"]["value"];
    let chrome_invalid = matches!(ax_property(item, "invalid"), Some(value) if value == &json!(true) || value == &json!("true"));
    check!(
        description
            .as_str()
            .is_some_and(|value| value.contains(text))
            && chrome_invalid == invalid,
        "Chrome exposes the full localized helper and invalid state: {item}"
    );
    let rect = bounds(tab, id).await?;
    let editor = bounds(tab, EDITOR).await?;
    check!(
        rect[1] >= editor[1] + editor[3] - 0.5 && rect[2] > 0. && rect[3] > 0.,
        "the helper is painted below the editor: {rect:?}/{editor:?}"
    );
    Ok(json!({"description":description,"bounds":rect,"invalid":chrome_invalid}))
}

async fn tokens(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M211",
        "finite text choices show the cropped Material face and one localized Chrome spinbutton",
    );
    let data = read(tab).await?;
    check_eq!(
        data["order"],
        json!(ORDER),
        "Gallery starts with the authored stable IDs"
    );
    let face @ [x, y, width, height] = bounds(tab, FACE).await?;
    check!(
        width >= 120. && near(height, 200.),
        "the default medium face uses its token floor: {face:?}"
    );
    leave_rows(tab).await?;
    let plain = snapshot(env, tab, "tokens").await?;
    let fill = color(&data, "face")?;
    let surrounding = color(&data, "surface")?;
    let ink = color(&data, "center")?;
    check_eq!(
        plain.sample(x + 8., y + height / 2.)?,
        fill,
        "the face paints its surface token"
    );
    check_eq!(
        plain.sample(x + 1., y + 1.)?,
        surrounding,
        "the physical face rounds its corner"
    );
    let center = bounds(tab, CENTER).await?;
    let right = [
        center[0] + center[2] / 2.,
        center[1],
        center[2] / 2.,
        center[3],
    ];
    let left = [center[0], center[1], center[2] / 2., center[3]];
    let glyphs = [
        plain.colors(left)?.get(&ink).copied().unwrap_or(0),
        plain.colors(right)?.get(&ink).copied().unwrap_or(0),
    ];
    check!(
        glyphs.iter().all(|count| *count > 8),
        "both CJK glyphs paint in the center: {glyphs:?}"
    );
    let semantics = spin(tab, 1, &["板橋"]).await?;
    let no_paint_proxies = tab
        .eval(&format!(
            "[{}, {}, {}].every(node => node === null)",
            projected(ABOVE),
            projected(CENTER),
            projected(BELOW)
        ))
        .await?;
    check_eq!(
        no_paint_proxies,
        json!(true),
        "the visual value slots add no Chrome accessibility targets"
    );
    Ok(json!({"face":face,"center":center,"glyph_counts":glyphs,"ax":semantics,"order":ORDER}))
}

async fn prefix(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M212",
        "one typed character suggests a unique place and only explicit Enter accepts it",
    );
    edit(tab).await?;
    type_query(tab, "桃").await?;
    ready(
        tab,
        "d.suggestion === '桃園' && d.committed === 'banqiao' && d.commits === 0 && !d.error",
    )
    .await?;
    let suggestion = helper(tab, SUGGESTION, "桃園", false).await?;
    let query = tab
        .eval(&format!(
            "[{TEXTAREA}.value, {TEXTAREA}.selectionStart, {TEXTAREA}.selectionEnd]"
        ))
        .await?;
    check_eq!(
        query,
        json!(["桃", 1, 1]),
        "the completion preserves the raw query and caret"
    );
    snapshot(env, tab, "prefix").await?;
    tab.press("Tab").await?;
    focused(tab, USE_SCROLL).await?;
    ready(
        tab,
        "d.committed === 'banqiao' && d.commits === 0 && d.raw === '桃'",
    )
    .await?;
    tab.press("Shift+Tab").await?;
    editor_focus(tab).await?;
    tab.press("Enter").await?;
    settled(tab, "taoyuan", 4, 1).await?;
    ready(tab, "d.mode === 'scroll' && !d.suggestion").await?;
    focused(tab, ADJUSTABLE).await?;
    Ok(
        json!({"query":query,"suggestion":suggestion,"commits":1,"id":"taoyuan","ax":spin(tab,4,&["桃園"]).await?}),
    )
}

async fn validation(env: &Env, tab: &Tab) -> Result<Value> {
    step("M213", "ambiguous and unresolved input keep the current ID until an exact disambiguating alias is confirmed");
    edit(tab).await?;
    let mut errors = Vec::new();
    for (query, label) in [
        ("台北", "有多個相符地點，請輸入完整名稱"),
        ("無", "找不到地點"),
        ("", "請輸入地點"),
    ] {
        type_query(tab, query).await?;
        ready(tab, "d.suggestion === null && d.commits === 0").await?;
        tab.press("Enter").await?;
        ready(tab, "d.mode === 'input' && d.error !== null && d.committed === 'banqiao' && d.commits === 0").await?;
        editor_focus(tab).await?;
        let error = helper(tab, ERROR, label, true).await?;
        if query == "台北" {
            snapshot(env, tab, "ambiguous").await?;
        }
        errors.push(json!({"raw":query,"kind":read(tab).await?["error"],"ax":error}));
    }
    type_query(tab, "台北車站").await?;
    ready(
        tab,
        "!d.error && d.committed === 'banqiao' && d.commits === 0",
    )
    .await?;
    check!(
        tab.eval(&format!(
            "{TEXTAREA}.getAttribute('aria-invalid') !== 'true'"
        ))
        .await?
            == true,
        "correcting the query clears the actual invalid attribute before confirmation"
    );
    tab.press("Enter").await?;
    settled(tab, "taipei-station", 3, 1).await?;
    focused(tab, ADJUSTABLE).await?;
    let chosen = spin(tab, 3, &["台北", "車站"]).await?;
    edit(tab).await?;
    ready(tab, "d.raw === '台北' && d.commits === 1").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.mode === 'scroll' && !d.error && d.committed === 'taipei-station' && d.commits === 1",
    )
    .await?;
    focused(tab, ADJUSTABLE).await?;
    Ok(
        json!({"errors":errors,"resolved_query":"台北車站","id":"taipei-station","commits":1,"ax":chosen,
        "unchanged_duplicate_commits":0}),
    )
}

async fn catalog(tab: &Tab) -> Result<Value> {
    step(
        "M214",
        "reorder and relabel preserve the committed ID and publish its updated position and name",
    );
    choose(tab, "reorder").await?;
    settled(tab, "banqiao", 5, 0).await?;
    check_eq!(
        read(tab).await?["order"],
        json!(REVERSED),
        "the full catalog order is reversed"
    );
    let reordered = spin(tab, 5, &["板橋"]).await?;
    edit(tab).await?;
    type_query(tab, "板").await?;
    ready(tab, "d.suggestion === '板橋' && d.commits === 0").await?;
    choose(tab, "relabel").await?;
    ready(
        tab,
        "d.mode === 'input' && d.raw === '板' && d.suggestion === '板橋站' && d.commits === 0",
    )
    .await?;
    click(tab, EDITOR).await?;
    editor_focus(tab).await?;
    let rematched = helper(tab, SUGGESTION, "板橋站", false).await?;
    tab.press("Escape").await?;
    settled(tab, "banqiao", 5, 0).await?;
    ready(tab, "d.raw === '板橋站'").await?;
    let renamed = spin(tab, 5, &["板橋站"]).await?;
    focus_projected(tab, ADJUSTABLE).await?;
    tab.press("ArrowUp").await?;
    settled(tab, "banqiao", 5, 0).await?;
    tab.press("ArrowDown").await?;
    settled(tab, "taipei-north", 4, 1).await?;
    let shifted = spin(tab, 4, &["台北", "北門"]).await?;
    Ok(
        json!({"order":REVERSED,"external_commits":0,"id_retained":"banqiao","reordered":reordered,
        "renamed":renamed,"rematched_suggestion":rematched,
        "derived_previous_id":"taipei-north","commits":1,"previous":shifted}),
    )
}

async fn screen(tab: &Tab, point: Position) -> Result<Position> {
    let [x, y, _, _] = tab.canvas_box().await?;
    Ok([x + point[0], y + point[1]])
}

async fn touch(tab: &Tab, kind: &str, at: Option<Position>) -> Result<()> {
    let points = at.map_or_else(|| json!([]), |[x, y]| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

const CONTACT_TRACE: &str = r#"
    (() => {
        window.scrollChoiceContacts = [];
        const canvas = document.getElementById('surface');
        for (const type of ['pointerdown','pointerup','pointercancel']) {
            canvas.addEventListener(type, event => {
                if (event.isTrusted) window.scrollChoiceContacts.push({
                    type, device:event.pointerType, id:event.pointerId,
                    x:event.clientX, y:event.clientY,
                });
            }, {capture:true,passive:true});
        }
    })()
"#;

fn outside_point(zone: Rect, face: Rect, above: bool) -> Result<Position> {
    let [x, y, width, height] = zone;
    let edge = if above { face[1] } else { face[1] + face[3] };
    let outer = if above { y } else { y + height };
    check!(
        height >= 48.
            && if above {
                edge - outer >= 4.
            } else {
                outer - edge >= 4.
            },
        "a full 48 dp hit area protrudes beyond the physical face: {zone:?}/{face:?}"
    );
    Ok([x + width / 2., (edge + outer) / 2.])
}

async fn gestures(env: &Env, tab: &Tab) -> Result<Value> {
    step("M215", "real cropped mouse and touch targets pick stable IDs, and cancellation or catalog change rejects a held release");
    choose(tab, "compact").await?;
    ready(tab, "d.face_height === 120").await?;
    let face = bounds(tab, FACE).await?;
    let zones = [
        bounds(tab, ABOVE_TARGET).await?,
        bounds(tab, BELOW_TARGET).await?,
    ];
    let upper = outside_point(zones[0], face, true)?;
    let lower = outside_point(zones[1], face, false)?;
    tab.eval(CONTACT_TRACE).await?;
    let [x, y] = screen(tab, upper).await?;
    tab.click(x, y).await?;
    settled(tab, "taipei-north", 2, 1).await?;
    touch(tab, "touchStart", Some(screen(tab, lower).await?)).await?;
    touch(tab, "touchEnd", None).await?;
    settled(tab, "banqiao", 1, 2).await?;
    snapshot(env, tab, "cropped-zones").await?;

    reduced_motion(tab, false).await?;
    freeze(tab).await?;
    focus_projected(tab, REORDER).await?;
    let p = point(tab, ADJUSTABLE).await?;
    let pitch = read(tab).await?["row_height"]
        .as_f64()
        .context("actual choice row pitch")?;
    let drag_start = [p.x, p.y];
    let drag_end = [p.x, p.y + pitch * 1.23];
    let held = "d.dragging && d.preview === 'taipei-north' && d.committed === 'banqiao' && d.commits === 2";
    touch(tab, "touchStart", Some(drag_start)).await?;
    touch(tab, "touchMove", Some(drag_end)).await?;
    ready(tab, held).await?;
    let held_offset = read(tab).await?["offset"]
        .as_f64()
        .context("choice preview offset")?;
    check!(
        (held_offset - 1.23).abs() < 0.03,
        "the held pointer moves a real fractional row: {held_offset}"
    );
    let before = spin(tab, 1, &["板橋"]).await?;
    let held_image = snapshot(env, tab, "gesture-held").await?;
    let color = color(&read(tab).await?, "center")?;
    check!(
        held_image.colors(face)?.get(&color).copied().unwrap_or(0) > 16,
        "the held sample has real center-label ink inside the cropped face"
    );
    touch(tab, "touchCancel", None).await?;
    tick(tab, 700).await?;
    settled(tab, "banqiao", 1, 2).await?;

    touch(tab, "touchStart", Some(drag_start)).await?;
    touch(tab, "touchMove", Some(drag_end)).await?;
    ready(tab, held).await?;
    runtime_focus(tab, REORDER).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.ordinal === 5 && d.committed === 'banqiao' && d.commits === 2 && !d.dragging",
    )
    .await?;
    touch(tab, "touchEnd", None).await?;
    tick(tab, 1400).await?;
    settled(tab, "banqiao", 5, 2).await?;
    let after = spin(tab, 5, &["板橋"]).await?;

    reduced_motion(tab, true).await?;
    ready(tab, "d.reduced_motion").await?;
    touch(tab, "touchStart", Some(drag_start)).await?;
    touch(
        tab,
        "touchMove",
        Some([drag_start[0], drag_start[1] - pitch * 1.23]),
    )
    .await?;
    ready(
        tab,
        "d.dragging && d.preview === 'taipei-north' && d.committed === 'banqiao'",
    )
    .await?;
    touch(tab, "touchEnd", None).await?;
    settled(tab, "taipei-north", 4, 3).await?;
    check_eq!(
        read(tab).await?["offset"],
        json!(3.0),
        "Reduce Motion snaps to the new ordinal without another frame"
    );

    choose(tab, "disabled").await?;
    ready(tab, "d.disabled && d.committed === 'taipei-north'").await?;
    let disabled = tab
        .eval(&format!(
            "{}.getAttribute('aria-disabled')",
            projected(ADJUSTABLE)
        ))
        .await?;
    check_eq!(
        disabled,
        json!("true"),
        "Chrome marks the finite adjustable disabled"
    );
    let tree = ax_tree(tab).await?;
    let disabled_choice = ax(&tree, "spinbutton", LABEL)?;
    check_eq!(
        ax_property(disabled_choice, "disabled"),
        Some(&json!(true)),
        "Chrome's actual accessibility tree shows the disabled choice"
    );
    let [x, y] = screen(tab, upper).await?;
    tab.click(x, y).await?;
    touch(tab, "touchStart", Some(drag_start)).await?;
    touch(tab, "touchMove", Some(drag_end)).await?;
    touch(tab, "touchEnd", None).await?;
    settled(tab, "taipei-north", 4, 3).await?;
    let contacts = tab.eval("window.scrollChoiceContacts").await?;
    let sequence = contacts
        .as_array()
        .context("trusted choice contact trace")?;
    for (device, event) in [
        ("mouse", "pointerup"),
        ("touch", "pointerup"),
        ("touch", "pointercancel"),
    ] {
        check!(
            sequence
                .iter()
                .any(|contact| contact["device"] == device && contact["type"] == event),
            "a trusted {device} {event} reaches the canvas"
        );
    }
    Ok(
        json!({"face":face,"zones":zones,"outside":[upper,lower],"tap_ids":["taipei-north","banqiao"],
        "held_offset":held_offset,"cancel_restored_id":"banqiao","after_replacement_ax":after,"before_ax":before,
        "stale_release_ignored":true,"reduced_motion_id":"taipei-north","commits":3,
        "disabled_inert":true,"trusted_contacts":contacts}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step("M216", "dark RTL at 320 dp and 200 percent keeps the editor, raw draft and IME composition through resize");
    for option in ["theme", "rtl", "large-text"] {
        choose(tab, option).await?;
    }
    resize(tab, 320, 820).await?;
    until(tab, "s.dark === true").await?;
    ready(
        tab,
        "d.rtl && d.text_scale === 2 && !d.fits && d.mode === 'input' && d.committed === 'banqiao'",
    )
    .await?;
    click(tab, EDITOR).await?;
    editor_focus(tab).await?;
    let editor = bounds(tab, EDITOR).await?;
    check!(
        contains([0., 0., 320., 820.], editor) && editor[2] >= 48. && editor[3] >= 48.,
        "the 200 percent editor stays inside the narrow viewport: {editor:?}"
    );
    let no_spin = tab
        .eval(&format!("{} === null", projected(ADJUSTABLE)))
        .await?;
    check_eq!(
        no_spin,
        json!(true),
        "the fallback removes the adjustable accessibility stop"
    );
    let dark = snapshot(env, tab, "compact-dark-rtl").await?;
    let editor_color = color(&read(tab).await?, "editing")?;
    let editor_ink = dark
        .colors(editor)?
        .get(&editor_color)
        .copied()
        .unwrap_or(0);
    check!(
        editor_ink > 16,
        "the real 200 percent editor paints the dark theme ink"
    );

    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(EDITOR)
    );
    tab.shortcut("a").await?;
    tab.ime_compose("桃", 1, 1).await?;
    tab.wait_for(&composing).await?;
    ready(
        tab,
        "d.raw === '板橋' && d.committed === 'banqiao' && d.commits === 0",
    )
    .await?;
    resize(tab, 960, 820).await?;
    ready(
        tab,
        "d.fits && d.mode === 'input' && d.raw === '板橋' && d.commits === 0",
    )
    .await?;
    tab.wait_for(&composing).await?;
    editor_focus(tab).await?;
    snapshot(env, tab, "resized-ime-preedit").await?;

    tab.press("Enter").await?;
    ready(
        tab,
        "d.mode === 'input' && d.committed === 'banqiao' && d.commits === 0",
    )
    .await?;
    if read(tab).await?["raw"] != "桃" {
        tab.insert_text("桃").await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    let pending =
        "d.mode === 'input' && d.raw === '桃' && d.suggestion === '桃園' && d.commits === 0";
    ready(tab, pending).await?;
    resize(tab, 320, 820).await?;
    ready(tab, pending).await?;
    editor_focus(tab).await?;
    let description = helper(tab, SUGGESTION, "桃園", false).await?;
    resize(tab, 960, 820).await?;
    ready(tab, pending).await?;
    editor_focus(tab).await?;
    tab.press("Enter").await?;
    settled(tab, "taoyuan", 4, 1).await?;
    ready(
        tab,
        "d.fits && d.mode === 'scroll' && d.suggestion === null",
    )
    .await?;
    runtime_focus(tab, ADJUSTABLE).await?;
    tab.wait_for(CANVAS_FOCUS).await?;
    let accepted = spin(tab, 4, &["桃園"]).await?;
    snapshot(env, tab, "wide-rtl-value").await?;
    resize(tab, 320, 820).await?;
    ready(tab, "!d.fits && d.mode === 'input' && d.committed === 'taoyuan' && d.raw === '桃園' && d.commits === 1").await?;
    editor_focus(tab).await?;
    Ok(
        json!({"viewport":[320,820],"rtl":true,"text_scale":2,"editor":editor,"editor_ink":editor_ink,
        "preedit_survived_resize":true,"raw":"桃","first_enter_commits":0,"committed_id":"taoyuan",
        "commits":1,"restored_editor_focus":true,"description":description,"roomy_ax":accepted}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let tokens = tokens(env, tab).await?;
    reload(tab, url).await?;
    let prefix = prefix(env, tab).await?;
    reload(tab, url).await?;
    let validation = validation(env, tab).await?;
    reload(tab, url).await?;
    let catalog = catalog(tab).await?;
    reload(tab, url).await?;
    let gestures = gestures(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Scroll Choice Field page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-scroll-choice-fields.json",
        &json!({
            "browser":chrome.version,"tokens":tokens,"prefix":prefix,"validation":validation,
            "catalog":catalog,"gestures":gestures,"compact":compact,"errors":errors
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-scroll-choice-fields-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
                "contacts":tab.eval("window.scrollChoiceContacts ?? []").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-scroll-choice-fields-failure.png"))
            .await;
    }
    result.context("Material 3 finite Scroll Choice Fields")
}
