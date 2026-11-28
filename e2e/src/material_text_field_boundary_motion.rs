//! Retained Text Field boundary width and ink sampled from headless Canvas PNGs.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_list_corners::{leave_rows, reduced_motion, tick};
use super::material_text_field_label_motion::{
    enter, golden, leave, scene, textbox, toggle_error, Field,
};
use super::material_time_input::{ax, ax_tree, bounds, contains, Raster};
use super::{click, focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 3] = ["M354", "M355", "M356"];
const PREFIX: &str = "material-text-field-boundary-motion";
const DPI: f64 = 2.;
const END: u16 = 800;
const TITLE: &str = "專案名稱";
const HELP: &str = "請輸入中文或英文名稱。";
const ERROR_HELP: &str = "請輸入有效的專案名稱。";
const LONG: &str = "輸入團隊成員在共用專案清單中容易辨認的完整名稱";
const DRAFT: &str = "草稿";
type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Clone, Copy)]
struct WidthPose {
    dp: f64,
    velocity: f64,
}

impl WidthPose {
    fn at(dp: f64) -> Self {
        Self { dp, velocity: 0. }
    }

    fn towards(self, field: Field, target: f64, ms: u16) -> Self {
        let t = f64::from(ms) / 1000.;
        let offset = self.dp - target;
        let (displacement, derivative, damping) = match field {
            Field::Outlined => {
                let damping = 3800_f64.sqrt();
                let slope = self.velocity + damping * offset;
                (offset + slope * t, slope, damping)
            }
            Field::Filled => {
                let rate = 1400_f64.sqrt();
                let damping = rate * 0.9;
                let frequency = rate * (1_f64 - 0.9_f64.powi(2)).sqrt();
                let (sine, cosine) = (frequency * t).sin_cos();
                let weight = (self.velocity + damping * offset) / frequency;
                (
                    offset * cosine + weight * sine,
                    frequency * (weight * cosine - offset * sine),
                    damping,
                )
            }
        };
        let decay = (-damping * t).exp();
        Self {
            dp: target + displacement * decay,
            velocity: (derivative - damping * displacement) * decay,
        }
    }
}

fn rgb(model: &Value, key: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(model["palette"][key].clone())
        .with_context(|| format!("Gallery boundary role {key}"))?;
    check_eq!(a, 255, format!("opaque Gallery role {key}"));
    Ok([r, g, b])
}

struct Palette {
    paper: Rgb,
    base: Rgb,
    idle: Rgb,
    primary: Rgb,
    hover: Rgb,
    error: Rgb,
    error_hover: Rgb,
}

impl Palette {
    fn read(model: &Value, field: Field) -> Result<Self> {
        let paper = rgb(model, "background")?;
        let (base, idle) = match field {
            Field::Outlined => (paper, rgb(model, "outline")?),
            Field::Filled => (
                rgb(model, "surface_container_highest")?,
                rgb(model, "on_surface_variant")?,
            ),
        };
        Ok(Self {
            paper,
            base,
            idle,
            primary: rgb(model, "primary")?,
            hover: rgb(model, "on_surface")?,
            error: rgb(model, "error")?,
            error_hover: rgb(model, "on_error_container")?,
        })
    }
}

#[derive(Clone, Copy)]
struct Stroke {
    width: f64,
    rgb: Rgb,
    alpha: u8,
}

impl Stroke {
    fn opaque(width: f64, rgb: Rgb) -> Self {
        Self {
            width,
            rgb,
            alpha: 255,
        }
    }
}

fn mixture(base: Rgb, ink: Rgb, alpha: f64, linear: bool) -> Rgb {
    let decode = |channel: u8| {
        let v = f64::from(channel) / 255.;
        if !linear {
            v
        } else if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    std::array::from_fn(|i| {
        let v = decode(base[i]) * (1. - alpha) + decode(ink[i]) * alpha;
        let encoded = if !linear {
            v
        } else if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        };
        (encoded * 255.).round() as u8
    })
}

// Choose the same renderer blend convention for every partial sample.
struct BlendModes([bool; 2]);

impl BlendModes {
    fn match_color(&mut self, actual: Rgb, base: Rgb, ink: Rgb, alpha: f64) -> bool {
        if alpha == 0. {
            return actual == base;
        }
        if alpha == 1. {
            return actual == ink;
        }
        for (mode, is_linear) in self.0.iter_mut().zip([false, true]) {
            *mode &= actual
                .into_iter()
                .zip(mixture(base, ink, alpha, is_linear))
                .all(|(actual, expected)| actual.abs_diff(expected) <= 2);
        }
        self.0.into_iter().any(|valid| valid)
    }
}

struct Face {
    field: Field,
    id: String,
    bounds: Rect,
    clip: Rect,
    description: Option<Rect>,
    canvas: Rect,
}

impl Face {
    async fn read(tab: &Tab, field: Field, part: &str) -> Result<Self> {
        let id = field.id(part);
        let rect = bounds(tab, &id).await?;
        let clip = serde_json::from_value::<Rect>(
            tab.eval(&format!(
                "(() => {{
                    const s = JSON.parse(window.gallery.export_dom());
                    const i = s.dom.nodes.findIndex(n => n.stable_id === {});
                    const r = s.layout.nodes[i]?.clip;
                    return r ? [r.x,r.y,r.width,r.height] : null;
                }})()",
                js(&id)
            ))
            .await?,
        )
        .context("field clip in the actual exported layout")?;
        let canvas = tab.canvas_box().await?;
        let [x, y, w, h] = rect;
        check!(
            w >= 112.
                && h >= 48.
                && contains(clip, rect)
                && contains(
                    [0., 0., canvas[2], canvas[3]],
                    [x - 1., y - 1., w + 2., h + 2.]
                ),
            format!("the complete face and its exterior probes are visible: {rect:?}")
        );
        let description = if part == "editable" {
            Some(bounds(tab, &format!("{id}-description")).await?)
        } else {
            None
        };
        Ok(Self {
            field,
            id,
            bounds: rect,
            clip,
            description,
            canvas,
        })
    }

    async fn stable(&self, tab: &Tab) -> Result<()> {
        check_eq!(
            bounds(tab, &self.id).await?,
            self.bounds,
            "unchanged text input hit bounds"
        );
        if let Some(previous) = self.description {
            check_eq!(
                bounds(tab, &format!("{}-description", self.id)).await?,
                previous,
                "unchanged supporting text layout"
            );
        }
        Ok(())
    }

    fn move_cursor(&self) -> [f64; 2] {
        [
            self.canvas[0] + self.bounds[0] + self.bounds[2] / 2.,
            self.canvas[1] + self.bounds[1] + self.bounds[3] / 2.,
        ]
    }
}

#[derive(Clone, Copy)]
struct Strip {
    axis: usize,
    edge: f64,
    along: f64,
    inward: f64,
}

impl Strip {
    fn top(y: f64, x: f64) -> Self {
        Self {
            axis: 1,
            edge: y,
            along: x,
            inward: 1.,
        }
    }

    fn point(self, normal: f64) -> [f64; 2] {
        if self.axis == 1 {
            [self.along, normal]
        } else {
            [normal, self.along]
        }
    }

    fn samples(
        self,
        frame: &Frame,
        stroke: Stroke,
        ink_starts_at: f64,
        base: Rgb,
        paper: Rgb,
        blend: &mut BlendModes,
    ) -> Result<Value> {
        let origin = frame.canvas[self.axis];
        let grid_edge = ((origin + self.edge) * DPI).floor() as i64;
        let mut samples = Vec::new();
        for relative in -2..13 {
            let index = if self.inward > 0. {
                grid_edge + relative
            } else {
                grid_edge - 1 - relative
            };
            let coord = (index as f64 + 0.5) / DPI - origin;
            let depth = (coord - self.edge) * self.inward;
            let quarter = [-0.375, -0.125, 0.125, 0.375]
                .into_iter()
                .filter(|offset| (ink_starts_at..stroke.width).contains(&(depth + offset / DPI)))
                .count();
            let [x, y] = self.point(coord);
            let actual = frame.pixels.sample(x, y)?;
            let under = if depth < 0. { paper } else { base };
            check!(
                blend.match_color(actual, under, stroke.rgb, f64::from(stroke.alpha) * quarter as f64 / (255. * 4.)),
                format!("{} [{x:.3}, {y:.3}] depth {depth:.3}: {actual:?}, expected {:?} over {under:?} at {quarter}/4 coverage for width {:.4}",
                    frame.name, stroke.rgb, stroke.width)
            );
            samples.push(
                json!({"point":[x,y],"depth_dp":depth,"coverage_quarters":quarter,"rgb":actual}),
            );
        }
        Ok(
            json!({"edge_dp":self.edge, "normal_axis":self.axis, "inward":self.inward,
            "ink_starts_at_dp":ink_starts_at,"samples":samples}),
        )
    }
}

struct Frame {
    name: String,
    pixels: Raster,
    canvas: Rect,
}

impl Frame {
    async fn take(tab: &Tab, env: &Env, tag: &str) -> Result<Self> {
        tab.settle().await?;
        let name = format!("{PREFIX}-{tag}.png");
        let path = env.artifact(&name);
        tab.screenshot(&path).await?;
        Ok(Self {
            name,
            pixels: Raster::read(tab, path).await?,
            canvas: tab.canvas_box().await?,
        })
    }

    fn stripe_report(
        &self,
        face: &Face,
        ink: Stroke,
        colors: &Palette,
        base: Rgb,
        blend: &mut BlendModes,
        rtl: bool,
    ) -> Result<Value> {
        let [x, y, w, h] = face.bounds;
        let bottom = [0.25, 0.5, 0.75]
            .into_iter()
            .map(|fraction| {
                Strip {
                    axis: 1,
                    edge: y + h,
                    along: x + fraction * w,
                    inward: -1.,
                }
                .samples(self, ink, 0., base, colors.paper, blend)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut other = Vec::new();
        match face.field {
            Field::Outlined => {
                for strip in [
                    Strip {
                        axis: 0,
                        edge: x,
                        along: y + h / 2.,
                        inward: 1.,
                    },
                    Strip {
                        axis: 0,
                        edge: x + w,
                        along: y + h / 2.,
                        inward: -1.,
                    },
                    Strip::top(y, x + w * if rtl { 0.25 } else { 0.75 }),
                ] {
                    other.push(strip.samples(self, ink, 0., base, colors.paper, blend)?);
                }
            }
            Field::Filled => {
                for point in [
                    [x + 4., y + h / 2.],
                    [x + w - 4., y + h / 2.],
                    [x + w * 0.75, y + 0.25],
                ] {
                    let actual = self.pixels.sample(point[0], point[1])?;
                    check_eq!(actual, base, "Filled has only the bottom indicator");
                    other.push(json!({"point":point,"rgb":actual}));
                }
            }
        }
        Ok(
            json!({"png":self.name,"face":face.bounds,"clip":face.clip,"canvas":self.canvas,
            "width_dp":ink.width,"ink":ink.rgb,"ink_alpha":ink.alpha,"base":base,"paper":colors.paper,
            "bottom":bottom,"other":other}),
        )
    }

    fn base_sample(&self, face: &Face) -> Result<Rgb> {
        let [x, y, w, h] = face.bounds;
        self.pixels.sample(x + w * 0.75, y + h - 8.)
    }
}

struct Series<'a> {
    tab: &'a Tab,
    env: &'a Env,
    field: Field,
    colors: Palette,
    rtl: bool,
    blends: &'a mut BlendModes,
}

impl Series<'_> {
    async fn sample(
        &mut self,
        face: &Face,
        phase: &str,
        clock: u16,
        ink: Stroke,
        layer: u8,
    ) -> Result<(Value, Frame)> {
        face.stable(self.tab).await?;
        let tag = format!("{}-{}", self.field.appearance(), phase);
        let frame = Frame::take(self.tab, self.env, &tag).await?;
        let actual_base = frame.base_sample(face)?;
        let under = if layer == 10 {
            self.colors.paper
        } else {
            self.colors.base
        };
        check!(
            self.blends.match_color(
                actual_base,
                under,
                self.colors.hover,
                f64::from(layer) / 255.
            ),
            format!(
                "{} has the expected face color: {actual_base:?}",
                frame.name
            )
        );
        let mut report =
            frame.stripe_report(face, ink, &self.colors, actual_base, self.blends, self.rtl)?;
        report["clock_ms"] = json!(clock);
        report["layer_alpha"] = json!(layer);
        Ok((report, frame))
    }
}

async fn opening(s: &mut Series<'_>, face: &Face) -> Result<Value> {
    enter(s.tab, s.field).await?;
    s.tab.insert_text(DRAFT).await?;
    until(
        s.tab,
        &format!("s.{}.value === {}", s.field.model(), js(DRAFT)),
    )
    .await?;
    tick(s.tab, END).await?;
    leave(s.tab, s.field).await?;
    let start = 2 * END;
    tick(s.tab, start).await?;
    let before = textbox(s.tab, TITLE, false, Some(HELP)).await?;
    let color_start = s.colors.idle;
    let primary = s.colors.primary;
    let mut frames = vec![
        s.sample(face, "wide-idle", start, Stroke::opaque(1., color_start), 0)
            .await?
            .0,
    ];
    enter(s.tab, s.field).await?;
    let after = textbox(s.tab, TITLE, true, Some(HELP)).await?;
    check!(
        before["backend"] != after["backend"] && after["value"] == DRAFT,
        "focus changes host and preserves one named textbox value before the animation starts"
    );
    for (phase, ms) in [
        ("focus-start", 0),
        ("focus-14", 14),
        ("focus-44", 44),
        ("focus-end", END),
    ] {
        tick(s.tab, start + ms).await?;
        let (width, ink) = if ms == END {
            (3., primary)
        } else {
            (
                WidthPose::at(1.).towards(s.field, 3., ms).dp,
                golden::between(color_start, primary, ms),
            )
        };
        frames.push(
            s.sample(face, phase, start + ms, Stroke::opaque(width, ink), 0)
                .await?
                .0,
        );
    }
    let last = textbox(s.tab, TITLE, true, Some(HELP)).await?;
    let model = &state(s.tab).await?[s.field.model()];
    check!(
        last == after && model["edits"] == 1 && model["submits"] == 0,
        "paint keeps the active input backend and caller state"
    );
    Ok(json!({"frames":frames,"rest_ax":before,"focused_ax":last}))
}

async fn palette_cases(s: &mut Series<'_>, face: &Face) -> Result<Value> {
    let [x, y] = face.move_cursor();
    let layer = if matches!(s.field, Field::Filled) {
        20
    } else {
        0
    };
    s.tab.mouse_move(x, y).await?;
    s.tab.settle().await?;
    tick(s.tab, 44).await?;
    let mid = golden::between(s.colors.idle, s.colors.hover, 44);
    let primary = s.colors.primary;
    let error = s.colors.error;
    let hover = s.colors.hover;
    let error_hover = s.colors.error_hover;
    check!(
        mid != s.colors.idle && mid != hover,
        "hover color has an intermediate opaque value"
    );
    let mut frames = vec![
        s.sample(face, "hover-44", 44, Stroke::opaque(1., mid), layer)
            .await?
            .0,
    ];
    tick(s.tab, END).await?;
    frames.push(
        s.sample(face, "hover-end", END, Stroke::opaque(1., hover), layer)
            .await?
            .0,
    );
    enter(s.tab, s.field).await?;
    tick(s.tab, 2 * END).await?;
    frames.push(
        s.sample(
            face,
            "focus-over-hover",
            2 * END,
            Stroke::opaque(3., primary),
            0,
        )
        .await?
        .0,
    );
    leave_rows(s.tab).await?;
    toggle_error(s.tab, s.field, true).await?;
    let invalid = textbox(s.tab, TITLE, false, Some(ERROR_HELP)).await?;
    check_eq!(
        invalid["invalid"],
        true,
        "AX already reports invalid while border ink is moving"
    );
    let error_ms = 2 * END + 44;
    tick(s.tab, error_ms).await?;
    let width = WidthPose::at(3.).towards(s.field, 1., 44).dp;
    let color = golden::between(primary, error, 44);
    frames.push(
        s.sample(
            face,
            "invalid-44",
            error_ms,
            Stroke::opaque(width, color),
            0,
        )
        .await?
        .0,
    );
    tick(s.tab, 3 * END).await?;
    frames.push(
        s.sample(face, "invalid-idle", 3 * END, Stroke::opaque(1., error), 0)
            .await?
            .0,
    );
    s.tab.mouse_move(x, y).await?;
    s.tab.settle().await?;
    tick(s.tab, 4 * END).await?;
    frames.push(
        s.sample(
            face,
            "invalid-hover",
            4 * END,
            Stroke::opaque(1., error_hover),
            layer,
        )
        .await?
        .0,
    );
    enter(s.tab, s.field).await?;
    let active = textbox(s.tab, TITLE, true, Some(ERROR_HELP)).await?;
    tick(s.tab, 5 * END).await?;
    frames.push(
        s.sample(
            face,
            "invalid-focus-over-hover",
            5 * END,
            Stroke::opaque(3., error),
            0,
        )
        .await?
        .0,
    );

    let disabled = Face::read(s.tab, s.field, "disabled").await?;
    let [x, y] = disabled.move_cursor();
    s.tab.mouse_move(x, y).await?;
    click(s.tab, &disabled.id).await?;
    let tree = ax_tree(s.tab).await?;
    let node = ax(&tree, "textbox", "停用名稱")?;
    let props = node["properties"]
        .as_array()
        .context("disabled input AX properties")?;
    let model = state(s.tab).await?;
    check!(
        props
            .iter()
            .any(|p| p["name"] == "disabled" && p["value"]["value"] == true)
            && model["focused"] != disabled.id
            && model[s.field.model()]["locked_edit_requests"] == 0,
        "the disabled specimen stays inert on trusted hover and click"
    );
    let (face_alpha, alpha) = match s.field {
        Field::Filled => (10, 97),
        Field::Outlined => (0, 31),
    };
    frames.push(
        s.sample(
            &disabled,
            "disabled",
            5 * END,
            Stroke {
                width: 1.,
                rgb: hover,
                alpha,
            },
            face_alpha,
        )
        .await?
        .0,
    );
    Ok(json!({"frames":frames,"invalid_ax":[invalid,active],"disabled_ax":node}))
}

async fn compact(s: &mut Series<'_>, face: &Face) -> Result<Value> {
    let primary = s.colors.primary;
    let idle = s.colors.idle;
    enter(s.tab, s.field).await?;
    tick(s.tab, END).await?;
    let [x, y, w, _] = face.bounds;
    let [fx, _, fw, _] = bounds(s.tab, &s.field.id("editable-floating-label")).await?;
    check_eq!(
        x + w - fx - fw,
        16.,
        "the compact floating label keeps its physical right inset"
    );
    let proved = s
        .sample(face, "rtl-focus", END, Stroke::opaque(3., primary), 0)
        .await?;
    let notch = Strip::top(y, fx + fw + 3.).samples(
        &proved.1,
        Stroke::opaque(3., primary),
        3.,
        s.colors.paper,
        s.colors.paper,
        s.blends,
    )?;
    let mut frames = vec![proved.0];
    leave(s.tab, s.field).await?;
    let turn = END + 44;
    tick(s.tab, turn).await?;
    let width = WidthPose::at(3.).towards(s.field, 1., 44);
    let color = golden::Pose::at(primary).towards(idle, 44);
    let falling = s
        .sample(
            face,
            "rtl-falling",
            turn,
            Stroke::opaque(width.dp, color.rgb()),
            0,
        )
        .await?
        .0;
    enter(s.tab, s.field).await?;
    let same_time = s
        .sample(
            face,
            "rtl-retarget",
            turn,
            Stroke::opaque(width.dp, color.rgb()),
            0,
        )
        .await?
        .0;
    check_eq!(
        falling["bottom"],
        same_time["bottom"],
        "same-time retarget retains actual border coverage and color"
    );
    frames.extend([falling, same_time]);
    tick(s.tab, turn + 20).await?;
    let turned = width.towards(s.field, 3., 20).dp;
    let retargeted = color.towards(primary, 20).rgb();
    frames.push(
        s.sample(
            face,
            "rtl-retarget-20",
            turn + 20,
            Stroke::opaque(turned, retargeted),
            0,
        )
        .await?
        .0,
    );
    reduced_motion(s.tab, true).await?;
    frames.push(
        s.sample(
            face,
            "rtl-reduced-focus",
            turn + 20,
            Stroke::opaque(3., primary),
            0,
        )
        .await?
        .0,
    );
    leave(s.tab, s.field).await?;
    frames.push(
        s.sample(
            face,
            "rtl-reduced-blur",
            turn + 20,
            Stroke::opaque(1., idle),
            0,
        )
        .await?
        .0,
    );

    enter(s.tab, s.field).await?;
    s.tab.press("Tab").await?;
    focus(s.tab, &s.field.id("read-only"), "surface-text-input").await?;
    let long = Face::read(s.tab, s.field, "long-label").await?;
    let above = bounds(s.tab, &s.field.id("long-label-above-label")).await?;
    let floating = bounds(s.tab, &s.field.id("long-label-floating-label")).await?;
    check!(
        above[3] >= 64. && above[1] + above[3] <= long.bounds[1] && floating[2] == 0.,
        "the fallback label wraps above the compact field with no floating slot"
    );
    reduced_motion(s.tab, false).await?;
    s.tab.press("Tab").await?;
    focus(s.tab, &long.id, "surface-text-input").await?;
    s.tab.settle().await?;
    tick(s.tab, turn + 20 + 44).await?;
    let width = WidthPose::at(1.).towards(s.field, 3., 44).dp;
    let color = golden::between(idle, primary, 44);
    let (long_report, long_image) = s
        .sample(
            &long,
            "rtl-above-label-44",
            turn + 64,
            Stroke::opaque(width, color),
            0,
        )
        .await?;
    let [x, y, w, _] = long.bounds;
    let top = [0.25, 0.5, 0.75]
        .into_iter()
        .map(|fraction| {
            Strip::top(y, x + w * fraction).samples(
                &long_image,
                Stroke::opaque(width, color),
                0.,
                s.colors.paper,
                s.colors.paper,
                s.blends,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let active = textbox(s.tab, LONG, true, None).await?;
    check_eq!(
        active["value"],
        "共用名稱",
        "compact refocus preserves the actual text value"
    );
    Ok(
        json!({"frames":frames,"right_notch":notch,"floating": [fx,fw],
        "fallback":{"frame":long_report,"above":above,"closed_top":top,"ax":active}}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, DPI).await?;
    let mut report = json!({"browser":chrome.version,"dpr":DPI,"clock":"held performance.now"});
    let mut blends = BlendModes([true; 2]);
    let result: Result<()> = async {
        step(
            STEPS[0],
            "Filled and Outlined focus paint timed ink and different actual width curves",
        );
        for field in [Field::Filled, Field::Outlined] {
            let model = scene(&tab, url, field, false).await?;
            let face = Face::read(&tab, field, "editable").await?;
            let mut series = Series {
                tab: &tab,
                env,
                field,
                colors: Palette::read(&model, field)?,
                rtl: false,
                blends: &mut blends,
            };
            report["opening"][field.appearance()] = opening(&mut series, &face).await?;
            report["opening"][field.appearance()]["palette"] = model["palette"].clone();
        }
        report["adapter"] = tab
            .eval(
                r#"(async () => {
            const {info} = await navigator.gpu.requestAdapter();
            const {vendor,architecture,isFallbackAdapter} = info;
            return {vendor,architecture,isFallbackAdapter};
        })()"#,
            )
            .await?;
        check!(
            report["adapter"]["vendor"].is_string()
                && report["adapter"]["architecture"].is_string()
                && report["adapter"]["isFallbackAdapter"].is_boolean(),
            "actual Chrome WebGPU adapter"
        );
        check_eq!(
            tab.eval("devicePixelRatio").await?,
            DPI,
            "actual two-times device pixels"
        );

        step(
            STEPS[1],
            "Trusted hover, validation and disabled input keep distinct paint and semantics",
        );
        for field in [Field::Filled, Field::Outlined] {
            let model = scene(&tab, url, field, false).await?;
            let face = Face::read(&tab, field, "editable").await?;
            let mut series = Series {
                tab: &tab,
                env,
                field,
                colors: Palette::read(&model, field)?,
                rtl: false,
                blends: &mut blends,
            };
            report["palette_states"][field.appearance()] =
                palette_cases(&mut series, &face).await?;
        }

        step(
            STEPS[2],
            "Dark RTL short and wrapped fields respect notch, retarget and Reduce Motion",
        );
        let field = Field::Outlined;
        let model = scene(&tab, url, field, true).await?;
        let face = Face::read(&tab, field, "editable").await?;
        let mut series = Series {
            tab: &tab,
            env,
            field,
            colors: Palette::read(&model, field)?,
            rtl: true,
            blends: &mut blends,
        };
        report["compact"] = compact(&mut series, &face).await?;
        report["compact"]["palette"] = model["palette"].clone();
        check!(tab.errors().is_empty(), "no boundary motion page errors");
        Ok(())
    }
    .await;
    report["page_errors"] = json!(tab.errors());
    report["blend_modes"] = json!({"encoded":blends.0[0],"linear_light":blends.0[1]});
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        let _ = tab
            .screenshot(&env.artifact(&format!("{PREFIX}-failure.png")))
            .await;
    }
    std::fs::write(
        env.artifact(&format!("{PREFIX}.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    result
}
