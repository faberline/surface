//! Real headless Canvas pixels for the browser's disabled system text ink.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_multiline_text_field::wheel;
use super::material_time_input::{activate, ax, ax_property, ax_tree, bounds, resize};
use super::material_web_contrast::{contrast, media, palette_with_disabled};
use super::material_web_contrast_png::{CanvasPng, Rect, Rgb};
use super::{click, focus, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 3] = ["M357", "M358", "M359"];
const PREFIX: &str = "material-text-field-disabled-contrast";
const DRAFT: &str = "Surface draft";
const DPI: f64 = 2.;

#[derive(Clone, Copy)]
enum Kind {
    Outlined,
    Filled,
    Adorned,
    Counter,
    Multiline,
}

impl Kind {
    fn parts(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Outlined => (
                "outlined",
                "outlined-text-field-demo",
                "outlined_text_field_demo",
            ),
            Self::Filled => ("filled", "filled-text-field-demo", "filled_text_field_demo"),
            Self::Adorned => (
                "adorned",
                "text-field-adornments-demo",
                "text_field_adornments_demo",
            ),
            Self::Counter => (
                "counter",
                "text-field-counter-demo",
                "text_field_counter_demo",
            ),
            Self::Multiline => (
                "multiline",
                "multiline-text-field-demo",
                "multiline_text_field_demo",
            ),
        }
    }

    fn id(self, part: &str) -> String {
        format!("{}-{part}", self.parts().1)
    }

    fn lock_labels(self) -> [(&'static str, &'static str); 2] {
        match self {
            Self::Outlined | Self::Filled => [("停用名稱", "已封存專案"), ("唯讀名稱", "既有專案")],
            Self::Adorned => [("停用方案費用", "30"), ("目前方案費用", "18")],
            Self::Counter => [("停用標籤", "停用標籤內容"), ("唯讀標籤", "🇹🇼a\u{301}")],
            Self::Multiline => [
                ("停用內容", "停用的第一行\n停用的第二行"),
                ("唯讀內容", "已儲存的第一行\n已儲存的第二行\n"),
            ],
        }
    }

    fn filled(self) -> bool {
        matches!(self, Self::Filled)
    }
}

async fn scene(tab: &Tab, url: &str, kind: Kind) -> Result<Value> {
    media(tab, false, false).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2500).await?;
    until(tab, "s.section === 0").await?;
    activate(tab, &kind.id("show")).await?;
    let model = kind.parts().2;
    until(tab, &format!("s.{model}.visible === true")).await?;
    resize(
        tab,
        960,
        if matches!(kind, Kind::Multiline) {
            1120
        } else {
            820
        },
    )
    .await?;
    if matches!(kind, Kind::Multiline) {
        let [x, y, _, h] = bounds(tab, &kind.id("scroll")).await?;
        // Scroll on the outer padding so the locked fields both enter the viewport.
        wheel(tab, [x, y, 8., h], 720.).await?;
    }
    let normal = contrast(tab, "normal").await?;
    check!(
        normal["system_colors"].is_null(),
        "normal mode uses the authored palette"
    );
    Ok(state(tab).await?[model]["palette"].clone())
}

fn opaque(value: &Value, role: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] =
        serde_json::from_value(value.clone()).with_context(|| format!("{role} color bytes"))?;
    check_eq!(a, 255, format!("{role} has full opacity"));
    Ok([r, g, b])
}

fn mixed(base: Rgb, ink: Rgb, alpha: u8, linear: bool) -> Rgb {
    let decode = |byte: u8| {
        let c = f64::from(byte) / 255.;
        if !linear {
            c
        } else if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let alpha = f64::from(alpha) / 255.;
    std::array::from_fn(|channel| {
        let c = decode(base[channel]) * (1. - alpha) + decode(ink[channel]) * alpha;
        let c = if !linear {
            c
        } else if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1. / 2.4) - 0.055
        };
        (255. * c).round() as u8
    })
}

fn near_rgb(actual: Rgb, expected: Rgb) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(a, b)| a.abs_diff(b) <= 2)
}

#[derive(Clone, Copy, PartialEq)]
enum Ink {
    DisabledFace,
    DisabledPage,
    ReadOnlyValue,
    ReadOnlyLabel,
}

struct Probe {
    part: &'static str,
    rect: Rect,
    ink: Ink,
    minimum: usize,
}

impl Probe {
    async fn by_id(
        tab: &Tab,
        kind: Kind,
        part: &'static str,
        ink: Ink,
        minimum: usize,
    ) -> Result<Self> {
        Ok(Self {
            part,
            rect: bounds(tab, &kind.id(part)).await?,
            ink,
            minimum,
        })
    }

    fn ink_count(&self, image: &CanvasPng, expected: [Rgb; 2], opaque: bool) -> Result<usize> {
        if opaque {
            return image.count_color(self.rect, expected[0]);
        }
        let [left, top, right, bottom] = grid(self.rect);
        let mut count = 0;
        for y in top..bottom {
            for x in left..right {
                let pixel = image.pixel(x, y)?;
                count += usize::from(expected.into_iter().any(|ink| near_rgb(pixel, ink)));
            }
        }
        Ok(count)
    }
}

fn grid([x, y, width, height]: Rect) -> [usize; 4] {
    [x, y, x + width, y + height].map(|value| (value * DPI).round() as usize)
}

fn crop_changes(before: &CanvasPng, after: &CanvasPng, rect: Rect) -> Result<[usize; 2]> {
    let [left, top, right, bottom] = grid(rect);
    let mut changed = 0;
    for y in top..bottom {
        for x in left..right {
            changed += usize::from(before.pixel(x, y)? != after.pixel(x, y)?);
        }
    }
    Ok([(right - left) * (bottom - top), changed])
}

struct Regions {
    face: Rect,
    read_only: Rect,
    probes: Vec<Probe>,
}

impl Regions {
    async fn read(tab: &Tab, kind: Kind) -> Result<Self> {
        let face = bounds(
            tab,
            &kind.id(if kind.filled() {
                "disabled-indicator"
            } else {
                "disabled-outline"
            }),
        )
        .await?;
        let read_only = bounds(tab, &kind.id("read-only")).await?;
        let value = bounds(tab, &kind.id("disabled-disabled-value")).await?;
        let mut probes = vec![
            Probe::by_id(
                tab,
                kind,
                "disabled-floating-label",
                if kind.filled() {
                    Ink::DisabledFace
                } else {
                    Ink::DisabledPage
                },
                6,
            )
            .await?,
        ];
        if matches!(kind, Kind::Multiline) {
            let [x, y, w, h] = value;
            for (part, top) in [
                ("disabled-first-line", y),
                ("disabled-second-line", y + h / 2.),
            ] {
                probes.push(Probe {
                    part,
                    rect: [x, top, w, h / 2.],
                    ink: Ink::DisabledFace,
                    minimum: 12,
                });
            }
            probes.push(
                Probe::by_id(tab, kind, "disabled-description", Ink::DisabledPage, 12).await?,
            );
        } else {
            probes.push(Probe {
                part: "disabled-disabled-value",
                rect: value,
                ink: Ink::DisabledFace,
                minimum: 6,
            });
        }
        for &part in match kind {
            Kind::Adorned => &[
                "disabled-leading-icon",
                "disabled-trailing-icon",
                "disabled-prefix-visual",
                "disabled-suffix-visual",
            ][..],
            Kind::Counter => &["disabled-counter"][..],
            _ => &[][..],
        } {
            probes.push(
                Probe::by_id(
                    tab,
                    kind,
                    part,
                    if part.ends_with("-counter") {
                        Ink::DisabledPage
                    } else {
                        Ink::DisabledFace
                    },
                    3,
                )
                .await?,
            );
        }
        probes.push(
            Probe::by_id(tab, kind, "read-only-floating-label", Ink::ReadOnlyLabel, 6).await?,
        );
        let ro_line = if matches!(kind, Kind::Adorned | Kind::Multiline) {
            bounds(tab, &kind.id("read-only-value-box")).await?
        } else {
            let [x, y, w, h] = read_only;
            [x + 12., y + h * 0.43, w - 24., h * 0.57 - 5.]
        };
        probes.push(Probe {
            part: "read-only-value",
            rect: ro_line,
            ink: Ink::ReadOnlyValue,
            minimum: 3,
        });
        let [_, _, w, h] = tab.canvas_box().await?;
        let viewport = [0., 0., w, h];
        let clip = if matches!(kind, Kind::Multiline) {
            bounds(tab, &kind.id("scroll")).await?
        } else {
            viewport
        };
        check!(
            [face, read_only]
                .into_iter()
                .chain(probes.iter().map(|p| p.rect))
                .all(|[x, y, rw, rh]| {
                    rw >= 2.
                        && rh >= 2.
                        && [viewport, clip].into_iter().all(|[left, top, cw, ch]| {
                            x >= left && y >= top && x + rw <= left + cw && y + rh <= top + ch
                        })
                }),
            "all disabled and read-only glyph sampling rectangles lie in the visible Canvas"
        );
        Ok(Self {
            face,
            read_only,
            probes,
        })
    }

    fn geometry(&self) -> Value {
        json!({
            "disabled_face":self.face,"read_only_face":self.read_only,
            "ink_rects":self.probes.iter().map(|p| (p.part,p.rect)).collect::<std::collections::BTreeMap<_,_>>(),
        })
    }
}

async fn lock_state(tab: &Tab, kind: Kind) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let [(disabled_name, disabled_value), (readonly_name, readonly_value)] = kind.lock_labels();
    let disabled = ax(&tree, "textbox", disabled_name)?;
    let readonly = ax(&tree, "textbox", readonly_name)?;
    check!(
        disabled["value"]["value"] == disabled_value && readonly["value"]["value"] == readonly_value
            && ax_property(disabled, "disabled") == Some(&json!(true))
            && ax_property(readonly, "readonly") == Some(&json!(true)),
        "Chrome keeps the authored values and distinguishes the disabled and read-only textbox states"
    );
    let actions = if matches!(kind, Kind::Adorned) {
        let disabled_action = ax(&tree, "button", "套用停用方案金額")?;
        let read_action = ax(&tree, "button", "查看現有費率")?;
        check_eq!(
            ax_property(disabled_action, "disabled"),
            Some(&json!(true)),
            "the disabled trailing peer stays disabled"
        );
        check!(
            ax_property(read_action, "focusable") == Some(&json!(true)),
            "the read-only field keeps its focusable trailing action"
        );
        json!({"disabled":disabled_action["name"]["value"],"read_only":read_action["name"]["value"]})
    } else {
        Value::Null
    };
    let multiline = matches!(kind, Kind::Multiline);
    for (item, name) in [(disabled, disabled_name), (readonly, readonly_name)] {
        let amount = tree
            .iter()
            .filter(|node| {
                node["ignored"] == false
                    && node["role"]["value"] == "textbox"
                    && node["name"]["value"] == name
            })
            .count();
        check_eq!(amount, 1, format!("one named Chrome textbox for {name}"));
        check_eq!(
            ax_property(item, "multiline"),
            Some(&json!(multiline)),
            "textbox line mode stays stable"
        );
    }
    Ok(json!({
        "disabled":{"name":disabled_name,"value":disabled_value,"state":"disabled"},
        "read_only":{"name":readonly_name,"value":readonly_value,"state":"read-only"},
        "multiline":multiline,"actions":actions,
    }))
}

async fn draft(tab: &Tab, kind: Kind) -> Result<Value> {
    let id = kind.id("editable");
    focus(tab, &id, "surface-text-input").await?;
    let sample = tab
        .eval(&format!(
            r#"(() => {{
        const node = document.getElementById("surface-text-input");
        const input = JSON.parse(window.gallery.export_dom()).text_inputs[{}];
        const model = JSON.parse(window.gallery.inspect());
        return {{owner:model.focused,dom:node.value,active:document.activeElement===node,
            web_selection:[node.selectionStart,node.selectionEnd,node.selectionDirection],
            runtime_selection:[input.selection_anchor,input.selection_cursor],value:input.value,
            composing:input.composing, caller:model.{}.value}};
    }})()"#,
            js(&id),
            kind.parts().2
        ))
        .await?;
    check!(
        sample["owner"] == id
            && sample["active"] == true
            && sample["dom"] == DRAFT
            && sample["value"] == DRAFT
            && sample["caller"] == DRAFT
            && sample["web_selection"] == json!([DRAFT.len() - 2, DRAFT.len(), "backward"])
            && sample["runtime_selection"] == json!([DRAFT.len(), DRAFT.len() - 2])
            && sample["composing"] == false,
        "the real filled editor retains its letter range in both Chrome and Runtime"
    );
    Ok(sample)
}

struct Paint {
    paper: Rgb,
    ordinary: Rgb,
    quiet_label: Rgb,
    gray: Option<Rgb>,
}

impl Paint {
    fn from_normal(palette: &Value) -> Result<Self> {
        Ok(Self {
            paper: opaque(&palette["background"], "authored background")?,
            ordinary: opaque(&palette["on_surface"], "authored on-surface text")?,
            quiet_label: opaque(&palette["on_surface_variant"], "authored caption ink")?,
            gray: None,
        })
    }

    fn from_forced(witness: &Value) -> Result<Self> {
        let paper = opaque(&witness["pairs"]["canvas"]["background"], "CSS Canvas")?;
        let ordinary = opaque(&witness["pairs"]["canvas"]["foreground"], "CSS CanvasText")?;
        let gray = opaque(&witness["disabled_text"]["rgba"], "CSS GrayText")?;
        check!(
            gray != ordinary && gray != paper,
            format!(
                "Chrome gives a visible GrayText distinct from CanvasText and Canvas: {gray:?}"
            )
        );
        Ok(Self {
            paper,
            ordinary,
            quiet_label: ordinary,
            gray: Some(gray),
        })
    }

    fn sample(&self, image: &CanvasPng, kind: Kind, regions: &Regions) -> Result<Value> {
        let [x, y, w, h] = regions.face;
        let [base_point, border_point] = [[x + w * 0.70, y + h - 6.], [x + w * 0.70, y + h - 0.25]];
        let base = image.sample(base_point[0], base_point[1])?;
        let border = image.sample(border_point[0], border_point[1])?;
        let face_opacity = if kind.filled() { 10 } else { 0 };
        let stroke_opacity = if kind.filled() { 97 } else { 31 };
        check!(
            [false, true].into_iter().any(|linear| {
                near_rgb(base, mixed(self.paper, self.ordinary, face_opacity, linear))
                    && near_rgb(border, mixed(base, self.ordinary, stroke_opacity, linear))
            }),
            format!("the disabled face and boundary keep their own tokens: {base:?} / {border:?}")
        );
        let mut glyphs = Vec::new();
        for probe in &regions.probes {
            let (expected, solid) = match (probe.ink, self.gray) {
                (Ink::DisabledFace | Ink::DisabledPage, Some(gray)) => ([gray; 2], true),
                (Ink::ReadOnlyValue, _) => ([self.ordinary; 2], true),
                (Ink::ReadOnlyLabel, _) => ([self.quiet_label; 2], true),
                (location, None) => {
                    let under = if location == Ink::DisabledFace {
                        base
                    } else {
                        self.paper
                    };
                    (
                        [false, true].map(|linear| mixed(under, self.ordinary, 97, linear)),
                        false,
                    )
                }
            };
            let actual = probe.ink_count(image, expected, solid)?;
            check!(
                actual >= probe.minimum,
                format!(
                    "{} has real ink in its own crop: {actual} pixels near {expected:?}",
                    probe.part
                )
            );
            glyphs.push(
                json!({"part":probe.part,"bounds":probe.rect,"expected_rgb":expected,
                "exact_opaque":solid,"minimum":probe.minimum,"matching_pixels":actual}),
            );
        }
        Ok(
            json!({"glyphs":glyphs,"paper":self.paper,"ordinary":self.ordinary,"gray":self.gray,
            "face":{"point":base_point,"rgb":base,"on_surface_alpha":face_opacity},
            "boundary":{"point":border_point,"rgb":border,"on_surface_alpha":stroke_opacity}}),
        )
    }
}

async fn route(env: &Env, tab: &Tab, url: &str, kind: Kind) -> Result<Value> {
    let palette = scene(tab, url, kind).await?;
    let keep_draft = kind.filled();
    if keep_draft {
        let id = kind.id("editable");
        click(tab, &id).await?;
        focus(tab, &id, "surface-text-input").await?;
        tab.insert_text(DRAFT).await?;
        until(
            tab,
            &format!("s.{}.value === {}", kind.parts().2, js(DRAFT)),
        )
        .await?;
        tab.wait_for(&format!(
            "document.getElementById('surface-text-input')?.value === {}",
            js(DRAFT)
        ))
        .await?;
        for _ in 0..2 {
            tab.press("Shift+ArrowLeft").await?;
        }
        tab.wait_for(&format!(
            r#"(() => {{
            const input = JSON.parse(window.gallery.export_dom()).text_inputs[{}];
            return input?.selection_anchor === {} && input?.selection_cursor === {};
        }})()"#,
            js(&id),
            DRAFT.len(),
            DRAFT.len() - 2
        ))
        .await?;
    }
    let (tag, _, model) = kind.parts();
    let regions = Regions::read(tab, kind).await?;
    let geometry = regions.geometry();
    let mut frames = Vec::new();
    let mut normal = None;
    let mut initial_draft = None;
    for (phase, forced) in [("normal", false), ("forced", true), ("restored", false)] {
        media(tab, forced, false).await?;
        if forced {
            until(
                tab,
                "Array.isArray(s.display_contrast?.system_colors?.disabled_text)",
            )
            .await?;
        }
        let preference = contrast(tab, if forced { "forced" } else { "normal" }).await?;
        let witness = if forced {
            palette_with_disabled(tab, &preference).await?
        } else {
            check!(
                preference["system_colors"].is_null(),
                "leaving forced colors clears the system palette"
            );
            Value::Null
        };
        let paint = if forced {
            Paint::from_forced(&witness)?
        } else {
            Paint::from_normal(&palette)?
        };
        if !forced {
            check_eq!(
                state(tab).await?[model]["palette"],
                palette,
                "the authored field palette returns"
            );
        }
        check_eq!(
            Regions::read(tab, kind).await?.geometry(),
            geometry,
            "system ink keeps paint and hit rectangles"
        );
        let locks = lock_state(tab, kind).await?;
        let editor = if keep_draft {
            let editor = draft(tab, kind).await?;
            if let Some(initial) = &initial_draft {
                check_eq!(
                    &editor,
                    initial,
                    "mode switching preserves the focused draft and directed selection"
                );
            } else {
                initial_draft = Some(editor.clone());
            }
            editor
        } else {
            Value::Null
        };
        let name = format!("{PREFIX}-{tag}-{phase}.png");
        let image = CanvasPng::capture_file(env, tab, &name, DPI).await?;
        let sampled = paint.sample(&image, kind, &regions)?;
        let mut restored_crops = Vec::new();
        if phase == "restored" {
            let before = normal
                .as_ref()
                .context("actual screenshot of original Normal mode")?;
            for probe in &regions.probes {
                let [pixels, changed] = crop_changes(before, &image, probe.rect)?;
                check!(
                    pixels > 20 && changed == 0,
                    format!("{} restores its exact original pixels", probe.part)
                );
                restored_crops.push(json!({"part":probe.part,"pixels":pixels,"changed":changed}));
            }
        }
        frames.push(json!({"phase":phase,"png":name,"size":image.dimensions(),"paint":sampled,
            "system_colors":preference["system_colors"],"css":witness,"ax":locks,"focused_draft":editor,
            "restored_crops":restored_crops}));
        if phase == "normal" {
            normal = Some(image);
        }
    }
    Ok(json!({"geometry":geometry,"normal_palette":palette,"frames":frames}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, DPI).await?;
    let mut report = json!({"browser":chrome.version,"dpr":DPI,"preference_source":"CDP browser media emulation"});
    let result: Result<()> = async {
        step(STEPS[0], "Outlined and Filled fields paint actual CSS GrayText while an enabled draft stays selected");
        for kind in [Kind::Outlined, Kind::Filled] {
            report[kind.parts().0] = route(env, &tab, url, kind).await?;
        }
        report["adapter"] = tab.eval(r#"(async () => {
            const {info} = await navigator.gpu.requestAdapter();
            const {vendor,architecture,isFallbackAdapter} = info;
            return {vendor,architecture,isFallbackAdapter};
        })()"#).await?;
        check!(report["adapter"]["vendor"].is_string() && report["adapter"]["architecture"].is_string()
            && report["adapter"]["isFallbackAdapter"].is_boolean(), "actual browser WebGPU adapter");

        step(STEPS[1], "Adorned and counted disabled peers use GrayText for ink in the real Canvas");
        for kind in [Kind::Adorned, Kind::Counter] {
            report[kind.parts().0] = route(env, &tab, url, kind).await?;
        }
        step(STEPS[2], "Both lines and supporting text of the disabled multiline field use the same visible system ink");
        report["multiline"] = route(env, &tab, url, Kind::Multiline).await?;
        check!(tab.errors().is_empty(), "the field contrast route has no page errors");
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
