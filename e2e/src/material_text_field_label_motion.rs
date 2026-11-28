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
            if (event.isTrusted && (event.key === "Tab" || event.key === "Escape"))
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
    Ok(
        json!({"label":label,"backend":node["backendDOMNodeId"],"description":node["description"]["value"],"focused":focused}),
    )
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
    check!(pixels > 12, "the moving label has real glyph ink");
    Ok(Glyphs {
        bounds: [
            min[0] - 0.25,
            min[1] - 0.25,
            max[0] - min[0] + 0.5,
            max[1] - min[1] + 0.5,
        ],
        pixels,
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
            "placeholder_pixels":self.placeholder,"gap_pixel_columns":self.gap_columns,
            "notch":self.notch,"outline_sides":self.outline_sides})
    }
}

async fn capture(
    tab: &Tab,
    env: &Env,
    geometry: &Geometry,
    ink: Ink,
    active: bool,
    name: &str,
) -> Result<Shot> {
    geometry.assert_stable(tab).await?;
    let (name, image) = photo(tab, env, name).await?;
    let label = if active { ink.primary } else { ink.label };
    let gap = matches!(geometry.field, Field::Outlined);
    Ok(Shot {
        name,
        glyphs: glyphs(&image, geometry, label)?,
        placeholder: if active {
            color_pixels(&image, geometry.clearance(), ink.label)?
        } else {
            0
        },
        gap_columns: if gap {
            top_paper(&image, geometry, ink.paper)?
        } else {
            0
        },
        notch: gap.then(|| notch_sample(&image, geometry)).transpose()?,
        outline_sides: gap
            .then(|| {
                sides(
                    &image,
                    geometry.face,
                    if active { ink.primary } else { ink.outline },
                )
            })
            .transpose()?,
    })
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
    let rest = capture(tab, env, &geometry, ink, false, "outlined-rest").await?;
    let proxy = textbox(tab, TITLE, false, Some(HELP)).await?;
    enter(tab, field).await?;
    let active = textbox(tab, TITLE, true, Some(HELP)).await?;
    check!(
        proxy["backend"] != active["backend"],
        "focus hands the stable name to the textarea"
    );
    let first = capture(tab, env, &geometry, ink, true, "outlined-first").await?;
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
    let early = capture(tab, env, &geometry, ink, true, "outlined-early").await?;
    check_eq!(
        early.notch,
        Some([ink.paper, ink.primary]),
        "the early gap removes only the upper part of the three-dp stroke"
    );
    tick(tab, MIDDLE).await?;
    let mid = capture(tab, env, &geometry, ink, true, "outlined-middle").await?;
    tick(tab, SETTLED).await?;
    let settled = capture(tab, env, &geometry, ink, true, "outlined-settled").await?;
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
    let falling = capture(tab, env, geometry, ink, false, "outlined-falling").await?;
    check!(
        falling.glyphs.bounds[1] > settled.glyphs.bounds[1] + 2.
            && falling.glyphs.bounds[1] < geometry.inline[1] + 8.,
        "the falling label has an intermediate painted position"
    );
    enter(tab, field).await?;
    let retarget = capture(tab, env, geometry, ink, true, "outlined-retarget").await?;
    check_eq!(
        retarget.glyphs.bounds,
        falling.glyphs.bounds,
        "reversal starts from the exact current glyph pose"
    );
    reduced_motion(tab, true).await?;
    let snapped = capture(tab, env, geometry, ink, true, "outlined-reduced").await?;
    check_eq!(
        snapped.glyphs.bounds,
        settled.glyphs.bounds,
        "Reduce Motion snaps to the settled glyph pose at the same time"
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
    enter(tab, field).await?;
    let active = textbox(tab, TITLE, true, Some(HELP)).await?;
    let start = capture(tab, env, &geometry, ink, true, "filled-first").await?;
    tick(tab, MIDDLE).await?;
    let middle = capture(tab, env, &geometry, ink, true, "filled-middle").await?;
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
    let end = capture(tab, env, &geometry, ink, true, "filled-settled").await?;
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
        "frames":[start.report(0),middle.report(MIDDLE),end.report(SETTLED)]}),
    )
}

async fn compact(tab: &Tab, env: &Env, url: &str) -> Result<Value> {
    let field = Field::Outlined;
    let model = scene(tab, url, field, true).await?;
    let geometry = Geometry::read(tab, field, true).await?;
    let ink = Ink::read(&model, field)?;
    let closed = capture(tab, env, &geometry, ink, false, "rtl-rest").await?;
    enter(tab, field).await?;
    tick(tab, MIDDLE).await?;
    let moving = capture(tab, env, &geometry, ink, true, "rtl-middle").await?;
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
    tab.press("Tab").await?;
    let long = field.id("long-label");
    focus(tab, &long, "surface-text-input").await?;
    let face = bounds(tab, &long).await?;
    let above = bounds(tab, &field.id("long-label-above-label")).await?;
    let floated = bounds(tab, &field.id("long-label-floating-label")).await?;
    check!(
        contains(geometry.viewport, face)
            && contains(geometry.viewport, above)
            && above[3] >= 64.
            && bottom(above) + 3.5 <= face[1]
            && floated[2] == 0.,
        "the long label wraps above the complete narrow editor"
    );
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
            "solid_top":top_point,"float":floated,"ax":ax}}),
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
            "Filled label shares one editor and keeps placeholder out of marked text",
        );
        report["filled"] = filled(&tab, env, url).await?;
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
