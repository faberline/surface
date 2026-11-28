//! Label and outline motion sampled from real Canvas pixels at a held clock.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_list_corners::{freeze, reduced_motion, tick};
use super::material_time_input::{
    activate, ax, ax_tree, bounds, contains, focus_projected, focused, resize, Raster,
};
use super::{focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M347", "M348", "M349", "M350"];
const REPORT: &str = "material-text-field-label-motion.json";
const PREFIX: &str = "material-text-field-label-motion";
const TITLE: &str = "專案名稱";
const HELP: &str = "請輸入中文或英文名稱。";
const ERROR_HELP: &str = "請輸入有效的專案名稱。";
const DRAFT: &str = "草稿";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const EARLY: u16 = 14;
const MIDDLE: u16 = 44;
const SETTLED: u16 = 800;
const DPI: f64 = 2.;
type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Clone, Copy)]
enum Field {
    Outlined,
    Filled,
}

impl Field {
    fn prefix(self) -> &'static str {
        match self {
            Self::Outlined => "outlined-text-field-demo",
            Self::Filled => "filled-text-field-demo",
        }
    }

    fn model(self) -> &'static str {
        match self {
            Self::Outlined => "outlined_text_field_demo",
            Self::Filled => "filled_text_field_demo",
        }
    }

    fn appearance(self) -> &'static str {
        match self {
            Self::Outlined => "outlined",
            Self::Filled => "filled",
        }
    }

    fn id(self, suffix: &str) -> String {
        format!("{}-{suffix}", self.prefix())
    }
}

struct Geometry {
    face: Rect,
    inline: Rect,
    floated: Rect,
    help: Rect,
    viewport: Rect,
    rtl: bool,
    field: Field,
}

impl Geometry {
    async fn read(tab: &Tab, field: Field, compact: bool) -> Result<Self> {
        let editable = |part| field.id(&format!("editable-{part}"));
        let geometry = Self {
            face: bounds(tab, &field.id("editable")).await?,
            inline: bounds(tab, &editable("inline-label")).await?,
            floated: bounds(tab, &editable("floating-label")).await?,
            help: bounds(tab, &editable("description")).await?,
            viewport: if compact {
                [0., 0., 320., 960.]
            } else {
                [0., 0., 960., 820.]
            },
            rtl: compact,
            field,
        };
        check!(
            [
                geometry.face,
                geometry.inline,
                geometry.floated,
                geometry.help
            ]
            .into_iter()
            .all(|part| part[2] > 0. && part[3] > 0. && contains(geometry.viewport, part)),
            "the label endpoints, editor and help have fully captured geometry"
        );
        Ok(geometry)
    }

    fn anchor(&self) -> f64 {
        if self.rtl {
            right(self.floated)
        } else {
            self.floated[0]
        }
    }

    fn clearance(&self) -> Rect {
        let [x, y, _, h] = self.inline;
        let top = match self.field {
            Field::Outlined => y,
            Field::Filled => bottom(self.floated),
        };
        let width = self.floated[2] * 1.5 + 24.;
        let x = if self.rtl {
            right(self.inline) - width
        } else {
            x
        };
        [x, top, width, h]
    }

    async fn assert_stable(&self, tab: &Tab) -> Result<()> {
        check_eq!(
            bounds(tab, &self.field.id("editable")).await?,
            self.face,
            "animation preserves the editor bounds"
        );
        check_eq!(
            bounds(tab, &self.field.id("editable-description")).await?,
            self.help,
            "animation leaves the supporting text in place"
        );
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Ink {
    paper: Rgb,
    label: Rgb,
    primary: Rgb,
    outline: Rgb,
    error: Rgb,
    text: Rgb,
}

impl Ink {
    fn read(model: &Value, field: Field) -> Result<Self> {
        fn color(model: &Value, key: &str) -> Result<Rgb> {
            let [r, g, b, alpha]: [u8; 4] = serde_json::from_value(model["palette"][key].clone())
                .with_context(|| format!("opaque theme color {key}"))?;
            check_eq!(alpha, 255, "the sampled palette color is opaque");
            Ok([r, g, b])
        }
        Ok(Self {
            paper: color(
                model,
                match field {
                    Field::Outlined => "background",
                    Field::Filled => "surface_container_highest",
                },
            )?,
            label: color(model, "on_surface_variant")?,
            primary: color(model, "primary")?,
            outline: color(model, "outline")?,
            error: color(model, "error")?,
            text: color(model, "on_surface")?,
        })
    }
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

async fn scene(tab: &Tab, url: &str, field: Field, compact: bool) -> Result<Value> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2500).await?;
    reduced_motion(tab, true).await?;
    activate(tab, &field.id("show")).await?;
    until(tab, &format!("s.{}.visible", field.model())).await?;
    if compact {
        for setting in ["theme", "rtl", "large-text"] {
            activate(tab, &field.id(setting)).await?;
        }
        resize(tab, 320, 960).await?;
    } else {
        resize(tab, 960, 820).await?;
    }
    focus_projected(tab, &field.id("error")).await?;
    let model = state(tab).await?[field.model()].clone();
    check!(
        model["value"] == ""
            && model["edits"] == 0
            && model["submits"] == 0
            && model["dark"] == compact
            && model["rtl"] == compact
            && model["text_scale"] == if compact { 2. } else { 1. },
        "the scene begins with an untouched empty field and the required theme"
    );
    reduced_motion(tab, false).await?;
    freeze(tab).await?;
    tab.eval(r#"(() => {
        window.labelMotionKeys = [];
        window.addEventListener("keydown", event => {
            if (event.isTrusted && (event.key === "Tab" || event.key === "Escape" || event.key === "Enter"))
                window.labelMotionKeys.push({key:event.key, shift:event.shiftKey,id:event.target?.id ?? ""});
        }, {capture:true,passive:true});
    })()"#).await?;
    Ok(model)
}

async fn enter(tab: &Tab, field: Field) -> Result<()> {
    tab.press("Tab").await?;
    focus(tab, &field.id("editable"), "surface-text-input").await?;
    tab.settle().await
}

async fn leave(tab: &Tab, field: Field) -> Result<()> {
    tab.press("Shift+Tab").await?;
    focused(tab, &field.id("error")).await?;
    tab.settle().await
}

async fn textbox(tab: &Tab, label: &str, focused: bool, help: Option<&str>) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, "textbox", label)?;
    let count = tree
        .iter()
        .filter(|item| {
            item["ignored"] == false
                && item["role"]["value"] == "textbox"
                && item["name"]["value"] == label
        })
        .count();
    check_eq!(count, 1, "Chrome exposes just one accessible textbox");
    check_eq!(
        tab.eval(&format!("document.activeElement === {TEXTAREA}"))
            .await?,
        focused,
        "the real textarea owns focus only during editing"
    );
    if let Some(text) = help {
        check_eq!(
            node["description"]["value"],
            text,
            "the description survives the handoff"
        );
    }
    let invalid = node["properties"].as_array().is_some_and(|properties| {
        properties.iter().any(|property| {
            property["name"] == "invalid"
                && (property["value"]["value"] == true || property["value"]["value"] == "true")
        })
    });
    Ok(json!({
        "label":label,"backend":node["backendDOMNodeId"],"description":node["description"]["value"],
        "focused":focused,"value":node["value"]["value"],"invalid":invalid,"named_textboxes":count
    }))
}

async fn photo(tab: &Tab, env: &Env, suffix: &str) -> Result<(String, Raster)> {
    tab.settle().await?;
    let name = format!("{PREFIX}-{suffix}.png");
    let path = env.artifact(&name);
    tab.screenshot(&path).await?;
    Ok((name, Raster::read(tab, path).await?))
}

struct Glyphs {
    bounds: Rect,
    pixels: usize,
    color: Rgb,
}

fn glyphs(image: &Raster, geometry: &Geometry, color: Rgb) -> Result<Glyphs> {
    // Follow the anchored label only. The 1 dp caret and up to 3 dp of
    // outline use the same focus color and are outside this sample.
    let reach = geometry.floated[2] * 1.65 + 16.;
    let anchor = geometry.anchor();
    let (left, right) = if geometry.rtl {
        ((anchor - reach).max(geometry.face[0] + 4.), anchor - 2.)
    } else {
        (anchor + 2., (anchor + reach).min(right(geometry.face) - 4.))
    };
    let low = geometry.floated[1].min(geometry.inline[1]);
    let high = bottom(geometry.inline).max(bottom(geometry.floated));
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    let mut pixels = 0;
    for row in (low * DPI).ceil() as usize..(high * DPI).floor() as usize {
        let y = (row as f64 + 0.5) / DPI;
        if matches!(geometry.field, Field::Outlined)
            && (geometry.face[1]..geometry.face[1] + 3.).contains(&y)
        {
            continue;
        }
        for column in (left * DPI).ceil() as usize..(right * DPI).floor() as usize {
            let x = (column as f64 + 0.5) / DPI;
            if image.sample(x, y)? == color {
                min = [min[0].min(x), min[1].min(y)];
                max = [max[0].max(x), max[1].max(y)];
                pixels += 1;
            }
        }
    }
    check!(
        pixels > 12,
        format!("the label has real {color:?} glyph ink; found {pixels} pixels")
    );
    Ok(Glyphs {
        bounds: [
            min[0] - 0.25,
            min[1] - 0.25,
            max[0] - min[0] + 0.5,
            max[1] - min[1] + 0.5,
        ],
        pixels,
        color,
    })
}

fn color_pixels(image: &Raster, rect: Rect, color: Rgb) -> Result<usize> {
    Ok(image.colors(rect)?.get(&color).copied().unwrap_or(0))
}

fn top_paper(image: &Raster, geometry: &Geometry, paper: Rgb) -> Result<usize> {
    let start = (geometry.floated[0] - 4.) * DPI;
    let end = (right(geometry.floated) + 4.) * DPI;
    let mut count = 0;
    for column in start.ceil() as usize..end.floor() as usize {
        let x = (column as f64 + 0.5) / DPI;
        count += usize::from(image.sample(x, geometry.face[1] + 0.25)? == paper);
    }
    Ok(count)
}

fn notch_sample(image: &Raster, geometry: &Geometry) -> Result<[Rgb; 2]> {
    // Two dp into the four-dp leading notch padding is free of label ink.
    let x = geometry.anchor() + if geometry.rtl { 2. } else { -2. };
    Ok([
        image.sample(x, geometry.face[1] + 0.25)?,
        image.sample(x, geometry.face[1] + 2.5)?,
    ])
}

fn sides(image: &Raster, face: Rect, ink: Rgb) -> Result<[Rgb; 3]> {
    let [x, y, w, h] = face;
    let result = [
        image.sample(x + 0.5, y + h / 2.)?,
        image.sample(x + w - 0.5, y + h / 2.)?,
        image.sample(x + w / 2., y + h - 0.5)?,
    ];
    check_eq!(
        result,
        [ink; 3],
        "outline sides and bottom remain continuous"
    );
    Ok(result)
}

struct Shot {
    name: String,
    glyphs: Glyphs,
    placeholder: usize,
    gap_columns: usize,
    notch: Option<[Rgb; 2]>,
    outline_sides: Option<[Rgb; 3]>,
}

impl Shot {
    fn report(&self, ms: u16) -> Value {
        json!({"png":self.name,"ms":ms,"label":self.glyphs.bounds,"label_pixels":self.glyphs.pixels,
            "label_rgb":self.glyphs.color,"placeholder_pixels":self.placeholder,"gap_pixel_columns":self.gap_columns,
            "notch":self.notch,"outline_sides":self.outline_sides})
    }
}

enum LabelPaint<'a> {
    Idle(Rgb),
    Focused(Rgb),
    FirstFocus(&'a Raster),
}

fn assert_same_color_mask(base: &Raster, frame: &Raster, rect: Rect, color: Rgb) -> Result<()> {
    // At 0 ms the label and placeholder have the same RGB token.
    // Compare their exact masks against the inactive photo, where the
    // placeholder is hidden, rather than counting the idle label as a hint.
    let mut changes = 0;
    for row in (rect[1] * DPI).ceil() as usize..(bottom(rect) * DPI).floor() as usize {
        let y = (row as f64 + 0.5) / DPI;
        for column in (rect[0] * DPI).ceil() as usize..(right(rect) * DPI).floor() as usize {
            let x = (column as f64 + 0.5) / DPI;
            changes += usize::from((base.sample(x, y)? == color) != (frame.sample(x, y)? == color));
        }
    }
    check_eq!(
        changes,
        0,
        "first focus paints no new placeholder pixels at the idle label color"
    );
    Ok(())
}

fn sample(
    geometry: &Geometry,
    ink: Ink,
    paint: LabelPaint<'_>,
    name: String,
    image: &Raster,
) -> Result<Shot> {
    let (label, active, placeholder) = match paint {
        LabelPaint::Idle(color) => (color, false, 0),
        LabelPaint::Focused(color) => (
            color,
            true,
            color_pixels(image, geometry.clearance(), ink.label)?,
        ),
        LabelPaint::FirstFocus(rest) => {
            assert_same_color_mask(rest, image, geometry.clearance(), ink.label)?;
            (ink.label, true, 0)
        }
    };
    let gap = matches!(geometry.field, Field::Outlined);
    Ok(Shot {
        name,
        glyphs: glyphs(image, geometry, label)?,
        placeholder,
        gap_columns: if gap {
            top_paper(image, geometry, ink.paper)?
        } else {
            0
        },
        notch: gap.then(|| notch_sample(image, geometry)).transpose()?,
        outline_sides: gap
            .then(|| {
                sides(
                    image,
                    geometry.face,
                    if active { ink.primary } else { ink.outline },
                )
            })
            .transpose()?,
    })
}

async fn capture(
    tab: &Tab,
    env: &Env,
    geometry: &Geometry,
    ink: Ink,
    paint: LabelPaint<'_>,
    name: &str,
) -> Result<Shot> {
    geometry.assert_stable(tab).await?;
    let (name, image) = photo(tab, env, name).await?;
    sample(geometry, ink, paint, name, &image)
}

async fn outlined(
    tab: &Tab,
    env: &Env,
    url: &str,
    report: &mut Value,
) -> Result<(Geometry, Ink, Shot)> {
    let field = Field::Outlined;
    let model = scene(tab, url, field, false).await?;
    report["adapter"] = tab.eval(r#"(async () => {
        const {info} = await navigator.gpu.requestAdapter(); const {vendor,architecture,isFallbackAdapter}=info;
        return {vendor,architecture,isFallbackAdapter};
    })()"#).await?;
    check!(
        report["adapter"]["vendor"].is_string()
            && report["adapter"]["architecture"].is_string()
            && report["adapter"]["isFallbackAdapter"].is_boolean(),
        "actual Chrome WebGPU adapter"
    );
    let geometry = Geometry::read(tab, field, false).await?;
    let ink = Ink::read(&model, field)?;
    geometry.assert_stable(tab).await?;
    let (name, neutral) = photo(tab, env, "outlined-rest").await?;
    let rest = sample(&geometry, ink, LabelPaint::Idle(ink.label), name, &neutral)?;
    let proxy = textbox(tab, TITLE, false, Some(HELP)).await?;
    enter(tab, field).await?;
    let active = textbox(tab, TITLE, true, Some(HELP)).await?;
    check!(
        proxy["backend"] != active["backend"],
        "focus hands the stable name to the textarea"
    );
    let first = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::FirstFocus(&neutral),
        "outlined-first",
    )
    .await?;
    check_eq!(
        first.glyphs.bounds,
        rest.glyphs.bounds,
        "the first frame keeps the original label position"
    );
    check!(
        first.gap_columns == 0 && rest.gap_columns == 0 && first.placeholder == 0,
        "the first frame has a closed top and no overlapping placeholder"
    );
    tick(tab, EARLY).await?;
    let early = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Focused(golden::between(ink.label, ink.primary, EARLY)),
        "outlined-early",
    )
    .await?;
    check_eq!(
        early.notch,
        Some([ink.paper, ink.primary]),
        "the early gap removes only the upper part of the three-dp stroke"
    );
    tick(tab, MIDDLE).await?;
    let mid = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Focused(golden::between(ink.label, ink.primary, MIDDLE)),
        "outlined-middle",
    )
    .await?;
    tick(tab, SETTLED).await?;
    let settled = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Focused(ink.primary),
        "outlined-settled",
    )
    .await?;
    check!(
        early.gap_columns >= 8 && mid.gap_columns > early.gap_columns + 6,
        "the real shallow gap grows across the top edge"
    );
    check!(
        first.glyphs.bounds[1] > early.glyphs.bounds[1]
            && early.glyphs.bounds[1] > mid.glyphs.bounds[1] + 2.
            && mid.glyphs.bounds[1] > settled.glyphs.bounds[1] + 2.,
        "one real label moves between the resting and floating rows"
    );
    check_eq!(
        settled.notch,
        Some([ink.paper; 2]),
        "the anchored padding is fully cut at rest"
    );
    check!(
        settled.placeholder > 12,
        "the empty editor paints its placeholder after the label clears"
    );
    report["outlined"] = json!({"face":geometry.face,"inline":geometry.inline,"floated":geometry.floated,
        "helper":geometry.help,"palette":model["palette"],"proxy":proxy,"active":active,
        "frames":[rest.report(0),first.report(0),early.report(EARLY),mid.report(MIDDLE),settled.report(SETTLED)]});
    Ok((geometry, ink, settled))
}

async fn reversal(
    tab: &Tab,
    env: &Env,
    geometry: &Geometry,
    ink: Ink,
    settled: &Shot,
) -> Result<Value> {
    let field = Field::Outlined;
    let turn = SETTLED + MIDDLE;
    leave(tab, field).await?;
    let proxy = textbox(tab, TITLE, false, Some(HELP)).await?;
    tick(tab, turn).await?;
    let falling_color = golden::between(ink.primary, ink.label, MIDDLE);
    let falling = capture(
        tab,
        env,
        geometry,
        ink,
        LabelPaint::Idle(falling_color),
        "outlined-falling",
    )
    .await?;
    check!(
        falling.glyphs.bounds[1] > settled.glyphs.bounds[1] + 2.
            && falling.glyphs.bounds[1] < geometry.inline[1] + 8.,
        "the falling label has an intermediate painted position"
    );
    enter(tab, field).await?;
    let retarget = capture(
        tab,
        env,
        geometry,
        ink,
        LabelPaint::Focused(falling_color),
        "outlined-retarget",
    )
    .await?;
    check_eq!(
        retarget.glyphs.bounds,
        falling.glyphs.bounds,
        "reversal starts from the exact current glyph pose"
    );
    reduced_motion(tab, true).await?;
    let snapped = capture(
        tab,
        env,
        geometry,
        ink,
        LabelPaint::Focused(ink.primary),
        "outlined-reduced",
    )
    .await?;
    check_eq!(
        snapped.glyphs.bounds,
        settled.glyphs.bounds,
        "Reduce Motion snaps to the settled glyph pose at the same time"
    );
    check!(
        falling.glyphs.color != ink.label
            && falling.glyphs.color != ink.primary
            && retarget.glyphs.color == falling.glyphs.color
            && snapped.glyphs.color == ink.primary,
        "reversal keeps the current color and Reduce Motion snaps at the same clock"
    );
    check_eq!(
        snapped.notch,
        settled.notch,
        "Reduce Motion snaps the notch with the label"
    );
    let events = tab.eval("window.labelMotionKeys").await?;
    let press = events.as_array().context("trusted focus keys")?;
    check!(
        press.iter().any(|event| event["key"] == "Tab"
            && event["shift"] == true
            && event["id"] == "surface-text-input")
            && press
                .iter()
                .filter(|event| event["key"] == "Tab" && event["shift"] == false)
                .count()
                == 2,
        "trusted Tab and Shift+Tab enter, blur and retarget the field"
    );
    Ok(
        json!({"blurred":proxy,"trusted":events,"frames":[falling.report(turn),retarget.report(turn),snapped.report(turn)]}),
    )
}

async fn filled(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    let field = Field::Filled;
    let model = scene(tab, url, field, false).await?;
    let geometry = Geometry::read(tab, field, false).await?;
    let ink = Ink::read(&model, field)?;
    let (name, neutral) = photo(tab, env, "filled-rest").await?;
    let rest = sample(&geometry, ink, LabelPaint::Idle(ink.label), name, &neutral)?;
    enter(tab, field).await?;
    let active = textbox(tab, TITLE, true, Some(HELP)).await?;
    let start = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::FirstFocus(&neutral),
        "filled-first",
    )
    .await?;
    tick(tab, MIDDLE).await?;
    let middle = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Focused(golden::between(ink.label, ink.primary, MIDDLE)),
        "filled-middle",
    )
    .await?;
    check!(
        start.glyphs.bounds[1] > middle.glyphs.bounds[1]
            && start.placeholder == 0
            && middle.placeholder == 0,
        "the filled label moves and blocks the placeholder while they overlap"
    );
    let input = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]",
        js(field.id("editable"))
    );
    tab.ime_compose("中文", 2, 2).await?;
    tab.wait_for(&format!(
        "{input}.composing === true && {input}.value === '' && {TEXTAREA}.value === '中文'"
    ))
    .await?;
    let (preedit_name, preedit) = photo(tab, env, "filled-preedit").await?;
    let actual = color_pixels(&preedit, geometry.clearance(), ink.text)?;
    let placeholder = color_pixels(&preedit, geometry.clearance(), ink.label)?;
    check!(
        actual > 12 && placeholder == 0,
        "actual marked glyphs appear in the editor while the empty placeholder stays hidden"
    );
    check_eq!(
        state(tab).await?[field.model()]["edits"],
        0,
        "preedit never commits text"
    );
    tab.press("Escape").await?;
    if tab.eval(&format!("{input}.composing")).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!{input}.composing && {TEXTAREA}.value === ''"))
        .await?;
    tick(tab, SETTLED).await?;
    let end = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Focused(ink.primary),
        "filled-settled",
    )
    .await?;
    check!(
        end.glyphs.bounds[1] < middle.glyphs.bounds[1] && end.placeholder > 12,
        "the settled filled caption clears the placeholder after cancel"
    );
    let last = state(tab).await?[field.model()].clone();
    check!(
        last["edits"] == 0 && last["submits"] == 0 && last["value"] == "",
        "composition cancel preserves the untouched caller model"
    );
    let after = textbox(tab, TITLE, true, Some(HELP)).await?;
    check_eq!(
        after["backend"],
        active["backend"],
        "label movement keeps one actual editing host"
    );
    Ok(
        json!({"face":geometry.face,"inline":geometry.inline,"floated":geometry.floated,"palette":model["palette"],
        "ax":after,"preedit":{"png":preedit_name,"ms":MIDDLE,"ink_pixels":actual,"placeholder_pixels":placeholder},
        "rest":rest.report(0),"frames":[start.report(0),middle.report(MIDDLE),end.report(SETTLED)]}),
    )
}

fn solid_probes(image: &Raster, rect: Rect, color: Rgb) -> Result<[[f64; 2]; 4]> {
    // Find separate pixels of opaque glyph interior. Later frames must
    // actually paint the predicted color at the same points.
    let columns = (rect[0] * DPI).ceil() as usize..(right(rect) * DPI).floor() as usize;
    let rows = (rect[1] * DPI).ceil() as usize..(bottom(rect) * DPI).floor() as usize;
    let mut probes: Vec<[f64; 2]> = Vec::new();
    for column in columns {
        let x = (column as f64 + 0.5) / DPI;
        if probes.last().is_some_and(|[previous, _]| x - previous < 3.) {
            continue;
        }
        for row in rows.clone() {
            let y = (row as f64 + 0.5) / DPI;
            if image.sample(x, y)? == color
                && image.sample(x - 0.5, y)? == color
                && image.sample(x + 0.5, y)? == color
            {
                probes.push([x, y]);
                break;
            }
        }
        if probes.len() == 4 {
            break;
        }
    }
    probes
        .try_into()
        .ok()
        .context("four separated solid label glyph samples")
}

fn fixed_label_frame(
    name: String,
    image: &Raster,
    geometry: &Geometry,
    clock: u16,
    color: Rgb,
    probes: &[[f64; 2]; 4],
) -> Result<Value> {
    let glyphs = glyphs(image, geometry, color)?;
    let actual = probes
        .iter()
        .map(|&[x, y]| image.sample(x, y))
        .collect::<Result<Vec<_>>>()?;
    check!(
        actual.iter().all(|pixel| *pixel == color),
        format!("unchanged glyph positions paint golden {color:?}, got {actual:?}")
    );
    Ok(json!({
        "png":name,"ms":clock,"label":glyphs.bounds,"label_rgb":color,
        "label_pixels":glyphs.pixels,"solid_probe_rgb":actual
    }))
}

async fn fixed_photo(
    tab: &Tab,
    env: &Env,
    geometry: &Geometry,
    clock: u16,
    color: Rgb,
    probes: &[[f64; 2]; 4],
    phase: &str,
) -> Result<Value> {
    geometry.assert_stable(tab).await?;
    check_eq!(
        bounds(tab, &geometry.field.id("editable-floating-label")).await?,
        geometry.floated,
        "color-only motion preserves the floating label geometry"
    );
    let suffix = format!("{}-color-{phase}", geometry.field.appearance());
    let (name, image) = photo(tab, env, &suffix).await?;
    fixed_label_frame(name, &image, geometry, clock, color, probes)
}

async fn draft_ax(tab: &Tab, active: bool, invalid: bool) -> Result<Value> {
    let expected = if invalid { ERROR_HELP } else { HELP };
    let node = textbox(tab, TITLE, active, None).await?;
    check_eq!(
        node["invalid"],
        invalid,
        "validation semantics are already current"
    );
    check_eq!(
        node["value"],
        DRAFT,
        "label recoloring never changes the Chrome AX value"
    );
    check!(
        node["description"]
            .as_str()
            .is_some_and(|text| text.contains(expected)),
        "Chrome already describes the expected validation or supporting text"
    );
    if active {
        check_eq!(
            tab.eval(&format!("{TEXTAREA}.value")).await?,
            DRAFT,
            "the sole editing host keeps the draft"
        );
        check_eq!(
            tab.eval(&format!("{TEXTAREA}.getAttribute('aria-invalid')"))
                .await?,
            json!(invalid.then_some("true")),
            "the actual textarea applies error semantics without waiting for ink"
        );
    }
    Ok(node)
}

async fn toggle_error(tab: &Tab, field: Field, enabled: bool) -> Result<()> {
    leave(tab, field).await?;
    tab.press("Enter").await?;
    until(
        tab,
        &format!("s.{}.show_error === {enabled}", field.model()),
    )
    .await?;
    tab.settle().await
}

async fn fixed_label_color(tab: &Tab, env: &Env, url: &str, field: Field) -> Result<Value> {
    let model = scene(tab, url, field, false).await?;
    let geometry = Geometry::read(tab, field, false).await?;
    let ink = Ink::read(&model, field)?;
    enter(tab, field).await?;
    tab.insert_text(DRAFT).await?;
    until(tab, &format!("s.{}.value === {}", field.model(), js(DRAFT))).await?;
    tick(tab, SETTLED).await?;
    leave(tab, field).await?;
    let at_rest = 2 * SETTLED;
    tick(tab, at_rest).await?;

    let rest_ax = draft_ax(tab, false, false).await?;
    let suffix = format!("{}-color-rest", field.appearance());
    let (name, image) = photo(tab, env, &suffix).await?;
    let glyphs = glyphs(&image, &geometry, ink.label)?;
    let probes = solid_probes(&image, glyphs.bounds, ink.label)?;
    let rest = fixed_label_frame(name, &image, &geometry, at_rest, ink.label, &probes)?;

    enter(tab, field).await?;
    let active_ax = draft_ax(tab, true, false).await?;
    let focus_at = at_rest + MIDDLE;
    tick(tab, focus_at).await?;
    let focus_pose = golden::Pose::at(ink.label).towards(ink.primary, MIDDLE);
    let focus_color = focus_pose.rgb();
    check!(
        focus_color != ink.label && focus_color != ink.primary,
        "the expected focused ink is intermediate"
    );
    let focus_mid = fixed_photo(
        tab,
        env,
        &geometry,
        focus_at,
        focus_color,
        &probes,
        "focus-mid",
    )
    .await?;

    toggle_error(tab, field, true).await?;
    let error_proxy = draft_ax(tab, false, true).await?;
    enter(tab, field).await?;
    let error_ax = draft_ax(tab, true, true).await?;
    let error_first = fixed_photo(
        tab,
        env,
        &geometry,
        focus_at,
        focus_color,
        &probes,
        "error-first",
    )
    .await?;
    let error_at = focus_at + MIDDLE;
    tick(tab, error_at).await?;
    let error_color = focus_pose.towards(ink.error, MIDDLE).rgb();
    check!(
        error_color != focus_color && error_color != ink.error,
        "the expected error ink is intermediate"
    );
    let error_mid = fixed_photo(
        tab,
        env,
        &geometry,
        error_at,
        error_color,
        &probes,
        "error-mid",
    )
    .await?;
    tick(tab, error_at + SETTLED).await?;
    let invalid_settled = fixed_photo(
        tab,
        env,
        &geometry,
        error_at + SETTLED,
        ink.error,
        &probes,
        "error-settled",
    )
    .await?;

    // Clear the error, let the ink move toward focus, then reverse before
    // settling. Error semantics update immediately in both directions.
    let return_at = error_at + SETTLED + MIDDLE;
    toggle_error(tab, field, false).await?;
    enter(tab, field).await?;
    let clear_ax = draft_ax(tab, true, false).await?;
    tick(tab, return_at).await?;
    let clearing_pose = golden::Pose::at(ink.error).towards(ink.primary, MIDDLE);
    let clearing = fixed_photo(
        tab,
        env,
        &geometry,
        return_at,
        clearing_pose.rgb(),
        &probes,
        "clearing",
    )
    .await?;
    toggle_error(tab, field, true).await?;
    enter(tab, field).await?;
    let reversed_ax = draft_ax(tab, true, true).await?;
    let reversed = fixed_photo(
        tab,
        env,
        &geometry,
        return_at,
        clearing_pose.rgb(),
        &probes,
        "reversed",
    )
    .await?;
    reduced_motion(tab, true).await?;
    let snapped = fixed_photo(
        tab, env, &geometry, return_at, ink.error, &probes, "reduced",
    )
    .await?;

    check!(
        error_proxy["backend"] != active_ax["backend"]
            && [&error_ax, &clear_ax, &reversed_ax]
                .into_iter()
                .all(|node| node["backend"] == active_ax["backend"])
            && active_ax["backend"] != rest_ax["backend"],
        "color and validation keep one editor backend distinct from inactive proxies"
    );
    let after = state(tab).await?[field.model()].clone();
    check!(
        after["value"] == DRAFT
            && after["edits"] == 1
            && after["submits"] == 0
            && after["show_error"] == true,
        "one caller edit survives real focus and validation changes"
    );
    let keys = tab.eval("window.labelMotionKeys").await?;
    check_eq!(
        keys.as_array()
            .context("trusted fixture keys")?
            .iter()
            .filter(|key| key["key"] == "Enter")
            .count(),
        3,
        "trusted keys change validation three times"
    );
    Ok(json!({
        "face":geometry.face,"floated":geometry.floated,"helper":geometry.help,
        "palette":model["palette"],"probe_points":probes,
        "frames":[rest,focus_mid,error_first,error_mid,invalid_settled,clearing,reversed,snapped],
        "ax":{"rest":rest_ax,"focused":active_ax,"error_proxy":error_proxy,"error":error_ax,
            "cleared":clear_ax,"reversed":reversed_ax},
        "trusted":keys,"model":{"value":after["value"],"edits":after["edits"],"submits":after["submits"],"error":after["show_error"]}
    }))
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    let field = Field::Outlined;
    let model = scene(tab, url, field, true).await?;
    let geometry = Geometry::read(tab, field, true).await?;
    let ink = Ink::read(&model, field)?;
    let closed = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Idle(ink.label),
        "rtl-rest",
    )
    .await?;
    enter(tab, field).await?;
    tick(tab, MIDDLE).await?;
    let moving = capture(
        tab,
        env,
        &geometry,
        ink,
        LabelPaint::Focused(golden::between(ink.label, ink.primary, MIDDLE)),
        "rtl-middle",
    )
    .await?;
    check!(
        geometry.face[2] <= 320.
            && (right(geometry.face) - geometry.anchor() - 16.).abs() <= 0.5
            && moving.glyphs.bounds[1] < closed.glyphs.bounds[1] - 4.
            && moving.glyphs.bounds[0] > closed.glyphs.bounds[0] + 3.
            && (right(moving.glyphs.bounds) - right(closed.glyphs.bounds)).abs() <= 3.,
        "RTL preserves the physical right anchor as the ink moves up and shrinks"
    );
    check!(
        moving.gap_columns > 12 && closed.gap_columns == 0,
        "the moving short label opens the right-side notch"
    );
    tick(tab, SETTLED).await?;
    tab.press("Tab").await?;
    focus(tab, &field.id("read-only"), "surface-text-input").await?;
    let long = field.id("long-label");
    let face = bounds(tab, &long).await?;
    let above_id = field.id("long-label-above-label");
    let above = bounds(tab, &above_id).await?;
    let floated = bounds(tab, &field.id("long-label-floating-label")).await?;
    check!(
        contains(geometry.viewport, face)
            && contains(geometry.viewport, above)
            && above[3] >= 64.
            && bottom(above) + 3.5 <= face[1]
            && floated[2] == 0.,
        "the long label wraps above the complete narrow editor"
    );
    let (idle_name, idle) = photo(tab, env, "rtl-above-label-rest").await?;
    let idle_pixels = color_pixels(&idle, above, ink.label)?;
    check!(
        idle_pixels > 40,
        "the dark above-label fallback paints the idle ink"
    );
    let probes = solid_probes(&idle, above, ink.label)?;
    tab.press("Tab").await?;
    focus(tab, &long, "surface-text-input").await?;
    tick(tab, SETTLED + MIDDLE).await?;
    let color = golden::between(ink.label, ink.primary, MIDDLE);
    let (mid_name, middle) = photo(tab, env, "rtl-above-label-middle").await?;
    let mid_pixels = color_pixels(&middle, above, color)?;
    let probe_colors = probes
        .iter()
        .map(|&[x, y]| middle.sample(x, y))
        .collect::<Result<Vec<_>>>()?;
    check!(
        color != ink.label
            && color != ink.primary
            && mid_pixels > 40
            && probe_colors.iter().all(|sample| *sample == color),
        "the wrapped fallback changes only ink at the same glyph positions"
    );
    check_eq!(
        bounds(tab, &long).await?,
        face,
        "the dark fallback editor keeps its full bounds"
    );
    check_eq!(
        bounds(tab, &above_id).await?,
        above,
        "the wrapped label keeps its layout bounds"
    );
    check_eq!(
        bounds(tab, &field.id("long-label-floating-label")).await?,
        floated,
        "focus leaves the compact floating slot collapsed"
    );
    tick(tab, 2 * SETTLED).await?;
    let (name, frame) = photo(tab, env, "rtl-above-label").await?;
    let above_pixels = color_pixels(&frame, above, ink.primary)?;
    let top_point = [face[0] + face[2] / 2., face[1] + 0.5];
    check_eq!(
        frame.sample(top_point[0], top_point[1])?,
        ink.primary,
        "the focused above-label fallback has a closed top edge"
    );
    check!(
        above_pixels > 40,
        "the full fallback includes visible label ink"
    );
    let label = "輸入團隊成員在共用專案清單中容易辨認的完整名稱";
    let ax = textbox(tab, label, true, None).await?;
    Ok(
        json!({"face":geometry.face,"anchor":geometry.anchor(),"floated":geometry.floated,
        "palette":model["palette"],"frames":[closed.report(0),moving.report(MIDDLE)],
        "fallback":{"png":name,"ms":SETTLED,"face":face,"above":above,"pixels":above_pixels,
            "solid_top":top_point,"float":floated,"ax":ax,
            "label_rgb":ink.primary,"probe_points":probes,
            "color_frames":[
                {"png":idle_name,"ms":0,"label_rgb":ink.label,"label_pixels":idle_pixels},
                {"png":mid_name,"ms":MIDDLE,"label_rgb":color,"label_pixels":mid_pixels,
                    "solid_probe_rgb":probe_colors}
            ]}}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let mut report =
        json!({"browser":chrome.version,"visible_clock":"controlled performance.now","dpr":DPI});
    let result: Result<()> = async {
        step(
            STEPS[0],
            "Outlined label moves with a progressively wider and deeper real cutout",
        );
        let (geometry, ink, settled) = outlined(&tab, env, url, &mut report).await?;
        step(
            STEPS[1],
            "Trusted focus reversal preserves painted position and Reduce Motion snaps",
        );
        report["reversal"] = reversal(&tab, env, &geometry, ink, &settled).await?;
        step(
            STEPS[2],
            "Filled label protects marked text and both appearances retarget independent label ink",
        );
        report["filled"] = filled(&tab, env, url).await?;
        report["label_color"] = json!({
            "space":"Oklab","spring":{"stiffness":3800,"damping_ratio":1},
            "outlined":fixed_label_color(&tab, env, url, Field::Outlined).await?,
            "filled":fixed_label_color(&tab, env, url, Field::Filled).await?
        });
        step(
            STEPS[3],
            "Dark 320 dp RTL at 200 percent keeps the right anchor and the closed fallback",
        );
        report["compact"] = compact(&tab, env, url).await?;
        check!(tab.errors().is_empty(), "no label motion page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact(&format!("{PREFIX}-failure.png")))
            .await;
    }
    env.write_json(REPORT, &report)?;
    result
}

// The test uses the CSS Color 4 reference path through XYZ D65, independent
// of the direct sRGB/Oklab matrices in Surface. All fixture label inks are
// opaque. https://www.w3.org/TR/css-color-4/#color-conversion-code
mod golden {
    use super::Rgb;

    type Color = [f64; 3];

    fn matrix(rows: [[f64; 3]; 3], color: Color) -> Color {
        rows.map(|[a, b, c]| a * color[0] + b * color[1] + c * color[2])
    }

    fn to_lab(rgb: Rgb) -> Color {
        let linear = rgb.map(|byte| {
            let x = f64::from(byte) / 255.;
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        });
        let xyz = matrix(
            [
                [0.412_390_799_3, 0.357_584_339_4, 0.180_480_788_4],
                [0.212_639_005_9, 0.715_168_678_8, 0.072_192_315_4],
                [0.019_330_818_7, 0.119_194_779_8, 0.950_532_152_2],
            ],
            linear,
        );
        let lms = matrix(
            [
                [0.819_022_438, 0.361_906_260_1, -0.128_873_781_5],
                [0.032_983_653_9, 0.929_286_861_6, 0.036_144_666_4],
                [0.048_177_189_4, 0.264_239_531_8, 0.633_547_828_5],
            ],
            xyz,
        );
        matrix(
            [
                [0.210_454_268_3, 0.793_617_774_7, -0.004_072_043],
                [1.977_998_532_4, -2.428_592_242, 0.450_593_709_6],
                [0.025_904_042_5, 0.782_771_712_5, -0.808_675_754_9],
            ],
            lms.map(f64::cbrt),
        )
    }

    fn to_srgb(lab: Color) -> Rgb {
        let lms = matrix(
            [
                [1., 0.396_337_777_4, 0.215_803_757_3],
                [1., -0.105_561_345_8, -0.063_854_172_8],
                [1., -0.089_484_177_5, -1.291_485_548],
            ],
            lab,
        );
        let xyz = matrix(
            [
                [1.226_879_875_8, -0.557_814_994_5, 0.281_391_045_7],
                [-0.040_575_745_2, 1.112_286_803_3, -0.071_711_058_1],
                [-0.076_372_936_7, -0.421_493_332_4, 1.586_924_019_8],
            ],
            lms.map(|channel| channel.powi(3)),
        );
        matrix(
            [
                [3.240_969_941_9, -1.537_383_177_6, -0.498_610_760_3],
                [-0.969_243_636_3, 1.875_967_501_5, 0.041_555_057_4],
                [0.055_630_079_7, -0.203_976_958_9, 1.056_971_514_2],
            ],
            xyz,
        )
        .map(|linear| {
            let channel = if linear <= 0.003_130_8 {
                12.92 * linear
            } else {
                1.055 * linear.powf(1. / 2.4) - 0.055
            };
            (channel.clamp(0., 1.) * 255.).round() as u8
        })
    }

    pub(super) struct Pose {
        position: Color,
        velocity: Color,
    }

    impl Pose {
        pub(super) fn at(rgb: Rgb) -> Self {
            Self {
                position: to_lab(rgb),
                velocity: [0.; 3],
            }
        }

        pub(super) fn towards(&self, rgb: Rgb, ms: u16) -> Self {
            let target = to_lab(rgb);
            let seconds = f64::from(ms) / 1000.;
            let omega = 3800_f64.sqrt();
            let decay = (-omega * seconds).exp();
            // Analytic solution for a critically damped spring. Retarget
            // from unrounded position and velocity, not displayed bytes.
            let coefficient: Color =
                std::array::from_fn(|i| self.velocity[i] + omega * (self.position[i] - target[i]));
            Self {
                position: std::array::from_fn(|i| {
                    target[i] + (self.position[i] - target[i] + coefficient[i] * seconds) * decay
                }),
                velocity: std::array::from_fn(|i| {
                    (self.velocity[i] - omega * coefficient[i] * seconds) * decay
                }),
            }
        }

        pub(super) fn rgb(&self) -> Rgb {
            to_srgb(self.position)
        }
    }

    pub(super) fn between(from: Rgb, to: Rgb, ms: u16) -> Rgb {
        Pose::at(from).towards(to, ms).rgb()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn retained_color_is_not_a_byte_mix_or_a_fresh_spring() {
            let idle = [73, 69, 78];
            let focus = [101, 85, 143];
            let error = [186, 26, 26];
            assert_eq!(between(idle, focus, 14), [79, 73, 92]);
            let mid = Pose::at(idle).towards(focus, 44);
            assert_eq!(mid.rgb(), [94, 81, 127]);
            assert_eq!(mid.towards(error, 0).rgb(), mid.rgb());
            assert_ne!(mid.towards(error, 44).rgb(), between(mid.rgb(), error, 44));
        }
    }
}
