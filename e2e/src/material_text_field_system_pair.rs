//! The browser Field pair and a separate unequal-pair Canvas proof.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click, focus,
    material_time_input::{activate, ax, ax_property, ax_tree, bounds, contains, near, resize},
    material_web_contrast::{
        contrast, hidden_probe, media, media_scheme, palette_with_disabled, Blend, CSS_WITNESS,
    },
    material_web_contrast_png::{CanvasPng, Rect, Rgb},
    state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 3] = ["M360", "M361", "M362"];
const PREFIX: &str = "browser-material-text-field-system-pair";
const DPR: f64 = 2.;
const NAME: &str = "專案名稱";
const DRAFT: &str = "1688";
const FIELD: Rgb = [19, 45, 47];
const FIELD_TEXT: Rgb = [249, 220, 126];
const TEXTAREA: &str = "document.getElementById('surface-text-input')";

// Affect only the optional Surface probe. The separate CSS witness and all
// browser widgets still read the original system colors.
const INJECT: &str = r#"(() => {
    const read = window.getComputedStyle.bind(window);
    const setting = { background: "rgb(19, 45, 47)", foreground: "rgb(249, 220, 126)" };
    window.__surfaceFieldPairFixture = setting;
    window.getComputedStyle = (element, pseudo) => {
        const real = read(element, pseudo);
        if (pseudo || element?.getAttribute("data-surface-optional-system-pair") !== "field")
            return real;
        return new Proxy(real, {
            get(target, property) {
                if (property === "getPropertyValue") return key => {
                    if (key === "background-color") return setting.background;
                    if (key === "color") return setting.foreground;
                    return target.getPropertyValue(key);
                };
                const result = Reflect.get(target, property, target);
                return typeof result === "function" ? result.bind(target) : result;
            }
        });
    };
})()"#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Outlined,
    Filled,
}

impl Kind {
    fn tag(self) -> &'static str {
        match self {
            Self::Outlined => "outlined",
            Self::Filled => "filled",
        }
    }

    fn id(self, part: &str) -> String {
        format!("{}-text-field-demo-{part}", self.tag())
    }

    fn model(self) -> String {
        format!("{}_text_field_demo", self.tag())
    }
}

async fn scene(tab: &Tab, url: &str, kind: Kind) -> Result<()> {
    media(tab, false, false).await?;
    tab.viewport(960, 2444).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    // Both field launchers must fit before the actual form is opened.
    resize(tab, 960, 2400).await?;
    activate(tab, "section-0").await?;
    until(tab, "s.section === 0").await?;
    activate(tab, &kind.id("show")).await?;
    until(tab, &format!("s.{}.visible === true", kind.model())).await?;
    resize(tab, 960, 900).await?;
    contrast(tab, "normal").await?;
    Ok(())
}

fn rgb(value: &Value) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] =
        serde_json::from_value(value.clone()).with_context(|| format!("opaque RGB: {value}"))?;
    check_eq!(alpha, 255, "the sampled system ink or backdrop is opaque");
    Ok([r, g, b])
}

fn rgba(color: Rgb) -> [u8; 4] {
    let [r, g, b] = color;
    [r, g, b, 255]
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<(String, CanvasPng)> {
    let name = format!("{PREFIX}-{name}.png");
    let image = CanvasPng::capture_file(env, tab, &name, DPR).await?;
    Ok((name, image))
}

fn right([x, _, width, _]: Rect) -> f64 {
    x + width
}

fn bottom([_, y, _, height]: Rect) -> f64 {
    y + height
}

fn body(face: Rect, kind: Kind, scale: f64) -> Rect {
    let [x, y, width, height] = face;
    let line = 24. * scale;
    let top = if kind == Kind::Filled {
        8. + 16. * scale
    } else {
        (height - line) / 2.
    };
    [x + 16., y + top, width - 32., line]
}

fn quiet(face: Rect, rtl: bool) -> [f64; 2] {
    let factor = if rtl { 0.24 } else { 0.76 };
    [face[0] + factor * face[2], bottom(face) - 7.]
}

fn pixel(image: &CanvasPng, [x, y]: [f64; 2], expected: Rgb, part: &str) -> Result<Value> {
    let actual = image.sample(x, y)?;
    check_eq!(
        actual,
        expected,
        format!("the actual Canvas {part} pixel at ({x},{y})")
    );
    Ok(json!({"point":[x,y],"rgb":actual}))
}

fn focus_stripe(image: &CanvasPng, face: Rect, rtl: bool, ink: Rgb, paper: Rgb) -> Result<Value> {
    let [x, _] = quiet(face, rtl);
    let mut samples = Vec::new();
    for (depth, expected) in [(0.75, ink), (1.75, ink), (2.75, ink), (3.75, paper)] {
        samples.push(pixel(
            image,
            [x, bottom(face) - depth],
            expected,
            "focus edge depth",
        )?);
    }
    Ok(json!(samples))
}

fn glyphs(
    image: &CanvasPng,
    rect: Rect,
    expected: Rgb,
    minimum: usize,
    part: &str,
) -> Result<Value> {
    let count = image.count_color(rect, expected)?;
    check!(
        count >= minimum,
        "the {part} uses exact {expected:?} glyph ink within {rect:?}: {count} pixels"
    );
    Ok(json!({"rect":rect,"ink":expected,"count":count,"minimum":minimum}))
}

fn notch(image: &CanvasPng, face: Rect, label: Rect, canvas: Rgb, rtl: bool) -> Result<Value> {
    let leading = if rtl {
        right(face) - right(label)
    } else {
        label[0] - face[0]
    };
    check!(
        near(leading, 16.) && label[1] < face[1] && face[1] < bottom(label),
        "the actual floating caption straddles the border at the logical leading edge"
    );
    let mut samples = Vec::new();
    for x in [label[0] - 2., right(label) + 2.] {
        samples.push(pixel(
            image,
            [x, face[1] - 1.5],
            FIELD,
            "Field backer above the notch",
        )?);
        samples.push(pixel(
            image,
            [x, face[1] + 0.5],
            FIELD,
            "Field inside the notch gap",
        )?);
    }
    for x in [label[0] - 6., right(label) + 6.] {
        samples.push(pixel(
            image,
            [x, face[1] - 1.5],
            canvas,
            "Canvas beyond the notch backer",
        )?);
        samples.push(pixel(
            image,
            [x, face[1] + 0.5],
            FIELD_TEXT,
            "uncovered FieldText outline beside the notch",
        )?);
    }
    Ok(json!({"face":face,"label":label,"rtl":rtl,"samples":samples}))
}

async fn editor_geometry(tab: &Tab, kind: Kind, scale: f64) -> Result<Value> {
    let id = kind.id("editable");
    let face = bounds(tab, &id).await?;
    let boundary = if kind == Kind::Filled {
        "indicator"
    } else {
        "outline"
    };
    let edge = bounds(tab, &format!("{id}-{boundary}")).await?;
    let label = bounds(tab, &format!("{id}-floating-label")).await?;
    let support = bounds(tab, &format!("{id}-description")).await?;
    let body = body(face, kind, scale);
    let [_, _, width, height] = tab.canvas_box().await?;
    let viewport = [0., 0., width, height];
    let clip = bounds(tab, &kind.id("scroll")).await?;
    check!(
        face.into_iter().zip(edge).all(|(a, b)| near(a, b))
            && [face, label, support, body]
                .into_iter()
                .all(|r| contains(viewport, r) && contains(clip, r)),
        "the face, caption, body and supporting text share actual fully visible field geometry"
    );
    Ok(json!({"face":face,"label":label,"support":support,"body":body,"clip":clip}))
}

async fn select_suffix(tab: &Tab) -> Result<()> {
    for _ in 0..2 {
        tab.press("Shift+ArrowLeft").await?;
    }
    Ok(())
}

async fn seed(tab: &Tab, kind: Kind) -> Result<()> {
    let id = kind.id("editable");
    click(tab, &id).await?;
    focus(tab, &id, "surface-text-input").await?;
    tab.insert_text(DRAFT).await?;
    until(tab, &format!("s.{}.value === {}", kind.model(), js(DRAFT))).await?;
    editing(tab, kind, [4, 4]).await?;
    Ok(())
}

async fn editing(tab: &Tab, kind: Kind, [anchor, cursor]: [usize; 2]) -> Result<Value> {
    let id = kind.id("editable");
    focus(tab, &id, "surface-text-input").await?;
    let [start, end] = [anchor.min(cursor), anchor.max(cursor)];
    let direction = if anchor > cursor { "backward" } else { "none" };
    let read = format!(
        r#"(() => {{
            const textarea = {TEXTAREA};
            const editor = JSON.parse(window.gallery.export_dom()).text_inputs[{}];
            const caller = JSON.parse(window.gallery.inspect()).{}.value;
            return {{active:document.activeElement === textarea, dom:textarea?.value,
                browser:[textarea?.selectionStart,textarea?.selectionEnd,
                    textarea?.selectionStart === textarea?.selectionEnd ? "none" : textarea?.selectionDirection],
                runtime:[editor?.selection_anchor,editor?.selection_cursor],
                value:editor?.value,composing:editor?.composing,caller}};
        }})()"#,
        js(&id),
        kind.model()
    );
    tab.wait_for(&format!(
        "(() => {{const e={read};return e.active && e.runtime[0]==={anchor} && e.runtime[1]==={cursor}
            && e.browser[0]==={start} && e.browser[1]==={end};}})()"
    )).await?;
    tab.settle().await?;
    let result = tab.eval(&read).await?;
    check_eq!(
        result,
        json!({"active":true,"dom":DRAFT,"browser":[start,end,direction],"runtime":[anchor,cursor],
            "value":DRAFT,"composing":false,"caller":DRAFT}),
        "the same real text editor retains its Runtime value and exact browser range"
    );
    Ok(result)
}

async fn focused_ax(tab: &Tab) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let editor = ax(&tree, "textbox", NAME)?;
    let duplicates = tree
        .iter()
        .filter(|node| node["ignored"] == false && node["name"]["value"] == NAME)
        .count();
    check!(
        duplicates == 1
            && editor["value"]["value"] == DRAFT
            && ax_property(editor, "focused").and_then(Value::as_bool) == Some(true),
        "Chrome exposes one focused named textbox with the actual value"
    );
    Ok(
        json!({"role":"textbox","name":NAME,"value":DRAFT,"focused":true,"same_name_nodes":duplicates}),
    )
}

async fn caret_rect(tab: &Tab) -> Result<Rect> {
    let raw = tab.eval_f64s(&format!(
        "(() => {{const a={TEXTAREA}.getBoundingClientRect(),c=document.getElementById('surface').getBoundingClientRect();
            return [a.x-c.x,a.y-c.y,a.width,a.height];}})()"
    )).await?;
    raw.try_into()
        .map_err(|_| anyhow::anyhow!("expected a four-component real caret proxy"))
}

fn changes(before: &CanvasPng, after: &CanvasPng, [x, y, w, h]: Rect) -> Result<usize> {
    let px = |value: f64| (DPR * value).round() as usize;
    let mut changed = 0;
    for y in px(y)..px(y + h) {
        for x in px(x)..px(x + w) {
            changed += usize::from(before.pixel(x, y)? != after.pixel(x, y)?);
        }
    }
    Ok(changed)
}

fn field_pair() -> Value {
    json!({"background":rgba(FIELD),"foreground":rgba(FIELD_TEXT)})
}

async fn real_system(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let kind = Kind::Outlined;
    scene(tab, url, kind).await?;
    seed(tab, kind).await?;
    let geometry = editor_geometry(tab, kind, 1.).await?;
    let face: Rect = serde_json::from_value(geometry["face"].clone())?;
    let line: Rect = serde_json::from_value(geometry["body"].clone())?;
    let mut schemes = Vec::new();
    for (scheme, paper, ink) in [
        ("light", [255, 255, 255], [0, 0, 0]),
        ("dark", [0, 0, 0], [255, 255, 255]),
    ] {
        media_scheme(tab, true, false, Some(scheme)).await?;
        until(
            tab,
            &format!(
                "s.display_contrast?.mode === 'forced' &&
             JSON.stringify(s.display_contrast.system_colors?.field?.background) === '{}'",
                json!(rgba(paper))
            ),
        )
        .await?;
        let preference = contrast(tab, "forced").await?;
        let css = palette_with_disabled(tab, &preference).await?;
        hidden_probe(tab, true).await?;
        let expected = json!({"background":rgba(paper),"foreground":rgba(ink)});
        check_eq!(
            css["field"],
            expected,
            format!("real browser {scheme} resolves Field and FieldText")
        );
        check_eq!(
            css["pairs"]["canvas"],
            expected,
            format!("standard {scheme} emulation deliberately has an equal field pair")
        );
        check!(
            tab.eval("window.__surfaceFieldPairFixture === undefined")
                .await?
                == true,
            "the real CSS comparison has no synthetic input"
        );
        let focus = editing(tab, kind, [4, 4]).await?;
        check_eq!(
            editor_geometry(tab, kind, 1.).await?,
            geometry,
            "real forced palettes preserve field layout"
        );
        let (png, image) = capture(env, tab, &format!("real-{scheme}")).await?;
        let sampled = pixel(&image, quiet(face, false), paper, "equal-pair field face")?;
        let glyph = glyphs(&image, line, ink, 10, "real equal-pair editor value")?;
        schemes.push(json!({"scheme":scheme,"css":css,"accepted":preference["system_colors"],
            "png":png,"size":image.dimensions(),"geometry":geometry,"quiet_face":sampled,"value_ink":glyph,"editing":focus}));
    }
    media(tab, false, false).await?;
    let normal = contrast(tab, "normal").await?;
    check!(
        normal["system_colors"].is_null(),
        "the real system Field pair is cleared in Normal mode"
    );
    editing(tab, kind, [4, 4]).await?;
    Ok(json!(schemes))
}

async fn synthetic_palette(tab: &Tab, expected_field: &Value) -> Result<(Value, Value)> {
    let preference = contrast(tab, "forced").await?;
    let colors = preference["system_colors"].clone();
    let witness = tab.eval(CSS_WITNESS).await?;
    for role in ["canvas", "button", "highlight"] {
        check_eq!(
            &colors[role],
            &witness["pairs"][role],
            format!("injection leaves real CSS {role} untouched")
        );
    }
    check_eq!(
        &colors["disabled_text"],
        &witness["disabled_text"]["rgba"],
        "injection leaves true GrayText untouched"
    );
    check_eq!(
        &colors["field"],
        expected_field,
        "only the optional Field pair changes"
    );
    check_eq!(
        &witness["field"],
        &witness["pairs"]["canvas"],
        "the independent CSS witness still sees real system Field"
    );
    check!(
        colors["field"] != witness["field"]
            && tab.eval("!!window.__surfaceFieldPairFixture").await? == true,
        "the report distinguishes injected output from genuine browser CSS"
    );
    Ok((colors, witness))
}

async fn distinct(tab: &Tab, scheme: &str) -> Result<(Value, Value)> {
    media_scheme(tab, true, false, Some(scheme)).await?;
    until(
        tab,
        "s.display_contrast?.mode === 'forced' &&
        s.display_contrast.system_colors?.field?.background[0] === 19 &&
        s.display_contrast.system_colors?.field?.foreground[0] === 249",
    )
    .await?;
    synthetic_palette(tab, &field_pair()).await
}

async fn backing_layout(tab: &Tab, kind: Kind, id: &str, face: Rect) -> Result<()> {
    let container = bounds(tab, &format!("{id}-container")).await?;
    check_eq!(container, face, "the Field face shares the editor target");
    if kind == Kind::Outlined {
        let backing = bounds(tab, &format!("{id}-label-backing")).await?;
        check_eq!(
            backing,
            face,
            "the backing's passive paint uses the measured face anchor"
        );
    }
    Ok(())
}

fn selected_paint(
    image: &CanvasPng,
    line: Rect,
    colors: &Value,
    [start, end]: [f64; 2],
) -> Result<Value> {
    let highlight_rgba = serde_json::from_value(colors["highlight"]["background"].clone())
        .context("the host's RGBA Highlight background")?;
    let highlight_text = rgb(&colors["highlight"]["foreground"])?;
    check!(
        line[0] + 8. < start && start + 8. < end && end < right(line),
        "the actual host cursor positions separate the selected suffix and ordinary prefix"
    );
    let ordinary = [line[0], line[1], start - line[0] - 0.5, line[3]];
    let selected = [start + 0.5, line[1], end - start - 1., line[3]];
    let mut composite = None;
    for blend in [Blend::ByteSpace, Blend::LinearLight] {
        let highlight = blend.over(FIELD, highlight_rgba);
        if image.count_color(selected, highlight)? >= 35 {
            composite = Some((blend, highlight));
            break;
        }
    }
    let (blend, highlight) = composite
        .context("selection does not paint one application of browser Highlight over Field")?;
    check!(
        highlight != highlight_text
            && [highlight, highlight_text]
                .into_iter()
                .all(|c| c != FIELD && c != FIELD_TEXT),
        "the chosen injected Field pair is distinguishable from real selection"
    );
    check!(
        image.count_color(ordinary, highlight_text)? == 0
            && image.count_color(selected, FIELD_TEXT)? == 0,
        "the selected and unselected glyph runs use separate foregrounds"
    );
    Ok(json!({
        "cursor_edges":[start,end],
        "ordinary":glyphs(image,ordinary,FIELD_TEXT,10,"unselected text")?,
        "selected":glyphs(image,selected,highlight_text,10,"selected text")?,
        "highlight":glyphs(image,selected,highlight,35,"selection background")?,
        "highlight_composite":{
            "mode":blend.name(),"source_rgba":highlight_rgba,"base_rgb":FIELD,"result_rgb":highlight
        }
    }))
}

async fn wide(env: &Env, tab: &Tab, url: &str, kind: Kind) -> Result<Value> {
    scene(tab, url, kind).await?;
    seed(tab, kind).await?;
    select_suffix(tab).await?;
    let semantic = editing(tab, kind, [4, 2]).await?;
    let geometry = editor_geometry(tab, kind, 1.).await?;
    let face: Rect = serde_json::from_value(geometry["face"].clone())?;
    let label: Rect = serde_json::from_value(geometry["label"].clone())?;
    let line: Rect = serde_json::from_value(geometry["body"].clone())?;
    let support: Rect = serde_json::from_value(geometry["support"].clone())?;
    let tag = kind.tag();
    let (normal_png, normal) = capture(env, tab, &format!("{tag}-normal")).await?;

    let ax = focused_ax(tab).await?;
    let (colors, witness) = distinct(tab, "light").await?;
    hidden_probe(tab, true).await?;
    let paper = rgb(&colors["canvas"]["background"])?;
    let page_ink = rgb(&colors["canvas"]["foreground"])?;
    check_eq!(
        editing(tab, kind, [4, 2]).await?,
        semantic,
        "the selected suffix survives the distinct forced palette"
    );
    check_eq!(
        focused_ax(tab).await?,
        ax,
        "the forced palette keeps one focused named textbox"
    );
    check_eq!(
        editor_geometry(tab, kind, 1.).await?,
        geometry,
        "the distinct Field role changes only paint"
    );
    let id = kind.id("editable");
    backing_layout(tab, kind, &id, face).await?;
    let read_only = bounds(tab, &kind.id("read-only")).await?;
    let selected_cursor = caret_rect(tab).await?;
    let (selected_png, selected) = capture(env, tab, &format!("{tag}-selected")).await?;
    let notch = if kind == Kind::Outlined {
        notch(&selected, face, label, paper, false)?
    } else {
        Value::Null
    };
    tab.press("ArrowRight").await?;
    editing(tab, kind, [4, 4]).await?;
    let caret = caret_rect(tab).await?;
    check!(
        caret[2] > 0. && caret[3] > 4. && contains(face, caret),
        "the host caret proxy stays inside the focused face"
    );
    let (caret_png, collapsed) = capture(env, tab, &format!("{tag}-caret")).await?;
    // The textarea marks the left edge. The Runtime caret is one logical
    // pixel wide, so sample its center rather than the antialiased edge.
    let caret_sample = pixel(
        &collapsed,
        [caret[0] + 0.5, caret[1] + caret[3] / 2.],
        FIELD_TEXT,
        "FieldText insertion caret",
    )?;
    let caret_gap = pixel(
        &collapsed,
        [caret[0] + 2.25, caret[1] + caret[3] / 2.],
        FIELD,
        "quiet face just beyond the collapsed caret",
    )?;
    let active = json!({
        "quiet_face":pixel(&selected,quiet(face,false),FIELD,"Field face")?,
        "readonly_face":pixel(&selected,quiet(read_only,false),FIELD,"read-only Field face")?,
        "readonly_value":glyphs(&selected,body(read_only,kind,1.),FIELD_TEXT,10,"read-only value")?,
        "selection":selected_paint(&selected,line,&colors,[selected_cursor[0],caret[0]])?,
        "floating_label":glyphs(&selected,label,FIELD_TEXT,6,"in-face caption")?,
        "page_support":glyphs(&selected,support,page_ink,6,"page supporting text")?,
        "focus_edge":focus_stripe(&selected,face,false,FIELD_TEXT,FIELD)?,
        "notch":notch,
    });
    select_suffix(tab).await?;
    check_eq!(
        editing(tab, kind, [4, 2]).await?,
        semantic,
        "trusted keys restore the original partial selection"
    );

    media(tab, false, false).await?;
    check!(
        contrast(tab, "normal").await?["system_colors"].is_null(),
        "Normal drops the injected pair"
    );
    check_eq!(
        editing(tab, kind, [4, 2]).await?,
        semantic,
        "the original focus and selected draft stay after release"
    );
    check_eq!(
        focused_ax(tab).await?,
        ax,
        "the original named textbox returns with its unchanged value"
    );
    check_eq!(
        editor_geometry(tab, kind, 1.).await?,
        geometry,
        "Normal returns with its original geometry"
    );
    let (restored_png, restored) = capture(env, tab, &format!("{tag}-restored")).await?;
    let changed = [face, label, support]
        .into_iter()
        .map(|r| changes(&normal, &restored, r))
        .collect::<Result<Vec<_>>>()?;
    check!(
        changed.iter().all(|&n| n == 0),
        "Normal restores every sampled face, caption and support pixel: {changed:?}"
    );
    Ok(
        json!({"geometry":geometry,"css_witness":witness,"accepted":colors,"editing":semantic,"ax":ax,
        "images":[normal_png,selected_png,caret_png,restored_png],"size":selected.dimensions(),
        "selected":active,"caret":{"bounds":caret,"pixel":caret_sample,"trailing_face":caret_gap},"restored_changed_pixels":changed}),
    )
}

async fn missing_pair(tab: &Tab, key: &str, rejected: &str) -> Result<(Value, Value)> {
    tab.eval(&format!(
        "window.__surfaceFieldPairFixture[{}] = {}",
        js(key),
        js(rejected)
    ))
    .await?;
    until(
        tab,
        "s.display_contrast?.mode === 'forced' &&
        s.display_contrast.system_colors.field === null",
    )
    .await?;
    synthetic_palette(tab, &Value::Null).await
}

async fn compact(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let kind = Kind::Outlined;
    scene(tab, url, kind).await?;
    for part in ["theme", "rtl", "large-text"] {
        activate(tab, &kind.id(part)).await?;
    }
    resize(tab, 320, 960).await?;
    until(
        tab,
        "s.outlined_text_field_demo.width === 320 && s.outlined_text_field_demo.dark &&
        s.outlined_text_field_demo.rtl && s.outlined_text_field_demo.text_scale === 2",
    )
    .await?;
    seed(tab, kind).await?;
    let semantic = editing(tab, kind, [4, 4]).await?;
    let geometry = editor_geometry(tab, kind, 2.).await?;
    let face: Rect = serde_json::from_value(geometry["face"].clone())?;
    let label: Rect = serde_json::from_value(geometry["label"].clone())?;
    let id = kind.id("long-label");
    let long = bounds(tab, &id).await?;
    let long_label = bounds(tab, &format!("{id}-above-label")).await?;
    let unused_float = bounds(tab, &format!("{id}-floating-label")).await?;
    let clip: Rect = serde_json::from_value(geometry["clip"].clone())?;
    let long_parts = json!({"face":long,"above_label":long_label,"floating":unused_float});
    check!(
        [long, long_label].into_iter().all(|r| contains(clip, r))
            && near(long_label[0] - long[0], 16.)
            && near(right(long) - right(long_label), 16.)
            && long_label[3] >= 64.
            && near(long[1] - bottom(long_label), 4.)
            && unused_float[2..] == [0., 0.],
        "320dp RTL at 200% yields a real fully visible above-label fallback with no floating pose"
    );
    let (normal_png, normal) = capture(env, tab, "compact-normal").await?;
    let (colors, witness) = distinct(tab, "dark").await?;
    let canvas = rgb(&colors["canvas"]["background"])?;
    let canvas_text = rgb(&colors["canvas"]["foreground"])?;
    backing_layout(tab, kind, &id, long).await?;
    check_eq!(
        editor_geometry(tab, kind, 2.).await?,
        geometry,
        "compact Field paint leaves editor layout unchanged"
    );
    check_eq!(
        bounds(tab, &id).await?,
        long,
        "the long field face keeps its measured size and position"
    );
    check_eq!(
        bounds(tab, &format!("{id}-above-label")).await?,
        long_label,
        "the fallback keeps its measured geometry"
    );
    check_eq!(
        editing(tab, kind, [4, 4]).await?,
        semantic,
        "compact RTL keeps the focused draft and caret"
    );
    let (distinct_png, forced) = capture(env, tab, "compact-distinct").await?;
    let right_line = [right(long) - 94., long[1] + long[3] / 2. - 20., 78., 40.];
    let left_line = [long[0] + 16., long[1] + long[3] / 2. - 20., 52., 40.];
    let left_ink = forced.count_color(left_line, FIELD_TEXT)?;
    let wrong_caption_ink = forced.count_color(long_label, FIELD_TEXT)?;
    check!(
        left_ink == 0 && wrong_caption_ink == 0,
        "compact value stays on the right while the above-caption never borrows FieldText"
    );
    let mut closed_edge = Vec::new();
    for x in [long[0] + long[2] / 2., right(long) - 17.] {
        closed_edge.push(pixel(
            &forced,
            [x, long[1] + 0.5],
            FIELD_TEXT,
            "closed outline under above-caption",
        )?);
        closed_edge.push(pixel(
            &forced,
            [x, long[1] - 1.5],
            canvas,
            "Canvas outside the closed outline",
        )?);
    }
    let paint = json!({
        "floating_label":glyphs(&forced,label,FIELD_TEXT,12,"compact floating caption")?,
        "floating_notch":notch(&forced,face,label,canvas,true)?,
        "above_label":glyphs(&forced,long_label,canvas_text,40,"above label on Canvas")?,
        "closed_edge":closed_edge,
        "long_face":pixel(&forced,quiet(long,true),FIELD,"compact long field interior")?,
        "long_value":glyphs(&forced,right_line,FIELD_TEXT,20,"right-aligned long field value")?,
        "wrong_caption_field_text_pixels":wrong_caption_ink,"unwanted_left_field_text_pixels":left_ink,
    });

    let (malformed, _) = missing_pair(tab, "foreground", "not a color").await?;
    check_eq!(
        editing(tab, kind, [4, 4]).await?,
        semantic,
        "losing the optional pair keeps the same active editor"
    );
    let (fallback_png, fallback) = capture(env, tab, "compact-malformed-fallback").await?;
    let fallback_paint = json!({
        "quiet_face":pixel(&fallback,quiet(long,true),canvas,"Canvas face after incomplete Field pair")?,
        "ordinary_value":glyphs(&fallback,right_line,canvas_text,20,"CanvasText after incomplete Field pair")?,
    });

    tab.eval("window.__surfaceFieldPairFixture.foreground = 'rgb(249, 220, 126)'")
        .await?;
    until(
        tab,
        "s.display_contrast?.system_colors?.field?.foreground[0] === 249",
    )
    .await?;
    synthetic_palette(tab, &field_pair()).await?;
    let (missing, _) = missing_pair(tab, "background", "").await?;
    check_eq!(
        missing,
        malformed,
        "both malformed foreground and missing background drop the whole optional pair"
    );

    media(tab, false, false).await?;
    check!(
        contrast(tab, "normal").await?["system_colors"].is_null(),
        "the compact route releases the injected colors"
    );
    check_eq!(
        editing(tab, kind, [4, 4]).await?,
        semantic,
        "compact forced exit keeps the same editor and draft"
    );
    let (restored_png, restored) = capture(env, tab, "compact-restored").await?;
    let changed = [face, label, long, long_label]
        .into_iter()
        .map(|rect| changes(&normal, &restored, rect))
        .collect::<Result<Vec<_>>>()?;
    check!(
        changed.iter().all(|&n| n == 0),
        "compact Normal restores the original pixels: {changed:?}"
    );

    Ok(
        json!({"viewport":[320,960],"rtl":true,"text_scale":2,"geometry":geometry,"long":long_parts,
        "editing":semantic,"accepted":colors,"css_witness":witness,
        "paint":paint,"fallback":{"malformed_foreground":malformed,"missing_background":missing,"paint":fallback_paint},
        "images":[normal_png,distinct_png,fallback_png,restored_png],"size":forced.dimensions(),
        "restored_changed_pixels":changed}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 944, DPR).await?;
    tab.cdp("Page.enable", json!({})).await?;
    let mut report = json!({"browser":chrome.version,"dpr":DPR,
        "real_source":"CDP emulated system CSS with no palette override",
        "distinct_source":"synthetic unequal Field pair scoped to Surface optional probe; real browser Canvas/Button/Highlight/GrayText"});
    let result: Result<()> = async {
        step(STEPS[0], "The real light and dark browser CSS Field pairs match the standard Canvas pairs");
        report["real"] = real_system(env, &tab, url).await?;
        report["adapter"] = tab.eval(r#"(async () => {
            const {info} = await navigator.gpu.requestAdapter();
            const {vendor, architecture, isFallbackAdapter} = info;
            return {vendor, architecture, isFallbackAdapter};
        })()"#).await?;
        check!(report["adapter"]["vendor"].is_string() && report["adapter"]["architecture"].is_string()
            && report["adapter"]["isFallbackAdapter"].is_boolean(),"an observed headless WebGPU adapter");
        tab.cdp("Page.addScriptToEvaluateOnNewDocument",json!({"source":INJECT})).await?;
        step(STEPS[1], "An unequal Field pair paints both fields, with independent selection and exact restoration");
        for kind in [Kind::Outlined, Kind::Filled] {
            report[kind.tag()] = wide(env,&tab,url,kind).await?;
        }
        step(STEPS[2], "The compact RTL field keeps canvas captions and rejects an incomplete Field pair");
        report["compact"] = compact(env, &tab, url).await?;
        check!(tab.errors().is_empty(), "the genuine and synthetic contrast routes have no page errors");
        Ok(())
    }.await;
    report["page_errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["failure_state"] = state(&tab).await.unwrap_or(Value::Null);
        report["failure_dom"] = tab
            .eval_json("window.gallery.export_dom()")
            .await
            .unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact(&format!("{PREFIX}-failure.png")))
            .await;
    }
    env.write_json(&format!("{PREFIX}.json"), &report)?;
    result
}
