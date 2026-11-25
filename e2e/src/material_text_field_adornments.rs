//! Adorned Material fields in headless Chrome and WebGPU.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, near, projected, resize, Raster,
};
use super::{click, focus, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M333", "M334", "M335", "M336"];

const SHOW: &str = "text-field-adornments-demo-show";
const BACK: &str = "text-field-adornments-demo-back";
const APPEARANCE: &str = "text-field-adornments-demo-appearance";
const THEME: &str = "text-field-adornments-demo-theme";
const RTL: &str = "text-field-adornments-demo-rtl";
const TEXT: &str = "text-field-adornments-demo-large-text";
const NARROW: &str = "text-field-adornments-demo-narrow";
const ERROR: &str = "text-field-adornments-demo-error";
const EDITABLE: &str = "text-field-adornments-demo-editable";
const ACTION: &str = "text-field-adornments-demo-editable-trailing-action";
const READ_ONLY: &str = "text-field-adornments-demo-read-only";
const READ_ACTION: &str = "text-field-adornments-demo-read-only-trailing-action";
const DISABLED_ACTION: &str = "text-field-adornments-demo-disabled-trailing-action";
const EDITABLE_NAME: &str = "方案費用";
const ACTION_NAME: &str = "套用建議金額";
const SUPPORT: &str = "請輸入不含單位的金額。";
const VALIDATION: &str = "請輸入大於零的金額。";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const HOME: (u32, u32) = (960, 2400);
const IDLE: &str = "material-text-field-adornments-outlined-idle.png";
const OUTLINED: &str = "material-text-field-adornments-outlined-focused.png";
const FILLED: &str = "material-text-field-adornments-filled-focused.png";
const PREEDIT: &str = "material-text-field-adornments-ime-preedit.png";
const COMPACT: &str = "material-text-field-adornments-compact-dark-rtl-200.png";
const FALLBACK: &str = "material-text-field-adornments-fallback-112.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Clone, Copy, Serialize)]
struct Slots {
    face: Rect,
    editor: Rect,
    action: Rect,
    leading: Rect,
    trailing: Rect,
    prefix: Rect,
    suffix: Rect,
    value: Rect,
    floating: Rect,
}

async fn model(tab: &Tab, condition: &str) -> Result<Value> {
    until(
        tab,
        &format!("(d => {condition})(s.text_field_adornments_demo)"),
    )
    .await?;
    tab.settle().await?;
    Ok(state(tab).await?["text_field_adornments_demo"].clone())
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, HOME.0, HOME.1).await?;
    reduced_motion(tab, true).await?;
    until(
        tab,
        "s.section === 0 && !s.text_field_adornments_demo.visible",
    )
    .await?;
    activate(tab, SHOW).await?;
    model(tab, "d.visible && d.appearance === 'outlined' && !d.dark && !d.rtl && d.text_scale === 1 &&
          !d.narrow && !d.show_error && d.value === '' && d.edits === 0 && d.actions === 0 && d.submits === 0").await?;
    resize(tab, 960, 820).await
}

async fn editor(tab: &Tab, id: &str) -> Result<()> {
    focus(tab, id, "surface-text-input").await?;
    tab.settle().await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    tab.wait_for(&format!(
        "document.activeElement?.id === 'surface' || document.activeElement === {}",
        projected(id)
    ))
    .await?;
    tab.settle().await
}

async fn enter_editor(tab: &Tab) -> Result<()> {
    let current = state(tab).await?;
    if current["focused"] == ACTION {
        tab.press("Shift+Tab").await?;
    } else {
        for _ in 0..8 {
            if state(tab).await?["focused"] == EDITABLE {
                break;
            }
            tab.press("Tab").await?;
            tab.settle().await?;
        }
    }
    editor(tab, EDITABLE).await
}

fn editor_state() -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]",
        js(EDITABLE)
    )
}

async fn part(tab: &Tab, suffix: &str) -> Result<Rect> {
    bounds(tab, &format!("{EDITABLE}-{suffix}")).await
}

async fn slots(tab: &Tab, boundary: &str) -> Result<Slots> {
    Ok(Slots {
        face: part(tab, boundary).await?,
        editor: bounds(tab, EDITABLE).await?,
        action: bounds(tab, ACTION).await?,
        leading: part(tab, "leading-icon").await?,
        trailing: part(tab, "trailing-icon").await?,
        prefix: part(tab, "prefix-visual").await?,
        suffix: part(tab, "suffix-visual").await?,
        value: part(tab, "value-box").await?,
        floating: part(tab, "floating-label").await?,
    })
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

fn logical_start(face: Rect, rect: Rect, rtl: bool) -> f64 {
    if rtl {
        right(face) - right(rect)
    } else {
        rect[0] - face[0]
    }
}

fn logical_end(face: Rect, rect: Rect, rtl: bool) -> f64 {
    if rtl {
        right(face) - rect[0]
    } else {
        right(rect) - face[0]
    }
}

fn assert_slots(slots: Slots, rtl: bool, face_height: f64) -> Result<()> {
    let Slots {
        face,
        editor,
        action,
        leading,
        trailing,
        prefix,
        suffix,
        value,
        ..
    } = slots;
    check!(
        near(face[3], face_height)
            && near(editor[3], face[3])
            && near(editor[1], face[1])
            && near(action[2], 48.)
            && near(action[3], 48.)
            && near(logical_start(face, editor, rtl), 0.)
            && near(
                logical_end(face, editor, rtl),
                logical_start(face, action, rtl)
            )
            && near(logical_end(face, action, rtl), face[2])
            && near(action[1] + 24., face[1] + face[3] / 2.)
            && contains(face, action),
        "the whole face surrounds disjoint editor and 48 dp action bounds"
    );
    check!(
        [leading, trailing]
            .into_iter()
            .all(|icon| near(icon[2], 24.)
                && near(icon[3], 24.)
                && near(icon[1] + 12., face[1] + face[3] / 2.))
            && near(logical_start(face, leading, rtl), 12.)
            && near(face[2] - logical_end(face, trailing, rtl), 12.)
            && contains(action, trailing)
            && contains(editor, leading),
        "two 24 dp glyphs follow logical insets inside their separate targets"
    );
    check!(
        value[2] >= 48.
            && [prefix, value, suffix]
                .into_iter()
                .all(|slot| contains(editor, slot))
            && near(prefix[1], value[1])
            && near(suffix[1], value[1])
            && near(prefix[3], value[3])
            && near(suffix[3], value[3])
            && near(logical_start(face, prefix, rtl), 52.)
            && near(face[2] - logical_end(face, suffix, rtl), 52.)
            && near(
                logical_start(face, value, rtl) - logical_end(face, prefix, rtl),
                2.
            )
            && near(
                logical_start(face, suffix, rtl) - logical_end(face, value, rtl),
                2.
            ),
        "affixes and value share a line with two logical 2 dp gaps"
    );
    Ok(())
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn rgb(data: &Value, role: &str) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(data["palette"][role].clone())
        .with_context(|| format!("field adornment palette role {role}"))?;
    Ok([r, g, b])
}

fn ink(raster: &Raster, color: Rgb, rect: Rect) -> Result<usize> {
    Ok(raster.colors(rect)?.get(&color).copied().unwrap_or(0))
}

fn sample(raster: &Raster, point: [f64; 2], color: Rgb) -> Result<Value> {
    let pixels = raster.sample(point[0], point[1])?;
    check_eq!(pixels, color, format!("text field pixel at {point:?}"));
    Ok(json!({"point":point,"rgb":pixels}))
}

fn outline(
    raster: &Raster,
    slots: Slots,
    width: f64,
    color: Rgb,
    paper: Rgb,
    notched: bool,
) -> Result<Value> {
    let [x, y, w, h] = slots.face;
    let mut samples = Vec::new();
    // The right edge is beyond the editor's own hit rectangle.
    for d in [0.5, 1.5, 2.5, 3.5] {
        samples.push(sample(
            raster,
            [x + w - d, y + h / 2.],
            if d < width { color } else { paper },
        )?);
    }
    if notched {
        let label = slots.floating;
        for (x, expected) in [
            (label[0] - 6., color),
            (label[0] - 2., paper),
            (right(label) + 2., paper),
            (right(label) + 6., color),
        ] {
            samples.push(sample(raster, [x, y + 0.5], expected)?);
        }
    }
    Ok(json!({"width_dp":width,"samples":samples}))
}

fn indicator(raster: &Raster, face: Rect, color: Rgb, fill: Rgb) -> Result<Value> {
    let mut samples = Vec::new();
    for x in [face[0] + 6.5, right(face) - 6.5] {
        for d in [0.5, 1.5, 2.5, 3.5] {
            samples.push(sample(
                raster,
                [x, bottom(face) - d],
                if d < 3. { color } else { fill },
            )?);
        }
    }
    Ok(json!({"width_dp":3,"samples":samples}))
}

async fn describe(tab: &Tab, help: &str, invalid: bool) -> Result<Value> {
    let actual = tab.eval(&format!(
        r#"(() => {{
            const field = {TEXTAREA};
            const ids = field?.getAttribute('aria-describedby')?.trim().split(/\s+/) ?? [];
            return {{names:ids.map(id => document.getElementById(id)?.textContent),
                unique:new Set(ids).size === ids.length, invalid:field?.getAttribute('aria-invalid')}};
        }})()"#,
    )).await?;
    check_eq!(
        actual,
        json!({"names":["歐元","每月",help],"unique":true,"invalid":invalid.then_some("true")}),
        "the textarea points to unique ordered affixes and current support or error"
    );
    let tree = ax_tree(tab).await?;
    let textbox = ax(&tree, "textbox", EDITABLE_NAME)?;
    let action = ax(&tree, "button", ACTION_NAME)?;
    let spoken = textbox["description"]["value"]
        .as_str()
        .context("Chrome AX editor description")?;
    let (prefix, suffix, help) = (spoken.find("歐元"), spoken.find("每月"), spoken.find(help));
    check!(
        matches!((prefix,suffix,help), (Some(p),Some(s),Some(h)) if p < s && s < h),
        "Chrome reports the ordered description"
    );
    check!(
        textbox["nodeId"] != action["nodeId"]
            && [("textbox", EDITABLE_NAME), ("button", ACTION_NAME)]
                .into_iter()
                .all(|(role, name)| tree
                    .iter()
                    .filter(|node| node["ignored"] == false
                        && node["role"]["value"] == role
                        && node["name"]["value"] == name)
                    .count()
                    == 1),
        "Chrome exposes exactly one editor and a separate named action"
    );
    Ok(
        json!({"dom":actual,"ax_description":spoken,"action_role":"button","action_name":ACTION_NAME}),
    )
}

async fn presentations(tab: &Tab, env: &Env) -> Result<Value> {
    focused(tab, BACK).await?;
    let data = model(tab, "d.value === ''").await?;
    let paper = rgb(&data, "background")?;
    let neutral = rgb(&data, "on_surface_variant")?;
    let primary = rgb(&data, "primary")?;
    let idle_slots = slots(tab, "outline").await?;
    assert_slots(idle_slots, false, 56.)?;
    let idle = photo(tab, env, IDLE).await?;
    let idle_outline = outline(&idle, idle_slots, 1., rgb(&data, "outline")?, paper, false)?;
    let idle_leading = ink(&idle, neutral, idle_slots.leading)?;
    let idle_trailing = ink(&idle, neutral, idle_slots.trailing)?;
    let idle_suffix = ink(&idle, neutral, idle_slots.suffix)?;
    check!(
        idle_leading > 20 && idle_trailing > 20 && idle_suffix == 0,
        "idle paints both icons and keeps the far-side suffix hidden"
    );

    click(tab, &format!("{EDITABLE}-leading-icon")).await?;
    editor(tab, EDITABLE).await?;
    let outlined_slots = slots(tab, "outline").await?;
    assert_slots(outlined_slots, false, 56.)?;
    check!(
        near(outlined_slots.floating[0] - outlined_slots.face[0], 16.),
        "the Outlined floating caption keeps its own 16 dp inset"
    );
    let outlined = photo(tab, env, OUTLINED).await?;
    let focused_outline = outline(&outlined, outlined_slots, 3., primary, paper, true)?;
    let prefix_pixels = ink(&outlined, neutral, outlined_slots.prefix)?;
    let suffix_pixels = ink(&outlined, neutral, outlined_slots.suffix)?;
    let idle_prefix = ink(&idle, neutral, idle_slots.prefix)?;
    check!(
        prefix_pixels > 12 && suffix_pixels > 12 && idle_prefix != prefix_pixels,
        "focus paints different prefix ink and the newly visible suffix"
    );
    let description = describe(tab, SUPPORT, false).await?;

    tab.insert_text("中文42").await?;
    model(
        tab,
        "d.value === '中文42' && d.edits === 1 && d.actions === 0 && d.submits === 0",
    )
    .await?;
    tab.shortcut("a").await?;
    let state = editor_state();
    tab.wait_for(&format!(
        "{state}.selection_anchor === 0 && {state}.selection_cursor === 8"
    ))
    .await?;
    let before = tab.eval(&state).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, ERROR).await?;
    click(tab, APPEARANCE).await?;
    model(
        tab,
        "d.appearance === 'filled' && d.value === '中文42' && d.edits === 1",
    )
    .await?;
    enter_editor(tab).await?;
    let after = tab.eval(&state).await?;
    check_eq!(
        after,
        before,
        "the editor and selection survive the appearance rebuild"
    );
    let filled_slots = slots(tab, "indicator").await?;
    assert_slots(filled_slots, false, 56.)?;
    check!(
        near(filled_slots.floating[0] - filled_slots.face[0], 52.)
            && near(filled_slots.floating[1] - filled_slots.face[1], 8.)
            && near(filled_slots.value[1] - filled_slots.face[1], 24.),
        "the Filled caption uses the icon inset and the value sits a line below"
    );
    let filled = photo(tab, env, FILLED).await?;
    let fill = rgb(&data, "surface_container_highest")?;
    let active = indicator(&filled, filled_slots.face, primary, fill)?;
    let solid_fill = sample(
        &filled,
        [filled_slots.face[0] + 6.5, filled_slots.face[1] + 10.5],
        fill,
    )?;
    let filled_prefix = ink(&filled, neutral, filled_slots.prefix)?;
    let filled_suffix = ink(&filled, neutral, filled_slots.suffix)?;
    check!(
        filled_prefix > 12 && filled_suffix > 12,
        "Filled paints both measured affixes"
    );
    Ok(json!({"palette":data["palette"],
        "idle":{"slots":idle_slots,"outline":idle_outline,"leading_ink":idle_leading,
            "trailing_ink":idle_trailing,"prefix_region_ink":idle_prefix,"suffix_ink":idle_suffix,"capture":IDLE},
        "outlined":{"slots":outlined_slots,"outline":focused_outline,"prefix_ink":prefix_pixels,
            "suffix_ink":suffix_pixels,"description":description,"capture":OUTLINED},
        "filled":{"slots":filled_slots,"indicator":active,"fill":solid_fill,"prefix_ink":filled_prefix,
            "suffix_ink":filled_suffix,"capture":FILLED},
        "retained_editor":{"id":EDITABLE,"before":before,"after":after}}))
}

async fn actions(tab: &Tab) -> Result<Value> {
    tab.eval(
        r#"(() => {
        const keys = [];
        const pointer = [];
        window.adornmentInput = {keys, pointer};
        window.addEventListener('keydown', e => {
            if (e.key === 'Enter' || e.key === ' ') keys.push([e.key, e.isTrusted]);
        }, true);
        for (const type of ['pointerdown','pointerup']) {
            window.addEventListener(type, e => {
                if (e.target.id === 'surface') pointer.push([type,e.isTrusted]);
            }, true);
        }
    })()"#,
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Shift+Tab").await?;
    editor(tab, EDITABLE).await?;
    tab.press("Enter").await?;
    let submitted = model(
        tab,
        "d.actions === 0 && d.submits === 1 && d.last_submit === '中文42'",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Enter").await?;
    let applied = model(
        tab,
        "d.actions === 1 && d.submits === 1 && d.edits === 1 && d.value === '24'",
    )
    .await?;
    focused(tab, ACTION).await?;
    tab.press("Space").await?;
    let spaced = model(tab, "d.actions === 2 && d.submits === 1 && d.edits === 1").await?;
    let hit = point(tab, ACTION).await?;
    tab.mouse_move(hit.x, hit.y).await?;
    tab.mouse_down().await?;
    let held = model(tab, "d.actions === 2 && d.submits === 1").await?;
    tab.mouse_up().await?;
    let released = model(
        tab,
        "d.actions === 3 && d.submits === 1 && d.edits === 1 && d.value === '24'",
    )
    .await?;
    let trusted = tab.eval("window.adornmentInput").await?;
    check_eq!(trusted, json!({"keys":[["Enter",true],["Enter",true],[" ",true]],
        "pointer":[["pointerdown",true],["pointerup",true]]}),
        "trusted editor Enter, action Enter, action Space and pointer release each take separate routes");
    Ok(
        json!({"editor_submit":submitted,"action_enter":applied,"action_space":spaced,
        "held":held,"released":released,"trusted_events":trusted}),
    )
}

async fn composition(tab: &Tab, env: &Env) -> Result<Value> {
    // Re-enter by keyboard after the physical click uses canvas focus.
    enter_editor(tab).await?;
    tab.shortcut("a").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === 0 && {TEXTAREA}.selectionEnd === 2"
    ))
    .await?;
    let state = editor_state();
    let composing = format!("{state}.composing === true");
    tab.ime_compose("ㄓㄨㄥ", 1, 2).await?;
    tab.wait_for(&format!("{composing} && {TEXTAREA}.value === 'ㄓㄨㄥ'"))
        .await?;
    let before = bounds(tab, EDITABLE).await?;
    resize(tab, 480, 820).await?;
    editor(tab, EDITABLE).await?;
    tab.wait_for(&format!("{composing} && {TEXTAREA}.value === 'ㄓㄨㄥ'"))
        .await?;
    let after = bounds(tab, EDITABLE).await?;
    check!(
        after[2] < before[2],
        "a real resize reflows the composing value lane"
    );
    let pending = "d.value === '24' && d.edits === 1 && d.actions === 3 && d.submits === 1";
    click(tab, ACTION).await?;
    editor(tab, EDITABLE).await?;
    tab.wait_for(&composing).await?;
    let staged = model(tab, pending).await?;
    let preedit = tab.eval(&state).await?;
    let shaped = part(tab, "value-box").await?;
    let suffix = part(tab, "suffix-visual").await?;
    let frame = photo(tab, env, PREEDIT).await?;
    let preedit_ink = ink(&frame, rgb(&staged, "on_surface")?, shaped)?;
    let affix_ink = ink(&frame, rgb(&staged, "on_surface_variant")?, suffix)?;
    check!(
        preedit_ink > 12 && affix_ink > 12,
        "Canvas shows composed glyphs inside the value lane and passive affix outside it"
    );

    tab.press("Escape").await?;
    let used_cancel = tab.eval(&composing).await? == true;
    if used_cancel {
        // Some browser versions need CDP to end their synthetic composition.
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing}) && {TEXTAREA}.value === '24'"))
        .await?;
    editor(tab, EDITABLE).await?;
    let cancelled = model(tab, pending).await?;

    tab.shortcut("a").await?;
    tab.ime_compose("中文", 2, 2).await?;
    tab.wait_for(&composing).await?;
    tab.press("Enter").await?;
    editor(tab, EDITABLE).await?;
    let first = model(
        tab,
        "d.actions === 3 && d.submits === 1 && (d.value === '24' || d.value === '中文')",
    )
    .await?;
    let used_insert_text = first["value"] == "24";
    if used_insert_text {
        tab.insert_text("中文").await?;
    }
    tab.wait_for(&format!("!({composing}) && {TEXTAREA}.value === '中文'"))
        .await?;
    let committed = model(
        tab,
        "d.value === '中文' && d.edits === 2 && d.actions === 3 && d.submits === 1",
    )
    .await?;
    tab.press("Enter").await?;
    let submitted = model(
        tab,
        "d.value === '中文' && d.edits === 2 && d.actions === 3 &&
        d.submits === 2 && d.last_submit === '中文'",
    )
    .await?;
    Ok(
        json!({"method":"CDP synthetic composition with trusted click, Escape and Enter",
        "editor_before":before,"editor_after":after,"preedit":preedit,"value_box":shaped,
        "suffix_box":suffix,"preedit_ink":preedit_ink,"suffix_ink":affix_ink,
        "protected":staged,"cancelled":cancelled,"cancel_needed_cdp":used_cancel,
        "first_enter":first,"commit_needed_insert_text":used_insert_text,"committed":committed,
        "second_enter":submitted,"capture":PREEDIT}),
    )
}

async fn locked_fields(tab: &Tab, viewport: Rect) -> Result<Value> {
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Tab").await?;
    editor(tab, READ_ONLY).await?;
    tab.wait_for(&format!("{TEXTAREA}.readOnly && {TEXTAREA}.value === '18'"))
        .await?;
    let tree = ax_tree(tab).await?;
    let readonly = ax(&tree, "textbox", "目前方案費用")?;
    let disabled = ax(&tree, "button", "套用停用方案金額")?;
    let disabled_editor = ax(&tree, "textbox", "停用方案費用")?;
    check!(
        ax_property(readonly, "readonly") == Some(&json!(true))
            && ax_property(disabled, "disabled") == Some(&json!(true))
            && ax_property(disabled_editor, "disabled") == Some(&json!(true)),
        "Chrome distinguishes read-only text from disabled text and action"
    );
    tab.shortcut("a").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === 0 && {TEXTAREA}.selectionEnd === 2"
    ))
    .await?;
    tab.insert_text("不能改").await?;
    tab.press("Backspace").await?;
    tab.wait_for(&format!("{TEXTAREA}.readOnly && {TEXTAREA}.value === '18'"))
        .await?;
    model(
        tab,
        "d.locked_edit_requests === 0 && d.read_only_actions === 0",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, READ_ACTION).await?;
    tab.press("Space").await?;
    model(
        tab,
        "d.read_only_actions === 1 && d.locked_edit_requests === 0",
    )
    .await?;
    tab.press("Tab").await?;
    until(tab, &format!("s.focused !== {}", js(DISABLED_ACTION))).await?;
    let disabled_bounds = bounds(tab, DISABLED_ACTION).await?;
    check!(
        contains(viewport, disabled_bounds)
            && near(disabled_bounds[2], 48.)
            && near(disabled_bounds[3], 48.),
        "the disabled action retains a visible full-size target"
    );
    click(tab, DISABLED_ACTION).await?;
    let data = model(
        tab,
        "d.read_only_actions === 1 && d.disabled_action_requests === 0 &&
        d.actions === 0 && d.edits === 0 && d.submits === 0 && d.locked_edit_requests === 0",
    )
    .await?;
    until(tab, &format!("s.focused !== {}", js(DISABLED_ACTION))).await?;
    Ok(json!({"model":data,"readonly":true,"disabled":true,
        "readonly_action":READ_ACTION,"disabled_action":disabled_bounds}))
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    for toggle in [APPEARANCE, THEME, RTL, TEXT, ERROR] {
        click(tab, toggle).await?;
        tab.settle().await?;
    }
    resize(tab, 320, 960).await?;
    let data = model(
        tab,
        "d.width === 320 && d.appearance === 'filled' && d.dark && d.rtl &&
        d.text_scale === 2 && d.show_error && !d.narrow",
    )
    .await?;
    enter_editor(tab).await?;
    let compact = slots(tab, "indicator").await?;
    assert_slots(compact, true, 96.)?;
    let viewport = [0., 0., 320., 960.];
    check!(
        contains(viewport, compact.face)
            && contains(viewport, compact.action)
            && near(logical_start(compact.face, compact.floating, true), 52.),
        "the compact large-text label stays inside at logical start"
    );
    let described = describe(tab, VALIDATION, true).await?;
    let frame = photo(tab, env, COMPACT).await?;
    let error = rgb(&data, "error")?;
    let fill = rgb(&data, "surface_container_highest")?;
    let neutral = rgb(&data, "on_surface_variant")?;
    let compact_indicator = indicator(&frame, compact.face, error, fill)?;
    let compact_trailing = ink(&frame, error, compact.trailing)?;
    let compact_affixes = [
        ink(&frame, neutral, compact.prefix)?,
        ink(&frame, neutral, compact.suffix)?,
    ];
    check!(
        compact_trailing > 20 && compact_affixes.into_iter().all(|pixels| pixels > 12),
        "error icon and both enlarged mirrored affixes paint in dark mode"
    );
    let compact_capture = json!({"slots":compact,"indicator":compact_indicator,
        "trailing_error_ink":compact_trailing,"affix_ink":compact_affixes,
        "description":described,"model":data,"capture":COMPACT});
    let locked = locked_fields(tab, viewport).await?;

    click(tab, NARROW).await?;
    model(tab, "d.narrow && d.fixture_width === 112").await?;
    enter_editor(tab).await?;
    let narrow = slots(tab, "indicator").await?;
    let prefix = part(tab, "prefix-wrapped").await?;
    let suffix = part(tab, "suffix-wrapped").await?;
    let label = part(tab, "above-label").await?;
    check!(
        near(narrow.face[2], 112.)
            && near(narrow.editor[2], 112.)
            && narrow.value[2] >= 48.
            && contains(narrow.editor, narrow.value)
            && near(narrow.action[2], 48.)
            && near(narrow.action[3], 48.)
            && narrow.action[1] > bottom(narrow.face)
            && near(narrow.action[0], narrow.face[0])
            && narrow.leading[2] == 0.
            && narrow.prefix[2] == 0.
            && narrow.suffix[2] == 0.
            && narrow.floating[2] == 0.
            && label[3] > 0.
            && bottom(label) <= narrow.face[1]
            && prefix[1] > bottom(narrow.action)
            && suffix[1] > bottom(prefix)
            && [narrow.face, narrow.action, label, prefix, suffix]
                .into_iter()
                .all(|rect| contains(viewport, rect)),
        "112 dp fallback leaves a full editor, moves the action and wraps local affixes below"
    );
    let fallback = photo(tab, env, FALLBACK).await?;
    let fallback_description = describe(tab, VALIDATION, true).await?;
    let fallback_indicator = indicator(&fallback, narrow.face, error, fill)?;
    // Examine the old 24 dp glyph footprint only in the gutter beside the caption.
    let icon_gutter = [
        narrow.leading[0],
        narrow.leading[1],
        (label[0] - narrow.leading[0]).min(24.),
        24.,
    ];
    check!(
        icon_gutter[2] >= 8. && contains(viewport, icon_gutter),
        "the fallback leaves a visible gutter between the old glyph origin and label ink"
    );
    let omitted_icon_ink = ink(&fallback, neutral, icon_gutter)?;
    check_eq!(
        omitted_icon_ink,
        0,
        "the removed leading vector leaves no ink"
    );
    let wrapped_ink = [
        ink(&fallback, neutral, prefix)?,
        ink(&fallback, neutral, suffix)?,
    ];
    check!(
        wrapped_ink.into_iter().all(|pixels| pixels > 12),
        "both fallback captions contain real text ink"
    );
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Shift+Tab").await?;
    editor(tab, EDITABLE).await?;
    resize(tab, HOME.0, HOME.1).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, ERROR).await?;
    click(tab, BACK).await?;
    model(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(json!({"compact":compact_capture,"locked":locked,
        "fallback":{"slots":narrow,"above_label":label,"prefix":prefix,"suffix":suffix,
            "wrapped_ink":wrapped_ink,"indicator":fallback_indicator,
            "omitted_icon":{"gutter":icon_gutter,"ink":omitted_icon_ink},
            "description":fallback_description,"capture":FALLBACK},
        "back_focus":SHOW}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(
            STEPS[0],
            "Outlined and Filled keep the editor and paint separate passive affixes",
        );
        route(&tab, url).await?;
        let adapter = tab
            .eval(
                "(async () => {
                const {info} = await navigator.gpu.requestAdapter();
                const {vendor, architecture, isFallbackAdapter} = info;
                return {vendor, architecture, isFallbackAdapter};
            })()",
            )
            .await?;
        check!(
            adapter["vendor"].is_string()
                && adapter["architecture"].is_string()
                && adapter["isFallbackAdapter"].is_boolean(),
            "headless Chrome reports its WebGPU adapter"
        );
        report["adapter"] = adapter;
        report["presentations"] = presentations(&tab, env).await?;
        step(
            STEPS[1],
            "a separate named action takes trusted Enter, Space and pointer release",
        );
        report["actions"] = actions(&tab).await?;
        step(
            STEPS[2],
            "composition keeps the value isolated and protects the trailing action",
        );
        report["composition"] = composition(&tab, env).await?;
        step(
            STEPS[3],
            "dark RTL at 200 percent keeps locks, affixes and a 112 dp fallback",
        );
        report["compact"] = compact(&tab, env, url).await?;
        check!(tab.errors().is_empty(), "no adorned text field page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["failure_state"] = tab
            .eval("({model: JSON.parse(window.gallery.inspect()), active: document.activeElement?.id})")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-text-field-adornments-failure.png"))
            .await;
    }
    env.write_json("material-text-field-adornments.json", &report)?;
    result
}
