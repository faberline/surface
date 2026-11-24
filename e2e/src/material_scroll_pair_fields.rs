//! Independent quantity and unit Scroll Fields in Chrome and WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_list_corners::reduced_motion,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const OPEN: &str = "scroll-pair-demo-show";
const BACK: &str = "scroll-pair-demo-back";
const PAIR: &str = "scroll-pair-demo-pair";
const SCROLLER: &str = "scroll-pair-demo-scroll";
const QUANTITY_DISABLED: &str = "scroll-pair-demo-quantity-disabled";
const UNIT_DISABLED: &str = "scroll-pair-demo-unit-disabled";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const CANVAS_FOCUS: &str = "document.activeElement === document.getElementById('surface')";

#[derive(Clone, Copy)]
struct Field {
    base: &'static str,
    label: &'static str,
    hint: &'static str,
    range: [i32; 2],
}

const QUANTITY: Field = Field {
    base: "scroll-pair-demo-quantity",
    label: "數量",
    hint: "每次增加或減少 2",
    range: [-1800, 1800],
};
const UNIT: Field = Field {
    base: "scroll-pair-demo-unit",
    label: "單位",
    hint: "可輸入單位名稱",
    range: [1, 3],
};

type Rect = [f64; 4];

impl Field {
    fn part(self, suffix: &str) -> String {
        format!("{}-{suffix}", self.base)
    }

    async fn editor_focus(self, tab: &Tab) -> Result<()> {
        until(tab, &format!("s.focused === {}", js(self.part("editor")))).await?;
        tab.wait_for(&format!(
            "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
        ))
        .await?;
        tab.settle().await
    }

    async fn edit(self, tab: &Tab) -> Result<()> {
        focus_projected(tab, &self.part("adjustable")).await?;
        tab.press("Enter").await?;
        self.editor_focus(tab).await
    }

    async fn spin(self, tab: &Tab, value: i32, unit_name: Option<&str>) -> Result<Value> {
        let dom = tab.eval(&format!(r#"(() => {{
            const field = {};
            if (!field) return null;
            return {{
                min: Number(field.getAttribute('aria-valuemin')), max: Number(field.getAttribute('aria-valuemax')),
                now: Number(field.getAttribute('aria-valuenow')), text: field.getAttribute('aria-valuetext'),
                name: field.getAttribute('aria-label'), role:field.getAttribute('role'),
                inside_group: field.closest('[data-surface-id="{PAIR}"]') !== null,
            }};
        }})()"#, projected(&self.part("adjustable")))).await?;
        let [min, max] = self.range;
        let text = dom["text"].as_str().context("adjustable value text")?;
        let localized_text = match unit_name {
            Some(unit) => text == unit,
            None => text.replace(',', "").parse::<i32>().ok() == Some(value),
        };
        check!(
            dom["role"] == "spinbutton"
                && dom["name"] == self.label
                && dom["inside_group"] == true
                && dom["min"] == min
                && dom["max"] == max
                && dom["now"] == value
                && localized_text,
            "the control describes its own range and value within the pair: {dom}"
        );

        let tree = ax_tree(tab).await?;
        let node = ax(&tree, "spinbutton", self.label)?;
        let description = &node["description"]["value"];
        let current = &node["value"]["value"];
        let bound = |key| ax_property(node, key).and_then(Value::as_f64);
        let numeric = current.as_f64() == Some(f64::from(value));
        let help = description
            .as_str()
            .context("Chrome's localized field help")?;
        let published_value = match unit_name {
            Some(unit) => current == unit || (numeric && help.contains(unit)),
            None => {
                numeric
                    && !["毫升", "公升", "液量盎司"]
                        .iter()
                        .any(|unit| help.contains(unit))
            }
        };
        check!(
            bound("valuemin") == Some(f64::from(min))
                && bound("valuemax") == Some(f64::from(max))
                && ax_property(node, "focusable") == Some(&json!(true))
                && help.contains(self.hint)
                && published_value,
            "Chrome's accessibility tree keeps the independent value and hint: {node}"
        );
        Ok(json!({"dom":dom,"chrome_value":current,"chrome_description":description}))
    }

    async fn disabled(self, tab: &Tab) -> Result<Value> {
        let dom = tab
            .eval(&format!(
                "[{}, {}].map(el => el?.getAttribute('aria-disabled'))",
                projected(&self.part("adjustable")),
                projected(&self.part("use-input"))
            ))
            .await?;
        check_eq!(
            dom,
            json!(["true", "true"]),
            "disabled wheel and its mode button are both unavailable"
        );
        let tree = ax_tree(tab).await?;
        let node = ax(&tree, "spinbutton", self.label)?;
        check_eq!(
            ax_property(node, "disabled"),
            Some(&json!(true)),
            "Chrome exposes the disabled field state"
        );
        Ok(dom)
    }

    async fn error(self, tab: &Tab) -> Result<String> {
        self.editor_focus(tab).await?;
        let id = self.part("error");
        let helper = projected(&id);
        let attributes = tab.eval(&format!(r#"(() => {{
            const input = {TEXTAREA};
            const error = {helper};
            const related = input?.getAttribute('aria-describedby')?.split(/\s+/) ?? [];
            return {{ invalid:input?.getAttribute('aria-invalid'), linked:related.some(id => document.getElementById(id) === error), text:error?.textContent }};
        }})()"#)).await?;
        let message = attributes["text"]
            .as_str()
            .context("the visible field error")?;
        check!(
            attributes["invalid"] == "true" && attributes["linked"] == true && !message.is_empty(),
            "only this editor publishes invalid and references its own error: {attributes}"
        );
        let tree = ax_tree(tab).await?;
        let node = ax(&tree, "textbox", self.label)?;
        check!(
            node["description"]["value"]
                .as_str()
                .is_some_and(|description| description.contains(message))
                && matches!(ax_property(node, "invalid"),Some(value) if value == &json!(true) || value == &json!("true")),
            "the actual Chrome textbox has its own name, error and invalid state"
        );
        Ok(message.to_owned())
    }
}

async fn read(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["scroll_pair"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.scroll_pair)")).await?;
    tab.settle().await
}

async fn preset(tab: &Tab, suffix: &str) -> Result<()> {
    activate(tab, &format!("scroll-pair-demo-{suffix}")).await
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
        "d.visible && d.quantity.value === 1200 && d.unit.id === 'ml'",
    )
    .await?;
    resize(tab, 960, 820).await?;
    ready(tab, "!d.stacked && d.quantity_fits && d.unit_fits && d.quantity.mode === 'scroll' && d.unit.mode === 'scroll'").await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-scroll-pair-fields-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn replace(tab: &Tab, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    tab.settle().await
}

async fn semantics(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M223",
        "one passive volume group holds two distinct labeled and bounded controls",
    );
    let group = tab.eval(&format!(r#"(() => {{
        const group = {};
        if (!group) return null;
        return {{
            role:group.getAttribute('role'), name:group.getAttribute('aria-label'),
            tabindex:group.getAttribute('tabindex'), action:group.getAttribute('data-surface-action'),
            names:Array.from(group.querySelectorAll('[role="spinbutton"]'), e=>e.getAttribute('aria-label')),
        }};
    }})()"#, projected(PAIR))).await?;
    check_eq!(
        group,
        json!({"role":"group","name":"容量","tabindex":null,"action":null,"names":["數量","單位"]}),
        "the pair groups exactly two controls without adding an action or Tab stop"
    );
    let tree = ax_tree(tab).await?;
    let group_ax = ax(&tree, "group", "容量")?;
    check!(
        ax_property(group_ax, "focusable") != Some(&json!(true)),
        "the actual accessibility group is passive"
    );
    let amount = QUANTITY.spin(tab, 1200, None).await?;
    let unit = UNIT.spin(tab, 1, Some("毫升")).await?;
    let faces = [
        bounds(tab, &QUANTITY.part("face")).await?,
        bounds(tab, &UNIT.part("face")).await?,
    ];
    let frame = capture(env, tab, "wide-ltr").await?;
    let painted = [frame.colors(faces[0])?.len(), frame.colors(faces[1])?.len()];
    check!(
        painted.into_iter().all(|count| count > 8),
        "both independent faces paint text in real WebGPU pixels"
    );
    Ok(json!({"group":group,"quantity":amount,"unit":unit,"faces":faces,"paint_colors":painted}))
}

async fn edits(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M224",
        "quantity and unit keyboard steps, validation, and direct commits stay independent",
    );
    reduced_motion(tab, true).await?;
    focus_projected(tab, &QUANTITY.part("adjustable")).await?;
    tab.press("ArrowUp").await?;
    ready(tab, "d.quantity.value === 1202 && d.quantity.commits === 1 && d.unit.id === 'ml' && d.unit.commits === 0").await?;
    focus_projected(tab, &UNIT.part("adjustable")).await?;
    tab.press("ArrowUp").await?;
    ready(tab, "d.quantity.value === 1202 && d.quantity.commits === 1 && d.unit.id === 'liter' && d.unit.commits === 1").await?;
    let liters = UNIT.spin(tab, 2, Some("公升")).await?;

    UNIT.edit(tab).await?;
    replace(tab, "無").await?;
    tab.press("Enter").await?;
    ready(tab, "d.unit.mode === 'input' && d.unit.error === 'NoMatch' && d.unit.raw === '無' && d.unit.id === 'liter' && d.unit.commits === 1 && d.quantity.value === 1202 && !d.quantity.error").await?;
    let unit_error = UNIT.error(tab).await?;
    capture(env, tab, "unit-error").await?;
    replace(tab, "fl oz").await?;
    ready(tab, "d.unit.mode === 'input' && !d.unit.error && d.unit.raw === 'fl oz' && d.unit.id === 'liter' && d.unit.commits === 1 && d.quantity.value === 1202").await?;
    tab.press("Enter").await?;
    ready(tab, "d.unit.mode === 'scroll' && d.unit.id === 'fluid-ounce' && d.unit.commits === 2 && d.quantity.value === 1202 && d.quantity.commits === 1").await?;
    focused(tab, &UNIT.part("adjustable")).await?;
    let ounces = UNIT.spin(tab, 3, Some("液量盎司")).await?;

    QUANTITY.edit(tab).await?;
    replace(tab, "1201").await?;
    tab.press("Enter").await?;
    ready(tab, "d.quantity.mode === 'input' && d.quantity.error === 'OffStep' && d.quantity.raw === '1201' && d.quantity.value === 1202 && d.quantity.commits === 1 && d.unit.id === 'fluid-ounce' && d.unit.commits === 2").await?;
    let error = QUANTITY.error(tab).await?;
    capture(env, tab, "quantity-error").await?;
    replace(tab, "１２０８").await?;
    ready(tab, "!d.quantity.error && d.quantity.raw === '１２０８' && d.quantity.value === 1202 && d.quantity.commits === 1").await?;
    tab.press("Enter").await?;
    ready(tab, "d.quantity.mode === 'scroll' && d.quantity.value === 1208 && d.quantity.commits === 2 && d.unit.id === 'fluid-ounce' && d.unit.commits === 2").await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    let amount = QUANTITY.spin(tab, 1208, None).await?;
    capture(env, tab, "independent-commits").await?;
    Ok(
        json!({"quantity_values":[1200,1202,1208],"unit_ids":["ml","liter","fluid-ounce"],
        "commits":[2,2],"rejected_quantity":"1201","error":error,"alias":"fl oz","unit_error":unit_error,
        "quantity_ax":amount,"liter_ax":liters,"unit_ax":ounces}),
    )
}

async fn faces(tab: &Tab) -> Result<[Rect; 2]> {
    Ok([
        bounds(tab, &QUANTITY.part("face")).await?,
        bounds(tab, &UNIT.part("face")).await?,
    ])
}

async fn ordered_tabs(tab: &Tab) -> Result<Vec<String>> {
    focus_projected(tab, UNIT_DISABLED).await?;
    let stops = [
        QUANTITY.part("adjustable"),
        QUANTITY.part("use-input"),
        UNIT.part("adjustable"),
        UNIT.part("use-input"),
    ];
    let mut seen = Vec::new();
    for id in stops {
        tab.press("Tab").await?;
        focused(tab, &id).await?;
        seen.push(id);
    }
    Ok(seen)
}

async fn mirror(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M225",
        "960 dp mirrors the visual fields in RTL and retains the authored keyboard order",
    );
    let ltr_faces @ [q0, u0] = faces(tab).await?;
    let ltr_gap = u0[0] - (q0[0] + q0[2]);
    check!(
        ltr_gap > 0. && near(q0[1], u0[1]),
        "quantity leads the LTR row: {ltr_faces:?}"
    );
    let ltr_order = ordered_tabs(tab).await?;
    preset(tab, "rtl").await?;
    ready(tab, "d.rtl && !d.stacked && d.quantity_fits && d.unit_fits").await?;
    let rtl_faces @ [q1, u1] = faces(tab).await?;
    let rtl_gap = q1[0] - (u1[0] + u1[2]);
    check!(rtl_gap > 0. && near(q1[1],u1[1]) && near(ltr_gap,rtl_gap)
        && near(q0[2],q1[2]) && near(u0[2],u1[2]) && near(q0[3],q1[3]) && near(u0[3],u1[3]),
        "RTL mirrors the physical order without shrinking either face: LTR={ltr_faces:?} RTL={rtl_faces:?}");
    let rtl_order = ordered_tabs(tab).await?;
    check_eq!(
        ltr_order,
        rtl_order,
        "quantity still precedes unit in real Tab order"
    );
    let image = capture(env, tab, "wide-rtl").await?;
    check!(
        image.colors(q1)?.len() > 8 && image.colors(u1)?.len() > 8,
        "both mirrored fields paint their glyphs"
    );
    Ok(
        json!({"viewport":[960,820],"ltr":ltr_faces,"rtl":rtl_faces,"gaps":[ltr_gap,rtl_gap],"tab_order":rtl_order}),
    )
}

async fn disabled(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M226",
        "each disabled field and its mode button are skipped while the other field still changes",
    );
    reduced_motion(tab, true).await?;
    activate(tab, QUANTITY_DISABLED).await?;
    ready(tab, "d.quantity.disabled && !d.unit.disabled").await?;
    let quantity_attributes = QUANTITY.disabled(tab).await?;
    click(tab, &QUANTITY.part("face")).await?;
    ready(tab, "d.quantity.disabled && d.quantity.mode === 'scroll' && d.quantity.value === 1200 && d.quantity.commits === 0").await?;
    focus_projected(tab, UNIT_DISABLED).await?;
    tab.press("Tab").await?;
    focused(tab, &UNIT.part("adjustable")).await?;
    tab.press("ArrowUp").await?;
    ready(tab, "d.quantity.value === 1200 && d.quantity.commits === 0 && d.unit.id === 'liter' && d.unit.commits === 1").await?;
    capture(env, tab, "quantity-disabled").await?;

    activate(tab, QUANTITY_DISABLED).await?;
    activate(tab, UNIT_DISABLED).await?;
    ready(tab, "!d.quantity.disabled && d.unit.disabled").await?;
    let unit_attributes = UNIT.disabled(tab).await?;
    click(tab, &UNIT.part("face")).await?;
    ready(tab, "d.unit.disabled && d.unit.mode === 'scroll' && d.unit.id === 'liter' && d.unit.commits === 1").await?;
    focus_projected(tab, UNIT_DISABLED).await?;
    tab.press("Tab").await?;
    focused(tab, &QUANTITY.part("adjustable")).await?;
    tab.press("ArrowDown").await?;
    ready(tab, "d.quantity.value === 1198 && d.quantity.commits === 1 && d.unit.id === 'liter' && d.unit.commits === 1").await?;
    capture(env, tab, "unit-disabled").await?;
    Ok(
        json!({"quantity_disabled_attributes":quantity_attributes,"unit_disabled_attributes":unit_attributes,
        "enabled_tab_stops":[UNIT.part("adjustable"),QUANTITY.part("adjustable")],
        "quantity_when_unit_changes":1200,"unit_when_quantity_changes":"liter","final_quantity":1198,"commits":[1,1]}),
    )
}

async fn compact_presets(tab: &Tab, height: u32) -> Result<()> {
    for option in ["theme", "rtl", "large-text"] {
        preset(tab, option).await?;
    }
    resize(tab, 320, height).await?;
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2 && d.stacked && !d.quantity_fits && !d.unit_fits && d.quantity.mode === 'input' && d.unit.mode === 'input'").await
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step("M227", "320 dp dark RTL stacks both 200 percent editors and preserves active composition through resize");
    reduced_motion(tab, true).await?;
    focus_projected(tab, &QUANTITY.part("adjustable")).await?;
    tab.press("ArrowUp").await?;
    ready(
        tab,
        "d.quantity.value === 1202 && d.quantity.commits === 1 && d.unit.id === 'ml'",
    )
    .await?;
    compact_presets(tab, 820).await?;
    let ids = [QUANTITY.part("editor"), UNIT.part("editor")];
    let editors = [bounds(tab, &ids[0]).await?, bounds(tab, &ids[1]).await?];
    let [q, u] = editors;
    check!(
        q[1] + q[3] <= u[1]
            && editors.into_iter().all(|editor| {
                contains([0., 0., 320., 820.], editor) && editor[2] >= 48. && editor[3] >= 48.
            }),
        "both real editors fit, with quantity above unit: {editors:?}"
    );
    let no_wheels = tab
        .eval(&format!(
            "{} === null && {} === null",
            projected(&QUANTITY.part("adjustable")),
            projected(&UNIT.part("adjustable"))
        ))
        .await?;
    check_eq!(
        no_wheels,
        json!(true),
        "compact mode has no hidden adjustable focus targets"
    );

    click(tab, &ids[1]).await?;
    UNIT.editor_focus(tab).await?;
    let tree = ax_tree(tab).await?;
    let active = ax(&tree, "textbox", UNIT.label)?;
    check!(
        active["description"]["value"]
            .as_str()
            .is_some_and(|text| text.contains(UNIT.hint)),
        "the unit textbox keeps its own accessible hint"
    );
    let image = capture(env, tab, "compact-dark-rtl").await?;
    let paint = [image.colors(q)?.len(), image.colors(u)?.len()];
    check!(
        paint.into_iter().all(|colors| colors > 8),
        "real dark 200 percent ink appears in both editors"
    );

    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(&ids[1])
    );
    tab.shortcut("a").await?;
    tab.ime_compose("公升", 2, 2).await?;
    tab.wait_for(&composing).await?;
    ready(tab, "d.unit.raw === '毫升' && d.unit.id === 'ml' && d.unit.commits === 0 && d.quantity.value === 1202 && d.quantity.commits === 1").await?;
    resize(tab, 960, 820).await?;
    ready(tab, "d.unit_fits && d.unit.mode === 'input' && d.unit.raw === '毫升' && d.unit.commits === 0 && d.quantity.value === 1202").await?;
    UNIT.editor_focus(tab).await?;
    tab.wait_for(&composing).await?;
    capture(env, tab, "resized-unit-preedit").await?;
    tab.press("Enter").await?;
    ready(tab, "d.unit.mode === 'input' && d.unit.id === 'ml' && d.unit.commits === 0 && d.quantity.value === 1202").await?;
    if read(tab).await?["unit"]["raw"] != "公升" {
        tab.insert_text("公升").await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    ready(tab, "d.unit.mode === 'input' && d.unit.raw === '公升' && d.unit.id === 'ml' && d.unit.commits === 0 && d.quantity.value === 1202").await?;
    tab.press("Enter").await?;
    ready(tab, "d.unit.mode === 'scroll' && d.unit.id === 'liter' && d.unit.commits === 1 && d.quantity.value === 1202 && d.quantity.commits === 1").await?;
    until(
        tab,
        &format!("s.focused === {}", js(UNIT.part("adjustable"))),
    )
    .await?;
    tab.wait_for(CANVAS_FOCUS).await?;
    let before_scroll = scroll_top(tab).await?;
    let owner = point(tab, &UNIT.part("adjustable")).await?;
    tab.mouse_move(owner.x, owner.y).await?;
    tab.wheel(0., 120.).await?;
    ready(tab, &format!("d.scroll_top > {before_scroll} && d.unit.id === 'liter' && d.unit.commits === 1 && d.quantity.value === 1202")).await?;
    let after_scroll = scroll_top(tab).await?;
    let help = bounds(tab, &UNIT.part("hint")).await?;
    check!(
        contains(bounds(tab, SCROLLER).await?, help),
        "a real wheel scroll reveals all of the unit hint"
    );
    let accepted = UNIT.spin(tab, 2, Some("公升")).await?;
    capture(env, tab, "wide-after-composition").await?;
    resize(tab, 320, 820).await?;
    ready(tab, "d.stacked && !d.unit_fits && d.unit.mode === 'input' && d.unit.raw === '公升' && d.unit.commits === 1 && d.quantity.value === 1202").await?;
    UNIT.editor_focus(tab).await?;
    Ok(
        json!({"viewport":[320,820],"rtl":true,"text_scale":2,"editors":editors,"paint_colors":paint,
        "preedit_survived_resize":true,"raw":"公升","first_enter_commits":0,"id":"liter","unit_commits":1,
        "quantity":1202,"quantity_commits":1,"restored_editor_focus":true,"hint_scroll":[before_scroll,after_scroll],"unit_ax":accepted}),
    )
}

async fn scroll_top(tab: &Tab) -> Result<f64> {
    read(tab).await?["scroll_top"]
        .as_f64()
        .context("the paired-field scroller offset")
}

async fn visible_editor(tab: &Tab, field: Field, scroller: Rect) -> Result<(Rect, Rect)> {
    field.editor_focus(tab).await?;
    let editor = bounds(tab, &field.part("editor")).await?;
    let ring = bounds(tab, &field.part("focus")).await?;
    let [_, _, width, height] = tab.canvas_box().await?;
    check!(contains(ring,editor) && contains(scroller,ring) && contains([0.,0.,width,height],ring),
        "the focused {} editor and exterior ring are fully inside the short clip: {editor:?}/{ring:?}/{scroller:?}",field.label);
    Ok((editor, ring))
}

async fn short(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M228",
        "Tab reveals both fields inside the short scroller and Back restores the Gallery launcher",
    );
    compact_presets(tab, 360).await?;
    focus_projected(tab, UNIT_DISABLED).await?;
    let scroller = bounds(tab, SCROLLER).await?;
    let before = scroll_top(tab).await?;
    let unit_initial = bounds(tab, &UNIT.part("editor")).await?;
    check!(
        !contains(scroller, unit_initial),
        "the unit begins clipped in the short scroll viewport"
    );

    tab.press("Tab").await?;
    let (quantity, quantity_ring) = visible_editor(tab, QUANTITY, scroller).await?;
    let first = scroll_top(tab).await?;
    tab.press("Tab").await?;
    let (unit, unit_ring) = visible_editor(tab, UNIT, scroller).await?;
    let second = scroll_top(tab).await?;
    check!(
        second > first && first >= before,
        "forward focus scrolls the ancestor to reveal unit: {before}/{first}/{second}"
    );
    let frame = capture(env, tab, "short-unit-focused").await?;
    let [x, y, w, h] = unit_ring;
    let center = x + w / 2.;
    let ring_ink = [
        frame.sample(center, y + 1.5)?,
        frame.sample(center, y + h - 1.5)?,
    ];
    let gap = frame.sample(center, y + 4.)?;
    check!(ring_ink[0] == ring_ink[1] && ring_ink[1] != gap,
        "the true bottom focus stroke matches the top and differs from the gap: {ring_ink:?}/{gap:?}");
    tab.press("Shift+Tab").await?;
    let (restored_quantity, restored_ring) = visible_editor(tab, QUANTITY, scroller).await?;
    let restored = scroll_top(tab).await?;
    check!(restored < second, "reverse focus scrolls back to quantity");
    ready(tab, "d.quantity.commits === 0 && d.unit.commits === 0").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, BACK).await?;
    focused(tab, OPEN).await?;
    Ok(
        json!({"viewport":[320,360],"scroller":scroller,"clipped_unit":unit_initial,
        "focus_order":[QUANTITY.part("editor"),UNIT.part("editor"),QUANTITY.part("editor")],
        "focused_editors":[quantity,unit,restored_quantity],"focus_rings":[quantity_ring,unit_ring,restored_ring],
        "unit_ring_ink":ring_ink,"unit_ring_gap":gap,"scroll_offsets":[before,first,second,restored],
        "commits":[0,0],"back_focus":OPEN}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let semantics = semantics(env, tab).await?;
    let edits = edits(env, tab).await?;
    reload(tab, url).await?;
    let mirror = mirror(env, tab).await?;
    reload(tab, url).await?;
    let disabled = disabled(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    reload(tab, url).await?;
    let short = short(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "paired Scroll Field page errors: {errors:?}"
    );
    env.write_json("browser-material-scroll-pair-fields.json", &json!({
        "browser":chrome.version,"semantics":semantics,"edits":edits,"mirror":mirror,"disabled":disabled,
        "compact":compact,"short":short,"errors":errors,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-scroll-pair-fields-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-scroll-pair-fields-failure.png"))
            .await;
    }
    result.context("Material 3 paired Scroll Fields")
}
