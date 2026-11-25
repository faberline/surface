//! Baseline outlined fields through real headless Chrome and WebGPU.

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

pub(super) const STEPS: [&str; 4] = ["M325", "M326", "M327", "M328"];

const SHOW: &str = "outlined-text-field-demo-show";
const BACK: &str = "outlined-text-field-demo-back";
const THEME: &str = "outlined-text-field-demo-theme";
const RTL: &str = "outlined-text-field-demo-rtl";
const TEXT: &str = "outlined-text-field-demo-large-text";
const ERROR: &str = "outlined-text-field-demo-error";
const EDITABLE: &str = "outlined-text-field-demo-editable";
const READ_ONLY: &str = "outlined-text-field-demo-read-only";
const DISABLED: &str = "outlined-text-field-demo-disabled";
const LONG: &str = "outlined-text-field-demo-long-label";
const NAME: &str = "專案名稱";
const ERROR_TEXT: &str = "請輸入有效的專案名稱。";
const SUPPORT: &str = "請輸入中文或英文名稱。";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const LIVE_TRACE: &str = "window.outlinedValidationLive";
const HOME: (u32, u32) = (960, 2400);
const IDLE: &str = "material-outlined-text-field-idle.png";
const FOCUSED: &str = "material-outlined-text-field-focused.png";
const PREEDIT: &str = "material-outlined-text-field-ime-preedit.png";
const ERROR_PNG: &str = "material-outlined-text-field-error.png";
const COMPACT: &str = "material-outlined-text-field-compact-dark-rtl-200.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn model(tab: &Tab, condition: &str) -> Result<Value> {
    until(
        tab,
        &format!("(d => {condition})(s.outlined_text_field_demo)"),
    )
    .await?;
    tab.settle().await?;
    Ok(state(tab).await?["outlined_text_field_demo"].clone())
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, HOME.0, HOME.1).await?;
    reduced_motion(tab, true).await?;
    until(
        tab,
        "s.section === 0 && !s.outlined_text_field_demo.visible",
    )
    .await?;
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
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(data["palette"][name].clone())
        .with_context(|| format!("outlined field palette {name}"))?;
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

fn label_body(rect: Rect) -> Rect {
    [rect[0], rect[1] + 2., rect[2].min(145.), rect[3] - 4.]
}

fn stripe(raster: &Raster, face: Rect, x: f64, width: f64, ink: Rgb, paper: Rgb) -> Result<Value> {
    let mut samples = Vec::new();
    for depth in [0.5, 1.5, 2.5, 3.5] {
        let point = [x, face[1] + depth];
        let actual = raster.sample(point[0], point[1])?;
        let expected = if depth < width { ink } else { paper };
        check_eq!(
            actual,
            expected,
            format!("outlined field top stroke at {point:?}")
        );
        samples.push(json!({"point":point,"rgb":actual}));
    }
    Ok(json!({"width_dp":width,"samples":samples}))
}

fn notch(
    raster: &Raster,
    face: Rect,
    label: Rect,
    stroke: Rgb,
    paper: Rgb,
    rtl: bool,
) -> Result<Value> {
    let leading = if rtl {
        right(face) - right(label)
    } else {
        label[0] - face[0]
    };
    check!(
        label[2] > 0. && label[1] < face[1] && bottom(label) > face[1] && near(leading, 16.),
        "the floating label intersects the outline at the logical leading inset"
    );
    let y = face[1] + 0.5;
    let mut samples = Vec::new();
    for (x, expected) in [
        (label[0] - 6., stroke),
        (label[0] - 2., paper),
        (right(label) + 2., paper),
        (right(label) + 6., stroke),
    ] {
        let actual = raster.sample(x, y)?;
        check_eq!(
            actual,
            expected,
            format!("outline notch padding at ({x},{y})")
        );
        samples.push(json!({"point":[x,y],"rgb":actual}));
    }
    Ok(json!({"label":label,"face":face,"samples":samples}))
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
    let outline = part(tab, EDITABLE, "outline").await?;
    let inline = part(tab, EDITABLE, "inline-label").await?;
    let support = part(tab, EDITABLE, "description").await?;
    check!(
        face.into_iter().zip(outline).all(|(a, b)| near(a, b))
            && near(face[3], 56.)
            && contains(face, inline)
            && near(support[1] - bottom(face), 4.),
        "a measured 56 dp editor owns its outline and the helper starts 4 dp below"
    );
    let fine = rgb(&data, "outline")?;
    let paper = rgb(&data, "background")?;
    let neutral = rgb(&data, "on_surface_variant")?;
    let active = rgb(&data, "primary")?;
    let idle = photo(tab, env, IDLE).await?;
    let x = face[0] + 0.72 * face[2];
    let idle_stroke = stripe(&idle, face, x, 1., fine, paper)?;
    let idle_label_pixels = color_pixels(&idle, neutral, label_body(inline))?;
    let support_pixels = color_pixels(&idle, neutral, support)?;
    check!(
        idle_label_pixels > 12 && support_pixels > 12,
        "the idle label and support have actual neutral ink"
    );

    click(tab, EDITABLE).await?;
    editor(tab, EDITABLE).await?;
    let data = model(tab, "d.value === '' && d.edits === 0 && d.submits === 0").await?;
    let float = part(tab, EDITABLE, "floating-label").await?;
    let focus_frame = photo(tab, env, FOCUSED).await?;
    let focus_stroke = stripe(&focus_frame, face, x, 3., active, paper)?;
    let cut = notch(&focus_frame, face, float, active, paper, false)?;
    let focused_placeholder_pixels = color_pixels(&focus_frame, neutral, label_body(inline))?;
    let float_pixels = color_pixels(&focus_frame, active, float)?;
    let helper = description(tab, SUPPORT, false).await?;
    let mut changed = 0;
    let [x, y, _, h] = inline;
    for dx in 0..72 {
        for dy in 2..(h as usize - 2) {
            let point = [x + dx as f64 + 0.25, y + dy as f64 + 0.25];
            let before = idle.sample(point[0], point[1])?;
            let after = focus_frame.sample(point[0], point[1])?;
            if before != after && (before == neutral || after == neutral) {
                changed += 1;
            }
        }
    }
    check!(
        focused_placeholder_pixels > 12 && float_pixels > 12 && changed > 16,
        "focus paints the relocated primary label and different neutral placeholder glyphs"
    );
    Ok(
        json!({"face":face,"inline":inline,"description_bounds":support,
        "idle":idle_stroke,"focused":focus_stroke,"notch":cut,"label_pixels":idle_label_pixels,
        "floating_pixels":float_pixels,"placeholder_pixels":focused_placeholder_pixels,
        "glyph_difference_samples":changed,"description_pixels":support_pixels,"description":helper,
        "palette":data["palette"],"captures":[IDLE,FOCUSED]}),
    )
}

async fn composition(tab: &Tab, env: &Env) -> Result<Value> {
    editor(tab, EDITABLE).await?;
    let before = bounds(tab, EDITABLE).await?;
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
    let inline = part(tab, EDITABLE, "inline-label").await?;
    let text_body = label_body(inline);
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
    resize(tab, 960, 820).await?;
    editor(tab, EDITABLE).await?;
    Ok(
        json!({"method":"CDP synthetic composition and trusted Enter keys","before_bounds":before,
        "after_bounds":after,"before_selection":initial_selection,"after_selection":selection_after,
        "composing_pixels":composing_pixels,"placeholder_pixels":placeholder_pixels,"first_enter":first,
        "insert_text_needed":used_insert_text,"settled":accepted,"second_enter":submitted,"capture":PREEDIT}),
    )
}

async fn validation_and_locked(tab: &Tab, env: &Env) -> Result<Value> {
    tab.eval(
        "(() => {
            const region = document.querySelector('#surface-accessibility [aria-live=assertive]');
            if (!region || region.getAttribute('aria-atomic') !== 'true' || region.textContent)
                throw new Error('expected one primed and blank assertive validation region');
            const messages = [];
            const observer = new MutationObserver(() => {
                if (region.textContent) messages.push(region.textContent);
            });
            observer.observe(region, {childList:true,characterData:true,subtree:true});
            window.outlinedValidationLive = {region, messages, observer};
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
    let paper = rgb(&data, "background")?;
    let raster = photo(tab, env, ERROR_PNG).await?;
    let stroke = stripe(&raster, face, face[0] + 0.72 * face[2], 3., ink, paper)?;
    let cut = notch(&raster, face, label, ink, paper, false)?;
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
    Ok(json!({"description":described,"stroke":stroke,"notch":cut,
        "error_bounds":support,"error_pixels":error_pixels,"readonly":true,"disabled":true,
        "tab_skipped_disabled_to":LONG,"model":unchanged,"assertive_messages":live_messages,
        "capture":ERROR_PNG}))
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
    let unused_notch = part(tab, LONG, "floating-label").await?;
    let viewport = [0., 0., 320., 960.];
    check!(
        near(face[3],80.) && near(long[3],80.) &&
        [face, floated, long, above].into_iter().all(|part| contains(viewport,part)) &&
        near(above[0] - long[0],16.) && near(right(long) - right(above),16.) &&
        above[3] >= 64. && near(long[1] - bottom(above),4.) && unused_notch[2] == 0.,
        "the wide label wraps above a complete 80 dp editor while the short label retains its notch"
    );
    let paper = rgb(&data, "background")?;
    let error = rgb(&data, "error")?;
    let outline = rgb(&data, "outline")?;
    let frame = photo(tab, env, COMPACT).await?;
    let short_border = stripe(&frame, face, face[0] + 0.3 * face[2], 3., error, paper)?;
    let short_notch = notch(&frame, face, floated, error, paper, true)?;
    let long_border = stripe(&frame, long, long[0] + 0.5 * long[2], 1., outline, paper)?;
    let long_label_pixels = color_pixels(&frame, rgb(&data, "on_surface_variant")?, above)?;
    let midline = long[1] + long[3] / 2. - 20.;
    let right_body = [right(long) - 94., midline, 78., 40.];
    let left_body = [long[0] + 16., midline, 52., 40.];
    let value = rgb(&data, "on_surface")?;
    let right_pixels = color_pixels(&frame, value, right_body)?;
    let left_pixels = color_pixels(&frame, value, left_body)?;
    check!(
        long_label_pixels > 40 && right_pixels > 30 && left_pixels == 0,
        "the uncut long label and right-aligned text paint in the dark compact frame"
    );
    let described = description(tab, ERROR_TEXT, true).await?;
    resize(tab, HOME.0, HOME.1).await?;
    activate(tab, BACK).await?;
    model(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"model":data,"short_border":short_border,"short_notch":short_notch,
        "long_border":long_border,"long_label":above,"long_label_pixels":long_label_pixels,
        "right_value_bounds":right_body,"right_value_pixels":right_pixels,
        "left_value_bounds":left_body,"left_value_pixels":left_pixels,
        "description":described,"back_focus":SHOW,"capture":COMPACT}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(STEPS[0], "actual idle and focused outlined fields paint the label, placeholder, notch and helper");
        route(&tab, url).await?;
        report["paint"] = rest_and_focus(&tab, env).await?;
        step(STEPS[1], "CDP preedit survives a real reflow and only a separate trusted Enter submits");
        report["composition"] = composition(&tab, env).await?;
        step(STEPS[2], "the invalid editor describes its error while read-only and disabled editors stay inert");
        report["states"] = validation_and_locked(&tab, env).await?;
        step(STEPS[3], "dark RTL at 200 percent keeps a short notch and a readable long label above its outline");
        report["compact"] = compact(&tab, env, url).await?;
        check!(tab.errors().is_empty(), "no outlined text field page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-outlined-text-field-failure.png"))
            .await;
    }
    env.write_json("material-outlined-text-field.json", &report)?;
    result
}
