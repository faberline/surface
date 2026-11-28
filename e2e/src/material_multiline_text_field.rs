//! Material multiline editing and nested scrolling through Chrome and WebGPU.

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

pub(super) const STEPS: [&str; 6] = ["M341", "M342", "M343", "M344", "M345", "M346"];

const SHOW: &str = "multiline-text-field-demo-show";
const BACK: &str = "multiline-text-field-demo-back";
const APPEARANCE: &str = "multiline-text-field-demo-appearance";
const THEME: &str = "multiline-text-field-demo-theme";
const RTL: &str = "multiline-text-field-demo-rtl";
const LARGE: &str = "multiline-text-field-demo-large-text";
const SCROLL: &str = "multiline-text-field-demo-scroll";
const GROW: &str = "multiline-text-field-demo-grow";
const FIXED: &str = "multiline-text-field-demo-fixed";
const SUBMIT: &str = "multiline-text-field-demo-submit";
const READ_ONLY: &str = "multiline-text-field-demo-read-only";
const DISABLED: &str = "multiline-text-field-demo-disabled";
const ACTION: &str = "multiline-text-field-demo-grow-trailing-action";
const GROW_HELP: &str = "請簡短記錄這次的變更。";
const FIXED_HELP: &str = "內容會在固定高度內捲動。";
const READ_ONLY_HELP: &str = "這段文字可以選取和複製。";
const READ_ONLY_TEXT: &str = "已儲存的第一行\n已儲存的第二行\n";
const DISABLED_TEXT: &str = "停用的第一行\n停用的第二行";
const INITIAL: &str = "第一行";
const FIXED_TEXT: &str =
    "第一行😀\n第二行 a\u{301}\n第三行 ABC 中文文字\n第四行\n第五行\n第六行\n第七行\n第八行\n";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const EXPORT: &str = "JSON.parse(window.gallery.export_dom())";
const WIDE: (u32, u32) = (960, 840);
const COMPACT: (u32, u32) = (320, 900);
const REPORT: &str = "browser-material-multiline-text-field.json";

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn part(id: &str, suffix: &str) -> String {
    format!("{id}-{suffix}")
}

fn input(id: &str) -> String {
    format!("{EXPORT}.text_inputs[{}]", js(id))
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn canvas(mode: (u32, u32)) -> Rect {
    [0., 0., f64::from(mode.0), f64::from(mode.1)]
}

async fn model(tab: &Tab, condition: &str) -> Result<Value> {
    until(
        tab,
        &format!("(d => d && ({condition}))(s.multiline_text_field_demo)"),
    )
    .await?;
    tab.settle().await?;
    Ok(state(tab).await?["multiline_text_field_demo"].clone())
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2500).await?;
    reduced_motion(tab, true).await?;
    activate(tab, SHOW).await?;
    model(
        tab,
        &format!(
            "d.visible && d.appearance === 'outlined' && !d.rtl && !d.dark &&
        d.text_scale === 1 && d.grow.value === {} && d.fixed.value === {} &&
        d.grow.limit === 12 && d.fixed.rows === 3 && d.submits === 0",
            js(INITIAL),
            js(FIXED_TEXT)
        ),
    )
    .await?;
    resize(tab, WIDE.0, WIDE.1).await?;
    tab.eval(
        r#"(() => {
        window.multilineTrusted = [];
        for (const kind of ['keydown', 'beforeinput', 'wheel']) {
            window.addEventListener(kind, e => {
                if (e.isTrusted) window.multilineTrusted.push({
                    kind, id:e.target?.id ?? null, key:e.key ?? null,
                    input:e.inputType ?? null, dy:e.deltaY ?? null,
                });
            }, {capture:true,passive:true});
        }
    })()"#,
    )
    .await?;
    Ok(())
}

async fn host_edit(tab: &Tab, id: &str) -> Result<()> {
    click(tab, id).await?;
    focus(tab, id, "surface-text-input").await?;
    tab.settle().await
}

async fn textarea(tab: &Tab, value: &str) -> Result<()> {
    tab.wait_for(&format!("{TEXTAREA}.value === {}", js(value)))
        .await
}

async fn selection(tab: &Tab, id: &str) -> Result<Value> {
    tab.settle().await?;
    tab.eval(&format!(
        r#"(() => {{
        const e = {TEXTAREA};
        return {{web:[e.selectionStart,e.selectionEnd,e.selectionDirection],rust:{}}};
    }})()"#,
        input(id)
    ))
    .await
}

async fn ax_editor(tab: &Tab, id: &str, label: &str, help: &str) -> Result<Value> {
    let helper = projected(&part(id, "description"));
    tab.wait_for(&format!(r#"(() => {{
        const e = {TEXTAREA};
        const ids = e?.getAttribute('aria-describedby')?.trim().split(/\s+/) ?? [];
        return e === document.activeElement && ids.some(id => document.getElementById(id) === {helper});
    }})()"#)).await?;
    let tree = ax_tree(tab).await?;
    let live = ax(&tree, "textbox", label)?;
    let description = live["description"]["value"]
        .as_str()
        .context("Chrome textbox description")?;
    check!(
        description.contains(help),
        "the textarea exposes its own help: {description}"
    );
    check!(
        ax_property(live, "multiline") == Some(&json!(true)),
        "Chrome AX sees a multiline textbox: {live}"
    );
    check_eq!(
        tree.iter()
            .filter(|node| node["ignored"] == false
                && node["role"]["value"] == "textbox"
                && node["name"]["value"] == label)
            .count(),
        1,
        "one named editor with the focused real textarea"
    );
    Ok(json!({"label":label,"description":description,
        "backend_dom_node":live["backendDOMNodeId"],"multiline":true}))
}

async fn photo(tab: &Tab, env: &Env, tag: &str) -> Result<(String, Raster)> {
    tab.settle().await?;
    let name = format!("material-multiline-text-field-{tag}.png");
    let file = env.artifact(&name);
    tab.screenshot(&file).await?;
    Ok((name, Raster::read(tab, file).await?))
}

fn rgb(data: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(data["palette"][name].clone())
        .with_context(|| format!("palette {name}"))?;
    check_eq!(alpha, 255, "opaque color token");
    Ok([r, g, b])
}

fn ink(frame: &Raster, rect: Rect, token: Rgb) -> Result<usize> {
    Ok(frame.colors(rect)?.get(&token).copied().unwrap_or(0))
}

async fn profile(
    tab: &Tab,
    env: &Env,
    tag: &str,
    size: (u32, u32),
    scale: f64,
    filled: bool,
) -> Result<Value> {
    let field = bounds(tab, FIXED).await?;
    let body = bounds(tab, &part(FIXED, "value-box")).await?;
    let grow = bounds(tab, GROW).await?;
    let scroller = bounds(tab, SCROLL).await?;
    let disabled = bounds(tab, DISABLED).await?;
    let face = bounds(
        tab,
        &part(FIXED, if filled { "indicator" } else { "outline" }),
    )
    .await?;
    let help = bounds(tab, &part(FIXED, "description")).await?;
    check_eq!(
        tab.canvas_box().await?[2..],
        canvas(size)[2..],
        "canvas size"
    );
    check!(
        contains(canvas(size), scroller)
            && contains(scroller, body)
            && contains(field, body)
            && near(scroller[3], 480.)
            && near(body[3], 72. * scale)
            && bottom(disabled) > bottom(scroller)
            && help[1] >= bottom(face) + 3.5,
        "fixed rows are clipped and outer content has scroll extent: {body:?}, {scroller:?}"
    );
    let data = state(tab).await?["multiline_text_field_demo"].clone();
    let (name, frame) = photo(tab, env, tag).await?;
    let glyphs = ink(&frame, body, rgb(&data, "on_surface")?)?;
    check!(glyphs > 30, "actual fixed body glyphs: {glyphs}");
    let sample = [field[0] + 9., bottom(face) - 5.];
    let expected = if filled {
        "surface_container_highest"
    } else {
        "background"
    };
    check_eq!(
        frame.sample(sample[0], sample[1])?,
        rgb(&data, expected)?,
        "appearance face color"
    );
    let below = [body[0], bottom(body) + 1., body[2], 3.];
    check_eq!(
        ink(&frame, below, rgb(&data, "on_surface")?)?,
        0,
        "body ink stays out of the bottom gutter"
    );
    Ok(
        json!({"screenshot":name,"canvas":canvas(size),"grow":grow,"fixed":field,
        "body":body,"face":face,"outer":scroller,"help":help,"below_body":below,
        "sample":sample,"surface":frame.sample(sample[0],sample[1])?,"glyphs":glyphs,"model":data}),
    )
}

async fn outlined(tab: &Tab, env: &Env) -> Result<Value> {
    let original = bounds(tab, GROW).await?;
    let next = bounds(tab, FIXED).await?;
    let action = bounds(tab, ACTION).await?;
    host_edit(tab, GROW).await?;
    let before_ax = ax_editor(tab, GROW, "多行備註", GROW_HELP).await?;
    check!(
        before_ax["description"]
            .as_str()
            .is_some_and(|s| s.contains("井字號") && s.contains("3")),
        "prefix and count descriptions survive"
    );
    tab.shortcut("a").await?;
    tab.press("ArrowRight").await?;
    let with_lf = format!("{INITIAL}\n");
    tab.wait_for(&format!(
        "{}.selection_cursor === {}",
        input(GROW),
        INITIAL.len()
    ))
    .await?;
    tab.press("Enter").await?;
    let entered = model(
        tab,
        &format!(
            "d.grow.value === {} && d.grow.edits === 1 && d.submits === 0",
            js(&with_lf)
        ),
    )
    .await?;
    textarea(tab, &with_lf).await?;
    let after = bounds(tab, GROW).await?;
    let following = bounds(tab, FIXED).await?;
    check!(
        near(after[3] - original[3], 24.) && near(following[1] - next[1], 24.),
        "one terminal newline grows the field and moves the next field"
    );
    check_eq!(
        bounds(tab, ACTION).await?,
        action,
        "trailing hit stays on the first row"
    );
    tab.press("ArrowUp").await?;
    tab.wait_for(&format!("{}.selection_cursor === 0", input(GROW)))
        .await?;
    let up = selection(tab, GROW).await?;
    tab.press("ArrowDown").await?;
    tab.wait_for(&format!(
        "{}.selection_cursor === {}",
        input(GROW),
        with_lf.len()
    ))
    .await?;
    let down = selection(tab, GROW).await?;
    let fixed = profile(tab, env, "wide-outlined", WIDE, 1., false).await?;
    activate(tab, SUBMIT).await?;
    let submitted = model(
        tab,
        "d.submits === 1 && d.grow.edits === 1 && d.fixed.edits === 0",
    )
    .await?;
    check_eq!(
        submitted["last_submit"],
        json!({"grow":with_lf,"fixed":FIXED_TEXT}),
        "the application action alone submits both committed multiline values"
    );
    let trace = tab.eval("window.multilineTrusted").await?;
    check!(
        trace.as_array().is_some_and(|events| events
            .iter()
            .any(|event| event["id"] == "surface-text-input" && event["key"] == "Enter")),
        "trusted Enter reached the editor"
    );
    Ok(
        json!({"before":original,"after":after,"outlined":fixed,"ax":before_ax,
        "enter_model":entered,"up":up,"down":down,"submitted":submitted,"trusted":trace}),
    )
}

async fn filled_ime(tab: &Tab, env: &Env) -> Result<Value> {
    activate(tab, APPEARANCE).await?;
    let old = format!("{INITIAL}\n");
    model(tab, "d.appearance === 'filled' && d.grow.edits === 1").await?;
    host_edit(tab, GROW).await?;
    let ax = ax_editor(tab, GROW, "多行備註", GROW_HELP).await?;
    tab.shortcut("a").await?;
    tab.press("ArrowRight").await?;
    tab.press("Shift+ArrowLeft").await?;
    tab.press("Shift+ArrowLeft").await?;
    tab.wait_for(&format!(
        "{}.selection_anchor === {} && {}.selection_cursor === {}",
        input(GROW),
        old.len(),
        input(GROW),
        "第一".len()
    ))
    .await?;
    let before = selection(tab, GROW).await?;
    check_eq!(
        before["web"],
        json!([2, 4, "backward"]),
        "trusted keys select the backward multiline range"
    );
    tab.ime_compose("中文", 0, 1).await?;
    tab.wait_for(&format!("{}.composing === true", input(GROW)))
        .await?;
    let pending = selection(tab, GROW).await?;
    check_eq!(
        pending["rust"]["value"],
        json!(old),
        "preedit retains the committed value"
    );
    check_eq!(
        pending["rust"]["display_value"],
        "第一中文",
        "marked text replaces the display range"
    );
    let view = model(tab, "d.grow.edits === 1 && d.submits === 1").await?;
    let counter = part(GROW, "counter");
    tab.wait_for(&format!(
        "{EXPORT}.dom.nodes.find(n => n.stable_id === {})?.text === '4/12'",
        js(&counter)
    ))
    .await?;
    let painted = profile(tab, env, "wide-filled-preedit", WIDE, 1., true).await?;
    let body = bounds(tab, &part(GROW, "value-box")).await?;
    let frame = Raster::read(
        tab,
        env.artifact("material-multiline-text-field-wide-filled-preedit.png"),
    )
    .await?;
    check!(
        ink(&frame, body, rgb(&view, "on_surface")?)? > 12,
        "the marked value paints in the real viewport"
    );
    tab.press("Escape").await?;
    let used_cdp_cancel = tab.eval(&format!("{}.composing", input(GROW))).await? == true;
    if used_cdp_cancel {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!{}.composing", input(GROW))).await?;
    textarea(tab, &old).await?;
    let restored = selection(tab, GROW).await?;
    check_eq!(
        restored,
        before,
        "cancel restores the backward range in Rust and the real textarea"
    );
    model(tab, "d.grow.edits === 1 && d.submits === 1").await?;
    Ok(
        json!({"ax":ax,"before":before,"preedit":pending,"capture":painted,
        "after_cancel":restored,"used_cdp_cancel":used_cdp_cancel}),
    )
}

async fn outer_offset(tab: &Tab) -> Result<f64> {
    tab.eval(&format!(
        "{EXPORT}.dom.nodes.find(n => n.stable_id === {})?.style.scroll_y ?? 0",
        js(SCROLL)
    ))
    .await?
    .as_f64()
    .context("outer scroller offset")
}

pub(super) async fn wheel(tab: &Tab, body: Rect, dy: f64) -> Result<()> {
    let [left, top, ..] = tab.canvas_box().await?;
    let (x, y) = (left + body[0] + body[2] / 2., top + body[1] + body[3] / 2.);
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":x,"y":y}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseWheel","x":x,"y":y,"deltaX":0.,"deltaY":dy}),
    )
    .await?;
    tab.settle().await
}

fn changed(a: &Raster, p: Rect, b: &Raster, q: Rect) -> Result<usize> {
    check!(
        near(p[2], q[2]) && near(p[3], q[3]),
        "pixel comparison uses the same inner viewport"
    );
    let mut count = 0;
    for y in (2..p[3] as usize - 2).step_by(2) {
        for x in (2..p[2] as usize - 2).step_by(2) {
            let (x, y) = (x as f64 + 0.5, y as f64 + 0.5);
            count += usize::from(a.sample(p[0] + x, p[1] + y)? != b.sample(q[0] + x, q[1] + y)?);
        }
    }
    Ok(count)
}

async fn scroll(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    route(tab, url).await?;
    let id = part(FIXED, "value-box");
    let p = bounds(tab, &id).await?;
    let initial = outer_offset(tab).await?;
    let dpr = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("Canvas device scale")?;
    check_eq!(dpr, 2., "the wheel replay uses the declared device scale");
    // Winit exposes physical pixel deltas. Convert the logical distances
    // to the CDP units observed by the canvas.
    let [first, crossing, reverse] = [48. * dpr, 160. * dpr, -208. * dpr];
    let (before_png, before) = photo(tab, env, "wheel-before").await?;
    wheel(tab, p, first).await?;
    let mid = outer_offset(tab).await?;
    let (inner_png, inner) = photo(tab, env, "wheel-inner").await?;
    let inner_changes = changed(&before, p, &inner, p)?;
    let overlap = p[3] - 48.;
    let first_translation = changed(
        &before,
        [p[0], p[1] + 48., p[2], overlap],
        &inner,
        [p[0], p[1], p[2], overlap],
    )?;
    check!(
        near(initial, mid) && inner_changes > 15 && first_translation <= 2,
        "trusted wheel translates just the inner pixels by 48 dp: {initial},{mid},{inner_changes},{first_translation}"
    );

    wheel(tab, p, crossing).await?;
    let after_outer = outer_offset(tab).await?;
    let after_rect = bounds(tab, &id).await?;
    let (residual_png, residual) = photo(tab, env, "wheel-residual").await?;
    let residual_changes = changed(&inner, p, &residual, after_rect)?;
    check!(near(after_outer - mid, 64.) && near(p[1] - after_rect[1],after_outer - initial)
        && residual_changes > 15,
        "one wheel moves inner pixels and passes its residual to the parent: {after_outer}, {residual_changes}");

    wheel(tab, after_rect, reverse).await?;
    let restored = outer_offset(tab).await?;
    let end = bounds(tab, &id).await?;
    let (restored_png, restored_frame) = photo(tab, env, "wheel-restored").await?;
    let restore_changes = changed(&before, p, &restored_frame, end)?;
    check!(
        near(initial, restored) && near(p[1], end[1]) && restore_changes <= 2,
        "reverse wheel restores parent and inner viewport pixels: {restored}, {restore_changes}"
    );
    let trace = tab
        .eval("window.multilineTrusted.filter(e => e.kind === 'wheel')")
        .await?;
    check_eq!(
        trace
            .as_array()
            .context("trusted wheel receipts")?
            .iter()
            .map(|e| e["dy"].as_f64())
            .collect::<Vec<_>>(),
        vec![Some(48.), Some(160.), Some(-208.)],
        "trusted DOM wheel deltas are the exact logical distances"
    );
    let unchanged = model(
        tab,
        &format!(
            "d.fixed.value === {} && d.fixed.edits === 0 && d.grow.edits === 0 && d.submits === 0",
            js(FIXED_TEXT)
        ),
    )
    .await?;
    Ok(
        json!({"canvas":canvas(WIDE),"body":p,"body_after_residual":after_rect,"restored_body":end,
        "scroll_offsets":[initial,mid,after_outer,restored],"changed":[inner_changes,residual_changes,restore_changes],
        "copied_overlap_changes":first_translation,"dpr":dpr,
        "cdp_deltas":[first,crossing,reverse],
        "trusted":trace,"unchanged":unchanged,"captures":[before_png,inner_png,residual_png,restored_png]}),
    )
}

async fn compact(tab: &Tab, env: &Env) -> Result<Value> {
    for control in [THEME, RTL, LARGE] {
        activate(tab, control).await?;
    }
    resize(tab, COMPACT.0, COMPACT.1).await?;
    model(
        tab,
        "d.width === 320 && d.rtl && d.dark && d.text_scale === 2",
    )
    .await?;
    host_edit(tab, FIXED).await?;
    let fixed_ax = ax_editor(tab, FIXED, "詳細說明", FIXED_HELP).await?;
    textarea(tab, FIXED_TEXT).await?;
    let dark_outline = profile(tab, env, "compact-outlined", COMPACT, 2., false).await?;
    activate(tab, APPEARANCE).await?;
    model(tab, "d.appearance === 'filled'").await?;
    host_edit(tab, FIXED).await?;
    let dark_filled = profile(tab, env, "compact-filled", COMPACT, 2., true).await?;
    let grow = bounds(tab, GROW).await?;
    let clear = bounds(tab, ACTION).await?;
    let body = bounds(tab, &part(GROW, "value-box")).await?;
    check!(
        near(clear[2], 48.)
            && near(clear[3], 48.)
            && right(clear) <= grow[0] + 0.5
            && body[0] >= grow[0]
            && right(body) <= right(grow) + 0.5,
        "RTL places the independent trailing button at the left edge"
    );
    let locked = tab
        .eval(&format!(
            r#"(() => {{
        const data = {EXPORT};
        const disabled = data.dom.nodes.find(n => n.stable_id === {});
        return {{read_only_value:data.text_inputs[{}]?.value, disabled:disabled?.disabled,
            disabled_value:disabled?.text}};
    }})()"#,
            js(DISABLED),
            js(READ_ONLY)
        ))
        .await?;
    check!(
        locked["disabled"] == true
            && locked["disabled_value"]
                .as_str()
                .is_some_and(|s| s.contains('\n'))
            && locked["read_only_value"]
                .as_str()
                .is_some_and(|s| s.ends_with('\n')),
        "disabled and read-only preserve their authored line breaks"
    );
    activate(tab, BACK).await?;
    model(tab, "!d.visible").await?;
    until(tab, &format!("s.focused === {}", js(SHOW))).await?;
    Ok(
        json!({"outlined":dark_outline,"filled":dark_filled,"fixed_ax":fixed_ax,
        "readonly_disabled":locked,"grow_body":body,"clear_action":clear}),
    )
}

async fn page_keys(tab: &Tab, env: &Env) -> Result<Value> {
    host_edit(tab, FIXED).await?;
    tab.shortcut("a").await?;
    tab.press("ArrowLeft").await?;
    let input = input(FIXED);
    tab.wait_for(&format!(
        "{input}.selection_anchor === 0 && {input}.selection_cursor === 0"
    ))
    .await?;
    let start = selection(tab, FIXED).await?;
    let fourth = FIXED_TEXT.find("第四行").context("authored fourth row")?;
    tab.press("PageDown").await?;
    tab.wait_for(&format!(
        "{input}.selection_anchor === {fourth} && {input}.selection_cursor === {fourth}"
    ))
    .await?;
    let down = selection(tab, FIXED).await?;
    let fourth_utf16 = FIXED_TEXT[..fourth].encode_utf16().count();
    check_eq!(
        down["web"][0],
        json!(fourth_utf16),
        "PageDown updates the real textarea offset"
    );
    let painted = profile(tab, env, "page-down", WIDE, 1., false).await?;
    tab.press("Shift+PageUp").await?;
    tab.wait_for(&format!(
        "{input}.selection_anchor === {fourth} && {input}.selection_cursor === 0"
    ))
    .await?;
    let backward = selection(tab, FIXED).await?;
    check_eq!(
        backward["web"],
        json!([0, fourth_utf16, "backward"]),
        "Shift+PageUp extends the range backward"
    );
    tab.press("PageUp").await?;
    tab.wait_for(&format!(
        "{input}.selection_anchor === 0 && {input}.selection_cursor === 0"
    ))
    .await?;
    let up = selection(tab, FIXED).await?;
    check_eq!(
        up["rust"],
        start["rust"],
        "PageUp returns the retained editor to its first row"
    );
    let trace = tab
        .eval("window.multilineTrusted.filter(e => ['PageUp','PageDown'].includes(e.key))")
        .await?;
    check_eq!(
        trace
            .as_array()
            .context("trusted paging keys")?
            .iter()
            .filter(|e| e["id"] == "surface-text-input")
            .count(),
        3,
        "three trusted paging key events reached the real textarea"
    );
    Ok(
        json!({"start":start,"down":down,"backward":backward,"up":up,"paint":painted,"trusted":trace}),
    )
}

async fn touch(tab: &Tab, kind: &str, point: Option<[f64; 2]>) -> Result<()> {
    let points = point.map_or_else(|| json!([]), |[x, y]| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn touch_loss(tab: &Tab, env: &Env) -> Result<Value> {
    tab.eval(
        r#"(() => {
        const canvas = document.getElementById('surface');
        window.multilineTouch = [];
        for (const kind of ['pointerdown','pointermove','pointerup','pointercancel',
            'gotpointercapture','lostpointercapture']) {
            canvas.addEventListener(kind, e => {
                if(e.pointerType === 'touch') window.multilineTouch.push({
                    kind, id:e.pointerId, trusted:e.isTrusted,
                    capture:canvas.hasPointerCapture(e.pointerId), y:e.clientY,
                });
            }, true);
        }
    })()"#,
    )
    .await?;
    let id = part(FIXED, "value-box");
    let body = bounds(tab, &id).await?;
    let start_outer = outer_offset(tab).await?;
    let start_selection = selection(tab, FIXED).await?;
    let (before_png, before) = photo(tab, env, "touch-before").await?;
    let [left, top, ..] = tab.canvas_box().await?;
    let [x, y] = [left + body[0] + body[2] / 2., top + body[1] + body[3] / 2.];
    touch(tab, "touchStart", Some([x, y])).await?;
    touch(tab, "touchMove", Some([x, y - 40.])).await?;
    tab.wait_for(
        "window.multilineTouch.some(e => e.kind === 'gotpointercapture' && e.trusted && e.capture)",
    )
    .await?;
    let owner = tab.eval("window.multilineTouch.find(e => e.kind === 'gotpointercapture' && e.trusted && e.capture).id")
        .await?.as_i64().context("real touch capture owner")?;
    check!(
        near(start_outer, outer_offset(tab).await?),
        "early pan scrolls only the inner editor"
    );

    touch(tab, "touchMove", Some([x, y - 200.])).await?;
    let held_outer = outer_offset(tab).await?;
    let held_body = bounds(tab, &id).await?;
    let (held_png, held) = photo(tab, env, "touch-held").await?;
    let held_changes = changed(&before, body, &held, held_body)?;
    check!(near(held_outer - start_outer,56.) && held_changes > 15,
        "one real touch pans the editor and gives the remaining 56 dp to its parent: {held_outer}, {held_changes}");

    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    touch(tab, "touchMove", Some([x, y - 208.])).await?;
    tab.wait_for(&format!(
        "window.multilineTouch.some(e => e.id === {owner} && e.kind === 'lostpointercapture' && e.trusted)"
    )).await?;
    touch(tab, "touchEnd", None).await?;
    let actual = outer_offset(tab).await?;
    let restored = bounds(tab, &id).await?;
    let (restored_png, restored_img) = photo(tab, env, "touch-restored").await?;
    let restore_changes = changed(&before, body, &restored_img, restored)?;
    check!(near(actual,start_outer) && near(body[1],restored[1]) && restore_changes <= 2,
        "actual capture loss restores both offsets and original inner pixels: {actual}, {restore_changes}");
    check_eq!(
        selection(tab, FIXED).await?["rust"],
        start_selection["rust"],
        "a cancelled pan preserves the fixed editor selection"
    );
    model(
        tab,
        "d.grow.edits === 0 && d.fixed.edits === 0 && d.submits === 0",
    )
    .await?;
    let trace = tab.eval("window.multilineTouch").await?;
    let mut events = trace.as_array().context("trusted touch trace")?.iter();
    check!(
        [
            "pointerdown",
            "gotpointercapture",
            "pointermove",
            "lostpointercapture"
        ]
        .into_iter()
        .all(|kind| events.any(|e| e["trusted"] == true && e["id"] == owner && e["kind"] == kind)),
        "Chrome recorded touch contact and real capture loss in order: {trace}"
    );
    Ok(
        json!({"owner":owner,"trace":trace,"outer":[start_outer,held_outer,actual],
        "body":body,"held_body":held_body,"restored_body":restored,"changed":[held_changes,restore_changes],
        "captures":[before_png,held_png,restored_png]}),
    )
}

async fn locked_ax(tab: &Tab) -> Result<Value> {
    let scroll = bounds(tab, SCROLL).await?;
    // Use the outer padding, outside either editable body.
    wheel(tab, [scroll[0], scroll[1], 8., scroll[3]], 720.).await?;
    let tree = ax_tree(tab).await?;
    let readonly = ax(&tree, "textbox", "唯讀內容")?;
    let disabled = ax(&tree, "textbox", "停用內容")?;
    for node in [readonly, disabled] {
        check!(
            ax_property(node, "multiline") == Some(&json!(true)),
            "the locked proxy is multiline"
        );
    }
    check!(
        ax_property(readonly, "readonly") == Some(&json!(true))
            && ax_property(disabled, "disabled") == Some(&json!(true)),
        "Chrome AX exposes the locked states"
    );
    tab.press("Tab").await?;
    until(tab, &format!("s.focused === {}", js(SUBMIT))).await?;
    tab.press("Tab").await?;
    focus(tab, READ_ONLY, "surface-text-input").await?;
    let active = ax_editor(tab, READ_ONLY, "唯讀內容", READ_ONLY_HELP).await?;
    textarea(tab, READ_ONLY_TEXT).await?;
    check_eq!(
        tab.eval(&format!("{TEXTAREA}.readOnly")).await?,
        true,
        "focused textarea is read-only"
    );
    tab.press("Enter").await?;
    textarea(tab, READ_ONLY_TEXT).await?;
    model(tab, "d.locked_edit_requests === 0").await?;
    tab.press("Tab").await?;
    until(tab, &format!("s.focused === {}", js(BACK))).await?;
    Ok(json!({"readonly_proxy":readonly,"disabled_proxy":disabled,
        "active":active,"readonly_value":READ_ONLY_TEXT,"skipped_disabled":true}))
}

async fn inactive_value(
    tab: &Tab,
    id: &str,
    label: &str,
    value: &str,
    multiline: bool,
) -> Result<Value> {
    tab.wait_for(&format!("{} !== null", projected(id))).await?;
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "textbox", label)?;
    check_eq!(
        node["value"]["value"],
        value,
        "{label} exposes its committed value"
    );
    check_eq!(
        tree.iter()
            .filter(|node| node["ignored"] == false
                && node["role"]["value"] == "textbox"
                && node["name"]["value"] == label)
            .count(),
        1,
        "{label} has one accessible textbox"
    );
    check_eq!(
        ax_property(node, "multiline"),
        Some(&json!(multiline)),
        "{label} retains its line mode"
    );
    check!(
        !tree.iter().any(|child| child["parentId"] == node["nodeId"]
            && child["ignored"] == false
            && child["role"]["value"] == "StaticText"),
        "{label} has no separate readable value child"
    );
    Ok(node.clone())
}

async fn textbox_values(tab: &Tab) -> Result<Value> {
    const UPDATED: &str = "第一行\n修訂 a\u{301}";
    let before = inactive_value(tab, GROW, "多行備註", INITIAL, true).await?;
    let fixed = inactive_value(tab, FIXED, "詳細說明", FIXED_TEXT, true).await?;
    tab.eval(&format!("{}.focus()", projected(GROW))).await?;
    focus(tab, GROW, "surface-text-input").await?;
    let active = ax_editor(tab, GROW, "多行備註", GROW_HELP).await?;
    textarea(tab, INITIAL).await?;
    tab.shortcut("a").await?;
    tab.insert_text(UPDATED).await?;
    textarea(tab, UPDATED).await?;
    model(
        tab,
        &format!("d.grow.value === {} && d.submits === 0", js(UPDATED)),
    )
    .await?;
    tab.press("Tab").await?;
    until(tab, &format!("s.focused === {}", js(ACTION))).await?;
    let after = inactive_value(tab, GROW, "多行備註", UPDATED, true).await?;
    for (node, count) in [(&before, 3), (&after, 8)] {
        let description = node["description"]["value"]
            .as_str()
            .context("inactive AX description")?;
        check!(
            description.contains("井字號")
                && description.contains(GROW_HELP)
                && description.contains(&format!("已輸入 {count} 個字")),
            "the inactive description includes the affix, help and current count"
        );
    }

    let scroll = bounds(tab, SCROLL).await?;
    wheel(tab, [scroll[0], scroll[1], 8., scroll[3]], 720.).await?;
    let readonly = inactive_value(tab, READ_ONLY, "唯讀內容", READ_ONLY_TEXT, true).await?;
    let disabled = inactive_value(tab, DISABLED, "停用內容", DISABLED_TEXT, true).await?;
    check!(
        ax_property(&readonly, "readonly") == Some(&json!(true))
            && ax_property(&disabled, "disabled") == Some(&json!(true)),
        "inactive locked boxes keep their accessibility states"
    );
    Ok(json!({"before":before,"fixed":fixed,"active":active,
        "after":after,"readonly":readonly,"disabled":disabled}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(WIDE.0, WIDE.1 + 44, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(
            STEPS[0],
            "Outlined Grow expands with trusted Enter; application submit is separate",
        );
        route(&tab, url).await?;
        let adapter = tab
            .eval(
                r#"(async () => {
            const {info} = await navigator.gpu.requestAdapter();
            const {vendor,architecture,isFallbackAdapter} = info;
            return {vendor,architecture,isFallbackAdapter};
        })()"#,
            )
            .await?;
        check!(
            adapter["vendor"].is_string()
                && adapter["architecture"].is_string()
                && adapter["isFallbackAdapter"].is_boolean(),
            "real Chrome WebGPU adapter: {adapter}"
        );
        report["adapter"] = adapter;
        report["outlined"] = outlined(&tab, env).await?;
        step(
            STEPS[1],
            "Filled multiline selection and preedit stay in one named editor",
        );
        report["filled_ime"] = filled_ime(&tab, env).await?;
        step(
            STEPS[2],
            "trusted wheel consumes the fixed editor first and passes only the residual",
        );
        report["wheel"] = scroll(&tab, env, url).await?;
        step(
            STEPS[3],
            "compact dark RTL and double text retain both appearances and independent controls",
        );
        report["compact"] = compact(&tab, env).await?;
        step(
            STEPS[4],
            "trusted viewport paging, touch capture loss and locked textarea accessibility",
        );
        route(&tab, url).await?;
        report["page_keys"] = page_keys(&tab, env).await?;
        report["touch"] = touch_loss(&tab, env).await?;
        report["locked_ax"] = locked_ax(&tab).await?;
        step(
            STEPS[5],
            "Inactive textboxes expose their committed text in Chrome",
        );
        route(&tab, url).await?;
        report["inactive_ax"] = textbox_values(&tab).await?;
        super::material_text_field_counter::route(&tab, url).await?;
        report["inactive_ax"]["single_line"] = inactive_value(
            &tab,
            "text-field-counter-demo-editable",
            "標籤名稱",
            "中a\u{301}👍🏽",
            false,
        )
        .await?;
        check!(tab.errors().is_empty(), "no multiline page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact("material-multiline-text-field-failure.png"))
            .await;
    }
    env.write_json(REPORT, &report)?;
    result
}
