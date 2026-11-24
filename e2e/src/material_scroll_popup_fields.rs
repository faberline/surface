//! Anchored modeless quantity and unit Scroll Fields in Chrome and WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_list_corners::reduced_motion,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "scroll-popup-demo-show";
const OPENER: &str = "scroll-popup-demo-opener";
const OUTSIDE: &str = "scroll-popup-demo-outside";
const HIDE: &str = "scroll-popup-demo-hide-opener";
const BACK: &str = "scroll-popup-demo-back";
const DIALOG: &str = "scroll-popup-demo-dialog";
const BODY: &str = "scroll-popup-demo-body";
const CLOSE: &str = "scroll-popup-demo-close";
const PAIR: &str = "scroll-popup-demo-pair";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const CANVAS_FOCUS: &str = "document.activeElement === document.getElementById('surface')";

#[derive(Clone, Copy)]
struct Field {
    id: &'static str,
    label: &'static str,
}

const QUANTITY: Field = Field {
    id: "scroll-popup-demo-quantity",
    label: "數量",
};
const UNIT: Field = Field {
    id: "scroll-popup-demo-unit",
    label: "單位",
};

type Rect = [f64; 4];
type Position = [f64; 2];

impl Field {
    fn part(self, suffix: &str) -> String {
        format!("{}-{suffix}", self.id)
    }

    async fn editor_focus(self, tab: &Tab) -> Result<()> {
        runtime_focus(tab, &self.part("editor")).await?;
        tab.wait_for(&format!(
            "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
        ))
        .await?;
        tab.settle().await
    }

    async fn edit(self, tab: &Tab) -> Result<()> {
        activate(tab, &self.part("adjustable")).await?;
        self.editor_focus(tab).await
    }

    async fn error(self, tab: &Tab, message: &str) -> Result<Value> {
        self.editor_focus(tab).await?;
        let helper = projected(&self.part("error"));
        let result = tab
            .eval(&format!(
                r#"(() => {{
            const input = {TEXTAREA};
            const helper = {helper};
            const refs = input?.getAttribute('aria-describedby')?.split(/\s+/) ?? [];
            return {{invalid:input?.getAttribute('aria-invalid'), text:helper?.textContent,
                linked:refs.some(id => document.getElementById(id) === helper)}};
        }})()"#
            ))
            .await?;
        check_eq!(
            result,
            json!({"invalid":"true","linked":true,"text":message}),
            "the active editor names its own visible validation message"
        );
        let tree = ax_tree(tab).await?;
        let input = ax(&tree, "textbox", self.label)?;
        let description = &input["description"]["value"];
        check!(
            description
                .as_str()
                .is_some_and(|value| value.contains(message))
                && matches!(ax_property(input,"invalid"),Some(value) if value == &json!(true) || value == &json!("true")),
            "actual Chrome accessibility describes the named invalid textbox"
        );
        Ok(json!({"dom":result,"description":description}))
    }
}

async fn read(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["scroll_popup"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.scroll_popup)")).await?;
    tab.settle().await
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn preset(tab: &Tab, suffix: &str) -> Result<()> {
    activate(tab, &format!("scroll-popup-demo-{suffix}")).await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-0").await?;
    until(tab, "s.section === 0").await?;
    activate(tab, SHOW).await?;
    ready(
        tab,
        "d.visible && !d.open && d.quantity.value === 1200 && d.unit.id === 'ml'",
    )
    .await?;
    resize(tab, 960, 820).await?;
    ready(tab, "d.visible && !d.open && d.anchor !== null").await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-scroll-popup-fields-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn replace(tab: &Tab, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    tab.settle().await
}

fn right([x, _, w, _]: Rect) -> f64 {
    x + w
}
fn bottom([_, y, _, h]: Rect) -> f64 {
    y + h
}

fn intersection(a: Rect, b: Rect) -> Rect {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    [
        x,
        y,
        right(a).min(right(b)) - x,
        bottom(a).min(bottom(b)) - y,
    ]
}

async fn viewport(tab: &Tab) -> Result<Rect> {
    let [_, _, w, h] = tab.canvas_box().await?;
    Ok([0., 0., w, h])
}

async fn positioned(tab: &Tab, above: bool) -> Result<(Rect, Rect, f64)> {
    ready(tab, &format!("d.open && d.above === {above}")).await?;
    let popup = bounds(tab, DIALOG).await?;
    let opener = bounds(tab, OPENER).await?;
    let model = read(tab).await?;
    let modeled_anchor: Rect = serde_json::from_value(model["anchor"].clone())?;
    let modeled_popup: Rect = serde_json::from_value(model["bounds"].clone())?;
    check!(
        modeled_anchor
            .into_iter()
            .zip(opener)
            .all(|(a, b)| near(a, b))
            && modeled_popup
                .into_iter()
                .zip(popup)
                .all(|(a, b)| near(a, b)),
        "the popup plan uses the measured, currently rendered anchor and panel"
    );
    let gap = if above {
        opener[1] - bottom(popup)
    } else {
        popup[1] - bottom(opener)
    };
    check!(contains(viewport(tab).await?, popup) && popup[2] > 0. && popup[3] > 0.
        && near(gap,4.),
        "the real dialog sits on the requested side of the opener and stays in the viewport: {popup:?}/{opener:?}");
    Ok((popup, opener, gap))
}

async fn closed(tab: &Tab) -> Result<()> {
    ready(tab, "!d.open").await?;
    tab.wait_for(&format!(
        "{} === null && {} === null",
        projected(DIALOG),
        projected(CLOSE)
    ))
    .await
}

async fn named_popup(tab: &Tab) -> Result<Value> {
    let dom = tab.eval(&format!(r#"(() => {{
        const dialog = {};
        const pair = {};
        return {{role:dialog?.getAttribute('role'), name:dialog?.getAttribute('aria-label'),
            modal:dialog?.getAttribute('aria-modal'),
            options: Array.from(pair?.querySelectorAll('[role="spinbutton"]') ?? [], el => el.getAttribute('aria-label')),
            outside:({})?.getAttribute('aria-label'), close:({})?.getAttribute('aria-label'),
        }};
    }})()"#,projected(DIALOG),projected(PAIR),projected(OUTSIDE),projected(CLOSE))).await?;
    check!(dom["name"] == "編輯容量" && dom["role"] == "dialog" && dom["modal"] != "true"
        && dom["outside"] == "外部 0 次" && dom["close"] == "關閉"
        && dom["options"] == json!(["數量","單位"]),
        "one named nonmodal dialog keeps its distinct controls and the outside action in the projected DOM: {dom}");
    let tree = ax_tree(tab).await?;
    let dialog = ax(&tree, "dialog", "編輯容量")?;
    let quantity = ax(&tree, "spinbutton", "數量")?;
    let unit = ax(&tree, "spinbutton", "單位")?;
    ax(&tree, "button", "外部 0 次")?;
    let modal = ax_property(dialog, "modal");
    check!(
        modal != Some(&json!(true))
            && quantity["value"]["value"] == 1200
            && unit["description"]["value"]
                .as_str()
                .is_some_and(|name| name.contains("毫升")),
        "actual Chrome accessibility exposes a modeless dialog with independent initial values"
    );
    Ok(
        json!({"dom":dom,"chrome_modal":modal,"quantity":quantity["value"]["value"],"unit_description":unit["description"]["value"]}),
    )
}

async fn expanded(tab: &Tab, expected: bool) -> Result<()> {
    let attributes = tab
        .eval(&format!(
            "[{}.getAttribute('aria-expanded'), {}.getAttribute('aria-haspopup')]",
            projected(OPENER),
            projected(OPENER)
        ))
        .await?;
    let expected_attr = if expected { "true" } else { "false" };
    check!(
        attributes[0] == expected_attr && attributes[1] != "menu",
        "the anchor exposes its open state with no menu semantics: {attributes}"
    );
    let tree = ax_tree(tab).await?;
    let anchor_name = tab
        .eval_str(&format!("{}.getAttribute('aria-label')", projected(OPENER)))
        .await?;
    let opener = ax(&tree, "button", &anchor_name)?;
    check_eq!(
        ax_property(opener, "expanded"),
        Some(&json!(expected)),
        "actual Chrome AX reports disclosure state"
    );
    Ok(())
}

async fn opening(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M229",
        "keyboard opens the named nonmodal popup below its anchor and enters the quantity field",
    );
    closed(tab).await?;
    expanded(tab, false).await?;
    activate(tab, OPENER).await?;
    let (popup, opener, gap) = positioned(tab, false).await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    check!(
        near(popup[0], opener[0]),
        "the wide LTR popup aligns to the logical start of its anchor"
    );
    expanded(tab, true).await?;
    let ax = named_popup(tab).await?;
    let image = capture(env, tab, "wide-keyboard").await?;
    check!(
        image.colors(popup)?.len() > 8,
        "the dialog paints real varied pixels in headless WebGPU"
    );
    let focus = QUANTITY.part("adjustable");
    Ok(
        json!({"dialog":popup,"anchor":opener,"gap":gap,"side":"below","focus":focus,"expanded":[false,true],"ax":ax}),
    )
}

async fn outside_pressed(tab: &Tab) -> Result<&'static str> {
    const LABEL: &str = "外部 1 次";
    ready(tab, "d.open && d.outside_presses === 1").await?;
    runtime_focus(tab, OUTSIDE).await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "button", LABEL)?;
    Ok(LABEL)
}

async fn modeless(env: &Env, tab: &Tab) -> Result<Value> {
    step("M230", "pointer opening keeps the anchor focused, then Tab enters and an outside action stays live");
    click(tab, OPENER).await?;
    positioned(tab, false).await?;
    runtime_focus(tab, OPENER).await?;
    tab.wait_for(CANVAS_FOCUS).await?;
    tab.press("Tab").await?;
    until(tab, "s.focused === 'scroll-popup-demo-quantity-adjustable' || s.focused === 'scroll-popup-demo-close'").await?;
    let entered = state(tab).await?["focused"].clone();
    let inside = tab
        .eval(&format!(
            "{}.contains({})",
            projected(DIALOG),
            projected(entered.as_str().context("tab target")?)
        ))
        .await?;
    check_eq!(
        inside,
        json!(true),
        "the first Tab after pointer opening reaches the actual dialog subtree"
    );

    click(tab, OUTSIDE).await?;
    let outside_label = outside_pressed(tab).await?;
    tab.press("Escape").await?;
    ready(tab, "d.open && d.outside_presses === 1").await?;
    runtime_focus(tab, OUTSIDE).await?;
    capture(env, tab, "outside-focus").await?;

    activate(tab, CLOSE).await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    expanded(tab, false).await?;
    Ok(
        json!({"pointer_open_focus":OPENER,"tab_focus":entered,"outside_focus":OUTSIDE,"outside_presses":1,"outside_label":outside_label,
        "outside_escape_leaves_open":true,"close_focus":OPENER}),
    )
}

async fn edits(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M231",
        "independent field commits survive Close while both pending invalid drafts are discarded",
    );
    reduced_motion(tab, true).await?;
    activate(tab, OPENER).await?;
    positioned(tab, false).await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    tab.press("ArrowUp").await?;
    ready(tab, "d.open && d.quantity.value === 1202 && d.quantity.commits === 1 && d.unit.id === 'ml' && d.unit.commits === 0").await?;
    UNIT.edit(tab).await?;
    replace(tab, "Fl Oz").await?;
    ready(tab, "d.open && d.unit.mode === 'input' && d.unit.raw === 'Fl Oz' && d.unit.commits === 0 && d.quantity.value === 1202").await?;
    tab.press("Enter").await?;
    ready(tab, "d.open && d.unit.mode === 'scroll' && d.unit.id === 'fluid-ounce' && d.unit.commits === 1 && d.quantity.value === 1202 && d.quantity.commits === 1").await?;
    focused(tab, &UNIT.part("adjustable")).await?;

    QUANTITY.edit(tab).await?;
    replace(tab, "1203").await?;
    tab.press("Enter").await?;
    ready(tab, "d.open && d.quantity.mode === 'input' && d.quantity.error === 'OffStep' && d.quantity.raw === '1203' && d.quantity.value === 1202 && d.quantity.commits === 1 && d.unit.id === 'fluid-ounce' && !d.unit.error").await?;
    let amount_error = QUANTITY.error(tab, "請輸入偶數").await?;
    UNIT.edit(tab).await?;
    replace(tab, "無").await?;
    tab.press("Enter").await?;
    ready(tab, "d.open && d.unit.mode === 'input' && d.unit.error === 'NoMatch' && d.unit.raw === '無' && d.unit.id === 'fluid-ounce' && d.unit.commits === 1 && d.quantity.raw === '1203' && d.quantity.value === 1202").await?;
    let unit_error = UNIT.error(tab, "找不到單位").await?;
    capture(env, tab, "both-drafts").await?;

    click(tab, CLOSE).await?;
    closed(tab).await?;
    runtime_focus(tab, OPENER).await?;
    let kept = "d.quantity.value === 1202 && d.quantity.raw === '1202' && !d.quantity.error && d.quantity.commits === 1 && d.unit.id === 'fluid-ounce' && d.unit.raw === '液量盎司' && !d.unit.error && d.unit.commits === 1";
    ready(tab, kept).await?;
    activate(tab, OPENER).await?;
    ready(
        tab,
        &format!("d.open && d.quantity.mode === 'scroll' && d.unit.mode === 'scroll' && ({kept})"),
    )
    .await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    capture(env, tab, "accepted-after-reopen").await?;
    Ok(
        json!({"quantity":1202,"unit":"fluid-ounce","alias":"Fl Oz","commits":[1,1],
        "rejected":["1203","無"],"errors":[amount_error,unit_error],"discarded_both":true,"restored_focus":OPENER}),
    )
}

async fn escape(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M232",
        "Escape gives active composition and an editor draft priority before dismissing the dialog",
    );
    activate(tab, OPENER).await?;
    positioned(tab, false).await?;
    QUANTITY.edit(tab).await?;
    let editor = QUANTITY.part("editor");
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(editor)
    );
    tab.shortcut("a").await?;
    tab.ime_compose("１２", 2, 2).await?;
    tab.wait_for(&composing).await?;
    let old = "d.open && d.quantity.mode === 'input' && d.quantity.raw === '1200' && d.quantity.value === 1200 && d.quantity.commits === 0 && d.unit.id === 'ml' && d.unit.commits === 0";
    ready(tab, old).await?;
    capture(env, tab, "preedit-before-escape").await?;
    tab.press("Escape").await?;
    ready(tab, old).await?;
    QUANTITY.editor_focus(tab).await?;
    if tab.eval(&composing).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    replace(tab, "1206").await?;
    ready(
        tab,
        "d.open && d.quantity.raw === '1206' && d.quantity.commits === 0",
    )
    .await?;
    tab.press("Escape").await?;
    ready(tab,"d.open && d.quantity.mode === 'scroll' && d.quantity.raw === '1200' && d.quantity.value === 1200 && d.quantity.commits === 0").await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    tab.press("Escape").await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    activate(tab, OPENER).await?;
    focus_projected(tab, CLOSE).await?;
    tab.press("Escape").await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    Ok(
        json!({"preedit_escape_preserved_popup":true,"draft_escape_preserved_popup":true,"discarded":"1206",
        "escapes_from":[QUANTITY.part("adjustable"),CLOSE],"restored_focus":OPENER,"commits":[0,0]}),
    )
}

async fn anchor(env: &Env, tab: &Tab) -> Result<Value> {
    step("M233", "bottom anchoring flips above, RTL aligns, and anchor removal closes without stealing outside focus");
    preset(tab, "bottom").await?;
    ready(tab, "d.near_bottom && !d.open").await?;
    activate(tab, OPENER).await?;
    let (ltr, ltr_anchor, ltr_gap) = positioned(tab, true).await?;
    check!(
        near(ltr[0], ltr_anchor[0]),
        "the LTR flipped panel aligns its left edge to the anchor"
    );
    focus_projected(tab, CLOSE).await?;
    tab.press("Escape").await?;
    closed(tab).await?;
    preset(tab, "rtl").await?;
    ready(tab, "d.rtl && d.near_bottom").await?;
    activate(tab, OPENER).await?;
    let (rtl, rtl_anchor, rtl_gap) = positioned(tab, true).await?;
    check!(
        near(right(rtl), right(rtl_anchor)),
        "the RTL flipped panel aligns its right edge to the anchor"
    );
    capture(env, tab, "flipped-rtl").await?;
    focus_projected(tab, CLOSE).await?;
    tab.press("Escape").await?;
    closed(tab).await?;
    preset(tab, "bottom").await?;
    activate(tab, OPENER).await?;
    positioned(tab, false).await?;
    focus_projected(tab, HIDE).await?;
    tab.press("Enter").await?;
    ready(tab, "d.hide_opener && !d.open").await?;
    closed(tab).await?;
    focused(tab, HIDE).await?;
    tab.wait_for(&format!("{} === null", projected(OPENER)))
        .await?;
    tab.press("Enter").await?;
    ready(tab, "!d.hide_opener && !d.open && d.anchor !== null").await?;
    closed(tab).await?;
    expanded(tab, false).await?;
    activate(tab, OPENER).await?;
    positioned(tab, false).await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    ready(tab,"d.quantity.value === 1200 && d.quantity.commits === 0 && d.unit.id === 'ml' && d.unit.commits === 0").await?;

    resize(tab, 960, 2000).await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible && !d.open").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"ltr":{"popup":ltr,"anchor":ltr_anchor,"gap":ltr_gap},"rtl":{"popup":rtl,"anchor":rtl_anchor,"gap":rtl_gap},
        "closed_without_anchor":true,"retained_outside_focus":HIDE,"reopened_clean":true,"back_focus":SHOW}),
    )
}

async fn body_scroll(tab: &Tab) -> Result<f64> {
    read(tab).await?["body_scroll_top"]
        .as_f64()
        .context("popup body scroll offset")
}

async fn visible_editor(tab: &Tab, field: Field, body: Rect) -> Result<(Rect, Rect)> {
    field.editor_focus(tab).await?;
    let editor = bounds(tab, &field.part("editor")).await?;
    let ring = bounds(tab, &field.part("focus")).await?;
    let view = viewport(tab).await?;
    check!(editor[2] >= 48. && editor[3] >= 48. && contains(ring,editor)
        && contains(body,ring) && contains(view,ring),
        "the full {} editor and its outer focus ring fit inside the short scroll clip: {editor:?}/{ring:?}/{body:?}",field.label);
    Ok((editor, ring))
}

fn ring_ink(image: &Raster, [x, y, w, h]: Rect) -> Result<[[u8; 3]; 2]> {
    let x = x + w / 2.;
    let top = image.sample(x, y + 1.5)?;
    let bottom = image.sample(x, y + h - 1.5)?;
    let gap = image.sample(x, y + 4.)?;
    check!(top == bottom && top != gap,
        "both sides of the actual exterior focus stroke paint, with a visible gap: {top:?}/{bottom:?}/{gap:?}");
    Ok([top, bottom])
}

async fn covered_setting(tab: &Tab, dialog: Rect, close: Rect) -> Result<(&'static str, Position)> {
    for id in ["scroll-popup-demo-theme", "scroll-popup-demo-large-text"] {
        let area = intersection(dialog, bounds(tab, id).await?);
        if area[2] < 8. || area[3] < 8. {
            continue;
        }
        for [u, v] in [[0.5, 0.5], [0.25, 0.25], [0.75, 0.75]] {
            let [x, y] = [area[0] + u * area[2], area[1] + v * area[3]];
            if !contains(close, [x, y, 0., 0.]) {
                return Ok((id, [x, y]));
            }
        }
    }
    anyhow::bail!("no underlying theme or scale setting overlaps the real compact panel")
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step("M234", "dark RTL at 200 percent scrolls distinct editors, shields covered controls, and leaves the outside action reachable");
    for option in ["theme", "rtl", "large-text", "bottom"] {
        preset(tab, option).await?;
    }
    resize(tab, 320, 400).await?;
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2 && d.near_bottom").await?;
    activate(tab, OPENER).await?;
    let (dialog, anchor, gap) = positioned(tab, true).await?;
    ready(
        tab,
        "d.open && d.quantity.mode === 'input' && d.unit.mode === 'input'",
    )
    .await?;
    let body = bounds(tab, BODY).await?;
    let close = bounds(tab, CLOSE).await?;
    let outside = bounds(tab, OUTSIDE).await?;
    check!(contains(dialog,body) && contains(dialog,close) && close[2] >= 48. && close[3] >= 48.,
        "the compact dialog has a bounded body and a fully visible fixed Close: {dialog:?}/{body:?}/{close:?}");
    check!(contains(viewport(tab).await?,outside) && outside[2] >= 48. && outside[3] >= 48.
        && intersection(dialog,outside)[3] <= 0.,
        "the complete outside target stays visible beyond the compact popup: {outside:?}/{dialog:?}");

    let (quantity, quantity_ring) = visible_editor(tab, QUANTITY, body).await?;
    let start = body_scroll(tab).await?;
    tab.press("Tab").await?;
    let (unit, unit_ring) = visible_editor(tab, UNIT, body).await?;
    let down = body_scroll(tab).await?;
    let after = bounds(tab, CLOSE).await?;
    check!(
        down > start && after.into_iter().zip(close).all(|(a, b)| near(a, b)),
        "Tab scrolls the body from {start} to {down} while the complete Close stays fixed"
    );
    let image = capture(env, tab, "compact-dark-rtl").await?;
    let ink = ring_ink(&image, unit_ring)?;
    tab.press("Shift+Tab").await?;
    let (restored, restored_ring) = visible_editor(tab, QUANTITY, body).await?;
    let up = body_scroll(tab).await?;
    check!(
        up < down,
        "reverse Tab scrolls back to the complete quantity editor"
    );

    replace(tab, "1206").await?;
    ready(
        tab,
        "d.open && d.quantity.raw === '1206' && d.quantity.commits === 0",
    )
    .await?;
    tab.press("Escape").await?;
    ready(tab,"d.open && d.quantity.mode === 'input' && d.quantity.raw === '1200' && d.quantity.commits === 0 && d.unit.commits === 0").await?;
    QUANTITY.editor_focus(tab).await?;

    let (covered, [x, y]) = covered_setting(tab, dialog, close).await?;
    let [left, top, _, _] = tab.canvas_box().await?;
    tab.click(left + x, top + y).await?;
    ready(tab,"d.open && d.text_scale === 2 && d.outside_presses === 0 && d.quantity.commits === 0 && d.unit.commits === 0").await?;
    until(tab, "s.dark === true").await?;
    click(tab, OUTSIDE).await?;
    let outside_label = outside_pressed(tab).await?;
    click(tab, CLOSE).await?;
    closed(tab).await?;
    runtime_focus(tab, OPENER).await?;
    expanded(tab, false).await?;
    Ok(
        json!({"viewport":[320,400],"rtl":true,"text_scale":2,"dialog":dialog,"anchor":anchor,"gap":gap,
        "body":body,"close":close,"outside":outside,"outside_label":outside_label,"editors":[quantity,unit,restored],"rings":[quantity_ring,unit_ring,restored_ring],
        "unit_ring_ink":ink,"scroll_offsets":[start,down,up],"covered_setting":covered,"shielded_point":[x,y],
        "fallback_escape_discards_draft":true,"close_focus":OPENER,"shielded_outside_presses":0,"outside_presses":1,"commits":[0,0]}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let opening = opening(env, tab).await?;
    reload(tab, url).await?;
    let modeless = modeless(env, tab).await?;
    reload(tab, url).await?;
    let edits = edits(env, tab).await?;
    reload(tab, url).await?;
    let escape = escape(env, tab).await?;
    reload(tab, url).await?;
    let anchor = anchor(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "popup Scroll Field page errors: {errors:?}"
    );
    env.write_json("browser-material-scroll-popup-fields.json",&json!({
        "browser":chrome.version,"opening":opening,"modeless":modeless,"edits":edits,"escape":escape,
        "anchor":anchor,"compact":compact,"errors":errors,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-scroll-popup-fields-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-scroll-popup-fields-failure.png"))
            .await;
    }
    result.context("Material 3 anchored modeless Scroll Fields")
}
