//! CDP-emulated browser display contrast through real headless WebGPU and AX.

use super::material_web_contrast_png as pixels;
#[path = "material_web_contrast_selection.rs"]
mod selection;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize,
    },
    state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};
use pixels::{intersection, CanvasPng, Rect, Rgb};

const POPUP_SHOW: &str = "scroll-popup-demo-show";
const POPUP: &str = "scroll-popup-demo-dialog";
const BODY: &str = "scroll-popup-demo-body";
const OPENER: &str = "scroll-popup-demo-opener";
const CLOSE: &str = "scroll-popup-demo-close";
const EDITOR: &str = "scroll-popup-demo-quantity-editor";
const FACE: &str = "scroll-popup-demo-quantity-face";
const RING: &str = "scroll-popup-demo-quantity-focus";
const WHEEL_SHOW: &str = "scroll-field-demo-show";
const WHEEL: &str = "scroll-field-demo-value-adjustable";
const WHEEL_FACE: &str = "scroll-field-demo-value-face";
const WHEEL_RING: &str = "scroll-field-demo-value-focus";
const WHEEL_SCROLL: &str = "scroll-field-demo-scroll";
const WHEEL_CENTER: &str = "scroll-field-demo-value-center";
const WHEEL_ABOVE: &str = "scroll-field-demo-value-above";
const WHEEL_BELOW: &str = "scroll-field-demo-value-below";
const WHEEL_DISABLED: &str = "scroll-field-demo-disabled";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";

// Resolve browser keywords independently of the application's probe. Convert
// the owner's canonical sRGB CSS strings directly to bytes. Reading pixels
// back through an alpha-premultiplied 2D buffer would round Highlight twice.
const CSS_WITNESS: &str = r#"(() => {
    const canvas = document.getElementById("surface");
    const owner = canvas.ownerDocument;
    const view = owner.defaultView;
    const parent = canvas.parentElement;
    const witness = owner.createElement("span");
    witness.style.cssText = "position:fixed;visibility:hidden;pointer-events:none;forced-color-adjust:auto";
    const scheme = view.getComputedStyle(canvas).getPropertyValue("color-scheme");
    witness.style.setProperty("color-scheme", scheme, "important");
    parent.appendChild(witness);
    const byte = (part, scale) => {
        const fraction = part.endsWith("%") ? Number(part.slice(0,-1))/100 : Number(part)/scale;
        if (!Number.isFinite(fraction) || fraction < 0 || fraction > 1)
            throw Error("Unexpected system CSS channel " + part);
        return Math.round(fraction * 255);
    };
    const rgba = css => {
        const match = css.trim().match(/^(rgba?|color)\(([^)]+)\)$/);
        if (!match) throw Error("Unsupported canonical system color " + css);
        const parts = match[2].trim().split(/[\s,\/]+/);
        let scale = 255;
        if (match[1] === "color") {
            if (parts.shift() !== "srgb") throw Error("Expected sRGB witness " + css);
            scale = 1;
        }
        if (parts.length !== 3 && parts.length !== 4) throw Error("Unexpected CSS channels " + css);
        return parts.slice(0,3).map(v => byte(v,scale)).concat(parts.length === 4 ? byte(parts[3],1) : 255);
    };
    try {
        const pairs = {}, raw = {};
        for (const [key, background, foreground] of [
            ["canvas","Canvas","CanvasText"],
            ["button","ButtonFace","ButtonText"],
            ["highlight","Highlight","HighlightText"]
        ]) {
            const item = owner.createElement("span");
            item.style.setProperty("background-color", background, "important");
            item.style.setProperty("color", foreground, "important");
            witness.appendChild(item);
            const css = view.getComputedStyle(item);
            raw[key] = {background:css.backgroundColor,foreground:css.color};
            pairs[key] = {background:rgba(raw[key].background),foreground:rgba(raw[key].foreground)};
        }
        const disabled = owner.createElement("span");
        disabled.style.setProperty("color", "GrayText", "important");
        witness.appendChild(disabled);
        const grayCss = disabled.style.getPropertyValue("color")
            ? view.getComputedStyle(disabled).getPropertyValue("color") : null;
        return {scheme, pairs, raw, disabled_text: grayCss === null ? null :
            {css: grayCss, rgba: rgba(grayCss)} };
    } finally {
        witness.remove();
    }
})()"#;

pub(super) async fn media(tab: &Tab, forced: bool, more: bool) -> Result<()> {
    let forced_value = if forced { "active" } else { "none" };
    let contrast_value = if more { "more" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[
            {"name":"forced-colors","value":forced_value},
            {"name":"prefers-contrast","value":contrast_value},
            {"name":"prefers-reduced-motion","value":"reduce"}
        ]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia('(forced-colors: active)').matches === {forced} &&
         matchMedia('(prefers-contrast: more)').matches === {more} &&
         matchMedia('(prefers-reduced-motion: reduce)').matches"
    ))
    .await
}

pub(super) async fn contrast(tab: &Tab, mode: &str) -> Result<Value> {
    until(tab, &format!("s.display_contrast?.mode === {}", js(mode))).await?;
    tab.settle().await?;
    Ok(state(tab).await?["display_contrast"].take())
}

pub(super) async fn palette(tab: &Tab, appearance: &Value) -> Result<Value> {
    let mut css = palette_with_disabled(tab, appearance).await?;
    css.as_object_mut()
        .context("CSS palette witness")?
        .remove("disabled_text");
    Ok(css)
}

pub(super) async fn palette_with_disabled(tab: &Tab, appearance: &Value) -> Result<Value> {
    let css = tab.eval(CSS_WITNESS).await?;
    let pairs = &css["pairs"];
    let theme = &appearance["theme"];
    check_eq!(
        appearance["system_colors"],
        json!({"canvas":pairs["canvas"],"button":pairs["button"],"highlight":pairs["highlight"],
            "disabled_text":css["disabled_text"]["rgba"]}),
        "the three accepted system pairs and optional GrayText match independent browser measurements"
    );
    for (role, source) in [
        ("surface", &pairs["canvas"]["background"]),
        ("surface_container", &pairs["canvas"]["background"]),
        ("surface_container_lowest", &pairs["canvas"]["background"]),
        ("on_surface", &pairs["canvas"]["foreground"]),
        ("focus", &pairs["canvas"]["foreground"]),
        ("outline", &pairs["canvas"]["foreground"]),
        ("error", &pairs["canvas"]["foreground"]),
        ("tonal", &pairs["button"]["background"]),
        ("on_tonal", &pairs["button"]["foreground"]),
    ] {
        check_eq!(
            &theme[role],
            source,
            "forced Theme role {role} retains the browser pair"
        );
    }
    Ok(css)
}

async fn no_probe(tab: &Tab) -> Result<()> {
    tab.wait_for("document.querySelectorAll('[data-surface-system-colors]').length === 0")
        .await
}

async fn hidden_probe(tab: &Tab) -> Result<()> {
    tab.wait_for(
        r#"(() => {
            const roots = document.querySelectorAll("[data-surface-system-colors]");
            if (roots.length !== 1) return false;
            const [root] = roots;
            let style = getComputedStyle(root);
            return root.getAttribute("aria-hidden") === "true" && style.visibility === "hidden"
                && style.pointerEvents === "none"
                && root.querySelectorAll("[data-surface-system-pair]").length === 3;
        })()"#,
    )
    .await?;
    let document = tab.cdp("DOM.getDocument", json!({})).await?;
    let node_id = document["root"]["nodeId"]
        .as_u64()
        .context("Chrome DOM root")?;
    let nodes = tab
        .cdp(
            "DOM.querySelectorAll",
            json!({"nodeId":node_id,
                "selector":"[data-surface-system-colors], [data-surface-system-pair]"}),
        )
        .await?;
    let probes = nodes["nodeIds"]
        .as_array()
        .context("hidden probe DOM nodes")?;
    check_eq!(
        probes.len(),
        4,
        "one DOM probe owns exactly three sample children"
    );
    let tree = ax_tree(tab).await?;
    for node in probes {
        let info = tab.cdp("DOM.describeNode", json!({"nodeId":node})).await?;
        let backend = &info["node"]["backendNodeId"];
        check!(
            !backend.is_null()
                && !tree.iter().any(|entry| {
                    entry["ignored"] == false && entry["backendDOMNodeId"] == *backend
                }),
            "no mounted system color probe node is exposed in the Chrome AX tree"
        );
    }
    Ok(())
}

async fn initial_preferences(tab: &Tab, url: &str) -> Result<Value> {
    step("M241", "the first ready browser state accepts Forced over More and keeps all three exact system pairs");
    media(tab, true, true).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    check_eq!(
        state(tab).await?["display_contrast"]["mode"],
        json!("forced"),
        "Forced is already accepted when the browser's Gallery becomes ready"
    );
    let startup = contrast(tab, "forced").await?;
    let startup_css = palette(tab, &startup).await?;
    hidden_probe(tab).await?;

    media(tab, false, true).await?;
    let increased = contrast(tab, "increased").await?;
    check!(
        increased["system_colors"].is_null(),
        "Increased does not take a forced palette"
    );
    no_probe(tab).await?;
    media(tab, false, false).await?;
    let normal = contrast(tab, "normal").await?;
    check!(
        normal["system_colors"].is_null() && normal["theme"] != increased["theme"],
        "Increased and Normal supply different real Gallery theme roles"
    );
    no_probe(tab).await?;
    media(tab, true, true).await?;
    let forced = contrast(tab, "forced").await?;
    let ending_css = palette(tab, &forced).await?;
    hidden_probe(tab).await?;
    Ok(json!({"modes":["forced","increased","normal","forced"],
        "startup":{"css":startup_css,"theme":startup["theme"]},
        "ending":{"css":ending_css,"theme":forced["theme"]},
        "normal_theme":normal["theme"],"increased_theme":increased["theme"],
        "probe_hidden_and_ax_excluded":true,"probe_removed_on_exit":true}))
}

async fn scene(tab: &Tab, url: &str, show: &str) -> Result<()> {
    media(tab, false, false).await?;
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-0").await?;
    until(tab, "s.section === 0").await?;
    activate(tab, show).await?;
    contrast(tab, "normal").await?;
    resize(tab, 960, 820).await
}

async fn popup_state(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.scroll_popup)")).await?;
    tab.settle().await
}

async fn editor_focus(tab: &Tab) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(EDITOR))).await?;
    tab.wait_for(&format!(
        "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
    ))
    .await?;
    tab.settle().await
}

async fn pointer_away(tab: &Tab) -> Result<()> {
    tab.mouse_move(4., 16.).await?;
    tab.settle().await
}

async fn popup(tab: &Tab, url: &str) -> Result<()> {
    scene(tab, url, POPUP_SHOW).await?;
    for choice in ["theme", "rtl", "large-text", "bottom"] {
        activate(tab, &format!("scroll-popup-demo-{choice}")).await?;
    }
    until(tab, "s.dark === true").await?;
    resize(tab, 320, 400).await?;
    popup_state(
        tab,
        "d.visible && !d.open && d.rtl && d.text_scale === 2 && d.anchor !== null",
    )
    .await?;
    activate(tab, OPENER).await?;
    popup_state(tab, "d.open && d.above && d.quantity.mode === 'input'").await?;
    editor_focus(tab).await?;
    pointer_away(tab).await
}

fn opaque(value: &Value, role: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(value.clone())
        .with_context(|| format!("theme role {role} has a byte RGBA color"))?;
    check_eq!(a, 255, "the measured browser paint role {role} is opaque");
    Ok([r, g, b])
}

fn role(theme: &Value, name: &str) -> Result<Rgb> {
    opaque(&theme[name], name)
}

#[derive(Clone, Copy)]
struct PopupPaint {
    body: Rect,
    panel: Rect,
    face: Rect,
    ring: Rect,
    opener: Rect,
}

impl PopupPaint {
    async fn read(tab: &Tab) -> Result<Self> {
        let geometry = Self {
            body: bounds(tab, BODY).await?,
            panel: bounds(tab, POPUP).await?,
            face: bounds(tab, FACE).await?,
            ring: bounds(tab, RING).await?,
            opener: bounds(tab, OPENER).await?,
        };
        check!(
            contains(geometry.body, geometry.ring) && contains(geometry.ring, geometry.face),
            "the actual focused outline and field face fit the popup body"
        );
        Ok(geometry)
    }

    fn rects(self) -> [Rect; 5] {
        [self.body, self.panel, self.face, self.ring, self.opener]
    }

    fn sample(self, image: &CanvasPng, theme: &Value) -> Result<Value> {
        let [px, py, pw, _] = self.panel;
        let [fx, fy, fw, fh] = self.face;
        let [rx, ry, rw, rh] = self.ring;
        let [bx, by, bw, bh] = self.opener;
        let root = image.sample(3., 397.)?;
        let panel = image.sample(px + pw / 2., py + 4.)?;
        let field = image.sample(fx + 0.88 * fw, fy + 0.75 * fh)?;
        let button = image.sample(bx + 12., by + bh / 2.)?;
        let top = image.sample(rx + rw / 2., ry + 1.5)?;
        let bottom = image.sample(rx + rw / 2., ry + rh - 1.5)?;
        let gap = image.sample(rx + rw / 2., ry + 4.)?;
        let button_text = image.count_color(
            [bx + 18., by + 8., bw - 36., bh - 16.],
            role(theme, "on_tonal")?,
        )?;
        for (actual, token) in [
            (root, "surface"),
            (panel, "surface_container"),
            (field, "surface_container_lowest"),
            (button, "tonal"),
            (top, "focus"),
            (bottom, "focus"),
        ] {
            check_eq!(actual, role(theme, token)?, "actual WebGPU {token} pixel");
        }
        check!(
            top != gap && button_text > 0,
            "the outline has a real gap and the tonal opener contains its foreground ink"
        );
        Ok(
            json!({"root":root,"panel":panel,"field":field,"button":button,
            "focus_top":top,"focus_bottom":bottom,"focus_gap":gap,"button_text_pixels":button_text}),
        )
    }
}

async fn popup_palette_pixels(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M242",
        "live browser system pairs recolor actual popup Canvas paint and then restore Normal",
    );
    popup(tab, url).await?;
    tab.press("End").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === {TEXTAREA}.selectionEnd"
    ))
    .await?;
    let layout = PopupPaint::read(tab).await?;
    let mut samples = Vec::new();
    for (name, forced) in [("normal", false), ("forced", true), ("restored", false)] {
        media(tab, forced, false).await?;
        let theme = contrast(tab, if forced { "forced" } else { "normal" }).await?;
        let css = if forced {
            palette(tab, &theme).await?
        } else {
            tab.eval(CSS_WITNESS).await?
        };
        until(tab, "s.dark === true").await?;
        editor_focus(tab).await?;
        let current = PopupPaint::read(tab).await?;
        check!(
            layout
                .rects()
                .into_iter()
                .flatten()
                .zip(current.rects().into_iter().flatten())
                .all(|(a, b)| near(a, b)),
            "preference changes repaint the popup without shifting its measured controls"
        );
        let image = CanvasPng::capture(env, tab, &format!("popup-{name}")).await?;
        let paint = current.sample(&image, &theme["theme"])?;
        samples.push(
            json!({"mode":name,"css":css,"accepted":theme,"paint":paint,"size":image.dimensions()}),
        );
    }
    let before = &samples[0]["paint"];
    let forced = &samples[1]["paint"];
    let after = &samples[2]["paint"];
    for role in [
        "root",
        "panel",
        "field",
        "button",
        "focus_top",
        "focus_bottom",
        "focus_gap",
    ] {
        check_eq!(
            &before[role],
            &after[role],
            "normal {role} pixels recover on exit"
        );
    }
    check!(
        before["root"] != forced["root"]
            && before["field"] != forced["field"]
            && before["focus_top"] != forced["focus_top"],
        "genuinely changed Forced pixels replace the old indistinguishable editor capture"
    );
    Ok(
        json!({"viewport":[320,400],"rtl":true,"text_scale":2,"authored_dark":true,
        "selection_paint_measured":false,"geometry":layout.rects(),"samples":samples}),
    )
}

async fn editing_snapshot(tab: &Tab) -> Result<Value> {
    editor_focus(tab).await?;
    let snapshot = tab.eval(&format!(
        r#"(() => {{
            const runtime = JSON.parse(window.gallery.inspect());
            const editor = JSON.parse(window.gallery.export_dom()).text_inputs[{}];
            const node = {TEXTAREA};
            const popup = runtime.scroll_popup;
            return {{editor, owner:runtime.focused, selection:[node.selectionStart,node.selectionEnd],
                open:popup.open,rtl:popup.rtl,text_scale:popup.text_scale,
                quantity:popup.quantity,unit:popup.unit,
                panel:popup.bounds,anchor:popup.anchor,body_scroll:popup.body_scroll_top}};
        }})()"#,
        js(EDITOR)
    )).await?;
    check!(
        snapshot["editor"]["composing"] == true
            && snapshot["editor"]["preedit_range"].is_object()
            && snapshot["editor"]["value"] == "1203"
            && snapshot["editor"]["display_value"] != snapshot["editor"]["value"]
            && snapshot["editor"]["selection_anchor"].is_number()
            && snapshot["editor"]["selection_cursor"].is_number()
            && snapshot["owner"] == EDITOR
            && snapshot["open"] == true
            && snapshot["rtl"] == true
            && snapshot["text_scale"] == 2
            && snapshot["quantity"]["raw"] == "1203"
            && snapshot["quantity"]["value"] == 1200
            && snapshot["quantity"]["error"] == "OffStep"
            && snapshot["quantity"]["commits"] == 0
            && snapshot["unit"]["id"] == "ml"
            && snapshot["unit"]["commits"] == 0,
        "selection, preedit and both accepted fields remain independently observable: {snapshot}"
    );
    Ok(snapshot)
}

async fn popup_invalid_ax(tab: &Tab) -> Result<Value> {
    let aria = tab
        .eval(&format!(
            r#"(() => {{
        const node = {TEXTAREA};
        const refs = (node.getAttribute('aria-describedby') ?? '').split(/\s+/).filter(Boolean);
        return {{invalid:node.getAttribute('aria-invalid'),
            descriptions:refs.map(id=>document.getElementById(id)?.textContent ?? null)}};
    }})()"#
        ))
        .await?;
    check!(
        aria["invalid"] == "true"
            && aria["descriptions"].as_array().is_some_and(|refs| {
                refs.iter().any(|description| description == "請輸入偶數")
            }),
        "the mounted textarea keeps its resolved invalid description"
    );
    let tree = ax_tree(tab).await?;
    let dialog = ax(&tree, "dialog", "編輯容量")?;
    let editor = ax(&tree, "textbox", "數量")?;
    let outside = ax(&tree, "button", "外部 0 次")?;
    check!(
        ax_property(dialog, "modal") != Some(&json!(true))
            && ax_property(outside, "disabled") != Some(&json!(true))
            && matches!(ax_property(editor, "invalid"), Some(value) if value == &json!(true) || value == &json!("true"))
            && ax_property(editor, "focused") == Some(&json!(true))
            && editor["description"]["value"].as_str().is_some_and(|text| text.contains("請輸入偶數")),
        "Chrome AX keeps the named modeless scope, focused invalid editor and actionable outside button"
    );
    Ok(
        json!({"dom":aria,"chrome_description":editor["description"]["value"],"chrome_focused":true}),
    )
}

async fn synthetic_preedit(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M243",
        "live contrast preserves focused invalid popup drafts and active synthetic preedit",
    );
    popup(tab, url).await?;
    tab.shortcut("a").await?;
    tab.insert_text("1203").await?;
    tab.press("Enter").await?;
    popup_state(
        tab,
        "d.open && d.quantity.raw === '1203' && d.quantity.error === 'OffStep'",
    )
    .await?;
    tab.shortcut("a").await?;
    tab.ime_compose("１２", 2, 2).await?;
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(EDITOR)
    );
    tab.wait_for(&composing).await?;
    let before = editing_snapshot(tab).await?;
    let mut named_editor = vec![popup_invalid_ax(tab).await?];
    for forced in [true, false] {
        media(tab, forced, false).await?;
        let appearance = contrast(tab, if forced { "forced" } else { "normal" }).await?;
        if forced {
            palette(tab, &appearance).await?;
        }
        check_eq!(
            editing_snapshot(tab).await?,
            before,
            "live CDP contrast changes keep exact focus, UTF-8 selection, preedit and drafts"
        );
        named_editor.push(popup_invalid_ax(tab).await?);
    }
    CanvasPng::capture(env, tab, "popup-synthetic-preedit").await?;

    click(tab, CLOSE).await?;
    check_eq!(
        editing_snapshot(tab).await?,
        before,
        "Close leaves active synthetic composition untouched"
    );
    tab.press("Escape").await?;
    popup_state(
        tab,
        "d.open && d.quantity.commits === 0 && d.unit.commits === 0",
    )
    .await?;
    editor_focus(tab).await?;
    if tab.eval(&composing).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    tab.shortcut("a").await?;
    tab.insert_text("1206").await?;
    tab.press("Enter").await?;
    popup_state(tab, "d.open && d.quantity.value === 1206 && d.quantity.commits === 1 && d.unit.id === 'ml' && d.unit.commits === 0").await?;
    click(tab, CLOSE).await?;
    popup_state(
        tab,
        "!d.open && d.quantity.value === 1206 && d.quantity.commits === 1",
    )
    .await?;
    until(tab, &format!("s.focused === {}", js(OPENER))).await?;
    Ok(
        json!({"composition":"CDP synthetic preedit","selection_paint_measured":false,
        "snapshot":before,"chrome_modes":named_editor,"close_during_preedit_ignored":true,
        "recovered_quantity":1206,"quantity_commits":1,"returned_focus":OPENER}),
    )
}

fn boundary(
    image: &CanvasPng,
    outer: [f64; 2],
    edge: [[f64; 2]; 3],
    inner: [f64; 2],
    background: Rgb,
    ink: Rgb,
) -> Result<Value> {
    let exterior = image.sample(outer[0], outer[1])?;
    let stroke = [
        image.sample(edge[0][0], edge[0][1])?,
        image.sample(edge[1][0], edge[1][1])?,
        image.sample(edge[2][0], edge[2][1])?,
    ];
    let interior = image.sample(inner[0], inner[1])?;
    let present = background != ink
        && exterior == background
        && interior == background
        && stroke.contains(&ink);
    Ok(
        json!({"exterior":exterior,"edge":stroke,"interior":interior,
        "expected_fill":background,"expected_ink":ink,"border_present":present}),
    )
}

async fn popup_boundary(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M244",
        "the popup edge remains visible when its forced fill and the surrounding Canvas match",
    );
    popup(tab, url).await?;
    media(tab, true, false).await?;
    let forced = contrast(tab, "forced").await?;
    let css = palette(tab, &forced).await?;
    let [x, y, width, _] = bounds(tab, POPUP).await?;
    let mid = x + width / 2.;
    let image = CanvasPng::capture(env, tab, "popup-boundary").await?;
    let outline = boundary(
        &image,
        [mid, y - 2.],
        [[mid, y + 0.5], [mid, y + 1.5], [mid, y + 2.5]],
        [mid, y + 4.],
        opaque(&css["pairs"]["canvas"]["background"], "Canvas")?,
        opaque(&css["pairs"]["canvas"]["foreground"], "CanvasText")?,
    )?;
    env.write_json("material-web-contrast-popup-boundary.json", &outline)?;
    check!(
        outline["border_present"] == true,
        "the popup silhouette has an actual independent forced border: {outline}"
    );
    Ok(outline)
}

async fn wheel_boundary(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M245",
        "the resting wheel has its own forced edge, clipped neighbor ink and a real disabled state",
    );
    scene(tab, url, WHEEL_SHOW).await?;
    until(
        tab,
        "s.scroll_field.visible && s.scroll_field.fits && s.scroll_field.mode === 'scroll'",
    )
    .await?;
    focus_projected(tab, "scroll-field-demo-medium").await?;
    pointer_away(tab).await?;
    media(tab, true, false).await?;
    let forced = contrast(tab, "forced").await?;
    let css = palette(tab, &forced).await?;
    focused(tab, "scroll-field-demo-medium").await?;
    let face = bounds(tab, WHEEL_FACE).await?;
    let [x, y, _, height] = face;
    let mid = y + height / 2.;
    let image = CanvasPng::capture(env, tab, "wheel-resting").await?;
    let canvas = opaque(&css["pairs"]["canvas"]["background"], "Canvas")?;
    let ink = opaque(&css["pairs"]["canvas"]["foreground"], "CanvasText")?;
    let outline = boundary(
        &image,
        [x - 3., mid],
        [[x + 0.5, mid], [x + 1.5, mid], [x + 2.5, mid]],
        [x + 8., mid],
        canvas,
        ink,
    )?;
    env.write_json("material-web-contrast-wheel-boundary.json", &outline)?;
    let mut labels = Vec::new();
    for id in [WHEEL_ABOVE, WHEEL_CENTER, WHEEL_BELOW] {
        let node = bounds(tab, id).await?;
        let clip = intersection(face, node);
        let count = image.count_color(clip, ink)?;
        check!(
            count > 0 && (id == WHEEL_CENTER || clip[3] < node[3]),
            "wheel glyph {id} keeps its foreground ink within the cropped face: {node:?}/{clip:?}"
        );
        labels.push(json!({"id":id,"bounds":node,"visible_crop":clip,"ink_pixels":count}));
    }
    let enabled_center_ink =
        image.count_color(intersection(face, bounds(tab, WHEEL_CENTER).await?), ink)?;
    focus_projected(tab, WHEEL).await?;
    let ring = bounds(tab, WHEEL_RING).await?;
    let [rx, ry, rw, rh] = ring;
    check!(
        contains(bounds(tab, WHEEL_SCROLL).await?, ring)
            && contains(ring, bounds(tab, WHEEL_FACE).await?),
        "actual keyboard focus reveals the wheel's complete exterior outline"
    );
    let focused_image = CanvasPng::capture(env, tab, "wheel-focused").await?;
    let focus_top = focused_image.sample(rx + rw / 2., ry + 1.5)?;
    let focus_bottom = focused_image.sample(rx + rw / 2., ry + rh - 1.5)?;
    check_eq!(
        [focus_top, focus_bottom],
        [ink, ink],
        "the focused wheel paints both full CanvasText strokes"
    );
    let committed = state(tab).await?["scroll_field"]["committed"].clone();
    activate(tab, WHEEL_DISABLED).await?;
    until(tab, "s.scroll_field.disabled === true").await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-disabled') === 'true'",
        projected(WHEEL)
    ))
    .await?;
    let tree = ax_tree(tab).await?;
    check_eq!(
        ax_property(ax(&tree, "spinbutton", "數量")?, "disabled"),
        Some(&json!(true)),
        "the actual Chrome wheel becomes disabled"
    );
    click(tab, WHEEL).await?;
    until(
        tab,
        "s.scroll_field.disabled && s.scroll_field.mode === 'scroll'",
    )
    .await?;
    check_eq!(
        state(tab).await?["scroll_field"]["committed"],
        committed,
        "disabled wheel rejects the real pointer"
    );
    pointer_away(tab).await?;
    let center = intersection(
        bounds(tab, WHEEL_FACE).await?,
        bounds(tab, WHEEL_CENTER).await?,
    );
    let disabled = CanvasPng::capture(env, tab, "wheel-disabled").await?;
    let disabled_exact_ink = disabled.count_color(center, ink)?;
    check!(
        disabled_exact_ink < enabled_center_ink,
        "disabled wheel paint attenuates the resting CanvasText glyphs"
    );
    media(tab, false, false).await?;
    contrast(tab, "normal").await?;
    no_probe(tab).await?;
    check!(
        outline["border_present"] == true,
        "the unfocused wheel silhouette has an actual independent forced border: {outline}"
    );
    Ok(
        json!({"outline":outline,"labels":labels,"focus":{"ring":ring,"top":focus_top,"bottom":focus_bottom},
        "disabled_aria":true,"accepted":committed,
        "disabled_ink_sample":{"enabled_exact_pixels":enabled_center_ink,
            "disabled_exact_pixels":disabled_exact_ink,"gray_text_claimed":false}}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    tab.cdp("Page.enable", json!({})).await?;
    let preferences = initial_preferences(tab, url).await?;
    let popup_paint = popup_palette_pixels(env, tab, url).await?;
    let preedit = synthetic_preedit(env, tab, url).await?;
    // Keep the two independent edge measurements even if only one component
    // has received a real boundary. Neither failure can be a passing receipt.
    let popup_edge = popup_boundary(env, tab, url)
        .await
        .context("M244 popup silhouette");
    let wheel_edge = wheel_boundary(env, tab, url)
        .await
        .context("M245 wheel silhouette");
    let selection = selection::run(env, chrome, url).await;
    let (popup_edge, wheel_edge) = (popup_edge?, wheel_edge?);
    let selection = selection?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "display contrast page errors: {errors:?}"
    );
    env.write_json("browser-material-web-contrast.json", &json!({
        "browser":chrome.version,"preference_source":"CDP-emulated browser preference",
        "paint_source":"headless Chrome WebGPU Canvas-only PNG","real_cases_used_palette_injection":false,
        "preferences":preferences,"popup_paint":popup_paint,"preedit":preedit,
        "popup_edge":popup_edge,"wheel_edge":wheel_edge,"selection":selection,"errors":errors
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-web-contrast-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "browser_css":tab.eval(CSS_WITNESS).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-web-contrast-failure.png"))
            .await;
    }
    result.context("Material browser forced-color contrast")
}
