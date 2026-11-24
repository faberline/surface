//! Separate exact-pixel test for paired forced-color selection at fractional DPR.

#[path = "material_web_contrast_preedit_underline.rs"]
mod preedit_underline;

use std::ops::Range;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    contrast, editor_focus, media, palette,
    pixels::{CanvasPng, Rect, Rgb},
    popup, popup_state, EDITOR, FACE, TEXTAREA,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::material::material_time_input::{bounds, near};
use crate::{check, check_eq, Env};

const DPR: f64 = 1.25;
const RAW: &str = "1688";
const TEXT_END: usize = 4;
const PREFIX: [usize; 2] = [0, 1];
const MIDDLE: [usize; 2] = [1, 3];
const SUFFIX: [usize; 2] = [3, TEXT_END];
const ALL: [usize; 2] = [0, TEXT_END];
const INJECTED_CANVAS: [[u8; 4]; 2] = [[15, 25, 35, 255], [238, 250, 47, 255]];
const INJECTED_HIGHLIGHT: [[u8; 4]; 2] = [[165, 63, 155, 204], [6, 245, 242, 255]];

// This runs only in a fresh synthetic page. The independent CSS witness
// and all ordinary document elements still use the original browser API.
const INJECTION: &str = r#"
(() => {
    const orig = window.getComputedStyle.bind(window);
    const paints = {
        canvas: {"background-color":"rgb(15,25,35)", color:"rgb(238,250,47)"},
        highlight: {"background-color":"rgba(165,63,155,0.8)", color:"rgb(6,245,242)"}
    };
    window.__surfaceSyntheticSelectionPalette = true;
    window.getComputedStyle = (element, pseudo) => {
        const real = orig(element, pseudo);
        const pair = paints[element?.getAttribute("data-surface-system-pair")];
        if (!pair || pseudo) return real;
        return new Proxy(real, {
            get(target, property) {
                if (property === "getPropertyValue")
                    return key => Object.hasOwn(pair, key) ? pair[key] : target.getPropertyValue(key);
                const result = Reflect.get(target, property, target);
                return typeof result === "function" ? result.bind(target) : result;
            }
        });
    };
})();
"#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lane {
    Browser,
    Injected,
}

impl Lane {
    fn file_prefix(self) -> &'static str {
        match self {
            Self::Browser => "selection-browser",
            Self::Injected => "selection-injected",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::Browser => "CDP-emulated browser system colors",
            Self::Injected => "synthetic injected system-probe colors",
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    canvas: Rgb,
    ordinary_ink: Rgb,
    highlight: [u8; 4],
    selected_ink: Rgb,
}

impl Palette {
    fn from_pairs(pairs: &Value) -> Result<Self> {
        let bytes = |pair: &str, role: &str| -> Result<[u8; 4]> {
            serde_json::from_value(pairs[pair][role].clone())
                .with_context(|| format!("selection color bytes for {pair}.{role}"))
        };
        let opaque = |rgba: [u8; 4]| -> Result<Rgb> {
            let [r, g, b, a] = rgba;
            check_eq!(
                a,
                255,
                "the chosen display palette has opaque Canvas and text roles"
            );
            Ok([r, g, b])
        };
        Ok(Self {
            canvas: opaque(bytes("canvas", "background")?)?,
            ordinary_ink: opaque(bytes("canvas", "foreground")?)?,
            highlight: bytes("highlight", "background")?,
            selected_ink: opaque(bytes("highlight", "foreground")?)?,
        })
    }
}

struct Clip {
    x: Range<usize>,
    y: Range<usize>,
}

impl Clip {
    fn editor(rect: Rect) -> Self {
        let [x, y, width, height] = rect;
        let inner_start = |logical: f64| (logical * DPR).ceil() as usize;
        let inner_end = |logical: f64| (logical * DPR).floor() as usize;
        Self {
            x: inner_start(x + 7.)..inner_end(x + width - 7.),
            y: inner_start(y + 7.)..inner_end(y + height - 7.),
        }
    }
}

#[derive(Clone, Copy)]
enum Blend {
    ByteSpace,
    LinearLight,
}

impl Blend {
    fn name(self) -> &'static str {
        match self {
            Self::ByteSpace => "byte-space source-over",
            Self::LinearLight => "linear-light source-over",
        }
    }

    fn over(self, base: Rgb, highlight: [u8; 4]) -> Rgb {
        let alpha = f64::from(highlight[3]) / 255.;
        std::array::from_fn(|i| {
            let from = f64::from(base[i]) / 255.;
            let to = f64::from(highlight[i]) / 255.;
            let channel = match self {
                Self::ByteSpace => from * (1. - alpha) + to * alpha,
                Self::LinearLight => {
                    encode_srgb(decode_srgb(from) * (1. - alpha) + decode_srgb(to) * alpha)
                }
            };
            (channel * 255.).round() as u8
        })
    }
}

fn decode_srgb(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn encode_srgb(value: f64) -> f64 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1. / 2.4) - 0.055
    }
}

fn matches_fill(actual: Rgb, reference: Rgb) -> bool {
    actual
        .into_iter()
        .zip(reference)
        .all(|(a, b)| a.abs_diff(b) <= 2)
}

fn longest_fill(
    image: &CanvasPng,
    row: usize,
    columns: Range<usize>,
    reference: Rgb,
) -> Result<Range<usize>> {
    let mut longest = 0..0;
    let mut current = columns.start..columns.start;
    for x in columns {
        if matches_fill(image.pixel(x, row)?, reference) {
            current.end = x + 1;
            if current.len() > longest.len() {
                longest = current.clone();
            }
        } else {
            current = x + 1..x + 1;
        }
    }
    Ok(longest)
}

struct Seams {
    axis_row: usize,
    full: Range<usize>,
    middle: Range<usize>,
    seen_fill: Rgb,
    composite: Blend,
    expected_fill: Rgb,
}

struct References<'a> {
    ordinary: &'a CanvasPng,
    all: &'a CanvasPng,
    prefix: &'a CanvasPng,
    suffix: &'a CanvasPng,
}

impl References<'_> {
    fn seams(&self, clip: &Clip, palette: Palette) -> Result<Seams> {
        let mut best: Option<Seams> = None;
        for blend in [Blend::ByteSpace, Blend::LinearLight] {
            let fill = blend.over(palette.canvas, palette.highlight);
            if matches_fill(fill, palette.canvas) {
                continue;
            }
            for row in clip.y.clone() {
                let full = longest_fill(self.all, row, clip.x.clone(), fill)?;
                if full.len() < 64
                    || best
                        .as_ref()
                        .is_some_and(|prior| full.len() <= prior.full.len())
                {
                    continue;
                }
                let first = longest_fill(self.prefix, row, clip.x.clone(), fill)?;
                let last = longest_fill(self.suffix, row, clip.x.clone(), fill)?;
                if full.start != first.start
                    || full.end != last.end
                    || first.len() < 12
                    || last.len() < 12
                    || first.end + 24 > last.start
                {
                    continue;
                }
                let middle = first.end..last.start;
                // Do not learn the seam from the actual partial image.
                let probe = middle.start + middle.len() / 2;
                if self.ordinary.pixel(probe, row)? != palette.canvas {
                    continue;
                }
                best = Some(Seams {
                    axis_row: row,
                    full,
                    middle,
                    seen_fill: self.all.pixel(probe, row)?,
                    composite: blend,
                    expected_fill: fill,
                });
            }
        }
        best.context("no one-pass Highlight band with independent prefix/suffix seams in the actual Canvas references")
    }

    fn compare_partial(
        &self,
        image: &CanvasPng,
        clip: &Clip,
        seams: &Seams,
        palette: Palette,
    ) -> Result<Value> {
        // Keep the distinct end-caret reference outside this comparison.
        // Both internal selection edges and all four digit bodies stay in it.
        let x_range = seams.full.start + 3..seams.full.end - 4;
        let mut selected_ink = 0;
        let mut left_ink = 0;
        let mut right_ink = 0;
        let mut checked = 0;
        let mut mismatch_count = 0;
        let mut mismatches = Vec::new();
        for y in clip.y.clone() {
            for x in x_range.clone() {
                let selected = seams.middle.contains(&x);
                let expected = if selected { self.all } else { self.ordinary }.pixel(x, y)?;
                let actual = image.pixel(x, y)?;
                checked += 1;
                if actual != expected {
                    mismatch_count += 1;
                    if mismatches.len() < 12 {
                        mismatches.push(json!({"x":x,"y":y,"expected":expected,"actual":actual}));
                    }
                }
                if selected && actual == palette.selected_ink {
                    selected_ink += 1;
                } else if x < seams.middle.start && actual == palette.ordinary_ink {
                    left_ink += 1;
                } else if x >= seams.middle.end && actual == palette.ordinary_ink {
                    right_ink += 1;
                }
            }
        }
        check!(
            checked > 1000 && mismatch_count == 0,
            "the partial image has one exact reference draw on each side of both 1.25 DPR seams: {mismatch_count} mismatches, first {mismatches:?}"
        );
        check!(
            selected_ink > 0 && left_ink > 0 && right_ink > 0,
            "the selected glyphs and both unselected glyph flanks actually paint their expected RGB"
        );
        let double = seams.composite.over(seams.expected_fill, palette.highlight);
        let can_detect_repeat = !matches_fill(seams.expected_fill, double);
        if can_detect_repeat {
            check!(
                !matches_fill(seams.seen_fill, double),
                "actual Highlight fill rejects double alpha blending"
            );
        }
        Ok(
            json!({"comparison_pixels":checked,"mismatches":mismatch_count,
            "seam_row":seams.axis_row,"full_span":[seams.full.start,seams.full.end],
            "seams":[seams.middle.start,seams.middle.end],"comparison_x":[x_range.start,x_range.end],
            "sample_color":seams.seen_fill,"expected_one_pass":seams.expected_fill,
            "expected_two_passes":double,"repeat_blend_discriminable":can_detect_repeat,
            "composite":seams.composite.name(),"selected_ink_pixels":selected_ink,
            "left_unselected_ink_pixels":left_ink,"right_unselected_ink_pixels":right_ink,
            "selected_expected_rgb":palette.selected_ink,"ordinary_expected_rgb":palette.ordinary_ink,
            "foreground_roles_distinguishable":palette.ordinary_ink != palette.selected_ink}),
        )
    }
}

async fn capture(env: &Env, tab: &Tab, lane: Lane, name: &str) -> Result<CanvasPng> {
    CanvasPng::capture_at_scale(env, tab, &format!("{}-{name}", lane.file_prefix()), DPR).await
}

async fn editing(tab: &Tab) -> Result<Value> {
    editor_focus(tab).await?;
    let snapshot = tab
        .eval(&format!(
            r#"(() => {{
            const runtime = JSON.parse(window.gallery.inspect());
            const input = {TEXTAREA};
            const editor = JSON.parse(window.gallery.export_dom()).text_inputs[{}];
            return {{editor, browser_selection:[input.selectionStart,input.selectionEnd],
                focus:runtime.focused, popup_open:runtime.scroll_popup.open,
                quantity:runtime.scroll_popup.quantity,unit:runtime.scroll_popup.unit,
                axis:runtime.scroll_popup.rtl}};
        }})()"#,
            js(EDITOR),
        ))
        .await?;
    check!(
        snapshot["editor"]["value"] == RAW
            && snapshot["quantity"]["raw"] == RAW
            && snapshot["quantity"]["value"] == 1200
            && snapshot["quantity"]["commits"] == 0
            && snapshot["unit"]["id"] == "ml"
            && snapshot["unit"]["commits"] == 0
            && snapshot["focus"] == EDITOR
            && snapshot["popup_open"] == true
            && snapshot["axis"] == true,
        "the same focused editor retains unsubmitted text and both accepted field values"
    );
    Ok(snapshot)
}

async fn select(tab: &Tab, [start, end]: [usize; 2]) -> Result<Value> {
    editor_focus(tab).await?;
    tab.press("Home").await?;
    for _ in 0..start {
        tab.press("ArrowRight").await?;
    }
    for _ in start..end {
        tab.press("Shift+ArrowRight").await?;
    }
    let expression = format!(
        "(() => {{const editor = JSON.parse(window.gallery.export_dom()).text_inputs[{}];
          return editor?.selection_anchor === {start} && editor?.selection_cursor === {end}
              && !editor.composing && {TEXTAREA}.selectionStart === {start} && {TEXTAREA}.selectionEnd === {end};
        }})()",
        js(EDITOR)
    );
    tab.wait_for(&expression).await?;
    editing(tab).await
}

async fn cursor_snapshot(tab: &Tab) -> Result<Value> {
    tab.eval(&format!(
        r#"(() => {{
            const textarea={TEXTAREA};
            const by_id=JSON.parse(window.gallery.export_dom()).text_inputs[{}];
            return {{browser:[textarea.selectionStart, textarea.selectionEnd],
                     runtime:[by_id.selection_anchor, by_id.selection_cursor],
                     direction:textarea.selectionStart === textarea.selectionEnd ? "none" : textarea.selectionDirection,
                     value:by_id.value, composing:by_id.composing,
                     focused:document.activeElement === textarea}};
        }})()"#,
        js(EDITOR)
    ))
    .await
}

async fn native_arrow_through_redraw(
    env: &Env,
    tab: &Tab,
    normal_image: &CanvasPng,
    delay_select: bool,
) -> Result<Value> {
    let phase = if delay_select {
        "delayed-select-arrow"
    } else {
        "native-arrow"
    };
    select(tab, [TEXT_END - 1; 2]).await?;
    let before = cursor_snapshot(tab).await?;
    let normal = normal_image.sample(3., 397.)?;
    let arrow = crate::keys::key("ArrowRight")?;
    tab.eval(&format!(
        r#"(() => {{
            const editor = {TEXTAREA};
            const probe = {{keydown:null, intercepted:0}};
            probe.capture = event => {{
                if (event.target === editor) {{
                    probe.intercepted++;
                    event.stopImmediatePropagation();
                }}
            }};
            window.__surfaceArrowWitness = probe;
            editor.addEventListener('keydown', event => {{
                probe.keydown = {{key:event.key, trusted:event.isTrusted,
                                 prevented:event.defaultPrevented}};
            }}, {{once:true}});
            if ({delay_select}) document.addEventListener('select', probe.capture, true);
        }})()"#
    ))
    .await?;
    let held = async {
        tab.key_down(&arrow).await?;
        // Snapshot before settling. The ordinary pass allows Chrome to have
        // already delivered its select event. The separate controlled pass
        // gates only select on this textarea until just before trusted keyup.
        let down = cursor_snapshot(tab).await?;
        media(tab, true, false).await?;
        let appearance = contrast(tab, "forced").await?;
        let css = palette(tab, &appearance).await?;
        let image = capture(env, tab, Lane::Browser, &format!("{phase}-forced")).await?;
        let forced = image.sample(3., 397.)?;
        let after_draw = cursor_snapshot(tab).await?;
        Ok::<_, anyhow::Error>((down, css, forced, after_draw))
    }
    .await;
    let witness = tab
        .eval(
            r#"(() => {
                const probe = window.__surfaceArrowWitness;
                document.removeEventListener('select', probe.capture, true);
                delete window.__surfaceArrowWitness;
                return {keydown:probe.keydown, intercepted:probe.intercepted};
            })()"#,
        )
        .await;
    let released = tab.key_up(&arrow).await;
    let (down, css, forced, after_draw) = held?;
    let witness = witness?;
    released?;
    tab.settle().await?;
    let after_up = cursor_snapshot(tab).await?;
    media(tab, false, false).await?;
    contrast(tab, "normal").await?;
    let restored = cursor_snapshot(tab).await?;
    let record = json!({
        "before":before, "after_key_down":down, "after_forced_redraw":after_draw,
        "after_key_up":after_up, "restored_normal":restored,
        "keydown_event":witness["keydown"], "scoped_select_delay":delay_select,
        "intercepted_select_events":witness["intercepted"],
        "normal_canvas_sample":normal, "forced_canvas_sample":forced,
        "css_canvas":css["pairs"]["canvas"]["background"],
    });
    env.write_json(
        &format!("material-web-contrast-selection-browser-{phase}.json"),
        &record,
    )?;

    let near_end = json!([TEXT_END - 1, TEXT_END - 1]);
    let end = json!([TEXT_END, TEXT_END]);
    check_eq!(
        before["browser"],
        near_end,
        "trusted fixture sets the browser cursor"
    );
    check_eq!(
        before["runtime"],
        near_end,
        "trusted fixture sets the Runtime cursor"
    );
    check_eq!(
        witness["keydown"],
        json!({"key":"ArrowRight","trusted":true,"prevented":false}),
        "ArrowRight reaches the browser's own selection handling"
    );
    check!(
        normal != forced
            && css["pairs"]["canvas"]["background"]
                == json!([forced[0], forced[1], forced[2], 255]),
        "a real WebGPU redraw paints the measured Canvas color while ArrowRight is held"
    );
    for snapshot in [&down, &after_draw, &after_up] {
        check!(
            snapshot["value"] == RAW
                && snapshot["composing"] == false
                && snapshot["focused"] == true,
            "the native arrow leaves the same focused editor and draft"
        );
        check_eq!(
            snapshot["browser"],
            end,
            format!("{phase}: the browser arrow selection never rolls back")
        );
        check!(
            snapshot["runtime"] == near_end || snapshot["runtime"] == end,
            "Runtime may observe the browser selection before keyup, but cannot move it elsewhere"
        );
    }
    if delay_select {
        check_eq!(
            down["runtime"],
            near_end,
            "the controlled ordering delays Runtime selection on keydown"
        );
        check_eq!(
            after_draw["runtime"],
            near_end,
            "the controlled redraw precedes selection ownership transfer"
        );
    }
    check_eq!(
        after_up["runtime"],
        end,
        "keyup leaves the exact browser selection in Runtime"
    );
    check_eq!(
        restored,
        after_up,
        "restoring the palette preserves the native cursor"
    );
    Ok(record)
}

async fn backward_selection(tab: &Tab) -> Result<Value> {
    tab.press("Shift+ArrowLeft").await?;
    tab.settle().await?;
    let selection = cursor_snapshot(tab).await?;
    check_eq!(
        selection["browser"],
        json!([TEXT_END - 1, TEXT_END]),
        "a trusted reverse arrow selects the last digit"
    );
    check_eq!(
        selection["runtime"],
        json!([TEXT_END, TEXT_END - 1]),
        "Runtime keeps the backward anchor and cursor"
    );
    check_eq!(
        selection["direction"],
        "backward",
        "the mounted textarea still carries backward selection"
    );
    Ok(selection)
}

async fn forced(tab: &Tab, lane: Lane) -> Result<(Palette, Value)> {
    media(tab, true, false).await?;
    let accepted = contrast(tab, "forced").await?;
    if lane == Lane::Browser {
        let witness = palette(tab, &accepted).await?;
        return Ok((Palette::from_pairs(&accepted["system_colors"])?, witness));
    }
    let pairs = &accepted["system_colors"];
    check_eq!(
        &pairs["canvas"],
        &json!({"background":INJECTED_CANVAS[0],"foreground":INJECTED_CANVAS[1]}),
        "the synthetic Canvas pair is accepted separately"
    );
    check_eq!(
        &pairs["highlight"],
        &json!({"background":INJECTED_HIGHLIGHT[0],"foreground":INJECTED_HIGHLIGHT[1]}),
        "the synthetic Highlight pair is accepted separately"
    );
    Ok((
        Palette::from_pairs(pairs)?,
        json!({"injected_system_colors":pairs}),
    ))
}

fn normal_restoration(
    normal: &CanvasPng,
    clear: &CanvasPng,
    restored: &CanvasPng,
    clip: &Clip,
    seams: &Seams,
) -> Result<Value> {
    let mut selected_changes = 0;
    let mut restoration_errors = 0;
    for y in clip.y.clone() {
        for x in seams.middle.clone() {
            let previous = normal.pixel(x, y)?;
            selected_changes += usize::from(previous != clear.pixel(x, y)?);
        }
        for x in clip.x.clone() {
            restoration_errors += usize::from(normal.pixel(x, y)? != restored.pixel(x, y)?);
        }
    }
    check!(
        selected_changes > 100 && restoration_errors == 0,
        "the old normal selection remains painted and restores exactly: {selected_changes} selected pixels changed from clear, {restoration_errors} restoration mismatches"
    );
    Ok(
        json!({"selected_pixels_changed_from_clear":selected_changes,
        "restore_mismatches":restoration_errors}),
    )
}

async fn keep_preedit(env: &Env, tab: &Tab) -> Result<Value> {
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(EDITOR)
    );
    tab.ime_compose("１２", 2, 2).await?;
    tab.wait_for(&composing).await?;
    let before = editing(tab).await?;
    check!(
        before["editor"]["composing"] == true
            && before["editor"]["preedit_range"].is_object()
            && before["editor"]["display_value"] != RAW,
        "synthetic IME still uses a separate pending display value"
    );
    for enabled in [true, false] {
        media(tab, enabled, false).await?;
        contrast(tab, if enabled { "forced" } else { "normal" }).await?;
        check_eq!(
            editing(tab).await?,
            before,
            "the paired-selection path keeps active preedit and focus stable"
        );
        if enabled {
            capture(env, tab, Lane::Browser, "forced-preedit").await?;
        }
    }
    tab.press("Escape").await?;
    popup_state(
        tab,
        "d.open && d.quantity.commits === 0 && d.unit.commits === 0",
    )
    .await?;
    if tab.eval(&composing).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    let after = editing(tab).await?;
    Ok(json!({"input":"CDP synthetic composition","before":before,
        "preserved_in_forced_and_normal":true,"retired_separately":true,"after_cancel":after}))
}

async fn lane(env: &Env, tab: &Tab, url: &str, lane: Lane) -> Result<(Value, bool)> {
    if lane == Lane::Injected {
        tab.cdp(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({"source":INJECTION}),
        )
        .await?;
    }
    popup(tab, url).await?;
    tab.wait_for(if lane == Lane::Injected {
        "window.__surfaceSyntheticSelectionPalette === true"
    } else {
        "window.__surfaceSyntheticSelectionPalette === undefined"
    })
    .await?;
    tab.shortcut("a").await?;
    tab.insert_text(RAW).await?;
    popup_state(tab, "d.open && d.quantity.raw === '1688' && d.quantity.value === 1200 && d.quantity.commits === 0").await?;
    let normal_face = bounds(tab, FACE).await?;
    let clip = Clip::editor(normal_face);
    select(tab, [TEXT_END, TEXT_END]).await?;
    let clear_normal = capture(env, tab, lane, "normal-clear").await?;
    let (native_arrow_redraw, delayed_select_redraw, backward_range) = if lane == Lane::Browser {
        let native = native_arrow_through_redraw(env, tab, &clear_normal, false).await;
        let delayed = native_arrow_through_redraw(env, tab, &clear_normal, true).await;
        (
            Some(native?),
            Some(delayed?),
            Some(backward_selection(tab).await?),
        )
    } else {
        (None, None, None)
    };
    let selected = select(tab, MIDDLE).await?;
    let normal = capture(env, tab, lane, "normal-partial").await?;
    let (colors, witness) = forced(tab, lane).await?;
    check_eq!(
        editing(tab).await?,
        selected,
        "the original partial selection survives the live forced switch"
    );
    let forced_face = bounds(tab, FACE).await?;
    check!(
        normal_face
            .into_iter()
            .zip(forced_face)
            .all(|(a, b)| near(a, b)),
        "the tested text run stays at the same measured face across modes"
    );
    let partial = capture(env, tab, lane, "forced-partial").await?;
    select(tab, [TEXT_END, TEXT_END]).await?;
    let ordinary = capture(env, tab, lane, "forced-clear").await?;
    select(tab, ALL).await?;
    let all = capture(env, tab, lane, "forced-all").await?;
    select(tab, PREFIX).await?;
    let prefix = capture(env, tab, lane, "forced-prefix").await?;
    select(tab, SUFFIX).await?;
    let suffix = capture(env, tab, lane, "forced-suffix").await?;
    let references = References {
        ordinary: &ordinary,
        all: &all,
        prefix: &prefix,
        suffix: &suffix,
    };
    let seams = references.seams(&clip, colors)?;
    let paint = references.compare_partial(&partial, &clip, &seams, colors)?;
    let distinguishable = colors.ordinary_ink != colors.selected_ink;
    if lane == Lane::Injected {
        check!(
            distinguishable && paint["repeat_blend_discriminable"] == true,
            "the injected fixture proves distinct foreground roles and one application of Highlight alpha"
        );
    }

    check_eq!(
        select(tab, MIDDLE).await?,
        selected,
        "trusted reference actions restore the same partial selection"
    );
    media(tab, false, false).await?;
    contrast(tab, "normal").await?;
    check_eq!(
        editing(tab).await?,
        selected,
        "returning to Normal retains the same selection and draft"
    );
    let restored = capture(env, tab, lane, "normal-restored").await?;
    let normal_proof = normal_restoration(&normal, &clear_normal, &restored, &clip, &seams)?;
    let preedit = if lane == Lane::Browser {
        Some(keep_preedit(env, tab).await?)
    } else {
        None
    };
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "selection browser page errors: {errors:?}"
    );
    Ok((
        json!({"palette_source":lane.source(),"palette":witness,"device_pixel_ratio":DPR,
        "physical_size":partial.dimensions(),"raw":RAW,"selected_range":MIDDLE,"selected_text":"68",
        "editing":selected,"paint":paint,"normal_restoration":normal_proof,
        "selected_foreground_distinguished":distinguishable,"preedit":preedit,
        "native_arrow_redraw":native_arrow_redraw,"delayed_select_redraw":delayed_select_redraw,
        "backward_range":backward_range,"errors":errors}),
        distinguishable,
    ))
}

async fn run_lane(env: &Env, chrome: &Chrome, url: &str, kind: Lane) -> Result<(Value, bool)> {
    let tab = chrome.page(960, 864, DPR).await?;
    tab.cdp("Page.enable", json!({})).await?;
    let result = lane(env, &tab, url, kind).await;
    if let Ok((report, _)) = &result {
        env.write_json(
            &format!("browser-material-web-contrast-{}.json", kind.file_prefix()),
            report,
        )?;
    }
    if let Err(error) = &result {
        let _ = env.write_json(&format!("material-web-contrast-{}-failure.json",kind.file_prefix()), &json!({
            "palette_source":kind.source(),"error":format!("{error:#}"),
            "state":tab.eval_json("window.gallery.inspect()").await.ok(),
            "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors(),
        }));
        let _ = tab
            .screenshot(&env.artifact(&format!(
                "material-web-contrast-{}-failure.png",
                kind.file_prefix()
            )))
            .await;
    }
    result.with_context(|| format!("M246 paired selection with {}", kind.source()))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<Value> {
    step("M246", "the actual partial text selection uses paired browser paint at 1.25 DPR and restores Normal");
    let (browser, distinguishable) = run_lane(env, chrome, url, Lane::Browser).await?;
    let injected = if distinguishable {
        None
    } else {
        // Two equal pigments cannot prove a foreground swap. Run a
        // separately named page with an intentionally distinct pair.
        Some(run_lane(env, chrome, url, Lane::Injected).await?.0)
    };
    let report = json!({"browser":browser,"browser_foreground_pair_distinguishable":distinguishable,
        "foreground_evidence":if distinguishable {"browser"} else {"separate synthetic contrast pair"},
        "synthetic_fallback":injected});
    env.write_json("browser-material-web-contrast-selection.json", &report)?;
    step(
        "M247",
        "the actual preedit underline switches ink inside its forced-color selection",
    );
    let underline = preedit_underline::run(env, chrome, url).await?;
    env.write_json(
        "browser-material-web-contrast-preedit-underline.json",
        &underline,
    )?;
    Ok(report)
}
