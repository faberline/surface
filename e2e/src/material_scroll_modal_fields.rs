//! Transactional Scroll Fields in the real browser's modal scope.

#[path = "material_scroll_modal_fields_helpers.rs"]
mod helpers;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use self::helpers::{
    canvas, composing, editor_clipped_error, editor_invalid, full_footer, modal_scope,
    press_scrim_at_background, revealed_ring, ring_pixels, tab_sequence, unmounted, FocusTarget,
    Rect,
};
use super::{
    click,
    material_list_corners::reduced_motion,
    material_time_input::{
        activate, ax, ax_tree, bounds, contains, focused, near, projected, resize, Raster,
    },
    state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, Env};

const SHOW: &str = "scroll-modal-demo-show";
const BACK: &str = "scroll-modal-demo-back";
const OPENER: &str = "scroll-modal-demo-opener";
const OUTSIDE: &str = "scroll-modal-demo-outside";
const DIALOG: &str = "scroll-modal-demo-dialog";
const LAYER: &str = "scroll-modal-demo-layer";
const BODY: &str = "scroll-modal-demo-body";
const ACTIONS: &str = "scroll-modal-demo-actions";
const CANCEL: &str = "scroll-modal-demo-cancel";
const SAVE: &str = "scroll-modal-demo-save";
const ACCEPTED: &str = "scroll-modal-demo-saved";
const STATUS: &str = "scroll-modal-demo-status";
const CANVAS_FOCUS: &str = "document.activeElement === document.getElementById('surface')";

struct Field {
    base: &'static str,
    adjustable: &'static str,
    use_input: &'static str,
    use_scroll: &'static str,
    label: &'static str,
}

const QUANTITY: Field = Field {
    base: "scroll-modal-demo-quantity",
    adjustable: "scroll-modal-demo-quantity-adjustable",
    use_input: "scroll-modal-demo-quantity-use-input",
    use_scroll: "scroll-modal-demo-quantity-use-scroll",
    label: "數量",
};
const UNIT: Field = Field {
    base: "scroll-modal-demo-unit",
    adjustable: "scroll-modal-demo-unit-adjustable",
    use_input: "scroll-modal-demo-unit-use-input",
    use_scroll: "scroll-modal-demo-unit-use-scroll",
    label: "單位",
};

impl Field {
    fn wheel(&self) -> FocusTarget<'static> {
        FocusTarget::Projected(self.adjustable)
    }

    fn editor(&self) -> FocusTarget<'static> {
        FocusTarget::Editor(self.base)
    }

    async fn enter_editor(&self, tab: &Tab) -> Result<()> {
        activate(tab, self.adjustable).await?;
        self.editor().verify(tab).await
    }

    async fn type_text(&self, tab: &Tab, text: &str) -> Result<()> {
        self.editor().verify(tab).await?;
        tab.shortcut("a").await?;
        tab.insert_text(text).await?;
        tab.settle().await
    }

    async fn submit_editor(&self, tab: &Tab) -> Result<()> {
        tab.press("Enter").await?;
        self.wheel().verify(tab).await
    }

    async fn invalid(&self, tab: &Tab, message: &str) -> Result<Value> {
        editor_invalid(tab, self.base, self.label, message).await
    }
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.scroll_modal)")).await?;
    tab.settle().await
}

async fn inspect(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["scroll_modal"].take())
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    tab.settle().await
}

async fn pointer_focus(tab: &Tab, id: &str) -> Result<()> {
    runtime_focus(tab, id).await?;
    tab.wait_for(&format!(
        "{CANVAS_FOCUS} || document.activeElement === {}",
        projected(id)
    ))
    .await
}

async fn saved(tab: &Tab, quantity: i32, unit: &str, saves: u32) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.accepted.quantity === {quantity} && d.accepted.unit_id === {} && d.saves === {saves}",
            js(unit)
        ),
    )
    .await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-0").await?;
    until(tab, "s.section === 0").await?;
    activate(tab, SHOW).await?;
    ready(tab, "d.visible && !d.open && d.working === null").await?;
    saved(tab, 1200, "ml", 0).await?;
    resize(tab, 960, 820).await
}

async fn open(tab: &Tab, first: FocusTarget<'_>) -> Result<()> {
    activate(tab, OPENER).await?;
    ready(tab, "d.open && d.working !== null").await?;
    first.verify(tab).await
}

async fn closed(tab: &Tab) -> Result<()> {
    ready(tab, "!d.open && d.working === null").await?;
    unmounted(tab, DIALOG, LAYER, OPENER).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0 && {} !== null",
        js(LAYER),
        projected(OUTSIDE),
    ))
    .await
}

async fn seed(tab: &Tab, quantity: i32, unit: &str) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.open && d.working.quantity.value === {quantity} && d.working.unit.id === {} &&
             d.working.quantity.error === null && d.working.unit.error === null &&
             d.working.first_invalid_editor === null && d.working.status === null",
            js(unit),
        ),
    )
    .await
}

async fn saved_label(tab: &Tab, label: &str) -> Result<()> {
    tab.wait_for(&format!(
        "{}?.textContent === {}",
        projected(ACCEPTED),
        js(label)
    ))
    .await
}

async fn status(tab: &Tab, label: &str) -> Result<()> {
    let element = projected(STATUS);
    let dialog = projected(DIALOG);
    let expected = js(label);
    tab.wait_for(&format!(
        "{element}?.getAttribute('role') === 'status' &&
         {element}.textContent === {expected} &&
         {dialog}?.querySelector('[aria-live=polite]')?.textContent === {expected}"
    ))
    .await?;
    check!(
        contains(bounds(tab, BODY).await?, bounds(tab, STATUS).await?),
        "the failed Save reveals the full status from the current scroll position"
    );
    Ok(())
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-scroll-modal-fields-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn opening(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M235",
        "the named modal owns browser and Runtime focus and wraps every authored Tab stop",
    );
    ax(&ax_tree(tab).await?, "button", "外部 0 次")?;
    open(tab, QUANTITY.wheel()).await?;
    let scope = modal_scope(
        tab,
        DIALOG,
        "編輯容量",
        &[(OPENER, "開啟欄位"), (OUTSIDE, "外部 0 次")],
    )
    .await?;
    let ax_nodes = ax_tree(tab).await?;
    ax(&ax_nodes, "group", "容量欄位")?;
    ax(&ax_nodes, "button", "取消")?;
    ax(&ax_nodes, "button", "儲存")?;
    let quantity_value = ax(&ax_nodes, "spinbutton", QUANTITY.label)?["value"]["value"].clone();
    // A centered modal caps its width. At this cap the pair can stack,
    // so prove the unit's AX output after the real Tab reveal.
    let mut forward = tab_sequence(
        tab,
        "Tab",
        &[FocusTarget::Projected(QUANTITY.use_input), UNIT.wheel()],
    )
    .await?;
    let unit_tree = ax_tree(tab).await?;
    let unit = ax(&unit_tree, "spinbutton", UNIT.label)?;
    let unit_value = &unit["value"]["value"];
    let unit_help = &unit["description"]["value"];
    check!(
        quantity_value == 1200
            && (unit_value.as_str() == Some("毫升")
                || (unit_value.as_i64() == Some(1)
                    && unit_help.as_str().is_some_and(|help| help.contains("毫升")))),
        "the named Chrome fields have the independent quantity and localized unit"
    );

    forward.extend(
        tab_sequence(
            tab,
            "Tab",
            &[
                FocusTarget::Projected(UNIT.use_input),
                FocusTarget::Projected(CANCEL),
                FocusTarget::Projected(SAVE),
                QUANTITY.wheel(),
            ],
        )
        .await?,
    );
    let reverse = tab_sequence(
        tab,
        "Shift+Tab",
        &[
            FocusTarget::Projected(SAVE),
            FocusTarget::Projected(CANCEL),
            FocusTarget::Projected(UNIT.use_input),
            UNIT.wheel(),
            FocusTarget::Projected(QUANTITY.use_input),
            QUANTITY.wheel(),
        ],
    )
    .await?;
    let panel = bounds(tab, DIALOG).await?;
    let layer = bounds(tab, LAYER).await?;
    let viewport = canvas(tab).await?;
    check!(
        contains(layer, viewport) && contains(viewport, layer) && contains(viewport, panel),
        "the actual scrim covers the viewport and bounds the panel: {layer:?}/{panel:?}"
    );
    let image = capture(env, tab, "wide-open").await?;
    check!(
        image.colors(panel)?.len() > 8,
        "the named modal paints varied content in WebGPU"
    );

    tab.press("Escape").await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    click(tab, OPENER).await?;
    ready(tab, "d.open").await?;
    pointer_focus(tab, QUANTITY.adjustable).await?;
    tab.press("Tab").await?;
    pointer_focus(tab, QUANTITY.use_input).await?;
    Ok(json!({
        "scope":scope,"panel":panel,"layer":layer,"forward":forward,"reverse":reverse,
        "quantity":quantity_value,"unit":unit_value,"unit_help":unit_help,
        "keyboard_initial_focus":QUANTITY.adjustable,"pointer_initial_focus":QUANTITY.adjustable,
        "pointer_tab_focus":QUANTITY.use_input,
    }))
}

async fn stage_locally(tab: &Tab, quantity: &str, unit: &str) -> Result<()> {
    QUANTITY.enter_editor(tab).await?;
    QUANTITY.type_text(tab, quantity).await?;
    QUANTITY.submit_editor(tab).await?;
    tab_sequence(
        tab,
        "Tab",
        &[FocusTarget::Projected(QUANTITY.use_input), UNIT.wheel()],
    )
    .await?;
    UNIT.enter_editor(tab).await?;
    UNIT.type_text(tab, unit).await?;
    UNIT.submit_editor(tab).await
}

async fn stage_raw(tab: &Tab, quantity: &str, unit: &str) -> Result<()> {
    QUANTITY.enter_editor(tab).await?;
    QUANTITY.type_text(tab, quantity).await?;
    tab_sequence(
        tab,
        "Tab",
        &[FocusTarget::Projected(QUANTITY.use_scroll), UNIT.wheel()],
    )
    .await?;
    UNIT.enter_editor(tab).await?;
    UNIT.type_text(tab, unit).await?;
    ready(
        tab,
        &format!(
            "d.working.quantity.mode === 'input' && d.working.unit.mode === 'input' &&
             d.working.quantity.raw === {} && d.working.unit.raw === {}",
            js(quantity),
            js(unit),
        ),
    )
    .await
}

async fn atomic_save(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M236",
        "explicit Save writes both locally accepted or still unsubmitted editor values once",
    );
    saved_label(tab, "已儲存 1200 毫升").await?;
    open(tab, QUANTITY.wheel()).await?;
    stage_locally(tab, "1204", "L").await?;
    seed(tab, 1204, "liter").await?;
    saved(tab, 1200, "ml", 0).await?;
    activate(tab, SAVE).await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    saved(tab, 1204, "liter", 1).await?;
    saved_label(tab, "已儲存 1204 公升").await?;
    open(tab, QUANTITY.wheel()).await?;
    seed(tab, 1204, "liter").await?;

    reload(tab, url).await?;
    open(tab, QUANTITY.wheel()).await?;
    stage_raw(tab, "１２０６", "Fl Oz").await?;
    seed(tab, 1200, "ml").await?;
    saved(tab, 1200, "ml", 0).await?;
    click(tab, SAVE).await?;
    closed(tab).await?;
    pointer_focus(tab, OPENER).await?;
    saved(tab, 1206, "fluid-ounce", 1).await?;
    saved_label(tab, "已儲存 1206 液量盎司").await?;
    open(tab, QUANTITY.wheel()).await?;
    seed(tab, 1206, "fluid-ounce").await?;
    capture(env, tab, "saved-and-reopened").await?;
    Ok(json!({
        "locally_confirmed":[1204,"liter"],"unsubmitted":[1206,"fluid-ounce"],
        "saves":[1,1],"open_seed_is_clean":true,"early_saves":0,"restored_focus":OPENER,
    }))
}

async fn both_invalid(tab: &Tab) -> Result<()> {
    ready(
        tab,
        "d.open && d.working.quantity.error === 'OffStep' && d.working.unit.error === 'NoMatch' &&
         d.working.quantity.raw === '1203' && d.working.unit.raw === '無' &&
         d.working.first_invalid_editor === 'scroll-modal-demo-quantity-editor' &&
         d.working.status === '請修正數量和單位'",
    )
    .await?;
    saved(tab, 1200, "ml", 0).await
}

async fn invalid_save(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M237",
        "failed Save reports both named errors, preserves a repaired peer, and never partially writes",
    );
    open(tab, QUANTITY.wheel()).await?;
    stage_raw(tab, "1203", "無").await?;
    activate(tab, SAVE).await?;
    both_invalid(tab).await?;
    focused(tab, SAVE).await?;
    status(tab, "請修正數量和單位").await?;

    tab_sequence(tab, "Tab", &[QUANTITY.editor()]).await?;
    let quantity_error = QUANTITY.invalid(tab, "請輸入偶數").await?;
    tab_sequence(
        tab,
        "Tab",
        &[FocusTarget::Projected(QUANTITY.use_scroll), UNIT.editor()],
    )
    .await?;
    let unit_body = bounds(tab, BODY).await?;
    let (unit_editor, unit_ring) = revealed_ring(tab, UNIT.base, unit_body).await?;
    let unit_error_bounds = bounds(tab, &format!("{}-error", UNIT.base)).await?;
    check!(
        contains(unit_body, unit_error_bounds),
        "Tab to the wide Unit editor reveals its full error without a wheel: {unit_body:?}/{unit_error_bounds:?}"
    );
    let unit_error = UNIT.invalid(tab, "找不到單位").await?;
    let image = capture(env, tab, "invalid-both").await?;
    let unit_ink = ring_pixels(&image, unit_ring)?;

    tab_sequence(
        tab,
        "Shift+Tab",
        &[
            FocusTarget::Projected(QUANTITY.use_scroll),
            QUANTITY.editor(),
        ],
    )
    .await?;
    QUANTITY.type_text(tab, "１２０６").await?;
    activate(tab, SAVE).await?;
    ready(
        tab,
        "d.open && d.working.quantity.mode === 'input' && d.working.quantity.raw === '１２０６' &&
         d.working.quantity.value === 1200 && d.working.quantity.error === null &&
         d.working.unit.error === 'NoMatch' && d.working.unit.raw === '無' &&
         d.working.first_invalid_editor === 'scroll-modal-demo-unit-editor' &&
         d.working.status === '請修正單位'",
    )
    .await?;
    status(tab, "請修正單位").await?;
    saved(tab, 1200, "ml", 0).await?;
    tab_sequence(
        tab,
        "Tab",
        &[
            QUANTITY.editor(),
            FocusTarget::Projected(QUANTITY.use_scroll),
            UNIT.editor(),
        ],
    )
    .await?;
    UNIT.type_text(tab, "fl oz").await?;
    activate(tab, SAVE).await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    saved(tab, 1206, "fluid-ounce", 1).await?;
    Ok(json!({
        "errors":[quantity_error,unit_error],"statuses":["請修正數量和單位","請修正單位"],
        "failed_saves":2,"valid_peer_raw":"１２０６","first_invalid":["quantity","unit"],
        "wide_unit_reveal":{"body":unit_body,"editor":unit_editor,"ring":unit_ring,
                            "error":unit_error_bounds,"ring_ink":unit_ink},
        "before":[1200,"ml"],"after":[1206,"fluid-ounce"],"saves":1,
    }))
}

async fn dismissals(tab: &Tab) -> Result<Value> {
    step(
        "M238",
        "Cancel, clean Escape and a real scrim click discard the transaction and restore the opener",
    );
    let outside = bounds(tab, OUTSIDE).await?;
    let mut scrim_point = None;
    for route in ["cancel", "escape", "scrim"] {
        open(tab, QUANTITY.wheel()).await?;
        seed(tab, 1200, "ml").await?;
        stage_locally(tab, "1204", "L").await?;
        seed(tab, 1204, "liter").await?;
        saved(tab, 1200, "ml", 0).await?;
        match route {
            "cancel" => activate(tab, CANCEL).await?,
            "escape" => tab.press("Escape").await?,
            _ => {
                let panel = bounds(tab, DIALOG).await?;
                scrim_point = Some(press_scrim_at_background(tab, outside, panel).await?);
            }
        }
        closed(tab).await?;
        if route == "scrim" {
            pointer_focus(tab, OPENER).await?;
        } else {
            focused(tab, OPENER).await?;
        }
        ready(tab, "d.outside_presses === 0").await?;
        saved(tab, 1200, "ml", 0).await?;
    }
    open(tab, QUANTITY.wheel()).await?;
    seed(tab, 1200, "ml").await?;
    activate(tab, CANCEL).await?;
    closed(tab).await?;
    click(tab, OUTSIDE).await?;
    ready(tab, "d.outside_presses === 1").await?;
    pointer_focus(tab, OUTSIDE).await?;
    ax(&ax_tree(tab).await?, "button", "外部 1 次")?;
    Ok(json!({
        "routes":["cancel","escape","scrim"],"scrim_point":scrim_point,"outside_target":outside,
        "outside_presses_during_dismissal":0,"outside_presses_after_close":1,
        "accepted":[1200,"ml"],"saves":0,"restored_focus":OPENER,
    }))
}

async fn projected_press(tab: &Tab, id: &str) -> Result<()> {
    // Exercise the Web accessibility proxy click path. The explicit script
    // click is synthetic. Physical targets use CDP mouse input elsewhere.
    let target = projected(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("(() => {{ {target}.click(); return true; }})()"))
        .await?;
    tab.settle().await
}

async fn composition(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M239",
        "preedit blocks pointer and projected Save, and only explicit Save writes the settled pair",
    );
    open(tab, QUANTITY.wheel()).await?;
    tab_sequence(
        tab,
        "Tab",
        &[FocusTarget::Projected(QUANTITY.use_input), UNIT.wheel()],
    )
    .await?;
    UNIT.enter_editor(tab).await?;
    let preedit = composing(UNIT.base);
    tab.shortcut("a").await?;
    tab.ime_compose("公升", 2, 2).await?;
    tab.wait_for(&preedit).await?;
    capture(env, tab, "ime-preedit").await?;

    let pending = "d.open && d.saves === 0 && d.accepted.quantity === 1200 &&
                   d.accepted.unit_id === 'ml' && d.working.unit.id === 'ml' &&
                   d.working.unit.mode === 'input' && d.working.unit.raw === '毫升'";
    for id in [SAVE, CANCEL] {
        click(tab, id).await?;
        ready(tab, pending).await?;
        tab.wait_for(&preedit).await?;
        UNIT.editor().verify(tab).await?;
        projected_press(tab, id).await?;
        ready(tab, pending).await?;
        tab.wait_for(&preedit).await?;
        UNIT.editor().verify(tab).await?;
    }
    tab.press("Enter").await?;
    ready(
        tab,
        "d.open && d.saves === 0 && d.accepted.unit_id === 'ml' &&
         d.working.unit.id === 'ml' && d.working.unit.mode === 'input'",
    )
    .await?;
    UNIT.editor().verify(tab).await?;
    if inspect(tab).await?["working"]["unit"]["raw"] == "毫升" {
        tab.insert_text("公升").await?;
    }
    tab.wait_for(&format!("!({preedit})")).await?;
    ready(
        tab,
        "d.working.unit.raw === '公升' && d.working.unit.id === 'ml' && d.saves === 0",
    )
    .await?;
    UNIT.submit_editor(tab).await?;
    seed(tab, 1200, "liter").await?;
    saved(tab, 1200, "ml", 0).await?;

    // The same projected action succeeds once the composition has ended.
    projected_press(tab, SAVE).await?;
    closed(tab).await?;
    saved(tab, 1200, "liter", 1).await?;
    open(tab, QUANTITY.wheel()).await?;
    QUANTITY.enter_editor(tab).await?;
    let composing_quantity = composing(QUANTITY.base);
    tab.shortcut("a").await?;
    tab.ime_compose("１２０６", 4, 4).await?;
    tab.wait_for(&composing_quantity).await?;
    tab.press("Escape").await?;
    ready(
        tab,
        "d.open && d.working.quantity.value === 1200 && d.working.quantity.mode === 'input' &&
         d.working.quantity.raw === '1200' && d.working.unit.id === 'liter'",
    )
    .await?;
    saved(tab, 1200, "liter", 1).await?;
    QUANTITY.editor().verify(tab).await?;
    if tab.eval(&composing_quantity).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing_quantity})")).await?;
    QUANTITY.type_text(tab, "1206").await?;
    ready(tab, "d.working.quantity.raw === '1206' && d.saves === 1").await?;
    tab.press("Escape").await?;
    ready(
        tab,
        "d.open && d.working.quantity.mode === 'scroll' && d.working.quantity.raw === '1200'",
    )
    .await?;
    QUANTITY.wheel().verify(tab).await?;
    tab.press("Escape").await?;
    closed(tab).await?;
    focused(tab, OPENER).await?;
    saved(tab, 1200, "liter", 1).await?;
    Ok(json!({
        "blocked_pointer":["save","cancel"],"blocked_projected":["save","cancel"],
        "first_enter_saves":0,"field_enter_saves":0,"projected_save_saves":1,
        "escape_order":["preedit","editor","modal"],"accepted":[1200,"liter"],
    }))
}

async fn scroll_top(tab: &Tab) -> Result<f64> {
    inspect(tab).await?["body_scroll_top"]
        .as_f64()
        .context("the modal body scroll offset")
}

fn same_rect(expected: Rect, actual: Rect) -> bool {
    expected.into_iter().zip(actual).all(|(a, b)| near(a, b))
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M240",
        "dark RTL at 200 percent keeps both actions and whole focused rings visible at 320 dp",
    );
    for suffix in ["theme", "rtl", "large-text"] {
        activate(tab, &format!("scroll-modal-demo-{suffix}")).await?;
    }
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2").await?;
    resize(tab, 320, 400).await?;
    open(tab, QUANTITY.editor()).await?;
    ready(
        tab,
        "d.working.quantity.mode === 'input' && d.working.unit.mode === 'input'",
    )
    .await?;
    modal_scope(tab, DIALOG, "編輯容量", &[(OPENER, "開啟欄位")]).await?;
    let panel = bounds(tab, DIALOG).await?;
    let body = bounds(tab, BODY).await?;
    let footer = bounds(tab, ACTIONS).await?;
    let actions = full_footer(tab, panel, body, [CANCEL, SAVE]).await?;
    let view = canvas(tab).await?;
    check!(
        contains(view, panel)
            && contains(panel, body)
            && contains(panel, footer)
            && actions[0][0] > actions[1][0]
            && near(actions[0][1], actions[1][1]),
        "the bounded compact panel mirrors the side-by-side footer in RTL: {panel:?}/{actions:?}"
    );
    tab.wait_for(&format!(
        "{} === null && {} === null",
        projected(QUANTITY.adjustable),
        projected(UNIT.adjustable),
    ))
    .await?;

    let (quantity, quantity_ring) = revealed_ring(tab, QUANTITY.base, body).await?;
    let start = scroll_top(tab).await?;
    let quantity_image = capture(env, tab, "compact-quantity-focus").await?;
    let quantity_ink = ring_pixels(&quantity_image, quantity_ring)?;

    tab_sequence(tab, "Tab", &[UNIT.editor()]).await?;
    let (unit, unit_ring) = revealed_ring(tab, UNIT.base, body).await?;
    let down = scroll_top(tab).await?;
    let unit_image = capture(env, tab, "compact-dark-rtl").await?;
    let unit_ink = ring_pixels(&unit_image, unit_ring)?;
    tab_sequence(tab, "Shift+Tab", &[QUANTITY.editor()]).await?;
    let (_, returned_ring) = revealed_ring(tab, QUANTITY.base, body).await?;
    let up = scroll_top(tab).await?;
    check!(
        down > start && up < down,
        "the actual Tab and reverse Tab reveal scrolls the body: {start}/{down}/{up}"
    );
    check!(
        same_rect(actions[0], bounds(tab, CANCEL).await?)
            && same_rect(actions[1], bounds(tab, SAVE).await?),
        "both footer actions stay fixed through forward and reverse focus reveal"
    );

    QUANTITY.type_text(tab, "1203").await?;
    tab_sequence(tab, "Tab", &[UNIT.editor()]).await?;
    UNIT.type_text(tab, "無").await?;
    click(tab, SAVE).await?;
    both_invalid(tab).await?;
    status(tab, "請修正數量和單位").await?;
    until(tab, "s.focused === 'scroll-modal-demo-save'").await?;
    let failed_actions = full_footer(tab, panel, body, [CANCEL, SAVE]).await?;
    check!(
        same_rect(actions[0], failed_actions[0]) && same_rect(actions[1], failed_actions[1]),
        "validation leaves both pinned actions at their original bounds"
    );
    capture(env, tab, "compact-invalid").await?;

    resize(tab, 320, 217).await?;
    ready(
        tab,
        "!d.open && d.working.quantity.raw === '1203' && d.working.unit.raw === '無'",
    )
    .await?;
    tab.wait_for(&format!("{} === null", projected(DIALOG)))
        .await?;
    saved(tab, 1200, "ml", 0).await?;
    resize(tab, 320, 218).await?;
    ready(
        tab,
        "d.open && d.working.quantity.raw === '1203' && d.working.unit.raw === '無'",
    )
    .await?;
    both_invalid(tab).await?;
    QUANTITY.editor().verify(tab).await?;
    let shortest_panel = bounds(tab, DIALOG).await?;
    let shortest_body = bounds(tab, BODY).await?;
    let (shortest_editor, shortest_ring) = revealed_ring(tab, QUANTITY.base, shortest_body).await?;
    let shortest_error = bounds(tab, &format!("{}-error", QUANTITY.base)).await?;
    check!(
        shortest_error[3] > 0. && shortest_error[1] >= shortest_body[1] + shortest_body[3] - 0.75,
        "the focused invalid Quantity editor fills the shortest clip and its error is below it: {shortest_body:?}/{shortest_error:?}"
    );
    let shortest_description =
        editor_clipped_error(tab, QUANTITY.base, QUANTITY.label, "請輸入偶數").await?;
    let shortest_actions = full_footer(tab, shortest_panel, shortest_body, [CANCEL, SAVE]).await?;
    let shortest_image = capture(env, tab, "shortest-editor").await?;
    let shortest_ink = ring_pixels(&shortest_image, shortest_ring)?;
    resize(tab, 320, 400).await?;
    click(tab, CANCEL).await?;
    closed(tab).await?;
    pointer_focus(tab, OPENER).await?;
    saved(tab, 1200, "ml", 0).await?;
    resize(tab, 960, 2000).await?;
    activate(tab, BACK).await?;
    ready(tab, "!d.visible && !d.open").await?;
    focused(tab, SHOW).await?;

    Ok(json!({
        "viewport":[320,400],"rtl":true,"text_scale":2,"panel":panel,"body":body,
        "footer":footer,"actions":actions,"editors":[quantity,unit],
        "rings":[quantity_ring,unit_ring,returned_ring],"ring_ink":[quantity_ink,unit_ink],
        "body_scroll_top":[start,down,up],"failed_save_kept_footer":true,
        "invalid_summary":"請修正數量和單位","invalid_focus":SAVE,
        "short_view":{"hidden_height":217,"restored_height":218,
                      "editor":shortest_editor,"ring":shortest_ring,"body":shortest_body,
                      "panel":shortest_panel,"actions":shortest_actions,"ring_ink":shortest_ink,
                      "error":shortest_error,"error_description":shortest_description,
                      "retained_raw":["1203","無"]},
        "saves":0,"accepted":[1200,"ml"],"restored_focus":OPENER,"back_focus":SHOW,
    }))
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    reduced_motion(tab, true).await?;
    let opening = opening(env, tab).await?;
    reload(tab, url).await?;
    let save = atomic_save(env, tab, url).await?;
    reload(tab, url).await?;
    let invalid = invalid_save(env, tab).await?;
    reload(tab, url).await?;
    let dismissals = dismissals(tab).await?;
    reload(tab, url).await?;
    let composition = composition(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "modal Scroll Field page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-scroll-modal-fields.json",
        &json!({
            "browser":chrome.version,"opening":opening,"save":save,"invalid":invalid,
            "dismissals":dismissals,"composition":composition,"compact":compact,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-scroll-modal-fields-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-scroll-modal-fields-failure.png"))
            .await;
    }
    result.context("Material 3 modal Scroll Fields")
}
