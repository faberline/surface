//! Browser proof for the controlled Material checkbox and its projected semantics.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{focus, point, state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "checkbox-demo-show";
const BACK: &str = "checkbox-demo-back";
const THEME: &str = "checkbox-demo-theme";
const RTL: &str = "checkbox-demo-rtl";
const LARGE: &str = "checkbox-demo-large-text";
const SCROLL: &str = "checkbox-demo-scroll";
const ALL: &str = "checkbox-demo-all";
const NOTES: &str = "checkbox-demo-notes";
const ATTACHMENTS: &str = "checkbox-demo-attachments";
const LOCKED: &str = "checkbox-demo-locked";
const CONSENT: &str = "checkbox-demo-consent";
const HELP: &str = "checkbox-demo-error-hint";
const HELP_TEXT: &str = "請勾選後再繼續。";
const IDS: [&str; 5] = [ALL, NOTES, ATTACHMENTS, LOCKED, CONSENT];
const NAMES: [&str; 5] = [
    "同步全部",
    "筆記",
    "附件",
    "已鎖定的專案",
    "允許在行動網路上同步大型附件",
];
const OFF: &str = "unchecked";
const ON: &str = "checked";
const MIXED: &str = "mixed";
const START: [&str; 5] = [MIXED, ON, OFF, MIXED, OFF];
const ALL_ON: [&str; 5] = [ON, ON, ON, MIXED, OFF];
const CHILD_OFF: [&str; 5] = [MIXED, OFF, ON, MIXED, OFF];
const ALL_OFF: [&str; 5] = [OFF, OFF, OFF, MIXED, OFF];
const ACCEPTED: [&str; 5] = [OFF, OFF, OFF, MIXED, ON];
const STEM: &str = "material-checkbox";

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn size(rect: Rect) -> [f64; 2] {
    [rect[2], rect[3]]
}

fn center(rect: Rect) -> (f64, f64) {
    (rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.)
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

#[derive(Serialize)]
struct Geometry {
    row: Rect,
    art: Rect,
    mark: Rect,
    layer: Rect,
    ring: Rect,
    label: Rect,
}

async fn geometry(tab: &Tab, id: &str, rtl: bool) -> Result<Geometry> {
    let row = bounds(tab, id).await?;
    let art = bounds(tab, &format!("{id}-box")).await?;
    let mark = bounds(tab, &format!("{id}-mark")).await?;
    let layer = bounds(tab, &format!("{id}-state-layer")).await?;
    let ring = bounds(tab, &format!("{id}-focus-ring")).await?;
    let label = bounds(tab, &format!("{id}-label")).await?;
    check_eq!(size(art), [18., 18.], format!("{id} box"));
    check_eq!(mark, art, format!("{id} mark bounds"));
    check_eq!(size(layer), [40., 40.], format!("{id} state layer"));
    check_eq!(size(ring), [50., 50.], format!("{id} focus ring"));
    let (cx, cy) = center(art);
    check!(
        [layer, ring].into_iter().all(|child| {
            let (x, y) = center(child);
            near(cx, x) && near(cy, y)
        }),
        "{id} box, state layer and focus ring are centered"
    );
    check!(
        row[2] >= 48. && row[3] >= 48. && contains(row, label) && contains(row, ring),
        "{id} keeps the whole label and artwork within the hit cell: {row:?}"
    );
    check!(
        if rtl {
            near(row[0] + row[2], ring[0] + ring[2]) && label[0] + label[2] <= ring[0]
        } else {
            near(ring[0], row[0]) && ring[0] + ring[2] <= label[0]
        },
        "{id} leads with artwork on the correct visual side"
    );
    Ok(Geometry {
        row,
        art,
        mark,
        layer,
        ring,
        label,
    })
}

async fn values(tab: &Tab, expected: [&str; 5], requests: u32) -> Result<Value> {
    until(
        tab,
        &format!(
            "s.checkbox_demo.requests === {requests} && JSON.stringify(s.checkbox_demo.values) === {}",
            json!(json!(expected).to_string())
        ),
    )
    .await?;
    tab.settle().await?;
    let actual = state(tab).await?["checkbox_demo"].clone();
    check_eq!(
        actual["values"],
        json!(expected),
        "controlled checkbox values"
    );
    check_eq!(
        actual["requests"],
        json!(requests),
        "number of delivered checkbox requests"
    );
    Ok(actual)
}

async fn interactions(tab: &Tab, pressed: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        json!(pressed)
    ))
    .await?;
    tab.settle().await
}

fn token(palette: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(palette[name].clone())
        .with_context(|| format!("the authored checkbox {name} token"))?;
    check_eq!(alpha, 255, format!("{name} is opaque"));
    Ok([r, g, b])
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    let path = env.artifact(&format!("{STEM}-{name}.png"));
    tab.settle().await?;
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    until(
        tab,
        "s.section === 0 && !s.dark && !s.checkbox_demo.visible",
    )
    .await?;
    activate(tab, SHOW).await?;
    until(
        tab,
        "s.checkbox_demo.visible && !s.checkbox_demo.rtl && s.checkbox_demo.text_scale === 1",
    )
    .await?;
    focused(tab, BACK).await?;
    resize(tab, 960, 800).await
}

async fn checkbox_nodes(tab: &Tab, expected: [&str; 5]) -> Result<Vec<Value>> {
    let tree = ax_tree(tab).await?;
    let mut nodes = Vec::with_capacity(IDS.len());
    for (index, (id, name)) in IDS.into_iter().zip(NAMES).enumerate() {
        let node = ax(&tree, "checkbox", name)?;
        let expected_state = match expected[index] {
            ON => "true",
            MIXED => "mixed",
            _ => "false",
        };
        check_eq!(
            ax_property(node, "checked"),
            Some(&json!(expected_state)),
            format!("{id} Chrome checked state")
        );
        let disabled = id == LOCKED;
        check_eq!(
            ax_property(node, "disabled") == Some(&json!(true)),
            disabled,
            format!("{id} Chrome disabled state")
        );
        let proxy = projected(id);
        check_eq!(
            tab.eval(&format!("{proxy}?.hasAttribute('data-surface-action')"))
                .await?,
            !disabled,
            format!("{id} projected action")
        );
        nodes.push(node.clone());
    }
    Ok(nodes)
}

async fn validation(tab: &Tab, node: &Value, invalid: bool) -> Result<Value> {
    let proxy = projected(CONSENT);
    let dom = tab
        .eval(&format!(
            "(() => {{
                const field = {proxy};
                const reference = field?.getAttribute('aria-describedby') ?? null;
                return {{
                    invalid: field?.getAttribute('aria-invalid') ?? null,
                    reference,
                    text: reference?.split(/\\s+/).map(id => document.getElementById(id)?.textContent).join(' ') ?? null
                }};
            }})()"
        ))
        .await?;
    let ax_invalid = matches!(
        ax_property(node, "invalid"),
        Some(value) if value == &json!(true) || value == &json!("true")
    );
    let description = node["description"]["value"].as_str().unwrap_or_default();
    check_eq!(ax_invalid, invalid, "Chrome AX invalid state");
    if invalid {
        check_eq!(dom["invalid"], "true", "the error sets aria-invalid");
        check!(
            dom["reference"].as_str().is_some_and(|id| !id.is_empty()),
            "the field references a real help node"
        );
        check_eq!(
            dom["text"],
            HELP_TEXT,
            "the ARIA reference resolves to authored help"
        );
        check_eq!(
            description,
            HELP_TEXT,
            "Chrome AX exposes the authored description"
        );
    } else {
        check_eq!(
            dom,
            json!({"invalid":null,"reference":null,"text":null}),
            "the accepted value removes the error relation"
        );
        check!(
            description.is_empty(),
            "Chrome AX retires the error description"
        );
        check_eq!(
            tab.eval_f64s(&format!("window.gallery.control_bounds({HELP:?})"))
                .await?,
            Vec::<f64>::new(),
            "the accepted value removes the painted help node"
        );
    }
    Ok(json!({"dom":dom,"chrome_invalid":ax_invalid,"chrome_description":description}))
}

async fn accessibility(env: &Env, tab: &Tab) -> Result<(Value, f64, Rgb)> {
    let model = values(tab, START, 0).await?;
    let order = [THEME, RTL, LARGE, ALL];
    for id in order {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    let nodes = checkbox_nodes(tab, START).await?;
    check_eq!(
        ax_property(&nodes[0], "focused"),
        Some(&json!(true)),
        "Chrome AX owns keyboard focus"
    );
    let described = validation(tab, &nodes[4], true).await?;
    let mut layouts = Vec::with_capacity(IDS.len());
    for id in IDS {
        layouts.push(geometry(tab, id, false).await?);
    }

    let focused = &layouts[0];
    let frame = capture(env, tab, "wide-focused").await?;
    let (cx, cy) = center(focused.art);
    let palette = &model["palette"];
    let mixed_fill = frame.sample(cx, cy - 5.)?;
    let mixed_mark = frame.sample(cx, cy)?;
    let focus_ring = frame.sample(cx, focused.ring[1] + 1.5)?;
    let focus_layer = frame.sample(cx + 14., cy)?;
    let focus_background = frame.sample(cx + 27., cy)?;
    let on_primary = token(palette, "on_primary")?;
    check_eq!(
        mixed_fill,
        token(palette, "primary")?,
        "the mixed box paints its selected fill"
    );
    check_eq!(
        mixed_mark,
        on_primary,
        "the mixed box paints a visible dash"
    );
    check_eq!(
        focus_ring,
        token(palette, "secondary")?,
        "keyboard focus paints the Secondary ring"
    );
    check!(
        focus_layer != focus_background && focus_layer != mixed_fill,
        "focus paints a translucent state layer outside the box"
    );
    let check_ink = frame
        .colors(layouts[1].art)?
        .get(&on_primary)
        .copied()
        .unwrap_or_default();
    check!(
        check_ink > 16,
        "the checked child paints its On Primary mark"
    );
    let off = layouts[2].art;
    let off_outline = frame.sample(center(off).0, off[1] + 1.)?;
    check_eq!(
        off_outline,
        token(palette, "on_surface_variant")?,
        "the empty checkbox paints its outline"
    );
    values(tab, START, 0).await?;
    let wide_label_height = layouts[4].label[3];
    Ok((
        json!({
            "state":model, "nodes":nodes, "invalid":described, "tab_order_to_all":order,
            "layouts": layouts, "capture": "material-checkbox-wide-focused.png",
            "paint": {"mixed_fill":mixed_fill,"mixed_mark":mixed_mark,"focus_ring":focus_ring,
                      "focus_layer":focus_layer,"focus_background":focus_background,
                      "checked_mark_pixels":check_ink,"off_outline":off_outline},
        }),
        wide_label_height,
        focus_layer,
    ))
}

async fn keyboard(tab: &Tab) -> Result<Value> {
    tab.press("Space").await?;
    let space = values(tab, ALL_ON, 1).await?;
    focused(tab, ALL).await?;
    let all_selected = checkbox_nodes(tab, ALL_ON).await?;
    tab.press("Tab").await?;
    focused(tab, NOTES).await?;
    tab.press("Enter").await?;
    let enter = values(tab, CHILD_OFF, 2).await?;
    focused(tab, NOTES).await?;
    let partial = checkbox_nodes(tab, CHILD_OFF).await?;
    for id in [ATTACHMENTS, CONSENT] {
        tab.press("Tab").await?;
        focused(tab, id).await?;
    }
    tab.press("Shift+Tab").await?;
    focused(tab, ATTACHMENTS).await?;
    values(tab, CHILD_OFF, 2).await?;
    Ok(json!({
        "space":{"id":ALL,"state":space,"nodes":all_selected},
        "enter":{"id":NOTES,"state":enter,"nodes":partial},
        "next_tab":[ATTACHMENTS, CONSENT],"reverse_tab":ATTACHMENTS,
    }))
}

async fn contact(tab: &Tab, id: &str) -> Result<(f64, f64)> {
    let p = point(tab, id).await?;
    let area: Rect =
        p.b.try_into()
            .map_err(|_| anyhow::anyhow!("hit bounds for {id}"))?;
    let scroller = bounds(tab, SCROLL).await?;
    check!(
        contains(scroller, area) && contains([0., 0., p.canvas[2], p.canvas[3]], area),
        "{id} is visible within the independent scroll view before physical contact: {area:?}"
    );
    Ok((p.x, p.y))
}

async fn pointer(env: &Env, tab: &Tab, keyboard_layer: Rgb) -> Result<Value> {
    let label = format!("{NOTES}-label");
    let (x, y) = contact(tab, &label).await?;
    tab.click(x, y).await?;
    let label_state = values(tab, ALL_ON, 3).await?;
    focus(tab, NOTES, "surface").await?;

    let (x, y) = contact(tab, &format!("{ALL}-box")).await?;
    tab.mouse_move(x, y).await?;
    tab.mouse_down().await?;
    interactions(tab, Some(ALL)).await?;
    focus(tab, ALL, "surface").await?;
    let held = values(tab, ALL_ON, 3).await?;
    let layout = geometry(tab, ALL, false).await?;
    let image = capture(env, tab, "wide-held").await?;
    let (cx, cy) = center(layout.art);
    let press_layer = image.sample(cx + 14., cy)?;
    let bare = image.sample(cx + 27., cy)?;
    let ring = image.sample(cx, layout.ring[1] + 1.5)?;
    check!(
        press_layer != keyboard_layer && press_layer != bare,
        "physical hold changes the same state layer painted during keyboard focus"
    );
    check_eq!(
        ring,
        token(&held["palette"], "secondary")?,
        "held focus remains visible"
    );

    let [left, top, _, _] = tab.canvas_box().await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":left+1.,"y":top+1.,"button":"none","buttons":1}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseReleased","x":left+1.,"y":top+1.,"button":"left","buttons":0,"clickCount":1}),
    )
    .await?;
    interactions(tab, None).await?;
    let outside = values(tab, ALL_ON, 3).await?;

    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":x,"y":y,"id":1}]}),
    )
    .await?;
    interactions(tab, Some(ALL)).await?;
    values(tab, ALL_ON, 3).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchCancel","touchPoints":[]}),
    )
    .await?;
    interactions(tab, None).await?;
    let canceled = values(tab, ALL_ON, 3).await?;
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;

    tab.click(x, y).await?;
    let box_state = values(tab, ALL_OFF, 4).await?;
    focus(tab, ALL, "surface").await?;
    let (x, y) = contact(tab, &format!("{LOCKED}-box")).await?;
    tab.click(x, y).await?;
    values(tab, ALL_OFF, 4).await?;
    let nodes = checkbox_nodes(tab, ALL_OFF).await?;
    Ok(json!({
        "label_click":{"id":NOTES,"target":label,"host":"surface","state":label_state},
        "pointer_held":{"id":ALL,"host":"surface","state":held,"layout":layout,
                        "layer_rgb":press_layer,"background_rgb":bare,"ring_rgb":ring},
        "mouse_release_outside":outside,"touch_cancel":canceled,
        "box_click":{"id":ALL,"host":"surface","state":box_state},"disabled_pointer":LOCKED,
        "nodes_after":nodes,"capture":"material-checkbox-wide-held.png",
    }))
}

async fn reveal_help(tab: &Tab) -> Result<(Rect, Rect, bool)> {
    let scroller = bounds(tab, SCROLL).await?;
    let before = bounds(tab, HELP).await?;
    let needs_scroll = !contains(scroller, before);
    if needs_scroll {
        let p = point(tab, SCROLL).await?;
        tab.mouse_move(p.x, p.y).await?;
        tab.wheel(0., 800.).await?;
        tab.wait_for(&format!(
            "(() => {{ const b = window.gallery.control_bounds({HELP:?});
                return b[1] + b[3] <= {} && b[1] < {};
            }})()",
            bottom(scroller) + 0.5,
            before[1]
        ))
        .await?;
    }
    focused(tab, LARGE).await?;
    focus_projected(tab, CONSENT).await?;
    let help = bounds(tab, HELP).await?;
    let clip = bounds(tab, SCROLL).await?;
    check_eq!(
        clip,
        scroller,
        "scrolling and focus keep the viewport fixed"
    );
    check!(
        contains(clip, help),
        "the scroll view contains the complete help text: {help:?}"
    );
    Ok((clip, help, needs_scroll))
}

async fn compact(env: &Env, tab: &Tab, wide_label_height: f64) -> Result<Value> {
    for id in [THEME, RTL, LARGE] {
        activate(tab, id).await?;
    }
    until(
        tab,
        "s.dark && s.checkbox_demo.rtl && s.checkbox_demo.text_scale === 2",
    )
    .await?;
    resize(tab, 320, 640).await?;
    let (clip, help, scrolled) = reveal_help(tab).await?;
    let before = geometry(tab, CONSENT, true).await?;
    check!(
        contains(clip, before.row) && help[1] >= bottom(before.row),
        "the wrapped consent and its below-row help stay in the clip"
    );
    check!(
        before.label[3] >= 96. && before.label[3] > 2. * wide_label_height,
        "the 200% consent label actually wraps to several lines: {:?}",
        before.label
    );
    let invalid_state = values(tab, ALL_OFF, 4).await?;
    let tree = ax_tree(tab).await?;
    let invalid_node = ax(&tree, "checkbox", NAMES[4])?;
    check_eq!(
        ax_property(invalid_node, "checked"),
        Some(&json!("false")),
        "consent starts unchecked"
    );
    let invalid = validation(tab, invalid_node, true).await?;
    let invalid_frame = capture(env, tab, "compact-invalid").await?;
    let error = token(&invalid_state["palette"], "error")?;
    let text = token(&invalid_state["palette"], "on_surface")?;
    let (cx, _) = center(before.art);
    let invalid_ink = invalid_frame.sample(cx, before.art[1] + 1.)?;
    check_eq!(
        invalid_ink,
        error,
        "the invalid box paints its Error outline"
    );
    let help_ink = invalid_frame
        .colors(help)?
        .get(&error)
        .copied()
        .unwrap_or_default();
    check!(help_ink > 24, "the visible helper text paints Error ink");
    let [lx, ly, lw, _] = before.label;
    let mut label_lines = Vec::new();
    for line in 0..2 {
        let line_y = ly + f64::from(line) * 48.;
        let pixels = invalid_frame
            .colors([lx, line_y, lw, 48.])?
            .get(&text)
            .copied()
            .unwrap_or_default();
        check!(
            pixels > 24,
            "wrapped line {} actually paints text",
            line + 1
        );
        label_lines.push(pixels);
    }

    let (x, y) = contact(tab, &format!("{CONSENT}-label")).await?;
    tab.click(x, y).await?;
    let accepted = values(tab, ACCEPTED, 5).await?;
    focus(tab, CONSENT, "surface").await?;
    let after = geometry(tab, CONSENT, true).await?;
    let tree = ax_tree(tab).await?;
    let resolved_node = ax(&tree, "checkbox", NAMES[4])?;
    check_eq!(
        ax_property(resolved_node, "checked"),
        Some(&json!("true")),
        "consent is checked"
    );
    let resolved = validation(tab, resolved_node, false).await?;
    let frame = capture(env, tab, "compact-resolved").await?;
    let (ax, ay) = center(after.art);
    let primary = frame.sample(ax, ay - 5.)?;
    check_eq!(
        primary,
        token(&accepted["palette"], "primary")?,
        "the accepted box paints Primary fill"
    );
    let check_color = token(&accepted["palette"], "on_primary")?;
    let tick_pixels = frame
        .colors(after.art)?
        .get(&check_color)
        .copied()
        .unwrap_or_default();
    check!(tick_pixels > 16, "the resolved checkbox paints its tick");
    check!(
        near(cx, ax),
        "accepting preserves the physical RTL art column"
    );

    resize(tab, 960, 1800).await?;
    activate(tab, BACK).await?;
    until(tab, "!s.checkbox_demo.visible").await?;
    focused(tab, SHOW).await?;
    values(tab, ACCEPTED, 5).await?;
    Ok(json!({
        "viewport":[320,640],"dark":true,"clip":clip,"scrolled_to_help":scrolled,
        "wide_label_height":wide_label_height,
        "invalid":{"state":invalid_state,"layout":before,"help":help,"node":invalid_node,
                   "semantics":invalid,"outline_rgb":invalid_ink,"help_ink_pixels":help_ink,
                   "line_ink_pixels":label_lines,"capture":"material-checkbox-compact-invalid.png"},
        "resolved":{"state":accepted,"layout":after,"node":resolved_node,"semantics":resolved,
                    "label_contact":[x,y],"host":"surface","fill_rgb":primary,
                    "check_ink_pixels":tick_pixels,"capture":"material-checkbox-compact-resolved.png"},
        "focus_restored":SHOW,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step("M278", "real Chrome checkbox states, disabled mixed value, error description and painted focus");
        route(&tab, url).await.context("M278 route")?;
        report["reduced_motion"] = tab.eval("matchMedia('(prefers-reduced-motion: reduce)').matches").await?;
        check_eq!(report["reduced_motion"], true, "static paint checks use reduced motion");
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter();if(!a)return null;const {isFallbackAdapter,vendor,architecture}=a.info;return {isFallbackAdapter,vendor,architecture};})()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(), "WebGPU reports actual adapter metadata");
        let (accessibility, wide_label_height, keyboard_layer) = accessibility(env, &tab).await.context("M278")?;
        report["accessibility"] = accessibility;
        step("M279", "Space and Enter each send one request, update parent and children, and skip the disabled checkbox");
        report["keyboard"] = keyboard(&tab).await.context("M279")?;
        step("M280", "physical label and box clicks, pressed paint, outside release and touch cancellation");
        report["pointer"] = pointer(env, &tab, keyboard_layer).await.context("M280")?;
        step("M281", "320 dp dark RTL at 200% keeps the wrapped label and help visible, then retires the error");
        report["compact"] = compact(env, &tab, wide_label_height).await.context("M281")?;
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }.await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        report["dom_at_failure"] = tab
            .eval_json("window.gallery.export_dom()")
            .await
            .unwrap_or(Value::Null);
        report["focus_host_at_failure"] = tab
            .eval("document.activeElement?.id ?? null")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact(&format!("{STEM}-failure.png")))
            .await;
    }
    env.write_json(&format!("browser-{STEM}.json"), &report)?;
    result
}
