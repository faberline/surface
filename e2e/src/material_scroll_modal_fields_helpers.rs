//! Draft helpers for a child module of material_scroll_modal_fields.rs.
//! Fixture IDs, inspection keys and release step integration are caller-owned.

use anyhow::Result;
use serde_json::{json, Value};

use crate::browser::{js, Tab};
use crate::material::{
    material_time_input::{ax, ax_property, ax_tree, bounds, contains, focused, projected, Raster},
    until,
};
use crate::{check, check_eq};

const TEXTAREA: &str = "document.getElementById('surface-text-input')";
pub(super) type Rect = [f64; 4];

#[derive(Clone, Copy)]
pub(super) enum FocusTarget<'a> {
    Projected(&'a str),
    Editor(&'a str),
}

impl FocusTarget<'_> {
    fn stable_id(self) -> String {
        match self {
            Self::Projected(id) => id.to_owned(),
            Self::Editor(base) => format!("{base}-editor"),
        }
    }

    pub(super) async fn verify(self, tab: &Tab) -> Result<()> {
        match self {
            Self::Projected(id) => focused(tab, id).await,
            Self::Editor(_) => {
                until(tab, &format!("s.focused === {}", js(self.stable_id()))).await?;
                tab.wait_for(&format!(
                    "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
                ))
                .await?;
                tab.settle().await
            }
        }
    }
}

pub(super) async fn modal_scope(
    tab: &Tab,
    dialog: &str,
    name: &str,
    background: &[(&str, &str)],
) -> Result<Value> {
    let element = projected(dialog);
    tab.wait_for(&format!(
        "{element}?.getAttribute('role') === 'dialog' &&
         {element}.getAttribute('aria-modal') === 'true' &&
         {element}.getAttribute('aria-label') === {}",
        js(name)
    ))
    .await?;
    let tree = ax_tree(tab).await?;
    let ax_dialog = ax(&tree, "dialog", name)?;
    check_eq!(
        ax_property(ax_dialog, "modal"),
        Some(&json!(true)),
        "Chrome marks the active dialog modal"
    );
    check_eq!(
        tree.iter()
            .filter(|node| node["ignored"] == false && node["role"]["value"] == "dialog")
            .count(),
        1,
        "there is one active Chrome dialog"
    );
    for (id, label) in background {
        check_eq!(
            tab.eval(&format!("{} === null", projected(id))).await?,
            json!(true),
            "active modal removes background control {id} from the projected DOM"
        );
        check!(
            ax(&tree, "button", label).is_err(),
            "active modal removes the named background button {label} from Chrome AX"
        );
    }
    Ok(json!({"name":name,"aria_modal":true,"count":1}))
}

pub(super) async fn tab_sequence(
    tab: &Tab,
    chord: &str,
    stops: &[FocusTarget<'_>],
) -> Result<Vec<String>> {
    let mut visited = Vec::with_capacity(stops.len());
    for target in stops {
        tab.press(chord).await?;
        target.verify(tab).await?;
        visited.push(target.stable_id());
    }
    Ok(visited)
}

pub(super) async fn editor_invalid(
    tab: &Tab,
    base: &str,
    label: &str,
    error_text: &str,
) -> Result<Value> {
    FocusTarget::Editor(base).verify(tab).await?;
    let helper = projected(&format!("{base}-error"));
    let proof = tab
        .eval(&format!(
            r#"(() => {{
                const input = {TEXTAREA};
                const error = {helper};
                const ids = input?.getAttribute('aria-describedby')?.split(/\s+/) ?? [];
                return {{
                    invalid: input?.getAttribute('aria-invalid'),
                    text: error?.textContent,
                    linked: ids.some(id => document.getElementById(id) === error)
                }};
            }})()"#
        ))
        .await?;
    check_eq!(
        proof,
        json!({"invalid":"true","linked":true,"text":error_text}),
        "the active editor names its own current error"
    );
    let description = chrome_invalid_description(tab, label, error_text).await?;
    Ok(json!({"dom":proof,"chrome_description":description}))
}

pub(super) async fn editor_clipped_error(
    tab: &Tab,
    base: &str,
    label: &str,
    error_text: &str,
) -> Result<Value> {
    FocusTarget::Editor(base).verify(tab).await?;
    let proof = tab
        .eval(&format!(
            r#"(() => {{
                const input = {TEXTAREA};
                const ids = input?.getAttribute('aria-describedby')?.split(/\s+/).filter(Boolean) ?? [];
                return {{
                    invalid: input?.getAttribute('aria-invalid'),
                    described_by: ids.map(id => document.getElementById(id)?.textContent ?? null),
                    description: input?.getAttribute('aria-description')
                }};
            }})()"#
        ))
        .await?;
    check!(
        proof["invalid"] == "true"
            && (proof["description"] == error_text
                || proof["described_by"]
                    .as_array()
                    .is_some_and(|entries| { entries.iter().any(|text| text == error_text) })),
        "the focused editor keeps its exact error description beyond the painted clip: {proof}"
    );
    let description = chrome_invalid_description(tab, label, error_text).await?;
    Ok(json!({"dom":proof,"chrome_description":description}))
}

async fn chrome_invalid_description(tab: &Tab, label: &str, error_text: &str) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let input = ax(&tree, "textbox", label)?;
    let description = &input["description"]["value"];
    let invalid = ax_property(input, "invalid");
    check!(
        description
            .as_str()
            .is_some_and(|value| value.contains(error_text))
            && matches!(invalid, Some(value) if value == &json!(true) || value == &json!("true")),
        "the named Chrome textbox is invalid and describes its current error"
    );
    Ok(description.clone())
}

pub(super) fn composing(base: &str) -> String {
    let id = js(format!("{base}-editor"));
    format!("JSON.parse(window.gallery.export_dom()).text_inputs[{id}]?.composing === true")
}

pub(super) async fn canvas(tab: &Tab) -> Result<Rect> {
    let [_, _, width, height] = tab.canvas_box().await?;
    Ok([0., 0., width, height])
}

pub(super) async fn full_footer(
    tab: &Tab,
    panel: Rect,
    body: Rect,
    actions: [&str; 2],
) -> Result<[Rect; 2]> {
    let rects = [
        bounds(tab, actions[0]).await?,
        bounds(tab, actions[1]).await?,
    ];
    let view = canvas(tab).await?;
    check!(
        rects.iter().all(|rect| {
            rect[2] >= 48.
                && rect[3] >= 48.
                && contains(panel, *rect)
                && contains(view, *rect)
                && !overlap(body, *rect)
        }) && !overlap(rects[0], rects[1]),
        "both footer actions retain independent, fully visible 48 dp targets: {rects:?}"
    );
    Ok(rects)
}

fn overlap([x, y, w, h]: Rect, [a, b, c, d]: Rect) -> bool {
    x < a + c && a < x + w && y < b + d && b < y + h
}

pub(super) async fn revealed_ring(tab: &Tab, base: &str, body: Rect) -> Result<(Rect, Rect)> {
    FocusTarget::Editor(base).verify(tab).await?;
    let edit = bounds(tab, &format!("{base}-editor")).await?;
    let ring = bounds(tab, &format!("{base}-focus")).await?;
    check!(
        edit[2] >= 48. && edit[3] >= 48.
            && contains(ring, edit)
            && contains(body, ring)
            && contains(canvas(tab).await?, ring),
        "the complete {base} editor and its outer focus ring fit the current clip: {edit:?}/{ring:?}/{body:?}"
    );
    Ok((edit, ring))
}

pub(super) fn ring_pixels(frame: &Raster, [x, y, w, h]: Rect) -> Result<Value> {
    let center = x + w / 2.;
    let top = frame.sample(center, y + 1.5)?;
    let bottom = frame.sample(center, y + h - 1.5)?;
    let gap = frame.sample(center, y + 4.)?;
    check!(
        top == bottom && top != gap,
        "both sides of the exterior focus stroke paint with a gap: {top:?}/{bottom:?}/{gap:?}"
    );
    Ok(json!({"top":top,"bottom":bottom,"gap":gap}))
}

pub(super) async fn press_scrim_at_background(
    tab: &Tab,
    background: Rect,
    panel: Rect,
) -> Result<[f64; 2]> {
    let [x, y, w, h] = background;
    let [left, top, width, height] = tab.canvas_box().await?;
    let point = [x + w / 2., y + h / 2.];
    let target = [point[0], point[1], 0., 0.];
    check!(
        contains([0., 0., width, height], target) && !contains(panel, target),
        "trusted scrim click is on the real background control, outside the panel: {point:?}"
    );
    tab.click(left + point[0], top + point[1]).await?;
    Ok(point)
}

pub(super) async fn unmounted(tab: &Tab, dialog: &str, layer: &str, opener: &str) -> Result<()> {
    tab.wait_for(&format!(
        "{} === null && {} === null",
        projected(dialog),
        projected(layer)
    ))
    .await?;
    until(tab, &format!("s.focused === {}", js(opener))).await?;
    tab.settle().await
}
