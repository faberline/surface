//! Baseline filled fields through real headless Chrome and WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focused, near, projected, resize, Raster,
};
use super::{click, focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M329", "M330", "M331", "M332"];

const SHOW: &str = "filled-text-field-demo-show";
const BACK: &str = "filled-text-field-demo-back";
const THEME: &str = "filled-text-field-demo-theme";
const RTL: &str = "filled-text-field-demo-rtl";
const TEXT: &str = "filled-text-field-demo-large-text";
const ERROR: &str = "filled-text-field-demo-error";
const EDITABLE: &str = "filled-text-field-demo-editable";
const READ_ONLY: &str = "filled-text-field-demo-read-only";
const DISABLED: &str = "filled-text-field-demo-disabled";
const LONG: &str = "filled-text-field-demo-long-label";
const NAME: &str = "專案名稱";
const ERROR_TEXT: &str = "請輸入有效的專案名稱。";
const SUPPORT: &str = "請輸入中文或英文名稱。";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const LIVE_TRACE: &str = "window.filledValidationLive";
const HOME: (u32, u32) = (960, 2400);
const IDLE: &str = "material-filled-text-field-idle.png";
const HOVER: &str = "material-filled-text-field-hover.png";
const FOCUSED: &str = "material-filled-text-field-focused.png";
const PREEDIT: &str = "material-filled-text-field-ime-preedit.png";
const ERROR_PNG: &str = "material-filled-text-field-error.png";
const COMPACT: &str = "material-filled-text-field-compact-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn model(tab: &Tab, condition: &str) -> Result<Value> {
    until(
        tab,
        &format!("(d => {condition})(s.filled_text_field_demo)"),
    )
    .await?;
    tab.settle().await?;
    Ok(state(tab).await?["filled_text_field_demo"].clone())
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, HOME.0, HOME.1).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.filled_text_field_demo.visible").await?;
    activate(tab, SHOW).await?;
    model(
        tab,
        "d.visible && !d.dark && !d.rtl && d.text_scale === 1 && !d.show_error &&
         d.value === '' && d.edits === 0 && d.submits === 0 && d.locked_edit_requests === 0",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn editor(tab: &Tab, id: &str) -> Result<()> {
    focus(tab, id, "surface-text-input").await?;
    tab.settle().await
}

async fn part(tab: &Tab, id: &str, suffix: &str) -> Result<Rect> {
    bounds(tab, &format!("{id}-{suffix}")).await
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(name);
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn rgb(data: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, _] = rgba(data, name)?;
    Ok([r, g, b])
}

fn color_pixels(raster: &Raster, ink: Rgb, rect: Rect) -> Result<usize> {
    Ok(raster.colors(rect)?.get(&ink).copied().unwrap_or(0))
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

fn same_rect(a: Rect, b: Rect) -> bool {
    a.into_iter().zip(b).all(|(a, b)| near(a, b))
}

fn entry_body(face: Rect, top: f64, rtl: bool, width: f64) -> Rect {
    let x = if rtl {
        right(face) - 16. - width
    } else {
        face[0] + 16.
    };
    [x, top, width, bottom(face) - top - 5.]
}

fn swatch(raster: &Raster, point: [f64; 2], expected: Rgb) -> Result<Value> {
    let actual = raster.sample(point[0], point[1])?;
    check_eq!(actual, expected, format!("filled field pixel at {point:?}"));
    Ok(json!({"point":point,"rgb":actual}))
}

fn indicator(raster: &Raster, face: Rect, width: f64, ink: Rgb, paper: Rgb) -> Result<Value> {
    let mut samples = Vec::new();
    for fraction in [0.62, 0.82] {
        for depth in [0.5, 1.5, 2.5, 3.5, 4.5] {
            let point = [face[0] + fraction * face[2], bottom(face) - depth];
            samples.push(swatch(
                raster,
                point,
                if depth < width { ink } else { paper },
            )?);
        }
    }
    Ok(json!({"width_dp":width,"samples":samples}))
}

fn blended(background: Rgb, ink: [u8; 4], linear: bool) -> Rgb {
    let decode = |n: u8| {
        let x = f64::from(n) / 255.;
        if !linear {
            x
        } else if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    let alpha = f64::from(ink[3]) / 255.;
    std::array::from_fn(|i| {
        let x = decode(background[i]) * (1. - alpha) + decode(ink[i]) * alpha;
        let encoded = if !linear {
            x
        } else if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1. / 2.4) - 0.055
        };
        (255. * encoded).round() as u8
    })
}

fn rgba(data: &Value, name: &str) -> Result<[u8; 4]> {
    serde_json::from_value(data["palette"][name].clone())
        .with_context(|| format!("filled field palette {name}"))
}

fn mixed_swatch(
    raster: &Raster,
    point: [f64; 2],
    background: Rgb,
    overlay: [u8; 4],
) -> Result<Value> {
    let actual = raster.sample(point[0], point[1])?;
    // WebGPU targets can blend in encoded or sRGB-decoded channels. Both
    // candidates use the exact authored token and original fill.
    let [encoded, linear] = [false, true].map(|linear| blended(background, overlay, linear));
    check!(
        actual != background
            && [encoded, linear].into_iter().any(|expected| actual
                .into_iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= 2)),
        format!("token overlay at {point:?}: {actual:?}, expected {encoded:?} or {linear:?}")
    );
    Ok(
        json!({"point":point,"rgb":actual,"base":background,"overlay":overlay,
        "encoded_mix":encoded,"linear_mix":linear}),
    )
}

async fn description(tab: &Tab, expected: &str, invalid: bool) -> Result<Value> {
    let helper = projected(&format!("{EDITABLE}-description"));
    let observed = tab
        .eval(&format!(
            r#"(() => {{
                const input = {TEXTAREA};
                const helper = {helper};
                const refs = input?.getAttribute('aria-describedby')?.split(/\s+/) ?? [];
                return {{invalid:input?.getAttribute('aria-invalid'), text:helper?.textContent,
                    linked:refs.some(id => document.getElementById(id) === helper)}};
            }})()"#,
        ))
        .await?;
    check_eq!(
        observed,
        json!({"text":expected,"linked":true,"invalid":invalid.then_some("true")}),
        "the mounted editor references its own visible helper"
    );
    let tree = ax_tree(tab).await?;
    let field = ax(&tree, "textbox", NAME)?;
    let spoken = &field["description"]["value"];
    check!(
        spoken.as_str().is_some_and(|text| text.contains(expected))
            && invalid
                == matches!(ax_property(field, "invalid"), Some(value) if value == &json!(true) || value == &json!("true")),
        "Chrome AX exposes the expected named field, description and invalid state"
    );
    Ok(json!({"dom":observed,"ax_description":spoken}))
}

async fn rest_and_focus(tab: &Tab, env: &Env) -> Result<Value> {
    focused(tab, BACK).await?;
    let data = model(tab, "d.width === 960 && d.value === ''").await?;
    let face = bounds(tab, EDITABLE).await?;
    let container = part(tab, EDITABLE, "container").await?;
    let hover_layer = part(tab, EDITABLE, "hover-layer").await?;
    let line = part(tab, EDITABLE, "indicator").await?;
    let inline = part(tab, EDITABLE, "inline-label").await?;
    let support = part(tab, EDITABLE, "description").await?;
    check!(
        [container, hover_layer, line]
            .into_iter()
            .all(|part| same_rect(face, part))
            && near(face[3], 56.)
            && contains(face, inline)
            && near(inline[0] - face[0], 16.)
            && near(support[1] - bottom(face), 4.),
        "the exact 56 dp text-input hit box owns the container, hover layer and indicator"
    );
    let paper = rgb(&data, "background")?;
    let fill = rgb(&data, "surface_container_highest")?;
    let neutral = rgb(&data, "on_surface_variant")?;
    let ink = rgb(&data, "on_surface")?;
    let active = rgb(&data, "primary")?;
    let hover_ink = rgba(&data, "on_surface_alpha_8")?;
    check_eq!(
        hover_ink,
        [ink[0], ink[1], ink[2], 20],
        "the hover overlay uses 8 percent on-surface"
    );
    let idle = photo(tab, env, IDLE).await?;
    let idle_line = indicator(&idle, face, 1., neutral, fill)?;
    let side = face[0];
    let top = face[1];
    let interior = [side + face[2] * 0.75, top + 12.5];
    let top_left = [side + 0.25, top + 0.25];
    let top_right = [right(face) - 0.25, top + 0.25];
    let corners = [
        swatch(&idle, top_left, paper)?,
        swatch(&idle, top_right, paper)?,
        swatch(&idle, [side + 4.5, top + 0.5], fill)?,
        swatch(&idle, [right(face) - 4.5, top + 0.5], fill)?,
        swatch(&idle, [side + 0.5, bottom(face) - 1.5], fill)?,
        swatch(&idle, [right(face) - 0.5, bottom(face) - 1.5], fill)?,
    ];
    let idle_fill = swatch(&idle, interior, fill)?;
    let idle_label_pixels = color_pixels(&idle, neutral, inline)?;
    let support_pixels = color_pixels(&idle, neutral, support)?;
    check!(
        idle_label_pixels > 12 && support_pixels > 12,
        "the label and support have actual neutral ink"
    );

    let [canvas_x, canvas_y, _, _] = tab.canvas_box().await?;
    tab.mouse_move(canvas_x + interior[0], canvas_y + interior[1])
        .await?;
    let hover = photo(tab, env, HOVER).await?;
    focused(tab, BACK).await?;
    let hover_fill = mixed_swatch(&hover, interior, fill, hover_ink)?;
    let hovered_rgb: Rgb = serde_json::from_value(hover_fill["rgb"].clone())?;
    let hover_line = indicator(&hover, face, 1., ink, hovered_rgb)?;
    let hover_corners = [
        swatch(&hover, top_left, paper)?,
        swatch(&hover, top_right, paper)?,
    ];

    tab.click(canvas_x + interior[0], canvas_y + interior[1])
        .await?;
    editor(tab, EDITABLE).await?;
    model(tab, "d.value === '' && d.edits === 0 && d.submits === 0").await?;
    let focused_face = bounds(tab, EDITABLE).await?;
    let float = part(tab, EDITABLE, "floating-label").await?;
    check!(
        same_rect(face, focused_face)
            && contains(face, float)
            && near(float[0] - side, 16.)
            && near(float[1] - top, 8.)
            && near(float[3], 16.),
        "the same editor retains its hit box and floats the 16 dp caption inside the fill"
    );
    let focus_frame = photo(tab, env, FOCUSED).await?;
    let focus_fill = swatch(&focus_frame, interior, fill)?;
    let focus_line = indicator(&focus_frame, face, 3., active, fill)?;
    let float_pixels = color_pixels(&focus_frame, active, float)?;
    let body = entry_body(face, bottom(float), false, 100.);
    let placeholder_pixels = color_pixels(&focus_frame, neutral, body)?;
    let upper_body = [body[0], top, body[2], bottom(float) - top];
    let upper_placeholder_pixels = color_pixels(&focus_frame, neutral, upper_body)?;
    check!(
        float_pixels > 12 && placeholder_pixels > 12 && upper_placeholder_pixels == 0,
        "the actual placeholder is below the filled caption, using only the lower text slot"
    );
    let helper = description(tab, SUPPORT, false).await?;

    let disabled = bounds(tab, DISABLED).await?;
    let disabled_box = swatch(
        &idle,
        [disabled[0] + 0.75 * disabled[2], disabled[1] - 2.5],
        paper,
    )?;
    let disabled_fill = rgba(&data, "on_surface_alpha_4")?;
    let disabled_line = rgba(&data, "on_surface_alpha_38")?;
    check_eq!(
        disabled_fill,
        [ink[0], ink[1], ink[2], 10],
        "disabled fill has 4 percent on-surface"
    );
    check_eq!(
        disabled_line,
        [ink[0], ink[1], ink[2], 97],
        "disabled indicator has 38 percent on-surface"
    );
    let disabled_center = [disabled[0] + 0.75 * disabled[2], disabled[1] + 12.5];
    let disabled_fill_sample = mixed_swatch(&idle, disabled_center, paper, disabled_fill)?;
    let disabled_rgb: Rgb = serde_json::from_value(disabled_fill_sample["rgb"].clone())?;
    let disabled_depths = [
        mixed_swatch(
            &idle,
            [disabled_center[0], bottom(disabled) - 0.5],
            disabled_rgb,
            disabled_line,
        )?,
        swatch(
            &idle,
            [disabled_center[0], bottom(disabled) - 1.5],
            disabled_rgb,
        )?,
    ];
    Ok(
        json!({"face":face,"focused_face":focused_face,"container":container,"hover_layer":hover_layer,
        "indicator_bounds":line,"float_bounds":float,"description_bounds":support,"corners":corners,
        "idle":{"fill":idle_fill,"indicator":idle_line,"label_pixels":idle_label_pixels},
        "hover":{"fill":hover_fill,"indicator":hover_line,"corners":hover_corners},
        "focused":{"fill":focus_fill,"indicator":focus_line,"float_pixels":float_pixels,
            "body_bounds":body,"placeholder_pixels":placeholder_pixels,"upper_placeholder_pixels":upper_placeholder_pixels},
        "disabled":{"face":disabled,"outside":disabled_box,"fill":disabled_fill_sample,"indicator":disabled_depths},
        "description_pixels":support_pixels,"description":helper,"palette":data["palette"],
        "captures":[IDLE,HOVER,FOCUSED]}),
    )
}

async fn composition(tab: &Tab, env: &Env) -> Result<Value> {
    editor(tab, EDITABLE).await?;
    let before = bounds(tab, EDITABLE).await?;
    tab.eval(
        "(() => {
            const keys = [];
            const listener = event => {
                if (event.target?.id === 'surface-text-input' && event.key === 'Enter') {
                    keys.push({trusted:event.isTrusted, dom_composing:event.isComposing});
                }
            };
            window.addEventListener('keydown', listener, true);
            window.filledEnterTrace = {keys, listener};
        })()",
    )
    .await?;
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(EDITABLE),
    );
    tab.ime_compose("中文", 2, 2).await?;
    tab.wait_for(&format!("{composing} && {TEXTAREA}?.value === '中文'"))
        .await?;
    let preedit_model = model(tab, "d.edits === 0 && d.value === '' && d.submits === 0").await?;
    let selection = format!("[{TEXTAREA}.value,{TEXTAREA}.selectionStart,{TEXTAREA}.selectionEnd]");
    let initial_selection = tab.eval(&selection).await?;

    resize(tab, 480, 820).await?;
    editor(tab, EDITABLE).await?;
    tab.wait_for(&format!("{composing} && {TEXTAREA}?.value === '中文'"))
        .await?;
    model(
        tab,
        "d.width === 480 && d.edits === 0 && d.value === '' && d.submits === 0",
    )
    .await?;
    let after = bounds(tab, EDITABLE).await?;
    let selection_after = tab.eval(&selection).await?;
    check!(after[2] < before[2], "preedit crossed a real field reflow");
    check_eq!(
        selection_after,
        initial_selection,
        "active preedit text and offsets survive reflow"
    );
    let frame = photo(tab, env, PREEDIT).await?;
    let float = part(tab, EDITABLE, "floating-label").await?;
    let text_body = entry_body(after, bottom(float), false, 100.);
    let composing_pixels = color_pixels(&frame, rgb(&preedit_model, "on_surface")?, text_body)?;
    let placeholder_pixels = color_pixels(
        &frame,
        rgb(&preedit_model, "on_surface_variant")?,
        text_body,
    )?;
    check!(
        composing_pixels > 12 && placeholder_pixels == 0,
        "the actual Canvas paints composing glyphs without the focused-empty placeholder"
    );

    tab.press("Enter").await?;
    editor(tab, EDITABLE).await?;
    let first = model(
        tab,
        "d.submits === 0 && (d.value === '' || d.value === '中文')",
    )
    .await?;
    check!(
        !tab.eval_str(&format!("{TEXTAREA}.value"))
            .await?
            .contains('\n'),
        "the trusted IME Enter leaves a single-line editor"
    );
    let used_insert_text = first["value"] == "";
    if used_insert_text {
        // Chromium can end a CDP composition without committing its text.
        tab.insert_text("中文").await?;
    }
    tab.wait_for(&format!("!({composing}) && {TEXTAREA}?.value === '中文'"))
        .await?;
    let accepted = model(
        tab,
        "d.value === '中文' && d.edits === 1 && d.submits === 0",
    )
    .await?;
    tab.press("Enter").await?;
    editor(tab, EDITABLE).await?;
    let submitted = model(
        tab,
        "d.value === '中文' && d.edits === 1 && d.submits === 1",
    )
    .await?;
    let enter_keys = tab
        .eval(
            "(() => {
                const {keys, listener} = window.filledEnterTrace;
                window.removeEventListener('keydown', listener, true);
                return keys;
            })()",
        )
        .await?;
    check!(
        enter_keys.as_array().is_some_and(|keys| {
            keys.len() == 2 && keys.iter().all(|key| key["trusted"] == true)
        }),
        "the first and second Enter are separate trusted browser events"
    );
    resize(tab, 960, 820).await?;
    editor(tab, EDITABLE).await?;
    Ok(
        json!({"method":"CDP synthetic composition and trusted Enter keys","enter_keys":enter_keys,"before_bounds":before,
        "after_bounds":after,"before_selection":initial_selection,"after_selection":selection_after,
        "composing_pixels":composing_pixels,"placeholder_pixels":placeholder_pixels,"first_enter":first,
        "insert_text_needed":used_insert_text,"settled":accepted,"second_enter":submitted,"capture":PREEDIT}),
    )
}

async fn validation_and_locked(tab: &Tab, env: &Env) -> Result<Value> {
    tab.eval(
        "(() => {
            const regions = document.querySelectorAll('#surface-accessibility [aria-live=assertive]');
            const region = regions[0];
            if (regions.length !== 1 || region.getAttribute('aria-atomic') !== 'true' || region.textContent)
                throw new Error('expected one primed and blank assertive validation region');
            const messages = [];
            const observer = new MutationObserver(() => {
                if (region.textContent) messages.push(region.textContent);
            });
            observer.observe(region, {childList:true,characterData:true,subtree:true});
            window.filledValidationLive = {region, messages, observer};
        })()",
    )
    .await?;
    activate(tab, ERROR).await?;
    let data = model(tab, "d.show_error && d.value === '中文' && d.submits === 1").await?;
    tab.wait_for(&format!(
        "{LIVE_TRACE}.region.textContent === {}",
        js(ERROR_TEXT)
    ))
    .await?;
    let authored = projected(&format!("{EDITABLE}-validation-status"));
    tab.wait_for(&format!("{authored}?.getAttribute('aria-live') === 'off'"))
        .await?;
    click(tab, EDITABLE).await?;
    editor(tab, EDITABLE).await?;
    let described = description(tab, ERROR_TEXT, true).await?;
    let face = bounds(tab, EDITABLE).await?;
    let label = part(tab, EDITABLE, "floating-label").await?;
    let support = part(tab, EDITABLE, "description").await?;
    let ink = rgb(&data, "error")?;
    let paper = rgb(&data, "surface_container_highest")?;
    let raster = photo(tab, env, ERROR_PNG).await?;
    let stroke = indicator(&raster, face, 3., ink, paper)?;
    let error_label_pixels = color_pixels(&raster, ink, label)?;
    check!(
        error_label_pixels > 12,
        "the inside label has actual error ink"
    );
    let error_pixels = color_pixels(&raster, ink, support)?;
    check!(
        error_pixels > 12,
        "the validation message uses actual error ink"
    );

    tab.press("Tab").await?;
    editor(tab, READ_ONLY).await?;
    tab.wait_for(&format!(
        "{TEXTAREA}?.readOnly && {TEXTAREA}.value === '既有專案'"
    ))
    .await?;
    let tree = ax_tree(tab).await?;
    let read_only = ax(&tree, "textbox", "唯讀名稱")?;
    let disabled = ax(&tree, "textbox", "停用名稱")?;
    check_eq!(
        ax_property(read_only, "readonly"),
        Some(&json!(true)),
        "read-only textbox is selectable"
    );
    check_eq!(
        ax_property(disabled, "disabled"),
        Some(&json!(true)),
        "the disabled textbox is unavailable in Chrome"
    );
    tab.shortcut("a").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === 0 && {TEXTAREA}.selectionEnd === 4"
    ))
    .await?;
    tab.insert_text("不能改").await?;
    tab.press("Backspace").await?;
    tab.press("Enter").await?;
    editor(tab, READ_ONLY).await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.readOnly && {TEXTAREA}.value === '既有專案'"
    ))
    .await?;
    model(
        tab,
        "d.value === '中文' && d.edits === 1 && d.submits === 1 && d.locked_edit_requests === 0",
    )
    .await?;

    tab.press("Tab").await?;
    editor(tab, LONG).await?;
    click(tab, DISABLED).await?;
    tab.settle().await?;
    let unchanged = model(
        tab,
        "d.value === '中文' && d.edits === 1 && d.submits === 1 && d.long_edits === 0 &&
         d.locked_edit_requests === 0",
    )
    .await?;
    until(tab, &format!("s.focused !== {}", js(DISABLED))).await?;
    check!(
        tab.eval(&format!(
            "document.activeElement !== {TEXTAREA} || {TEXTAREA}?.getAttribute('aria-label') !== '停用名稱'",
        ))
        .await?.as_bool() == Some(true),
        "clicking a disabled field never mounts it as the active editor"
    );
    check_eq!(
        tab.eval(&format!("{LIVE_TRACE}.messages")).await?,
        json!([ERROR_TEXT]),
        "editing, focus and the disabled click do not repeat the warning"
    );
    activate(tab, ERROR).await?;
    model(tab, "!d.show_error").await?;
    tab.wait_for(&format!("{LIVE_TRACE}.region.textContent === ''"))
        .await?;
    click(tab, EDITABLE).await?;
    editor(tab, EDITABLE).await?;
    description(tab, SUPPORT, false).await?;
    activate(tab, ERROR).await?;
    model(tab, "d.show_error").await?;
    tab.wait_for(&format!("{LIVE_TRACE}.messages.length === 2"))
        .await?;
    let live_messages = tab.eval(&format!("{LIVE_TRACE}.messages")).await?;
    check_eq!(
        live_messages,
        json!([ERROR_TEXT, ERROR_TEXT]),
        "the same error can appear again after clearing it"
    );
    check_eq!(
        tab.eval(&format!(
            "{LIVE_TRACE}.region === document.querySelector('#surface-accessibility [aria-live=assertive]')"
        ))
        .await?,
        json!(true),
        "the live region stays mounted when the editor updates"
    );
    Ok(
        json!({"description":described,"indicator":stroke,"label_bounds":label,"label_pixels":error_label_pixels,
        "error_bounds":support,"error_pixels":error_pixels,"readonly":true,"disabled":true,
        "tab_skipped_disabled_to":LONG,"model":unchanged,"live_priority":"assertive","assertive_messages":live_messages,
        "capture":ERROR_PNG}),
    )
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    for setting in [THEME, RTL, TEXT, ERROR] {
        activate(tab, setting).await?;
    }
    resize(tab, 320, 960).await?;
    let data = model(
        tab,
        "d.width === 320 && d.dark && d.rtl && d.text_scale === 2 && d.show_error",
    )
    .await?;
    click(tab, EDITABLE).await?;
    editor(tab, EDITABLE).await?;
    let face = bounds(tab, EDITABLE).await?;
    let floated = part(tab, EDITABLE, "floating-label").await?;
    let long = bounds(tab, LONG).await?;
    let above = part(tab, LONG, "above-label").await?;
    let unused_inside = part(tab, LONG, "floating-label").await?;
    let viewport = [0., 0., 320., 960.];
    check!(
        near(face[3], 96.) && near(long[3], 96.) && contains(face, floated)
            && [face, long, above].into_iter().all(|part| contains(viewport, part))
            && near(floated[1] - face[1], 8.) && near(floated[3], 32.)
            && near(right(face) - right(floated), 16.)
            && near(above[0] - long[0], 16.) && near(right(long) - right(above), 16.)
            && above[3] >= 64. && near(long[1] - bottom(above), 4.) && unused_inside[2] == 0.,
        "the long label wraps completely above a 96 dp field and the short caption stays inside at right"
    );
    let fill = rgb(&data, "surface_container_highest")?;
    let error = rgb(&data, "error")?;
    let neutral = rgb(&data, "on_surface_variant")?;
    let frame = photo(tab, env, COMPACT).await?;
    let short_line = indicator(&frame, face, 3., error, fill)?;
    let long_line = indicator(&frame, long, 1., neutral, fill)?;
    let short_fill = swatch(&frame, [face[0] + 8.5, face[1] + 10.5], fill)?;
    let long_fill = swatch(&frame, [long[0] + 8.5, long[1] + 10.5], fill)?;
    let long_label_pixels = color_pixels(&frame, neutral, above)?;
    let short_label_pixels = color_pixels(&frame, error, floated)?;
    let right_body = entry_body(long, long[1] + 40., true, 105.);
    let left_body = entry_body(long, long[1] + 40., false, 52.);
    let upper_body = [right_body[0], long[1], right_body[2], 40.];
    let value = rgb(&data, "on_surface")?;
    let right_pixels = color_pixels(&frame, value, right_body)?;
    let left_pixels = color_pixels(&frame, value, left_body)?;
    let upper_pixels = color_pixels(&frame, value, upper_body)?;
    check!(
        long_label_pixels > 40 && short_label_pixels > 30 && right_pixels > 30 && left_pixels == 0 && upper_pixels == 0,
        "the full wrapping caption has ink and the value only paints at the logical start in the lower slot"
    );
    let described = description(tab, ERROR_TEXT, true).await?;
    resize(tab, HOME.0, HOME.1).await?;
    activate(tab, BACK).await?;
    model(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"model":data,"short_face":face,"short_label":floated,"short_label_pixels":short_label_pixels,
        "short_fill":short_fill,"short_indicator":short_line,"long_face":long,"long_label":above,
        "long_fill":long_fill,"long_indicator":long_line,"long_label_pixels":long_label_pixels,
        "right_value_bounds":right_body,"right_value_pixels":right_pixels,"left_value_bounds":left_body,
        "left_value_pixels":left_pixels,"upper_value_pixels":upper_pixels,"description":described,
        "back_focus":SHOW,"capture":COMPACT}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(STEPS[0], "filled fields paint the container, hover layer, bottom indicator and lower placeholder");
        route(&tab, url).await?;
        report["paint"] = rest_and_focus(&tab, env).await?;
        step(STEPS[1], "CDP preedit survives a real reflow and only a separate trusted Enter submits");
        report["composition"] = composition(&tab, env).await?;
        step(STEPS[2], "the invalid editor describes its error while read-only and disabled editors stay inert");
        report["states"] = validation_and_locked(&tab, env).await?;
        step(STEPS[3], "dark RTL at 200 percent keeps an inside label, a readable long label and lower right-aligned text");
        report["compact"] = compact(&tab, env, url).await?;
        check!(tab.errors().is_empty(), "no filled text field page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-filled-text-field-failure.png"))
            .await;
    }
    env.write_json("material-filled-text-field.json", &report)?;
    result
}
