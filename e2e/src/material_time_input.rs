//! Material Expressive Time Input through the real headless Chrome host.

use std::{collections::BTreeMap, fs::File, io::BufReader, path::PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const INLINE: &str = "time-input-inline";
const MODAL: &str = "time-input-modal";
const OPEN: &str = "time-input-demo-modal-open";
const OUTSIDE: &str = "time-input-demo-outside";
const SUBMIT: &str = "time-input-demo-inline-submit";
const TEXTAREA: &str = "document.getElementById('surface-text-input')";

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn part(base: &str, suffix: &str) -> String {
    format!("{base}-{suffix}")
}

pub(super) fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.time_input)")).await?;
    tab.settle().await
}

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["time_input"].take())
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

pub(super) async fn focused(tab: &Tab, id: &str) -> Result<()> {
    runtime_focus(tab, id).await?;
    let target = if id.ends_with("-hour-input") || id.ends_with("-minute-input") {
        TEXTAREA.to_owned()
    } else {
        projected(id)
    };
    tab.wait_for(&format!(
        "{target} !== null && document.activeElement === {target}"
    ))
    .await?;
    tab.settle().await
}

pub(super) async fn focus_projected(tab: &Tab, id: &str) -> Result<()> {
    let target = projected(id);
    tab.wait_for(&format!("{target} !== null")).await?;
    tab.eval(&format!("{target}.focus()")).await?;
    focused(tab, id).await
}

pub(super) async fn activate(tab: &Tab, id: &str) -> Result<()> {
    focus_projected(tab, id).await?;
    tab.press("Enter").await?;
    tab.settle().await
}

pub(super) async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{ const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
           return v?.width === {width} && v?.height === {height}; }})()"
    ))
    .await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1204).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1160).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "time-input-demo-show").await?;
    ready(
        tab,
        "d.visible && d.modal === null && d.format === 'TwelveHour' &&
        d.locale === 'zh-Hant' && d.inline_commits === 0 && d.modal_commits === 0",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, setting: &str) -> Result<()> {
    activate(tab, &format!("time-input-demo-{setting}")).await
}

async fn open(tab: &Tab) -> Result<()> {
    activate(tab, OPEN).await?;
    ready(tab, "d.modal !== null").await?;
    focused(tab, &part(MODAL, "hour-input")).await
}

async fn replace(tab: &Tab, presentation: &str, field: &str, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    if raw.is_empty() {
        tab.press("Backspace").await?;
    } else {
        tab.insert_text(raw).await?;
    }
    ready(
        tab,
        &format!("d.{presentation}.raw_{field} === {}", js(raw)),
    )
    .await
}

pub(super) async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .try_into()
        .map_err(|rect| anyhow::anyhow!("expected four bounds for {id}: {rect:?}"))
}

pub(super) fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    [
        x,
        y,
        (a[0] + a[2]).min(b[0] + b[2]) - x,
        (a[1] + a[3]).min(b[1] + b[3]) - y,
    ]
}

pub(super) fn contains(outer: Rect, inner: Rect) -> bool {
    intersect(outer, inner)
        .into_iter()
        .zip(inner)
        .all(|(a, b)| near(a, b))
}

pub(super) async fn ax_tree(tab: &Tab) -> Result<Vec<Value>> {
    let mut result = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    match result["nodes"].take() {
        Value::Array(nodes) => Ok(nodes),
        _ => anyhow::bail!("Chrome did not return accessibility nodes"),
    }
}

pub(super) fn ax<'a>(tree: &'a [Value], role: &str, name: &str) -> Result<&'a Value> {
    tree.iter()
        .find(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"] == name
        })
        .with_context(|| format!("Chrome AX lacks {role} named {name}"))
}

pub(super) fn ax_property<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|property| property["name"] == key)
        .map(|property| &property["value"]["value"])
}

async fn description(tab: &Tab, label: &str, helper: &str, excerpt: &str) -> Result<Value> {
    let target = projected(helper);
    tab.wait_for(&format!(
        "(() => {{
          const reference = {TEXTAREA}?.getAttribute('aria-describedby');
          return reference && document.getElementById(reference) === {target};
        }})()",
    ))
    .await?;
    let spoken = ax_tree(tab).await?;
    let editor = ax(&spoken, "textbox", label)?;
    let desc = editor["description"]["value"]
        .as_str()
        .context("textbox AX description")?;
    let help = tab.eval_str(&format!("{target}?.textContent")).await?;
    let words = |value: &str| value.split_whitespace().collect::<Vec<_>>().join(" ");
    check!(
        !help.trim().is_empty() && words(desc).contains(&words(&help)) && desc.contains(excerpt),
        "the named editor reads its own helper: {label}, {desc:?}, {help:?}"
    );
    Ok(editor["description"].clone())
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<PathBuf> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-time-input-{name}.png"));
    tab.screenshot(&path).await?;
    Ok(path)
}

async fn inline(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M134",
        "inline Hour and Minute edit independently and AM/PM maps midnight and noon",
    );
    ready(tab, "d.inline.raw_hour === '12' && d.inline.raw_minute === '05' &&
        d.inline.period === 'Am' && d.inline.candidate === '00:05' && d.inline_committed === '00:05'").await?;
    let hour = part(INLINE, "hour-input");
    let minute = part(INLINE, "minute-input");
    let hour_rect = bounds(tab, &hour).await?;
    let minute_rect = bounds(tab, &minute).await?;
    check!(
        near(hour_rect[2], minute_rect[2]) && hour_rect[3] >= 47.5 && minute_rect[3] >= 47.5,
        "inline fields grow equally and have full targets: {hour_rect:?}, {minute_rect:?}"
    );
    focus_projected(tab, &hour).await?;
    let hour_hint = description(tab, "小時", &part(INLINE, "hour-hint"), "1–12").await?;
    replace(tab, "inline", "hour", "１２").await?;
    ready(
        tab,
        "d.inline.raw_minute === '05' && d.inline_commits === 0",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    ready(
        tab,
        "d.inline.raw_hour === '12' && d.inline.raw_minute === '05'",
    )
    .await?;
    let minute_hint = description(tab, "分鐘", &part(INLINE, "minute-hint"), "0–59").await?;
    replace(tab, "inline", "minute", "５").await?;
    ready(
        tab,
        "d.inline.raw_hour === '12' && d.inline.candidate === '00:05'",
    )
    .await?;
    tab.press("Enter").await?;
    focused(tab, &minute).await?;
    ready(
        tab,
        "d.inline.raw_minute === '05' && d.inline_commits === 0",
    )
    .await?;

    let am = part(INLINE, "am");
    let pm = part(INLINE, "pm");
    tab.press("Tab").await?;
    focused(tab, &am).await?;
    let a = bounds(tab, &am).await?;
    let p = bounds(tab, &pm).await?;
    let arrow = if near(a[1], p[1]) {
        if p[0] > a[0] {
            "ArrowRight"
        } else {
            "ArrowLeft"
        }
    } else if p[1] > a[1] {
        "ArrowDown"
    } else {
        "ArrowUp"
    };
    tab.press(arrow).await?;
    focused(tab, &pm).await?;
    ready(
        tab,
        "d.inline.period === 'Pm' && d.inline.raw_hour === '12' && d.inline.raw_minute === '05' &&
        d.inline.candidate === '12:05' && d.inline_committed === '00:05' && d.inline_commits === 0",
    )
    .await?;
    let tree = ax_tree(tab).await?;
    let hour_radio = ax(&tree, "radio", "上午")?;
    let minute_radio = ax(&tree, "radio", "下午")?;
    check_eq!(
        ax_property(hour_radio, "checked"),
        Some(&json!("false")),
        "AM is unselected"
    );
    check_eq!(
        ax_property(minute_radio, "checked"),
        Some(&json!("true")),
        "PM is selected"
    );
    tab.press("Tab").await?;
    until(
        tab,
        &format!(
            "s.focused !== null && s.focused !== {} && s.focused !== {}",
            js(&am),
            js(&pm)
        ),
    )
    .await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &pm).await?;
    let image = Raster::read(tab, capture(env, tab, "wide-inline-12h").await?).await?;
    let label = part(INLINE, "hour-label");
    let ink = authored_color(tab, &label, "style.foreground").await?;
    let label_contrast = painted_contrast(&image, bounds(tab, &label).await?, ink, 4.5)?;
    activate(tab, SUBMIT).await?;
    ready(
        tab,
        "d.inline_committed === '12:05' && d.inline_commits === 1 && d.modal_commits === 0",
    )
    .await?;
    let hit = point(tab, &hour).await?;
    tab.click(hit.x, hit.y).await?;
    focused(tab, &hour).await?;
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    tab.press("Tab").await?;
    runtime_focus(tab, &pm).await?;
    tab.wait_for("document.activeElement === document.querySelector('canvas')")
        .await?;
    let canvas_focus = tab.eval_str("document.activeElement.tagName").await?;
    ready(
        tab,
        "d.inline.period === 'Pm' && d.inline_committed === '12:05' && d.inline_commits === 1",
    )
    .await?;
    Ok(
        json!({"field_bounds":[hour_rect,minute_rect],"hour_hint":hour_hint,"minute_hint":minute_hint,
        "period_bounds":[a,p],"am":"00:05","pm":"12:05","submits":1,"label_contrast":label_contrast,
        "pointer_tab_focus":{"runtime":pm,"browser":canvas_focus}}),
    )
}

async fn modal_24(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M135",
        "modal 24-hour input accepts localized Japanese digits and rejects invalid or incomplete times",
    );
    preset(tab, "locale").await?;
    preset(tab, "format").await?;
    ready(tab, "d.locale === 'ja' && d.format === 'TwentyFourHour'").await?;
    open(tab).await?;
    ready(
        tab,
        "d.modal.period === null && d.modal.raw_hour === '０９' && d.modal.raw_minute === '０７' &&
        d.modal.candidate === '09:07' && d.modal_committed === '09:07'",
    )
    .await?;

    let ax = ax_tree(tab).await?;
    let dialog = self::ax(&ax, "dialog", "時刻を入力")?;
    check!(
        ax.iter()
            .all(|node| node["ignored"] != false || node["role"]["value"] != "radio"),
        "Chrome exposes no period options in the 24-hour modal"
    );
    for suffix in ["am", "pm", "period"] {
        check_eq!(
            tab.eval(&format!(
                "window.gallery.control_bounds({}).length",
                js(part(MODAL, suffix))
            ))
            .await?,
            json!(0),
            "period selectors consume no layout space in 24-hour time"
        );
    }

    let hour = part(MODAL, "hour-input");
    let minute = part(MODAL, "minute-input");
    let hour_hint = description(tab, "時", &part(MODAL, "hour-hint"), "０–２３").await?;
    replace(tab, "modal", "hour", "２３").await?;
    ready(
        tab,
        "d.modal.candidate === '23:07' && d.modal_commits === 0",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    let minute_hint = description(tab, "分", &part(MODAL, "minute-hint"), "０–５９").await?;
    replace(tab, "modal", "minute", "５９").await?;
    ready(
        tab,
        "d.modal.candidate === '23:59' && d.modal_commits === 0",
    )
    .await?;
    replace(tab, "modal", "minute", "59").await?;
    tab.press("Enter").await?;
    focused(tab, &minute).await?;
    ready(
        tab,
        "d.modal.raw_minute === '５９' && d.modal.candidate === '23:59' && d.modal_commits === 0",
    )
    .await?;

    replace(tab, "modal", "minute", "60").await?;
    tab.press("Tab").await?;
    tab.settle().await?;
    let after_minute = state(tab).await?["focused"].clone();
    check!(
        after_minute
            .as_str()
            .is_some_and(|id| id.starts_with(MODAL))
            && after_minute != part(MODAL, "am")
            && after_minute != part(MODAL, "pm"),
        "Tab stays inside the modal without a period stop: {after_minute}"
    );
    ready(
        tab,
        "d.modal.raw_minute === '60' && d.modal.minute_error === 'OutOfRange' &&
        d.modal.hour_error === null && d.modal.candidate === null && d.modal_commits === 0",
    )
    .await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &minute).await?;
    let minute_error = description(tab, "分", &part(MODAL, "minute-error"), "０から５９").await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &hour).await?;
    replace(tab, "modal", "hour", "24").await?;
    activate(tab, &part(MODAL, "confirm")).await?;
    ready(
        tab,
        "d.modal.raw_hour === '24' && d.modal.raw_minute === '60' &&
        d.modal.hour_error === 'OutOfRange' && d.modal.minute_error === 'OutOfRange' &&
        d.modal.candidate === null && d.modal_committed === '09:07' && d.modal_commits === 0",
    )
    .await?;
    focus_projected(tab, &hour).await?;
    let hour_error = description(tab, "時", &part(MODAL, "hour-error"), "０から２３").await?;
    capture(env, tab, "wide-modal-24h-error").await?;
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    replace(tab, "modal", "minute", "").await?;
    tab.press("Enter").await?;
    ready(tab, "d.modal.raw_minute === '' && d.modal.minute_error === 'Required' && d.modal.candidate === null &&
        d.modal.raw_hour === '24' && d.modal.hour_error === 'OutOfRange' && d.modal_commits === 0").await?;
    Ok(
        json!({"dialog":dialog["name"],"hour_hint":hour_hint,"minute_hint":minute_hint,
        "hour_error":hour_error,"minute_error":minute_error,"valid":"23:59",
        "invalid":["24:59","23:60","24:"],"commits":0}),
    )
}

async fn ax_fields(tab: &Tab) -> Result<Value> {
    step(
        "M136",
        "independent errors and focus stay with their named editors inside the modal",
    );
    open(tab).await?;
    let hour = part(MODAL, "hour-input");
    let minute = part(MODAL, "minute-input");
    let confirm = part(MODAL, "confirm");
    let tree = ax_tree(tab).await?;
    ax(&tree, "dialog", "選擇時間")?;
    check_eq!(
        tab.eval(&format!("{} === null", projected(OUTSIDE)))
            .await?,
        json!(true),
        "the background action is outside the modal accessibility scope"
    );
    let hour_hint = description(tab, "小時", &part(MODAL, "hour-hint"), "1–12").await?;
    replace(tab, "modal", "hour", "0").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.modal.hour_error === 'OutOfRange' && d.modal.raw_hour === '0' &&
        d.modal.raw_minute === '07' && d.modal.minute_error === null && d.modal.candidate === null",
    )
    .await?;
    replace(tab, "modal", "hour", "13").await?;
    tab.press("Enter").await?;
    ready(tab, "d.modal.hour_error === 'OutOfRange' && d.modal.raw_hour === '13' && d.modal.candidate === null").await?;
    let hour_error = description(
        tab,
        "小時",
        &part(MODAL, "hour-error"),
        "小時須介於 1 與 12",
    )
    .await?;
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    let minute_hint = description(tab, "分鐘", &part(MODAL, "minute-hint"), "0–59").await?;
    replace(tab, "modal", "minute", "60").await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.modal.minute_error === 'OutOfRange' && d.modal.hour_error === 'OutOfRange'",
    )
    .await?;
    let minute_error = description(
        tab,
        "分鐘",
        &part(MODAL, "minute-error"),
        "分鐘須介於 0 與 59",
    )
    .await?;
    check!(
        hour_error["value"] != minute_error["value"],
        "the two field errors have distinct text"
    );
    tab.press("Shift+Tab").await?;
    focused(tab, &hour).await?;
    replace(tab, "modal", "hour", "09").await?;
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    ready(
        tab,
        "d.modal.raw_hour === '9' && d.modal.hour_error === null && d.modal.raw_minute === '60' &&
        d.modal.minute_error === 'OutOfRange' && d.modal_commits === 0",
    )
    .await?;
    description(
        tab,
        "分鐘",
        &part(MODAL, "minute-error"),
        "分鐘須介於 0 與 59",
    )
    .await?;

    focus_projected(tab, &confirm).await?;
    tab.press("Tab").await?;
    focused(tab, &hour).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &confirm).await?;
    Ok(
        json!({"hour_hint":hour_hint,"minute_hint":minute_hint,"hour_error":hour_error,
        "minute_error":minute_error,"forward_and_reverse_trap":true}),
    )
}

async fn closed(tab: &Tab, expected: &str, commits: u32) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.modal === null && d.modal_committed === {} && d.modal_commits === {commits} &&
         d.inline_commits === 0 && d.outside_actions === 0",
            js(expected)
        ),
    )
    .await?;
    tab.wait_for(&format!(
        "{} === null && {} !== null",
        projected(MODAL),
        projected(OUTSIDE)
    ))
    .await
}

async fn composition(env: &Env, tab: &Tab) -> Result<Value> {
    step("M137", "only explicit Save commits; IME Enter and all three dismissal paths preserve the saved time");
    open(tab).await?;
    let minute = part(MODAL, "minute-input");
    tab.press("Tab").await?;
    focused(tab, &minute).await?;
    replace(tab, "modal", "minute", "").await?;
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(&minute),
    );
    tab.ime_compose("５９", 2, 2).await?;
    tab.wait_for(&format!(
        "{composing} && {TEXTAREA}?.value.includes('５９')"
    ))
    .await?;
    ready(
        tab,
        "d.modal.raw_minute === '' && d.modal_committed === '09:07' && d.modal_commits === 0",
    )
    .await?;
    capture(env, tab, "modal-ime-preedit").await?;
    tab.press("Enter").await?;
    focused(tab, &minute).await?;
    ready(
        tab,
        "d.modal !== null && d.modal_committed === '09:07' && d.modal_commits === 0",
    )
    .await?;
    if data(tab).await?["modal"]["raw_minute"] == "" {
        tab.insert_text("５９").await?;
    }
    ready(
        tab,
        "(d.modal.raw_minute === '５９' || d.modal.raw_minute === '59') && d.modal_commits === 0",
    )
    .await?;
    tab.wait_for(&format!("!({composing})")).await?;
    tab.press("Enter").await?;
    focused(tab, &minute).await?;
    ready(
        tab,
        "d.modal.raw_minute === '59' && d.modal.candidate === '09:59' && d.modal_commits === 0",
    )
    .await?;
    activate(tab, &part(MODAL, "confirm")).await?;
    closed(tab, "09:59", 1).await?;
    focused(tab, OPEN).await?;

    let outside = point(tab, OUTSIDE).await?;
    for action in ["cancel", "escape", "scrim"] {
        open(tab).await?;
        ready(tab, "d.modal.raw_hour === '9' && d.modal.raw_minute === '59' && d.modal.candidate === '09:59'").await?;
        replace(tab, "modal", "hour", "10").await?;
        ready(
            tab,
            "d.modal.candidate === '10:59' && d.modal_commits === 1",
        )
        .await?;
        match action {
            "cancel" => activate(tab, &part(MODAL, "cancel")).await?,
            "escape" => tab.press("Escape").await?,
            "scrim" => {
                let panel = point(tab, MODAL).await?;
                let x = outside.x - panel.canvas[0];
                let y = outside.y - panel.canvas[1];
                check!(
                    x < panel.b[0]
                        || x >= panel.b[0] + panel.b[2]
                        || y < panel.b[1]
                        || y >= panel.b[1] + panel.b[3],
                    "the trusted outside click hits the scrim over a real background action"
                );
                tab.click(outside.x, outside.y).await?;
            }
            _ => unreachable!("test dismissal case"),
        }
        closed(tab, "09:59", 1).await?;
        if action == "scrim" {
            runtime_focus(tab, OPEN).await?;
        } else {
            focused(tab, OPEN).await?;
        }
    }
    Ok(
        json!({"ime_enter_commits":0,"explicit_save_commits":1,"saved":"09:59",
        "discarded_by":["cancel","escape","scrim"],"outside_actions":0}),
    )
}

struct Raster {
    bytes: Vec<u8>,
    frame: png::OutputInfo,
    channels: usize,
    scale: f64,
    canvas: Rect,
}

impl Raster {
    async fn read(tab: &Tab, path: PathBuf) -> Result<Self> {
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let frame = reader.next_frame(&mut bytes)?;
        let channels = match frame.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => anyhow::bail!("unexpected PNG color type {other:?}"),
        };
        let width = tab
            .eval("innerWidth")
            .await?
            .as_f64()
            .context("web viewport width")?;
        let scale = f64::from(frame.width) / width;
        Ok(Self {
            bytes,
            frame,
            channels,
            scale,
            canvas: tab.canvas_box().await?,
        })
    }

    fn colors(&self, rect: Rect) -> Result<BTreeMap<Rgb, usize>> {
        let [x, y, w, h] = rect;
        let px = |x: f64| ((self.canvas[0] + x) * self.scale).round() as usize;
        let py = |y: f64| ((self.canvas[1] + y) * self.scale).round() as usize;
        let (x1, x2, y1, y2) = (px(x), px(x + w), py(y), py(y + h));
        check!(
            x1 < x2
                && y1 < y2
                && x2 <= self.frame.width as usize
                && y2 <= self.frame.height as usize,
            "the paint sample lies inside the actual headless PNG: {rect:?}"
        );
        let mut colors = BTreeMap::new();
        for row in y1..y2 {
            let start = row * self.frame.line_size;
            for pixel in self.bytes[start + x1 * self.channels..start + x2 * self.channels]
                .chunks_exact(self.channels)
            {
                *colors.entry([pixel[0], pixel[1], pixel[2]]).or_insert(0) += 1;
            }
        }
        Ok(colors)
    }
}

fn luminance(rgb: Rgb) -> f64 {
    let linear = |n| {
        let c = f64::from(n) / 255.;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb[0]) + 0.7152 * linear(rgb[1]) + 0.0722 * linear(rgb[2])
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn node(id: &str) -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})",
        js(id)
    )
}

async fn authored_color(tab: &Tab, id: &str, property: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] =
        serde_json::from_value(tab.eval(&format!("({}).{property}", node(id))).await?)?;
    check_eq!(a, 255, "the sampled design color is opaque");
    Ok([r, g, b])
}

fn painted_contrast(image: &Raster, rect: Rect, expected: Rgb, minimum: f64) -> Result<Value> {
    let colors = image.colors(rect)?;
    let close = |sample: &&Rgb| sample.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 5);
    let (ink, count) = colors
        .iter()
        .filter(|(sample, _)| close(sample))
        .max_by_key(|(_, count)| *count)
        .context("expected label or focus ink in actual screenshot")?;
    let (background, _) = colors
        .iter()
        .filter(|(sample, _)| !close(sample))
        .max_by_key(|(_, count)| *count)
        .context("adjacent background in actual screenshot")?;
    let ratio = contrast(*ink, *background);
    check!(
        *count >= 8 && ratio >= minimum,
        "visible ink covers pixels and meets {minimum}: {ink:?}, {background:?}, {ratio:.2}, {count}px"
    );
    Ok(json!({"ink":ink,"background":background,"ratio":ratio,"pixels":count}))
}

fn scroll_expression() -> String {
    format!("({}).style.scroll_y", node(&part(MODAL, "body")))
}

async fn scroll_y(tab: &Tab) -> Result<f64> {
    tab.eval(&scroll_expression())
        .await?
        .as_f64()
        .context("time input body scroll offset")
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M138",
        "short dark RTL at 200% keeps fields, help, period and actions reachable",
    );
    for toggle in ["theme", "rtl", "large-text"] {
        preset(tab, toggle).await?;
    }
    until(
        tab,
        "s.dark && s.time_input.rtl && s.time_input.text_scale === 2",
    )
    .await?;
    open(tab).await?;
    resize(tab, 320, 240).await?;
    let hour_id = part(MODAL, "hour-input");
    let minute_id = part(MODAL, "minute-input");
    focused(tab, &hour_id).await?;

    let panel = bounds(tab, MODAL).await?;
    let body_id = part(MODAL, "body");
    let body = bounds(tab, &body_id).await?;
    let hour = bounds(tab, &hour_id).await?;
    let minute = bounds(tab, &minute_id).await?;
    check!(
        contains([0., 0., 320., 240.], panel) && contains(panel, body),
        "the modal stays in the short viewport"
    );
    check!(
        near(hour[2], minute[2])
            && near(hour[1], minute[1])
            && hour[0] > minute[0]
            && contains(body, hour)
            && hour[3] >= 47.5,
        "RTL fields grow equally and the focused Hour fully fits: {hour:?}, {minute:?}, {body:?}"
    );
    let image = Raster::read(tab, capture(env, tab, "short-dark-rtl-focus").await?).await?;
    let digit_rect = [hour[0] + 8., hour[1] + 6., hour[2] - 16., hour[3] - 12.];
    let ring_rect = [hour[0] + hour[2] * 0.25, hour[1], hour[2] * 0.5, 7.];
    let digit_color = authored_color(tab, &hour_id, "style.foreground").await?;
    let focus_color = authored_color(tab, &hour_id, "focus_color").await?;
    let digit_contrast = painted_contrast(&image, digit_rect, digit_color, 4.5)?;
    let focus_contrast = painted_contrast(&image, ring_rect, focus_color, 3.)?;
    let hour_help = part(MODAL, "hour-hint");
    check!(
        contains(body, bounds(tab, &hour_help).await?),
        "the focused Hour hint fully fits"
    );
    let hour_hint = description(tab, "小時", &hour_help, "1–12").await?;

    tab.press("Tab").await?;
    focused(tab, &minute_id).await?;
    let visible_minute = bounds(tab, &minute_id).await?;
    check!(
        contains(body, visible_minute) && visible_minute[3] >= 47.5,
        "the separate Minute editor is fully revealed by focus"
    );
    let minute_help = part(MODAL, "minute-hint");
    check!(
        contains(body, bounds(tab, &minute_help).await?),
        "the focused Minute hint fully fits"
    );
    let minute_hint = description(tab, "分鐘", &minute_help, "0–59").await?;

    let am_id = part(MODAL, "am");
    let pm_id = part(MODAL, "pm");
    tab.press("Tab").await?;
    focused(tab, &am_id).await?;
    let am = bounds(tab, &am_id).await?;
    let pm = bounds(tab, &pm_id).await?;
    check!(
        pm[0] < am[0]
            && near(am[1], pm[1])
            && contains(body, am)
            && contains(body, pm)
            && [am, pm]
                .iter()
                .all(|bounds| bounds[2] >= 47.5 && bounds[3] >= 47.5),
        "the wrapped period row keeps both 48 dp controls visible in RTL: {am:?}, {pm:?}"
    );
    tab.press("ArrowLeft").await?;
    focused(tab, &pm_id).await?;
    ready(
        tab,
        "d.modal.period === 'Pm' && d.modal.candidate === '21:07' && d.modal_commits === 0",
    )
    .await?;

    let scroll = point(tab, &body_id).await?;
    let before_wheel = scroll_y(tab).await?;
    check!(
        before_wheel > 8.,
        "the short form scrolled to reveal the period"
    );
    tab.mouse_move(scroll.x, scroll.y).await?;
    tab.wheel(0., -220.).await?;
    tab.wait_for(&format!("{} < {}", scroll_expression(), before_wheel - 5.))
        .await?;
    let after_wheel = scroll_y(tab).await?;
    runtime_focus(tab, &pm_id).await?;
    ready(
        tab,
        "d.modal.period === 'Pm' && d.modal.candidate === '21:07' && d.modal_commits === 0",
    )
    .await?;

    tab.press("Shift+Tab").await?;
    focused(tab, &minute_id).await?;
    replace(tab, "modal", "minute", "60").await?;
    tab.press("Enter").await?;
    ready(tab, "d.modal.minute_error === 'OutOfRange' && d.modal.candidate === null && d.modal_commits === 0").await?;
    let helper = part(MODAL, "minute-error");
    let help = bounds(tab, &helper).await?;
    let delta = help[1] + help[3] - (body[1] + body[3]);
    if delta > 0.75 {
        let dpr = tab
            .eval("devicePixelRatio")
            .await?
            .as_f64()
            .context("device pixel ratio")?;
        tab.wheel(0., (delta + 1.) * dpr).await?;
        tab.settle().await?;
    }
    focused(tab, &minute_id).await?;
    let help = bounds(tab, &helper).await?;
    check!(
        contains(body, help),
        "the whole wrapped Minute error can scroll into view: {help:?}"
    );
    let error = description(tab, "分鐘", &helper, "分鐘須介於 0 與 59").await?;
    let image = Raster::read(tab, capture(env, tab, "short-dark-rtl-error").await?).await?;
    let error_color = authored_color(tab, &helper, "style.foreground").await?;
    let error_contrast = painted_contrast(&image, help, error_color, 4.5)?;

    let cancel = part(MODAL, "cancel");
    let confirm = part(MODAL, "confirm");
    let actions = [bounds(tab, &cancel).await?, bounds(tab, &confirm).await?];
    for target in actions {
        check!(
            contains(panel, target) && target[2] >= 47.5 && target[3] >= 47.5,
            "the short modal actions remain fully reachable: {target:?}"
        );
    }
    activate(tab, &confirm).await?;
    ready(tab, "d.modal.minute_error === 'OutOfRange' && d.modal_committed === '09:07' && d.modal_commits === 0").await?;
    activate(tab, &cancel).await?;
    ready(tab, "d.modal === null && d.modal_commits === 0").await?;
    runtime_focus(tab, OPEN).await?;
    resize(tab, 960, 820).await?;
    closed(tab, "09:07", 0).await?;
    focused(tab, OPEN).await?;
    Ok(
        json!({"panel":panel,"body":body,"fields":[hour,visible_minute],"period":[am,pm],"actions":actions,
        "wheel_offsets":[before_wheel,after_wheel],"digit_contrast":digit_contrast,
        "focus_contrast":focus_contrast,"error_contrast":error_contrast,
        "hour_hint":hour_hint,"minute_hint":minute_hint,"error":error}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let inline = inline(env, tab).await?;
    reload(tab, url).await?;
    let modal = modal_24(env, tab).await?;
    reload(tab, url).await?;
    let fields = ax_fields(tab).await?;
    reload(tab, url).await?;
    let composition = composition(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "time input page errors: {errors:?}");
    env.write_json(
        "browser-material-time-input.json",
        &json!({"browser":chrome.version,"inline":inline,"modal_24_hour":modal,"fields":fields,
            "composition":composition,"compact":compact,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 864, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-time-input-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-time-input-failure.png"))
            .await;
    }
    result
}
