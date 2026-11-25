//! Material character counter, browser editing, accessibility and Canvas pixels.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_list_corners::reduced_motion;
use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, near, projected, resize, Raster,
};
use super::{click, focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M337", "M338", "M339", "M340"];

const SHOW: &str = "text-field-counter-demo-show";
const BACK: &str = "text-field-counter-demo-back";
const APPEARANCE: &str = "text-field-counter-demo-appearance";
const THEME: &str = "text-field-counter-demo-theme";
const RTL: &str = "text-field-counter-demo-rtl";
const TEXT: &str = "text-field-counter-demo-large-text";
const NARROW: &str = "text-field-counter-demo-narrow";
const ERROR: &str = "text-field-counter-demo-error";
const EDITABLE: &str = "text-field-counter-demo-editable";
const ACTION: &str = "text-field-counter-demo-editable-trailing-action";
const READ_ONLY: &str = "text-field-counter-demo-read-only";
const DISABLED: &str = "text-field-counter-demo-disabled";
const EDITABLE_NAME: &str = "標籤名稱";
const ACTION_NAME: &str = "清除標籤";
const INITIAL: &str = "中a\u{301}👍🏽";
const AT_LIMIT: &str = "中a\u{301}👍🏽🇹🇼👩‍💻";
const OVER_LIMIT: &str = "中a\u{301}👍🏽🇹🇼👩‍💻文";
const LONGER: &str = "中a\u{301}👍🏽🇹🇼👩‍💻文字";
const HELP: &str = "請輸入 5 個字以內的標籤名稱。英文字母、中文或表情符號都可以使用。";
const OVERFLOW: &str = "請輸入最多 5 個字。";
const MANUAL: &str = "這個標籤已存在。";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const LIVE: &str = "window.fieldCounterLive";
const HOME: (u32, u32) = (960, 2400);
const WIDE: Rect = [0., 0., 960., 820.];
const IME: Rect = [0., 0., 480., 820.];
const SMALL: Rect = [0., 0., 320., 1240.];

const WIDE_NORMAL: &str = "material-text-field-counter-wide-outlined-normal.png";
const WIDE_OVERFLOW: &str = "material-text-field-counter-wide-outlined-overflow.png";
const WIDE_MANUAL: &str = "material-text-field-counter-wide-filled-manual.png";
const PREEDIT: &str = "material-text-field-counter-ime-preedit.png";
const COMPACT: &str = "material-text-field-counter-compact-rtl-200.png";
const NARROW_OUTLINED: &str = "material-text-field-counter-fallback-outlined-112.png";
const NARROW_FILLED: &str = "material-text-field-counter-fallback-filled-112.png";

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn part(id: &str, suffix: &str) -> String {
    format!("{id}-{suffix}")
}

fn node_text(id: &str) -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})?.text",
        js(id)
    )
}

fn editor_state() -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]",
        js(EDITABLE)
    )
}

async fn model(tab: &Tab, condition: &str) -> Result<Value> {
    until(
        tab,
        &format!("(d => {condition})(s.text_field_counter_demo)"),
    )
    .await?;
    tab.settle().await?;
    Ok(state(tab).await?["text_field_counter_demo"].clone())
}

async fn value(tab: &Tab, text: &str, edits: u32, submits: u32, actions: u32) -> Result<Value> {
    model(
        tab,
        &format!(
            "d.value === {} && d.limit === 5 && d.edits === {edits} &&
        d.submits === {submits} && d.actions === {actions}",
            js(text)
        ),
    )
    .await
}

pub(super) async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, HOME.0, HOME.1).await?;
    reduced_motion(tab, true).await?;
    until(tab, "s.section === 0 && !s.text_field_counter_demo.visible").await?;
    activate(tab, SHOW).await?;
    model(
        tab,
        "d.visible && d.appearance === 'outlined' && !d.dark && !d.rtl &&
        d.text_scale === 1 && !d.narrow && !d.show_error",
    )
    .await?;
    value(tab, INITIAL, 0, 0, 0).await?;
    resize(tab, 960, 820).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    tab.wait_for(&format!(
        "document.activeElement?.id === 'surface' ||
        document.activeElement === {}",
        projected(id)
    ))
    .await?;
    tab.settle().await
}

async fn editor(tab: &Tab, id: &str) -> Result<()> {
    focus(tab, id, "surface-text-input").await?;
    tab.settle().await
}

async fn enter_editor(tab: &Tab) -> Result<()> {
    if state(tab).await?["focused"] == ACTION {
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

async fn raw(tab: &Tab, text: &str) -> Result<()> {
    tab.wait_for(&format!("{TEXTAREA}?.value === {}", js(text)))
        .await
}

async fn ratio(tab: &Tab, id: &str, count: usize) -> Result<()> {
    tab.wait_for(&format!(
        "{} === {}",
        node_text(&part(id, "counter")),
        js(format!("{count}/5"))
    ))
    .await
}

async fn describe(tab: &Tab, count: usize, help: &str, invalid: bool) -> Result<Value> {
    ratio(tab, EDITABLE, count).await?;
    let count_id = part(EDITABLE, "counter-description");
    let spoken_count = format!("字數，已輸入 {count} 個字，上限 5 個字。");
    let actual = tab.eval(&format!(
        r#"(() => {{
            const field = {TEXTAREA};
            const ids = field?.getAttribute('aria-describedby')?.trim().split(/\s+/) ?? [];
            const items = ids.map(id => document.getElementById(id));
            const counter = items.find(item => item?.dataset.surfaceId === {});
            return {{names:items.map(item => item?.textContent), unique:new Set(ids).size === ids.length,
                invalid:field?.getAttribute('aria-invalid'),
                passive:!!counter && counter.tabIndex < 0 &&
                    !counter.closest('[aria-live]:not([aria-live="off"])')}};
        }})()"#, js(&count_id)
    )).await?;
    check_eq!(
        actual,
        json!({"names":["井字號",spoken_count,help],"unique":true,
        "invalid":invalid.then_some("true"),"passive":true}),
        "the real editor names the passive count before its help"
    );
    let tree = ax_tree(tab).await?;
    let textbox = ax(&tree, "textbox", EDITABLE_NAME)?;
    let clear = ax(&tree, "button", ACTION_NAME)?;
    check!(
        ax_property(clear, "disabled") != Some(&json!(true)),
        "the named clear action stays enabled and separate from the editor"
    );
    let spoken = textbox["description"]["value"]
        .as_str()
        .context("Chrome textbox description")?;
    let expected = format!("井字號 {spoken_count} {help}");
    check_eq!(
        spoken,
        expected.as_str(),
        "Chrome AX reads the count and help in order"
    );
    check!(
        tree.iter()
            .filter(|node| node["ignored"] == false
                && node["role"]["value"] == "textbox"
                && node["name"]["value"] == EDITABLE_NAME)
            .count()
            == 1,
        "the counter keeps one named editor"
    );
    Ok(json!({"ratio":format!("{count}/5"),"dom":actual,"ax_description":spoken}))
}

async fn start_live_trace(tab: &Tab) -> Result<()> {
    tab.eval(&format!(r#"(() => {{
        const regions = document.querySelectorAll('#surface-accessibility [aria-live=assertive]');
        const region = regions[0];
        if (regions.length !== 1 || region.getAttribute('aria-atomic') !== 'true' || region.textContent)
            throw new Error('expected one primed blank validation region');
        const messages = [];
        const observer = new MutationObserver(() => {{
            if (region.textContent) messages.push(region.textContent);
        }});
        observer.observe(region, {{childList:true,characterData:true,subtree:true}});
        {LIVE} = {{region,messages,observer}};
    }})()"#)).await?;
    Ok(())
}

async fn live(tab: &Tab, messages: &[&str]) -> Result<Value> {
    tab.settle().await?;
    let actual = tab.eval(&format!("{LIVE}.messages")).await?;
    check_eq!(
        actual,
        json!(messages),
        "only each new validation message writes to the live region"
    );
    check!(
        tab.eval(&format!("{LIVE}.region === document.querySelector('#surface-accessibility [aria-live=assertive]')")).await? == true,
        "the same assertive region receives those writes"
    );
    Ok(actual)
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

fn rgba(data: &Value, role: &str) -> Result<[u8; 4]> {
    serde_json::from_value(data["palette"][role].clone())
        .with_context(|| format!("counter palette role {role}"))
}

fn rgb(data: &Value, role: &str) -> Result<Rgb> {
    let [r, g, b, _] = rgba(data, role)?;
    Ok([r, g, b])
}

fn ink(frame: &Raster, rect: Rect, token: Rgb) -> Result<usize> {
    Ok(frame.colors(rect)?.get(&token).copied().unwrap_or(0))
}

async fn photo(tab: &Tab, env: &Env, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let file = env.artifact(name);
    tab.screenshot(&file).await?;
    Raster::read(tab, file).await
}

fn checked_pixel(frame: &Raster, point: [f64; 2], token: Rgb) -> Result<Value> {
    let measured = frame.sample(point[0], point[1])?;
    check_eq!(measured, token, format!("appearance pixel at {point:?}"));
    Ok(json!({"point":point,"rgb":measured}))
}

async fn capture(
    tab: &Tab,
    env: &Env,
    name: &str,
    data: &Value,
    viewport: Rect,
    stacked: bool,
    overflow: bool,
) -> Result<Value> {
    let filled = data["appearance"] == "filled";
    let rtl = data["rtl"] == true;
    let scale = data["text_scale"].as_f64().context("counter text scale")?;
    let manual = data["show_error"] == true;
    let invalid = manual || overflow;
    let face = bounds(
        tab,
        &part(EDITABLE, if filled { "indicator" } else { "outline" }),
    )
    .await?;
    let helper = bounds(tab, &part(EDITABLE, "description")).await?;
    let count = bounds(tab, &part(EDITABLE, "counter")).await?;
    let count_description = bounds(tab, &part(EDITABLE, "counter-description")).await?;
    let action = bounds(tab, ACTION).await?;
    let editor_rect = bounds(tab, EDITABLE).await?;
    let line = 16. * scale;
    check!(
        [face, helper, count, action, editor_rect]
            .into_iter()
            .all(|r| contains(viewport, r))
            && near(helper[1] - bottom(face), 4.)
            && near(count[3], line)
            && count_description == count
            && near(
                if rtl {
                    right(face) - right(helper)
                } else {
                    helper[0] - face[0]
                },
                16.
            )
            && near(
                if rtl {
                    count[0] - face[0]
                } else {
                    right(face) - right(count)
                },
                16.
            )
            && near(action[2], 48.)
            && near(action[3], 48.),
        "help and count stay below the face with 4 dp top gap, 16 dp insets and a full action"
    );
    if stacked {
        check!(
            near(face[2], 112.) && editor_rect == face && near(helper[2], 80.)
                && helper[3] >= 2. * line && near(count[1] - bottom(helper), 4.) && action[1] > bottom(count),
            "the narrow 112 dp editor stays full width with help, count and relocated action in order"
        );
    } else {
        check!(
            near(helper[1], count[1])
                && helper[2] >= 48.
                && near(
                    if rtl {
                        helper[0] - right(count)
                    } else {
                        count[0] - right(helper)
                    },
                    16.
                )
                && contains(face, action),
            "the parallel row leaves a 16 dp gap to a trailing count"
        );
    }
    let image = photo(tab, env, name).await?;
    let neutral = rgb(data, "on_surface_variant")?;
    let error = rgb(data, "error")?;
    let help_ink = ink(&image, helper, if invalid { error } else { neutral })?;
    let count_ink = ink(&image, count, if overflow { error } else { neutral })?;
    let wrong_count_ink = ink(&image, count, if overflow { neutral } else { error })?;
    check!(
        help_ink > 12 && count_ink > 12 && wrong_count_ink == 0,
        "help and ratio use the actual expected Canvas colors"
    );
    let stroke = rgb(data, if invalid { "error" } else { "primary" })?;
    let boundary = if filled {
        vec![
            checked_pixel(&image, [face[0] + 6.5, bottom(face) - 0.5], stroke)?,
            checked_pixel(&image, [right(face) - 6.5, bottom(face) - 0.5], stroke)?,
            checked_pixel(
                &image,
                [face[0] + 6.5, face[1] + 10.5],
                rgb(data, "surface_container_highest")?,
            )?,
        ]
    } else {
        vec![checked_pixel(
            &image,
            [right(face) - 0.5, face[1] + face[3] / 2.],
            stroke,
        )?]
    };
    Ok(
        json!({"capture":name,"model":data,"face":face,"editor":editor_rect,"helper":helper,
        "count":count,"count_description":count_description,"action":action,"stacked":stacked,
        "help_ink":help_ink,"count_ink":count_ink,"wrong_count_ink":wrong_count_ink,"boundary":boundary}),
    )
}

async fn authored(tab: &Tab, env: &Env) -> Result<Value> {
    focused(tab, BACK).await?;
    enter_editor(tab).await?;
    raw(tab, INITIAL).await?;
    let initial = value(tab, INITIAL, 0, 0, 0).await?;
    let description = describe(tab, 3, HELP, false).await?;
    start_live_trace(tab).await?;
    let paint = capture(tab, env, WIDE_NORMAL, &initial, WIDE, false, false).await?;
    tab.press("End").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionEnd === {TEXTAREA}.value.length"
    ))
    .await?;
    tab.insert_text("🇹🇼👩‍💻").await?;
    let limit = value(tab, AT_LIMIT, 1, 0, 0).await?;
    raw(tab, AT_LIMIT).await?;
    let at_limit = describe(tab, 5, HELP, false).await?;
    let no_announcement = live(tab, &[]).await?;
    Ok(
        json!({"initial":initial,"paint":paint,"description":description,"at_limit":at_limit,
        "limit_model":limit,"ordinary_live_messages":no_announcement}),
    )
}

async fn limits(tab: &Tab, env: &Env) -> Result<Value> {
    tab.insert_text("文").await?;
    let over = value(tab, OVER_LIMIT, 2, 0, 0).await?;
    raw(tab, OVER_LIMIT).await?;
    let description = describe(tab, 6, OVERFLOW, true).await?;
    let overflow_live = live(tab, &[OVERFLOW]).await?;
    let overflow_paint = capture(tab, env, WIDE_OVERFLOW, &over, WIDE, false, true).await?;

    tab.insert_text("字").await?;
    let longer = value(tab, LONGER, 3, 0, 0).await?;
    let continued = describe(tab, 7, OVERFLOW, true).await?;
    let continued_live = live(tab, &[OVERFLOW]).await?;
    tab.press("Enter").await?;
    let submitted = value(tab, LONGER, 3, 1, 0).await?;
    check_eq!(
        submitted["last_submit"],
        LONGER,
        "the caller receives over-limit content unchanged"
    );

    click(tab, ERROR).await?;
    model(tab, "d.show_error").await?;
    enter_editor(tab).await?;
    let precedence = describe(tab, 7, MANUAL, true).await?;
    let manual_live = live(tab, &[OVERFLOW, MANUAL]).await?;
    tab.press("End").await?;
    for _ in 0..2 {
        tab.press("Backspace").await?;
    }
    value(tab, AT_LIMIT, 5, 1, 0).await?;
    describe(tab, 5, MANUAL, true).await?;
    let at_limit_live = live(tab, &[OVERFLOW, MANUAL]).await?;

    let stable_editor = editor_state();
    let before = tab.eval(&stable_editor).await?;
    click(tab, APPEARANCE).await?;
    let manual = model(tab, "d.appearance === 'filled' && d.show_error").await?;
    enter_editor(tab).await?;
    raw(tab, AT_LIMIT).await?;
    let after = tab.eval(&stable_editor).await?;
    check_eq!(
        after,
        before,
        "appearance keeps the same committed editor snapshot"
    );
    let manual_description = describe(tab, 5, MANUAL, true).await?;
    let manual_paint = capture(tab, env, WIDE_MANUAL, &manual, WIDE, false, false).await?;
    click(tab, ERROR).await?;
    model(tab, "!d.show_error").await?;
    enter_editor(tab).await?;
    let cleared_error = describe(tab, 5, HELP, false).await?;
    let final_live = live(tab, &[OVERFLOW, MANUAL]).await?;
    Ok(
        json!({"overflow":over,"overflow_description":description,"overflow_paint":overflow_paint,
        "longer":longer,"longer_description":continued,"submitted":submitted,"precedence":precedence,
        "manual_description":manual_description,"manual_paint":manual_paint,"restored_help":cleared_error,
        "presentations":{"editor":EDITABLE,"before":before,"after":after},"live":{"overflow":overflow_live,
            "continued":continued_live,"manual":manual_live,"at_limit":at_limit_live,"final":final_live}}),
    )
}

async fn composition(tab: &Tab, env: &Env) -> Result<Value> {
    editor(tab, EDITABLE).await?;
    tab.eval(
        r#"(() => {
        window.counterImeKeys = [];
        window.addEventListener('keydown', e => {
            if (e.key === 'Escape' || e.key === 'Enter')
                window.counterImeKeys.push([e.key,e.isTrusted,e.target.id]);
        }, true);
    })()"#,
    )
    .await?;
    tab.shortcut("a").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === 0 && {TEXTAREA}.selectionEnd === {TEXTAREA}.value.length"
    ))
    .await?;
    let exported = editor_state();
    let composing = format!("{exported}.composing === true");
    tab.ime_compose("ㄓㄨㄥ", 1, 2).await?;
    tab.wait_for(&composing).await?;
    raw(tab, "ㄓㄨㄥ").await?;
    let before = bounds(tab, EDITABLE).await?;
    resize(tab, 480, 820).await?;
    editor(tab, EDITABLE).await?;
    let after = bounds(tab, EDITABLE).await?;
    check!(
        after[2] < before[2],
        "resizing reflows the same composing editor"
    );
    click(tab, ACTION).await?;
    editor(tab, EDITABLE).await?;
    tab.wait_for(&composing).await?;
    raw(tab, "ㄓㄨㄥ").await?;
    let pending = value(tab, AT_LIMIT, 5, 1, 0).await?;
    let preedit = tab.eval(&exported).await?;
    check_eq!(
        preedit["value"],
        AT_LIMIT,
        "the real editor keeps its original value during preedit"
    );
    check_eq!(
        preedit["display_value"],
        "ㄓㄨㄥ",
        "only the display value contains the preedit"
    );
    let pending_description = describe(tab, 5, HELP, false).await?;
    let pending_live = live(tab, &[OVERFLOW, MANUAL]).await?;
    let pending_paint = capture(tab, env, PREEDIT, &pending, IME, false, false).await?;
    let box_rect = bounds(tab, &part(EDITABLE, "value-box")).await?;
    let frame = Raster::read(tab, env.artifact(PREEDIT)).await?;
    let preedit_ink = ink(&frame, box_rect, rgb(&pending, "on_surface")?)?;
    check!(
        preedit_ink > 12,
        "the preedit has real glyphs while the count stays at 5/5"
    );

    tab.press("Escape").await?;
    let used_cancel = tab.eval(&composing).await? == true;
    if used_cancel {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    raw(tab, AT_LIMIT).await?;
    let cancelled = value(tab, AT_LIMIT, 5, 1, 0).await?;
    ratio(tab, EDITABLE, 5).await?;

    tab.shortcut("a").await?;
    tab.ime_compose("中文", 2, 2).await?;
    tab.wait_for(&composing).await?;
    tab.press("Enter").await?;
    editor(tab, EDITABLE).await?;
    let first = model(
        tab,
        &format!(
            "d.submits === 1 && d.actions === 0 &&
        (d.value === {} || d.value === '中文')",
            js(AT_LIMIT)
        ),
    )
    .await?;
    let used_insert_text = first["value"] == AT_LIMIT;
    if used_insert_text {
        tab.insert_text("中文").await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    raw(tab, "中文").await?;
    let committed = value(tab, "中文", 6, 1, 0).await?;
    let committed_description = describe(tab, 2, HELP, false).await?;
    tab.press("Enter").await?;
    let submitted = value(tab, "中文", 6, 2, 0).await?;
    check_eq!(
        submitted["last_submit"],
        "中文",
        "a later Enter submits the committed value once"
    );
    let trusted = tab.eval("window.counterImeKeys").await?;
    check_eq!(
        trusted,
        json!([
            ["Escape", true, "surface-text-input"],
            ["Enter", true, "surface-text-input"],
            ["Enter", true, "surface-text-input"]
        ]),
        "separate trusted Escape and Enter events reach the editing owner"
    );

    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Space").await?;
    let cleared = value(tab, "", 6, 2, 1).await?;
    ratio(tab, EDITABLE, 0).await?;
    let final_live = live(tab, &[OVERFLOW, MANUAL]).await?;
    Ok(
        json!({"method":"CDP synthetic composition and trusted keyboard/pointer","editor_before":before,
        "editor_after":after,"preedit":preedit,"value_box":box_rect,"preedit_ink":preedit_ink,
        "pending":pending,"pending_description":pending_description,"paint":pending_paint,
        "pending_live":pending_live,"cancelled":cancelled,"cancel_needed_cdp":used_cancel,
        "first_enter":first,"commit_needed_insert_text":used_insert_text,"committed":committed,
        "committed_description":committed_description,"second_enter":submitted,"cleared":cleared,
        "trusted_keys":trusted,"final_live":final_live}),
    )
}

fn blend(base: Rgb, ink: [u8; 4], linear: bool) -> Rgb {
    let decode = |channel: u8| {
        let normalized = f64::from(channel) / 255.;
        if !linear {
            normalized
        } else if normalized <= 0.04045 {
            normalized / 12.92
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4)
        }
    };
    let alpha = f64::from(ink[3]) / 255.;
    std::array::from_fn(|i| {
        let mixed = decode(base[i]) * (1. - alpha) + decode(ink[i]) * alpha;
        let channel = if !linear {
            mixed
        } else if mixed <= 0.0031308 {
            mixed * 12.92
        } else {
            1.055 * mixed.powf(1. / 2.4) - 0.055
        };
        (channel * 255.).round() as u8
    })
}

async fn locked(tab: &Tab, env: &Env, data: &Value) -> Result<Value> {
    let read_rect = bounds(tab, &part(READ_ONLY, "counter")).await?;
    let disabled_rect = bounds(tab, &part(DISABLED, "counter")).await?;
    check!(
        [read_rect, disabled_rect]
            .into_iter()
            .all(|rect| contains(SMALL, rect)),
        "both locked count areas fit inside the same compact screenshot"
    );
    ratio(tab, READ_ONLY, 2).await?;
    ratio(tab, DISABLED, 6).await?;
    let frame = Raster::read(tab, env.artifact(COMPACT)).await?;
    let paper = rgb(data, "background")?;
    let neutral = rgb(data, "on_surface_variant")?;
    let muted_source = rgba(data, "on_surface_alpha_38")?;
    check_eq!(
        muted_source[3],
        97,
        "disabled count uses 38 percent foreground"
    );
    let muted_candidates = [
        blend(paper, muted_source, false),
        blend(paper, muted_source, true),
    ];
    let disabled_colors = frame.colors(disabled_rect)?;
    let muted_ink: usize = disabled_colors
        .iter()
        .filter(|(pixel, _)| {
            muted_candidates
                .iter()
                .any(|expected| pixel.iter().zip(expected).all(|(a, b)| a.abs_diff(*b) <= 2))
        })
        .map(|(_, pixels)| *pixels)
        .sum();
    let read_ink = ink(&frame, read_rect, neutral)?;
    let disabled_error_ink = ink(&frame, disabled_rect, rgb(data, "error")?)?;
    check!(
        read_ink > 12 && muted_ink > 12 && disabled_error_ink == 0,
        "read-only ratio remains readable and disabled overflow paints muted ratio ink"
    );

    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Tab").await?;
    editor(tab, READ_ONLY).await?;
    raw(tab, "🇹🇼a\u{301}").await?;
    tab.wait_for(&format!("{TEXTAREA}.readOnly")).await?;
    let tree = ax_tree(tab).await?;
    let read = ax(&tree, "textbox", "唯讀標籤")?;
    let disabled = ax(&tree, "textbox", "停用標籤")?;
    let read_description = read["description"]["value"]
        .as_str()
        .context("read-only AX description")?;
    check!(
        ax_property(read, "readonly") == Some(&json!(true))
            && ax_property(disabled, "disabled") == Some(&json!(true))
            && read_description.contains("字數，已輸入 2 個字，上限 5 個字。"),
        "actual Chrome AX keeps the read-only count and separate disabled editor"
    );
    tab.shortcut("a").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === 0 && {TEXTAREA}.selectionEnd === 6"
    ))
    .await?;
    tab.insert_text("不能改").await?;
    tab.press("Backspace").await?;
    raw(tab, "🇹🇼a\u{301}").await?;
    ratio(tab, READ_ONLY, 2).await?;
    tab.press("Tab").await?;
    tab.settle().await?;
    check!(
        state(tab).await?["focused"] != DISABLED,
        "Tab skips the disabled editor"
    );
    let unchanged = model(
        tab,
        "d.locked_edit_requests === 0 && d.edits === 0 && d.submits === 0 && d.actions === 0",
    )
    .await?;
    Ok(
        json!({"model":unchanged,"read_bounds":read_rect,"disabled_bounds":disabled_rect,
        "read_ratio":"2/5","disabled_ratio":"6/5","readonly":true,"disabled":true,
        "read_description":read_description,"read_ink":read_ink,"muted_candidates":muted_candidates,
        "muted_ink":muted_ink,"disabled_error_ink":disabled_error_ink}),
    )
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    for control in [THEME, RTL, TEXT] {
        click(tab, control).await?;
        tab.settle().await?;
    }
    resize(tab, 320, 1240).await?;
    let base = model(
        tab,
        "d.width === 320 && d.appearance === 'outlined' && d.dark && d.rtl
        && d.text_scale === 2 && !d.narrow && !d.show_error",
    )
    .await?;
    enter_editor(tab).await?;
    raw(tab, INITIAL).await?;
    let description = describe(tab, 3, HELP, false).await?;
    let normal = capture(tab, env, COMPACT, &base, SMALL, false, false).await?;
    check!(
        normal["helper"][3]
            .as_f64()
            .context("compact helper height")?
            > 32.,
        "the normal 320 dp helper wraps without colliding with the ratio"
    );
    let locks = locked(tab, env, &base).await?;

    click(tab, NARROW).await?;
    let narrowed = model(tab, "d.narrow && d.fixture_width === 112").await?;
    enter_editor(tab).await?;
    raw(tab, INITIAL).await?;
    let outlined_description = describe(tab, 3, HELP, false).await?;
    let outlined = capture(tab, env, NARROW_OUTLINED, &narrowed, SMALL, true, false).await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Shift+Tab").await?;
    editor(tab, EDITABLE).await?;

    click(tab, APPEARANCE).await?;
    model(tab, "d.appearance === 'filled'").await?;
    click(tab, ERROR).await?;
    let filled_state = model(tab, "d.appearance === 'filled' && d.show_error && d.narrow").await?;
    enter_editor(tab).await?;
    raw(tab, INITIAL).await?;
    let filled_description = describe(tab, 3, MANUAL, true).await?;
    let filled = capture(tab, env, NARROW_FILLED, &filled_state, SMALL, true, false).await?;
    resize(tab, HOME.0, HOME.1).await?;
    click(tab, BACK).await?;
    model(tab, "!d.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"normal":normal,"normal_description":description,"locks":locks,
        "fallback_outlined":outlined,"fallback_outlined_description":outlined_description,
        "fallback_filled":filled,"fallback_filled_description":filled_description,
        "back_focus":SHOW}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(
            STEPS[0],
            "authored Unicode and combined emoji count once with passive descriptions",
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
        report["authored"] = authored(&tab, env).await?;
        step(
            STEPS[1],
            "the advisory limit keeps overflow and explicit errors take precedence",
        );
        report["limits"] = limits(&tab, env).await?;
        step(
            STEPS[2],
            "synthetic preedit stays outside the count and Enter stays independent",
        );
        report["composition"] = composition(&tab, env).await?;
        step(
            STEPS[3],
            "dark RTL at double text size keeps readable locked and stacked counts",
        );
        report["compact"] = compact(&tab, env, url).await?;
        check!(tab.errors().is_empty(), "no character counter page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-text-field-counter-failure.png"))
            .await;
    }
    env.write_json("material-text-field-counter.json", &report)?;
    result
}
