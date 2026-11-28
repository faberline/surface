//! Test the Material hint against the same painted field with no authored hint.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::material_list_corners::{freeze, reduced_motion, tick};
use super::material_time_input::{
    activate, ax, ax_tree, bounds, contains, focus_projected, focused, resize, Raster,
};
use super::{focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 3] = ["M351", "M352", "M353"];
const TITLE: &str = "名稱";
const ABOVE_TITLE: &str = "輸入團隊成員在共用專案清單中容易辨認的完整名稱";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const MODEL: &str = "text_field_placeholder_demo";
const PREFIX: &str = "material-text-field-placeholder";
const FIELD: &str = "text-field-placeholder-demo-editor";
const CLEAR: &str = "text-field-placeholder-demo-clear";
const SLOW_EFFECTS: f64 = 800.;
const FAST_EFFECTS: f64 = 3800.;
const DPI: f64 = 2.;
type Rect = [f64; 4];
type Rgb = [u8; 3];
type Pair = [f64; 2];

#[derive(Clone, Copy)]
enum Journey {
    Outlined,
    FilledRtl,
    AboveRtl,
}

impl Journey {
    fn name(self) -> &'static str {
        match self {
            Self::Outlined => "outlined",
            Self::FilledRtl => "filled-rtl",
            Self::AboveRtl => "above-rtl",
        }
    }

    fn compact(self) -> bool {
        !matches!(self, Self::Outlined)
    }

    fn filled(self) -> bool {
        matches!(self, Self::FilledRtl)
    }

    fn above(self) -> bool {
        matches!(self, Self::AboveRtl)
    }
}

fn setting(suffix: &str) -> String {
    format!("text-field-placeholder-demo-{suffix}")
}

#[derive(Debug, PartialEq)]
struct Layout {
    editor: Rect,
    crop: Rect,
    label: Rect,
    filled: bool,
    rtl: bool,
    foreground: Rgb,
    paper: Rgb,
}

impl Layout {
    fn placement_report(&self) -> Value {
        json!({
            "editor": self.editor, "crop": self.crop, "label": self.label,
            "filled": self.filled, "rtl": self.rtl,
            "foreground": self.foreground, "paper": self.paper
        })
    }

    fn coordinates(&self) -> impl Iterator<Item = Pair> {
        let [x, y, w, h] = self.crop;
        let left = (x * DPI).ceil() as usize;
        let right = ((x + w) * DPI).floor() as usize;
        let top = (y * DPI).ceil() as usize;
        let bottom = ((y + h) * DPI).floor() as usize;
        (top..bottom).flat_map(move |row| {
            (left..right).map(move |col| [(col as f64 + 0.5) / DPI, (row as f64 + 0.5) / DPI])
        })
    }
}

fn token(model: &Value, name: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] =
        serde_json::from_value(model["palette"][name].clone()).context("hint palette bytes")?;
    check_eq!(a, 255, "the hint fixture uses opaque palette endpoints");
    Ok([r, g, b])
}

async fn scene(tab: &Tab, url: &str, journey: Journey, without_hint: bool) -> Result<Layout> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2500).await?;
    reduced_motion(tab, true).await?;
    activate(tab, &setting("show")).await?;
    until(tab, &format!("s.{MODEL}.visible")).await?;
    for (enabled, option) in [
        (journey.filled(), "filled"),
        (journey.compact(), "theme"),
        (journey.compact(), "rtl"),
        (journey.compact(), "large"),
        (journey.above(), "above"),
        (without_hint, "no-hint"),
    ] {
        if enabled {
            activate(tab, &setting(option)).await?;
        }
    }
    let viewport = if journey.compact() {
        resize(tab, 320, 960).await?;
        [0., 0., 320., 960.]
    } else {
        resize(tab, 960, 820).await?;
        [0., 0., 960., 820.]
    };
    focus_projected(tab, CLEAR).await?;
    let model = state(tab).await?[MODEL].clone();
    check!(
        model["visible"] == true
            && model["without_hint"] == without_hint
            && model["above"] == journey.above()
            && model["dark"] == journey.compact()
            && model["rtl"] == journey.compact()
            && model["filled"] == journey.filled()
            && model["text_scale"] == if journey.compact() { 2. } else { 1. }
            && model["value"] == "",
        "the fixture starts in the expected empty phase"
    );
    let editor = bounds(tab, FIELD).await?;
    let [x, y, w, h] = editor;
    // Exclude outline strokes and the empty caret at either logical start.
    // Keep the full interior where any physically misplaced hint could draw.
    let crop = [x + 19., y + 5., w - 38., h - 10.];
    let name = if journey.above() {
        "above-label"
    } else {
        "floating-label"
    };
    let label = bounds(tab, &format!("{FIELD}-{name}")).await?;
    check!(
        contains(viewport, editor)
            && contains(viewport, crop)
            && contains(viewport, label)
            && crop[2] > 0.
            && crop[3] > 0.,
        "the label, editor and measured region fit the actual viewport"
    );
    if journey.above() {
        check!(
            label[3] >= 64. && label[1] + label[3] + 3.5 <= editor[1],
            "the wrapped caption stays fully above the dark compact editor"
        );
        check_eq!(
            bounds(tab, &format!("{FIELD}-floating-label")).await?[2],
            0.,
            "the compact editor has no floating outline slot"
        );
    }
    reduced_motion(tab, false).await?;
    freeze(tab).await?;
    Ok(Layout {
        editor,
        crop,
        label,
        filled: journey.filled(),
        rtl: journey.compact(),
        foreground: token(&model, "hint")?,
        paper: token(&model, "paper")?,
    })
}

async fn enter(tab: &Tab) -> Result<()> {
    tab.press("Tab").await?;
    focus(tab, FIELD, "surface-text-input").await
}

async fn leave(tab: &Tab) -> Result<()> {
    tab.press("Shift+Tab").await?;
    focused(tab, CLEAR).await
}

async fn textbox(tab: &Tab, label: &str, value: &str, active: bool) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "textbox", label)?;
    check_eq!(
        tree.iter()
            .filter(|node| node["ignored"] == false
                && node["role"]["value"] == "textbox"
                && node["name"]["value"] == label)
            .count(),
        1,
        "Chrome exposes one named textbox"
    );
    check_eq!(
        node["value"]["value"].as_str().unwrap_or(""),
        value,
        "the Chrome textbox exposes committed value rather than the placeholder"
    );
    check_eq!(
        tab.eval(&format!("document.activeElement === {TEXTAREA}"))
            .await?,
        active,
        "only the active editor uses the actual textarea"
    );
    Ok(json!({"name": label,"value": value,"active":active,"backend":node["backendDOMNodeId"]}))
}

struct Frame {
    name: String,
    phase_ms: u16,
    pixels: Vec<Rgb>,
    above_label_pixels: usize,
}

struct Series {
    layout: Layout,
    frames: BTreeMap<&'static str, Frame>,
}

impl Series {
    async fn take(
        &mut self,
        tab: &Tab,
        env: &Env,
        journey: Journey,
        without_hint: bool,
        phase: &'static str,
        phase_ms: u16,
    ) -> Result<()> {
        tick(tab, phase_ms).await?;
        check_eq!(
            bounds(tab, FIELD).await?,
            self.layout.editor,
            "placeholder motion and caller value preserve the editor rectangle"
        );
        let treatment = if without_hint { "control" } else { "hint" };
        let name = format!("{PREFIX}-{}-{treatment}-{phase}.png", journey.name());
        let path = env.artifact(&name);
        tab.screenshot(&path).await?;
        let raster = Raster::read(tab, path).await?;
        let above_label_pixels = if journey.above() && phase == "idle-above" {
            raster
                .colors(self.layout.label)?
                .get(&self.layout.foreground)
                .copied()
                .unwrap_or(0)
        } else {
            0
        };
        let pixels = self
            .layout
            .coordinates()
            .map(|[x, y]| raster.sample(x, y))
            .collect::<Result<Vec<_>>>()?;
        check!(
            !pixels.is_empty()
                && self
                    .frames
                    .insert(
                        phase,
                        Frame {
                            name,
                            phase_ms,
                            pixels,
                            above_label_pixels,
                        }
                    )
                    .is_none(),
            "one nonempty original crop per phase"
        );
        Ok(())
    }
}

async fn outlined(tab: &Tab, env: &Env, url: &str, without_hint: bool) -> Result<(Series, Value)> {
    let journey = Journey::Outlined;
    let layout = scene(tab, url, journey, without_hint).await?;
    let mut series = Series {
        layout,
        frames: BTreeMap::new(),
    };
    let idle = textbox(tab, TITLE, "", false).await?;
    series
        .take(tab, env, journey, without_hint, "idle", 0)
        .await?;
    enter(tab).await?;
    let active = textbox(tab, TITLE, "", true).await?;
    series
        .take(tab, env, journey, without_hint, "focus-start", 0)
        .await?;
    series
        .take(tab, env, journey, without_hint, "focus-overlap", 14)
        .await?;
    series
        .take(tab, env, journey, without_hint, "focus-alpha-96", 96)
        .await?;
    series
        .take(tab, env, journey, without_hint, "focus-alpha-144", 144)
        .await?;
    series
        .take(tab, env, journey, without_hint, "focus-settled", 800)
        .await?;
    leave(tab).await?;
    series
        .take(tab, env, journey, without_hint, "blur-start", 800)
        .await?;
    series
        .take(tab, env, journey, without_hint, "blur-alpha-12", 812)
        .await?;
    series
        .take(tab, env, journey, without_hint, "blur-alpha-20", 820)
        .await?;
    enter(tab).await?;
    series
        .take(tab, env, journey, without_hint, "reverse-start", 820)
        .await?;
    series
        .take(tab, env, journey, without_hint, "reverse-alpha-6", 826)
        .await?;
    reduced_motion(tab, true).await?;
    series
        .take(tab, env, journey, without_hint, "reduced", 826)
        .await?;
    check!(
        idle["backend"] != active["backend"],
        "the live editor replaces its named proxy"
    );
    Ok((series, json!({"idle":idle,"focused":active})))
}

async fn filled_ime(
    tab: &Tab,
    env: &Env,
    url: &str,
    without_hint: bool,
) -> Result<(Series, Value)> {
    let journey = Journey::FilledRtl;
    let layout = scene(tab, url, journey, without_hint).await?;
    let mut series = Series {
        layout,
        frames: BTreeMap::new(),
    };
    enter(tab).await?;
    series
        .take(tab, env, journey, without_hint, "focus-start", 0)
        .await?;
    series
        .take(tab, env, journey, without_hint, "focus-overlap", 14)
        .await?;
    series
        .take(tab, env, journey, without_hint, "focus-settled", 800)
        .await?;
    let input = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]",
        js(FIELD)
    );
    tab.ime_compose("中文", 2, 2).await?;
    tab.wait_for(&format!(
        "{input}.composing && {input}.value === '' && {TEXTAREA}.value === '中文'"
    ))
    .await?;
    series
        .take(tab, env, journey, without_hint, "marked", 800)
        .await?;
    tab.press("Escape").await?;
    if tab.eval(&format!("{input}.composing")).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!{input}.composing && {TEXTAREA}.value === ''"))
        .await?;
    series
        .take(tab, env, journey, without_hint, "cancelled-marked", 800)
        .await?;

    // Chrome's CDP empty update ends a composition. To exercise the
    // separate Runtime Begin while both values are empty, deliver a
    // synthetic DOM event through the real focused host and bridge.
    let begin = tab.eval(&format!(
        "(() => {{ const event = new CompositionEvent('compositionstart', {{bubbles:true,data:''}});
           {TEXTAREA}.dispatchEvent(event); return {{trusted:event.isTrusted,data:event.data}}; }})()"
    )).await?;
    check_eq!(
        begin,
        json!({"trusted":false,"data":""}),
        "explicit synthetic empty composition start"
    );
    tab.wait_for(&format!(
        "{input}.composing && {input}.value === '' && {TEXTAREA}.value === ''"
    ))
    .await?;
    series
        .take(tab, env, journey, without_hint, "empty-composition", 800)
        .await?;
    tab.eval(&format!(
        "{TEXTAREA}.dispatchEvent(new CompositionEvent('compositionend', {{bubbles:true,data:''}}))"
    ))
    .await?;
    tab.wait_for(&format!("!{input}.composing && {TEXTAREA}.value === ''"))
        .await?;
    series
        .take(tab, env, journey, without_hint, "cancelled-empty", 800)
        .await?;
    check_eq!(
        state(tab).await?[MODEL]["edits"],
        0,
        "composition does not commit a fixture edit"
    );
    let ax = textbox(tab, TITLE, "", true).await?;

    tab.insert_text("草稿").await?;
    until(
        tab,
        &format!("s.{MODEL}.value === '草稿' && s.{MODEL}.edits === 1"),
    )
    .await?;
    series
        .take(tab, env, journey, without_hint, "typed", 800)
        .await?;
    tab.shortcut("A").await?;
    tab.press("Backspace").await?;
    until(
        tab,
        &format!("s.{MODEL}.value === '' && s.{MODEL}.edits === 2"),
    )
    .await?;
    series
        .take(tab, env, journey, without_hint, "cleared", 800)
        .await?;
    Ok((
        series,
        json!({"synthetic_empty_begin":begin,"host_after_cancel":ax,"actual_cdp_marked":"中文"}),
    ))
}

async fn above(tab: &Tab, env: &Env, url: &str, without_hint: bool) -> Result<(Series, Value)> {
    let journey = Journey::AboveRtl;
    let layout = scene(tab, url, journey, without_hint).await?;
    let mut series = Series {
        layout,
        frames: BTreeMap::new(),
    };
    let idle = textbox(tab, ABOVE_TITLE, "", false).await?;
    series
        .take(tab, env, journey, without_hint, "idle-above", 0)
        .await?;
    enter(tab).await?;
    series
        .take(tab, env, journey, without_hint, "focused-above", 0)
        .await?;
    leave(tab).await?;
    series
        .take(tab, env, journey, without_hint, "blurred-above", 0)
        .await?;

    activate(tab, &setting("set")).await?;
    until(
        tab,
        &format!("s.{MODEL}.value === '範例' && s.{MODEL}.authored_writes === 1"),
    )
    .await?;
    series
        .take(tab, env, journey, without_hint, "nonempty", 800)
        .await?;
    let seeded = textbox(tab, ABOVE_TITLE, "範例", false).await?;
    activate(tab, CLEAR).await?;
    until(
        tab,
        &format!("s.{MODEL}.value === '' && s.{MODEL}.authored_writes === 2"),
    )
    .await?;
    focused(tab, CLEAR).await?;
    series
        .take(tab, env, journey, without_hint, "clear-start", 800)
        .await?;
    series
        .take(tab, env, journey, without_hint, "clear-alpha-44", 844)
        .await?;
    series
        .take(tab, env, journey, without_hint, "clear-alpha-96", 896)
        .await?;
    series
        .take(tab, env, journey, without_hint, "clear-settled", 1600)
        .await?;
    let cleared = textbox(tab, ABOVE_TITLE, "", false).await?;
    check_eq!(
        state(tab).await?[MODEL]["edits"],
        0,
        "authored clearing is not a user edit"
    );
    Ok((
        series,
        json!({"idle":idle,"seeded":seeded,"cleared":cleared}),
    ))
}

fn motion_level(stiffness: f64, ms: u16) -> f64 {
    let phase = stiffness.sqrt() * f64::from(ms) / 1000.;
    (1. + phase) * (-phase).exp()
}

fn resumed_focus_alpha(blur_ms: u16, focus_ms: u16) -> f64 {
    let blur_seconds = f64::from(blur_ms) / 1000.;
    let blur_rate = FAST_EFFECTS.sqrt();
    let velocity = -FAST_EFFECTS * blur_seconds * (-blur_rate * blur_seconds).exp();
    let offset = motion_level(FAST_EFFECTS, blur_ms) - 1.;
    let focus_seconds = f64::from(focus_ms) / 1000.;
    let focus_rate = SLOW_EFFECTS.sqrt();
    1. + (offset + (velocity + focus_rate * offset) * focus_seconds)
        * (-focus_rate * focus_seconds).exp()
}

fn composite(paper: Rgb, foreground: Rgb, opacity: f64, linear: bool) -> Rgb {
    let alpha = (opacity * 255.).round() / 255.;
    let decode = |sample: u8| {
        let channel = f64::from(sample) / 255.;
        if !linear {
            channel
        } else if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    std::array::from_fn(|i| {
        let srgb = decode(paper[i]) * (1. - alpha) + decode(foreground[i]) * alpha;
        let encoded = if !linear {
            srgb
        } else if srgb <= 0.0031308 {
            12.92 * srgb
        } else {
            1.055 * srgb.powf(1. / 2.4) - 0.055
        };
        (255. * encoded).round() as u8
    })
}

fn nearby(left: Rgb, right: Rgb) -> bool {
    left.into_iter().zip(right).all(|(a, b)| a.abs_diff(b) <= 2)
}

struct Masks<'a> {
    hint: &'a Series,
    control: &'a Series,
    probes: Vec<(usize, Pair)>,
    rgb_models: [bool; 2],
    receipts: BTreeMap<&'static str, Value>,
}

impl<'a> Masks<'a> {
    fn new(hint: &'a Series, control: &'a Series, settled: &'static str) -> Result<Self> {
        check_eq!(
            hint.layout,
            control.layout,
            "the positive and hint-free control differ only in paint"
        );
        let layout = &hint.layout;
        let measured = &hint.frames[settled];
        let blank = &control.frames[settled];
        let mask = layout
            .coordinates()
            .zip(measured.pixels.iter().zip(&blank.pixels))
            .enumerate()
            .filter(|(_, (_, (ink, paper)))| **ink == layout.foreground && **paper == layout.paper)
            .map(|(index, (position, _))| (index, position))
            .collect::<Vec<_>>();
        check!(
            mask.len() > 80,
            "the settled original contains solid hint pixels"
        );
        let logical_edge = if layout.rtl {
            layout.editor[0] + layout.editor[2] - 16.
        } else {
            layout.editor[0] + 16.
        };
        let painted_edge = if layout.rtl {
            mask.iter()
                .map(|(_, [x, _])| *x)
                .fold(f64::NEG_INFINITY, f64::max)
        } else {
            mask.iter()
                .map(|(_, [x, _])| *x)
                .fold(f64::INFINITY, f64::min)
        };
        check!(
            (logical_edge - painted_edge).abs() <= 10.,
            "the observed hint uses the field's actual logical-start alignment"
        );
        // Use distinct solid glyphs past the short caption. This isolates
        // hint ink while the caption follows its own path and color clock.
        let mut far = mask
            .into_iter()
            .filter(|(_, [x, _])| (x - logical_edge).abs() > if layout.rtl { 110. } else { 65. })
            .collect::<Vec<_>>();
        far.sort_by(|(_, [x, _]), (_, [y, _])| x.total_cmp(y));
        check!(
            far.len() >= 32,
            "the hint has a separate solid suffix to sample"
        );
        let probes = (1..=4).map(|i| far[far.len() * i / 5]).collect();
        Ok(Self {
            hint,
            control,
            probes,
            rgb_models: [true, true],
            receipts: BTreeMap::new(),
        })
    }

    fn sample(&mut self, phase: &'static str, opacity: f64) -> Result<()> {
        let layout = &self.hint.layout;
        let hint = &self.hint.frames[phase];
        let control = &self.control.frames[phase];
        check_eq!(
            hint.phase_ms,
            control.phase_ms,
            "paired crops share the exact held frame time"
        );
        check_eq!(
            hint.pixels.len(),
            control.pixels.len(),
            "paired crop inventory"
        );
        let changed = hint
            .pixels
            .iter()
            .zip(&control.pixels)
            .filter(|(a, b)| a != b)
            .count();
        let actual = self
            .probes
            .iter()
            .map(|(index, _)| hint.pixels[*index])
            .collect::<Vec<_>>();
        let mut expected = [layout.paper; 2];
        if opacity == 0. {
            check_eq!(
                changed,
                0,
                format!("{phase}: the full glyph region matches its no-hint control")
            );
        } else {
            check!(
                changed > 32,
                format!("{phase}: actual hint stroke pixels must differ from the control")
            );
            for (choice, linear) in [false, true].into_iter().enumerate() {
                expected[choice] = composite(layout.paper, layout.foreground, opacity, linear);
                self.rgb_models[choice] &= actual
                    .iter()
                    .all(|sample| nearby(*sample, expected[choice]));
            }
            check!(
                self.rgb_models.into_iter().any(|plausible| plausible)
                    && self.probes.iter().all(|(index, _)| control.pixels[*index] == layout.paper),
                format!("{phase}: actual original hint RGB {actual:?}, expected {expected:?} at opacity {opacity}")
            );
            if opacity == 1. {
                check!(
                    actual.iter().all(|sample| *sample == layout.foreground),
                    "the settled hint uses the exact ink token"
                );
            }
        }
        self.receipts.insert(phase, json!({
            "ms":hint.phase_ms,"opacity":opacity,"hint_png":hint.name,"control_png":control.name,
            "changed_pixels":changed,"actual_probe_rgb":actual,"expected_composite_rgb":expected,
        }));
        Ok(())
    }

    fn report(self, hint_ax: Value, control_ax: Value) -> Value {
        let probes = self
            .probes
            .iter()
            .map(|(_, point)| *point)
            .collect::<Vec<_>>();
        json!({
            "layout": self.hint.layout.placement_report(),"probe_points":probes,
            "frames":self.receipts, "accepted_blend_models":self.rgb_models,
            "hint_ax":hint_ax, "control_ax":control_ax
        })
    }
}

async fn outlined_proof(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    let (hint, hint_ax) = outlined(tab, env, url, false).await?;
    let (control, control_ax) = outlined(tab, env, url, true).await?;
    let mut proof = Masks::new(&hint, &control, "focus-settled")?;
    for stage in ["idle", "focus-start", "focus-overlap"] {
        proof.sample(stage, 0.)?;
    }
    proof.sample("focus-alpha-96", 1. - motion_level(SLOW_EFFECTS, 96))?;
    proof.sample("focus-alpha-144", 1. - motion_level(SLOW_EFFECTS, 144))?;
    proof.sample("focus-settled", 1.)?;
    proof.sample("blur-start", 1.)?;
    proof.sample("blur-alpha-12", motion_level(FAST_EFFECTS, 12))?;
    let turn = motion_level(FAST_EFFECTS, 20);
    proof.sample("blur-alpha-20", turn)?;
    proof.sample("reverse-start", turn)?;
    proof.sample("reverse-alpha-6", resumed_focus_alpha(20, 6))?;
    proof.sample("reduced", 1.)?;
    Ok(proof.report(hint_ax, control_ax))
}

async fn filled_ime_proof(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    let (hint, hint_ax) = filled_ime(tab, env, url, false).await?;
    let (control, control_ax) = filled_ime(tab, env, url, true).await?;
    let mut proof = Masks::new(&hint, &control, "focus-settled")?;
    for phase in [
        "focus-start",
        "focus-overlap",
        "marked",
        "empty-composition",
        "typed",
    ] {
        proof.sample(phase, 0.)?;
    }
    for phase in [
        "focus-settled",
        "cancelled-marked",
        "cancelled-empty",
        "cleared",
    ] {
        proof.sample(phase, 1.)?;
    }
    let model = state(tab).await?[MODEL].clone();
    let text = token(&model, "text")?;
    let painted = hint.frames["marked"]
        .pixels
        .iter()
        .filter(|p| **p == text)
        .count();
    check!(
        painted > 12,
        "actual marked glyphs remain present with the hint fully hidden"
    );
    let mut report = proof.report(hint_ax, control_ax);
    report["marked_ink_pixels"] = json!(painted);
    Ok(report)
}

async fn above_proof(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    let (hint, hint_ax) = above(tab, env, url, false).await?;
    let (control, control_ax) = above(tab, env, url, true).await?;
    let mut proof = Masks::new(&hint, &control, "clear-settled")?;
    for phase in [
        "idle-above",
        "focused-above",
        "blurred-above",
        "clear-settled",
    ] {
        proof.sample(phase, 1.)?;
    }
    proof.sample("nonempty", 0.)?;
    proof.sample("clear-start", 0.)?;
    proof.sample("clear-alpha-44", 1. - motion_level(SLOW_EFFECTS, 44))?;
    proof.sample("clear-alpha-96", 1. - motion_level(SLOW_EFFECTS, 96))?;
    let label_pixels = hint.frames["idle-above"].above_label_pixels;
    check!(
        label_pixels > 40 && label_pixels == control.frames["idle-above"].above_label_pixels,
        "the wrapped idle label is visibly separate from its hint in both originals"
    );
    let mut report = proof.report(hint_ax, control_ax);
    report["idle_above_label_pixels"] = json!(label_pixels);
    Ok(report)
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, DPI).await?;
    let mut report = json!({"browser":chrome.version,"clock":"controlled performance.now",
        "dpr":DPI,"stiffness":{"slow":SLOW_EFFECTS,"fast":FAST_EFFECTS},"damping_ratio":1});
    let result: Result<()> = async {
        step(STEPS[0], "the whole outline preserves hidden hint pixels and measures unequal focus and blur curves");
        report["outlined"] = outlined_proof(&tab, env, url).await?;
        report["adapter"] = tab.eval(r#"(async () => {
            const {info} = await navigator.gpu.requestAdapter();
            const {vendor,architecture,isFallbackAdapter}=info;
            return {vendor,architecture,isFallbackAdapter};
        })()"#).await?;
        check!(
            report["adapter"]["vendor"].is_string() && report["adapter"]["architecture"].is_string()
                && report["adapter"]["isFallbackAdapter"].is_boolean(),
            "the test records the actual WebGPU adapter"
        );
        step(STEPS[1], "dark RTL at 200 percent keeps the hint aligned and hides every hint pixel during IME");
        report["filled_rtl"] = filled_ime_proof(&tab, env, url).await?;
        step(STEPS[2], "a wrapped above-caption shows its idle hint and slowly reveals after an unfocused authored clear");
        report["above_rtl"] = above_proof(&tab, env, url).await?;
        check!(tab.errors().is_empty(), "no placeholder motion page errors");
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact(&format!("{PREFIX}-failure.png")))
            .await;
    }
    env.write_json(&format!("{PREFIX}.json"), &report)?;
    result
}
