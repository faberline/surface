//! A headless Chrome journey through the Material date range picker.

use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const PICKER: &str = "date-range-picker-demo-modal";
const OPEN: &str = "date-range-picker-demo-open";
const OPEN_INPUT: &str = "date-range-picker-demo-input-open";
const OUTSIDE: &str = "date-range-picker-demo-outside";
const START: &str = "2024-02-24";
const END: &str = "2024-03-05";

type Rect = [f64; 4];

fn part(suffix: &str) -> String {
    format!("{PICKER}-{suffix}")
}

fn day(date: &str) -> String {
    part(&format!("day-{date}"))
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{ const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
           return v?.width === {width} && v?.height === {height}; }})()"
    ))
    .await?;
    tab.settle().await
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.date_range_picker)")).await?;
    tab.settle().await
}

async fn range_state(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["date_range_picker"].take())
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1024).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 980).await?;
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    focus_projected(tab, "date-range-picker-demo-show").await?;
    tab.press("Enter").await?;
    ready(tab, "d.visible && d.session === null && d.commits === 0").await?;
    resize(tab, 960, 880).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    let element = if id.ends_with("-input") && id.starts_with(PICKER) {
        "document.getElementById('surface-text-input')".to_owned()
    } else {
        projected(id)
    };
    tab.wait_for(&format!(
        "{element} !== null && document.activeElement === {element}"
    ))
    .await?;
    tab.settle().await
}

async fn focus_projected(tab: &Tab, id: &str) -> Result<()> {
    let element = projected(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    focused(tab, id).await
}

async fn open(tab: &Tab, input: bool) -> Result<()> {
    let (opener, mode, initial) = if input {
        (OPEN_INPUT, "Input", part("start-input"))
    } else {
        (OPEN, "Calendar", day(START))
    };
    focus_projected(tab, opener).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.session?.mode === {}", js(mode))).await?;
    focused(tab, &initial).await
}

async fn closed(tab: &Tab, opener: &str, dates: [&str; 2], commits: u32) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.session === null && d.commits === {commits} &&
             d.committed[0] === {} && d.committed[1] === {}",
            js(dates[0]),
            js(dates[1]),
        ),
    )
    .await?;
    focused(tab, opener).await?;
    tab.wait_for(&format!(
        "{} === null && {} !== null",
        projected(PICKER),
        projected(OUTSIDE)
    ))
    .await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .try_into()
        .map_err(|bounds| anyhow::anyhow!("expected four bounds for {id}: {bounds:?}"))
}

fn near(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.75
}

async fn ax_tree(tab: &Tab) -> Result<Vec<Value>> {
    let mut tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    match tree["nodes"].take() {
        Value::Array(nodes) => Ok(nodes),
        _ => anyhow::bail!("missing Chrome accessibility nodes"),
    }
}

fn ax<'a>(tree: &'a [Value], role: &str, label: &str) -> Result<&'a Value> {
    tree.iter()
        .find(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"]
                    .as_str()
                    .is_some_and(|name| name.contains(label))
        })
        .with_context(|| format!("Chrome accessibility lacks {role} named {label}"))
}

fn ax_property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|property| property["name"] == name)
        .map(|property| &property["value"]["value"])
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<PathBuf> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-date-range-picker-{name}.png"));
    tab.screenshot(&path).await?;
    Ok(path)
}

fn export_node(id: &str) -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})",
        js(id)
    )
}

async fn band_geometry(tab: &Tab, month: &str, week: u8, x: f64, width: f64) -> Result<Rect> {
    let row_id = part(&format!("month-{month}-week-{week}"));
    let row = bounds(tab, &row_id).await?;
    let band_id = format!("{row_id}-band");
    let band = bounds(tab, &band_id).await?;
    check!(
        near(row[2], 336.)
            && near(row[3], 48.)
            && near(band[0] - row[0], x)
            && near(band[1] - row[1], 4.)
            && near(band[2], width)
            && near(band[3], 40.),
        "48 dp week and 40 dp band intersect at the requested date columns: {row:?}, {band:?}"
    );
    check_eq!(
        tab.eval(&format!("{} === null", projected(&band_id)))
            .await?,
        json!(true),
        "the paint band does not enter the accessibility tree"
    );
    Ok(band)
}

async fn paint_start(env: &Env, tab: &Tab, band: Rect) -> Result<Value> {
    let path = capture(env, tab, "calendar-light-cross-month").await?;
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
    let frame = reader.next_frame(&mut pixels)?;
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => anyhow::bail!("unexpected Chrome PNG color type {other:?}"),
    };
    let css_width = tab
        .eval("innerWidth")
        .await?
        .as_f64()
        .context("CSS width")?;
    let scale = f64::from(frame.width) / css_width;
    let canvas = tab.canvas_box().await?;
    let sample = |x: f64, y: f64| -> Result<[u8; 3]> {
        let x = ((x + canvas[0]) * scale).floor() as usize;
        let y = ((y + canvas[1]) * scale).floor() as usize;
        check!(
            x < frame.width as usize && y < frame.height as usize,
            "sample lies in the PNG"
        );
        let start = y * frame.line_size + x * channels;
        Ok(pixels[start..start + 3].try_into()?)
    };
    let face_id = format!("{}-face", day(START));
    let face = bounds(tab, &face_id).await?;
    let body = bounds(tab, &part("body")).await?;
    let x = band[0] + band[2] - 3.;
    let y = band[1] + 5.;
    check!(
        (body[0]..body[0] + body[2]).contains(&x) && (body[1]..body[1] + body[3]).contains(&y),
        "the screenshot band sample lies inside the scroll clip"
    );
    let band_color = sample(x, y)?;
    let face_color = sample(face[0] + face[2] * 0.5, face[1] + 8.)?;
    let expected_band: [u8; 4] = serde_json::from_value(
        tab.eval(&format!(
            "{}.style.background",
            export_node(&part("month-2024-02-week-3-band"))
        ))
        .await?,
    )?;
    let expected_face: [u8; 4] = serde_json::from_value(
        tab.eval(&format!("{}.style.background", export_node(&face_id)))
            .await?,
    )?;
    let matches = |a: [u8; 3], b: [u8; 4]| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 4);
    check!(
        matches(band_color, expected_band) && matches(face_color, expected_face) && band_color != face_color,
        "PNG shows the band behind the distinct endpoint face: {band_color:?}, {face_color:?}, {expected_band:?}, {expected_face:?}"
    );
    Ok(json!({"band_rgb":band_color,"face_rgb":face_color,"sample":[x,y]}))
}

async fn calendar_paint_and_ax(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M122",
        "cross-month range bands and endpoints paint inside named multi-select grids",
    );
    open(tab, false).await?;
    ready(
        tab,
        "d.session.mounted_months.length >= 2 && d.session.mounted_months.length <= 5",
    )
    .await?;
    let rows = [
        band_geometry(tab, "2024-02", 3, 312., 24.).await?,
        band_geometry(tab, "2024-02", 4, 0., 240.).await?,
        band_geometry(tab, "2024-03", 0, 240., 96.).await?,
        band_geometry(tab, "2024-03", 1, 0., 120.).await?,
    ];
    let day_bounds = bounds(tab, &day(START)).await?;
    let face_bounds = bounds(tab, &format!("{}-face", day(START))).await?;
    check!(
        near(day_bounds[2], 48.)
            && near(day_bounds[3], 48.)
            && near(face_bounds[2], 40.)
            && near(face_bounds[3], 40.),
        "48 dp target and 40 dp face are independent"
    );
    // Move the focus state layer away so the sampled face matches its fill.
    focus_projected(tab, &part("cancel")).await?;
    let paints = paint_start(env, tab, rows[0]).await?;

    let first = ax_tree(tab).await?;
    let dialog = ax(&first, "dialog", "選擇日期範圍")?;
    let february = ax(&first, "grid", "2024年2月")?;
    let selection = ax(&first, "gridcell", "開始日期 2024年2月24日")?;
    check_eq!(
        ax_property(february, "multiselectable"),
        Some(&json!(true)),
        "multi-select grid"
    );
    check_eq!(
        ax_property(selection, "selected"),
        Some(&json!(true)),
        "selected start"
    );
    ax(&first, "columnheader", "星期日")?;
    check_eq!(
        tab.eval(&format!("{} === null", projected(OUTSIDE)))
            .await?,
        json!(true),
        "background is outside the modal accessibility scope"
    );

    focus_projected(tab, &day(START)).await?;
    tab.press("ArrowDown").await?;
    focused(tab, &day("2024-03-02")).await?;
    let second = ax_tree(tab).await?;
    let march = ax(&second, "grid", "2024年3月")?;
    check_eq!(
        ax_property(march, "multiselectable"),
        Some(&json!(true)),
        "next month multi-select grid"
    );
    let disabled = ax(&second, "gridcell", "範圍內 2024年3月1日")?;
    check_eq!(
        ax_property(disabled, "disabled"),
        Some(&json!(true)),
        "blocked date within the range"
    );
    check_eq!(
        ax_property(disabled, "selected"),
        None,
        "blocked day cannot be an endpoint"
    );
    tab.press("ArrowDown").await?;
    focused(tab, &day("2024-03-09")).await?;
    let third = ax_tree(tab).await?;
    let end = ax(&third, "gridcell", "結束日期 2024年3月5日")?;
    check_eq!(
        ax_property(end, "selected"),
        Some(&json!(true)),
        "selected end"
    );
    tab.press("Escape").await?;
    closed(tab, OPEN, [START, END], 0).await?;
    Ok(
        json!({"weeks":rows,"day":day_bounds,"face":face_bounds,"paint":paints,
        "dialog":dialog["name"],"grid":february["properties"],"start":selection["properties"],
        "end":end["properties"],"disabled":disabled["properties"]}),
    )
}

async fn calendar_keyboard(tab: &Tab) -> Result<Value> {
    step(
        "M123",
        "keyboard navigation crosses months and years without editing the saved range",
    );
    open(tab, false).await?;
    for (key, date) in [
        ("ArrowDown", "2024-03-02"),
        ("ArrowLeft", "2024-02-29"),
        ("Shift+PageDown", "2025-02-28"),
        ("PageDown", "2025-03-29"),
    ] {
        tab.press(key).await?;
        focused(tab, &day(date)).await?;
        ready(
            tab,
            &format!(
                "d.session.active === {} && d.session.mounted_months.length <= 5 &&
             d.session.candidate[0] === {} && d.session.candidate[1] === {} && d.commits === 0",
                js(date),
                js(START),
                js(END)
            ),
        )
        .await?;
    }
    let tab_stops = tab
        .eval(&format!(
            "JSON.parse(window.gallery.export_dom()).dom.nodes
         .filter(n => n.stable_id?.startsWith({}) && n.tab_index === 0 && !n.disabled)
         .map(n => n.stable_id)",
            js(format!("{PICKER}-day-"))
        ))
        .await?;
    let last = day("2025-03-29");
    check_eq!(
        tab_stops,
        json!([last]),
        "one date Tab stop across all mounted months"
    );
    tab.press("Tab").await?;
    tab.wait_for(&format!(
        "document.activeElement?.dataset.surfaceId?.startsWith({}) && document.activeElement !== {}",
        js(PICKER), projected(&last)
    )).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &last).await?;
    let mounted = range_state(tab).await?["session"]["mounted_months"].clone();
    tab.press("Escape").await?;
    closed(tab, OPEN, [START, END], 0).await?;
    Ok(json!({"last":"2025-03-29","tab_stops":tab_stops,"mounted":mounted,"commits":0}))
}

async fn save(tab: &Tab) -> Result<()> {
    focus_projected(tab, &part("confirm")).await?;
    tab.press("Enter").await
}

async fn observe_announcements(tab: &Tab) -> Result<()> {
    // The live region can move during projection. Record its text additions
    // before the next rebuild moves or clears the element.
    tab.eval(
        "(() => {
           window.rangeAnnouncements = [];
           const region = document.querySelector('#surface-accessibility [role=dialog] [aria-live=polite]');
           new MutationObserver(records => {
             for (const record of records) {
               for (const child of record.addedNodes) {
                 if (child.textContent) window.rangeAnnouncements.push(child.textContent);
               }
             }
           }).observe(region, {childList:true});
         })()",
    )
    .await?;
    Ok(())
}

async fn pending_end_announced(tab: &Tab) -> Result<Value> {
    let summary = projected(&part("summary"));
    tab.wait_for(&format!(
        "{summary}?.textContent?.includes('尚未選擇')
          && window.rangeAnnouncements.some(text => text.includes('尚未選擇'))"
    ))
    .await?;
    tab.eval("window.rangeAnnouncements").await
}

async fn selection(tab: &Tab) -> Result<Value> {
    step(
        "M124",
        "partial, same-day and full calendar ranges commit only through Save",
    );
    open(tab, false).await?;
    observe_announcements(tab).await?;
    tab.press("Enter").await?;
    ready(tab, "d.session.candidate === null && d.session.raw_start === '2024/02/24' && d.session.raw_end === '' && d.commits === 0").await?;
    let bands = format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.filter(n => n.stable_id?.startsWith({}) && n.stable_id.endsWith('-band')).length",
        js(PICKER)
    );
    check_eq!(
        tab.eval(&bands).await?,
        json!(0),
        "provisional Start has no band"
    );
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-disabled')",
            projected(&part("confirm"))
        ))
        .await?,
        json!("true"),
        "Save is disabled for a provisional Start"
    );
    let announcements = pending_end_announced(tab).await?;
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-selected')",
            projected(&day(START))
        ))
        .await?,
        json!("true"),
        "provisional Start is selected"
    );
    focused(tab, &day(START)).await?;
    tab.press("Enter").await?;
    ready(tab, "d.session.candidate?.[0] === '2024-02-24' && d.session.candidate?.[1] === '2024-02-24' && d.commits === 0").await?;
    check_eq!(
        tab.eval(&bands).await?,
        json!(0),
        "same-day range has one endpoint face without a band"
    );
    save(tab).await?;
    closed(tab, OPEN, [START, START], 1).await?;

    open(tab, false).await?;
    tab.press("ArrowRight").await?;
    focused(tab, &day("2024-02-25")).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.session.raw_start === '2024/02/25' && d.session.raw_end === ''",
    )
    .await?;
    tab.press("ArrowLeft").await?;
    focused(tab, &day(START)).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.session.raw_start === '2024/02/24' && d.session.raw_end === ''",
    )
    .await?;
    tab.press("ArrowDown").await?;
    focused(tab, &day("2024-03-02")).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.session.candidate?.[1] === '2024-03-02' && d.commits === 1",
    )
    .await?;
    save(tab).await?;
    closed(tab, OPEN, [START, "2024-03-02"], 2).await?;
    Ok(
        json!({"same_day":[START,START],"full_range":[START,"2024-03-02"],
        "commits":2,"announcements":announcements}),
    )
}

async fn replace(tab: &Tab, endpoint: &str, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    ready(tab, &format!("d.session.raw_{endpoint} === {}", js(raw))).await
}

async fn input_help(tab: &Tab, id: &str, label: &str, words: &str) -> Result<Value> {
    tab.wait_for(&format!(
        "(() => {{
           const field = document.getElementById('surface-text-input');
           const ref = field?.getAttribute('aria-describedby');
           return ref && document.getElementById(ref) === {};
         }})()",
        projected(&part(id)),
    ))
    .await?;
    let nodes = ax_tree(tab).await?;
    let editor = ax(&nodes, "textbox", label)?;
    let description = editor["description"]["value"].as_str().unwrap_or_default();
    check!(
        description.contains(words) && description.contains("YYYY/MM/DD"),
        "Chrome describes {label} with its own help: {description}"
    );
    Ok(editor["description"].clone())
}

async fn input_errors(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M125",
        "separate date fields expose their own format hints and validation errors",
    );
    open(tab, true).await?;
    let start_hint = input_help(tab, "start-hint", "開始日期", "YYYY/MM/DD").await?;
    replace(tab, "start", "2100/02/29").await?;
    tab.press("Enter").await?;
    ready(tab, "d.session.start_error === 'InvalidDate' && d.session.end_error === null && d.commits === 0").await?;
    let start_error = input_help(tab, "start-error", "開始日期", "這個日期不存在").await?;
    tab.press("Tab").await?;
    focused(tab, &part("end-input")).await?;
    let end_hint = input_help(tab, "end-hint", "結束日期", "YYYY/MM/DD").await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &part("start-input")).await?;
    replace(tab, "start", "２０２４／３／６").await?;
    tab.press("Enter").await?;
    ready(tab, "d.session.raw_start === '2024/03/06' && d.session.pair_error === 'Reversed' && d.session.candidate === null && d.commits === 0").await?;
    tab.press("Tab").await?;
    focused(tab, &part("end-input")).await?;
    let end_error = input_help(tab, "end-error", "結束日期", "結束日期不可早於開始日期").await?;
    capture(env, tab, "input-reversed-help").await?;
    focus_projected(tab, &part("calendar-mode")).await?;
    tab.press("Enter").await?;
    ready(tab, "d.session.mode === 'Calendar' && d.session.raw_start === '2024/03/06' && d.session.raw_end === '2024/03/05' && d.session.candidate === null && d.commits === 0").await?;
    focus_projected(tab, &part("mode")).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.session.mode === 'Input' && d.session.pair_error === 'Reversed'",
    )
    .await?;
    tab.press("Escape").await?;
    closed(tab, OPEN_INPUT, [START, END], 0).await?;
    Ok(
        json!({"start_hint":start_hint,"start_error":start_error,"end_hint":end_hint,
        "end_error":end_error,"mode_preserves_raw_fields":true,"commits":0}),
    )
}

async fn composition(tab: &Tab) -> Result<Value> {
    step(
        "M126",
        "IME Enter completes text without saving and explicit Save commits once",
    );
    open(tab, true).await?;
    tab.press("Tab").await?;
    let input = part("end-input");
    focused(tab, &input).await?;
    replace(tab, "end", "2024/03/").await?;
    tab.ime_compose("０７", 2, 2).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true &&
         document.getElementById('surface-text-input').value.includes('０７')",
        js(&input)
    ))
    .await?;
    tab.press("Enter").await?;
    focused(tab, &input).await?;
    ready(tab, "d.session.mode === 'Input' && d.commits === 0").await?;
    if range_state(tab).await?["session"]["raw_end"] == "2024/03/" {
        // CDP can keep the preedit after Enter. InsertText ends that session.
        tab.insert_text("０７").await?;
    }
    ready(tab, "(d.session.raw_end === '2024/03/０７' || d.session.raw_end === '2024/03/07') && d.commits === 0").await?;
    tab.press("Enter").await?;
    focused(tab, &input).await?;
    ready(tab, "d.session.raw_end === '2024/03/07' && d.session.candidate?.[1] === '2024-03-07' && d.commits === 0").await?;
    save(tab).await?;
    closed(tab, OPEN_INPUT, [START, "2024-03-07"], 1).await?;
    Ok(
        json!({"ime_enter_commits":0,"next_enter_commits":0,"save_commits":1,
        "range":[START,"2024-03-07"]}),
    )
}

async fn scroll_offsets(tab: &Tab) -> Result<[f64; 2]> {
    let style = format!("{}.style", export_node(&part("body")));
    let offsets = tab
        .eval_f64s(&format!("([{style}.scroll_x,{style}.scroll_y])"))
        .await?;
    offsets
        .try_into()
        .map_err(|offsets| anyhow::anyhow!("expected scroll offsets: {offsets:?}"))
}

async fn touch(tab: &Tab, kind: &str, point: Option<(f64, f64)>) -> Result<()> {
    let points = point.map_or_else(|| json!([]), |(x, y)| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn short_calendar(env: &Env, tab: &Tab) -> Result<Value> {
    open(tab, false).await?;
    resize(tab, 320, 240).await?;
    let next_id = part("next-month");
    focus_projected(tab, &next_id).await?;
    let next = bounds(tab, &next_id).await?;
    let body = bounds(tab, &part("body")).await?;
    check!(
        next[0] >= -0.75
            && next[1] >= -0.75
            && next[2] >= 47.5
            && next[3] >= 47.5
            && next[0] + next[2] <= 320.75
            && next[1] + next[3] <= body[1] + 0.75
            && body[3] >= 47.5,
        "Next stays outside the short scroll body with a 48 dp target: {next:?}, {body:?}"
    );

    let mut browsed = Vec::new();
    for _ in 0..2 {
        let position = range_state(tab).await?["session"]["month"].clone();
        let year = position[0].as_u64().context("calendar year")?;
        let month = position[1].as_u64().context("calendar month")?;
        let (year, month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        tab.press("Enter").await?;
        ready(
            tab,
            &format!(
                "d.session.month[0] === {year} && d.session.month[1] === {month} &&
             d.session.candidate[0] === {} && d.session.candidate[1] === {} && d.commits === 0",
                js(START),
                js(END),
            ),
        )
        .await?;
        focused(tab, &next_id).await?;
        let measured = bounds(tab, &next_id).await?;
        check_eq!(
            measured,
            next,
            "focused Next stays visible through successive months"
        );
        browsed.push([year, month]);
    }

    let data = range_state(tab).await?;
    let active = data["session"]["active"]
        .as_str()
        .context("active date after browsing")?;
    tab.press("Tab").await?;
    focused(tab, &day(active)).await?;
    let cell = bounds(tab, &day(active)).await?;
    let summary = bounds(tab, &part("summary-text")).await?;
    check!(
        cell[0] >= body[0] - 0.75
            && cell[1] >= body[1] - 0.75
            && cell[0] + cell[2] <= body[0] + body[2] + 0.75
            && cell[1] + cell[3] <= body[1] + body[3] + 0.75,
        "the short body reveals the entire focused date: {cell:?}, {body:?}"
    );
    check!(
        summary[1] + summary[3] <= body[1] + 0.75,
        "the painted summary is completely above the scroll clip: {summary:?}, {body:?}"
    );
    observe_announcements(tab).await?;
    tab.press("Enter").await?;
    ready(
        tab,
        "d.session.candidate === null && d.session.raw_end === '' && d.commits === 0",
    )
    .await?;
    let announcements = pending_end_announced(tab).await?;
    check_eq!(
        bounds(tab, &next_id).await?,
        next,
        "editing the range keeps Next pinned in the short layout"
    );
    capture(env, tab, "compact-dark-rtl-200-short").await?;
    tab.press("Escape").await?;
    ready(tab, "d.session === null && d.commits === 0").await?;
    until(tab, &format!("s.focused === {}", js(OPEN))).await?;
    resize(tab, 320, 720).await?;
    closed(tab, OPEN, [START, END], 0).await?;
    Ok(
        json!({"next":next,"body":body,"browsed":browsed,"day":active,"cell":cell,
        "painted_summary":summary,"announcements":announcements}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M127",
        "320 dp dark RTL at 200 percent scrolls both axes and caps mounted months",
    );
    for toggle in ["theme", "rtl", "large-text"] {
        click(tab, &format!("date-range-picker-demo-{toggle}")).await?;
    }
    resize(tab, 320, 720).await?;
    until(
        tab,
        "s.dark && s.date_range_picker.rtl && s.date_range_picker.text_scale === 2",
    )
    .await?;
    open(tab, false).await?;
    let panel = bounds(tab, PICKER).await?;
    let body = point(tab, &part("body")).await?;
    check!(
        near(panel[0], 0.) && near(panel[1], 0.) && near(panel[2], 320.) && near(panel[3], 720.),
        "Range Calendar fills the compact canvas: {panel:?}"
    );
    let start = bounds(tab, &day(START)).await?;
    let same_week_first = bounds(tab, &day("2024-02-18")).await?;
    check!(
        near(start[2], 48.) && near(start[3], 48.) && near(same_week_first[0] - start[0], 288.),
        "RTL week reverses physical targets and retains 48 dp: {start:?}, {same_week_first:?}"
    );
    check!(
        start[0] >= body.b[0] - 0.75
            && start[0] + start[2] <= body.b[0] + body.b[2] + 0.75
            && start[1] >= body.b[1] - 0.75
            && start[1] + start[3] <= body.b[1] + body.b[3] + 0.75,
        "opening reveals the entire focused Saturday: {start:?} in {:?}",
        body.b
    );
    let band = band_geometry(tab, "2024-02", 3, 0., 24.).await?;
    capture(env, tab, "compact-dark-rtl-200").await?;

    let mut pinned = Vec::new();
    for id in ["cancel", "mode", "confirm"] {
        let id = part(id);
        let rect = bounds(tab, &id).await?;
        check!(
            rect[2] >= 47.5 && rect[3] >= 47.5,
            "{id} keeps a 48 dp target: {rect:?}"
        );
        pinned.push((id, rect));
    }
    tab.eval(
        "(() => {
            window.rangeInputEvents = [];
            for (const kind of ['wheel','pointermove','pointerdown','pointerup','pointercancel']) {
                document.addEventListener(kind, e => {
                    if (e.target?.id === 'surface') {
                        window.rangeInputEvents.push({kind,trusted:e.isTrusted,device:e.pointerType});
                    }
                }, {capture:true,passive:true});
            }
        })()"
    ).await?;
    let initial = scroll_offsets(tab).await?;
    let style = format!("{}.style", export_node(&part("body")));
    tab.mouse_move(body.x, body.y).await?;
    tab.wheel(160., 0.).await?;
    tab.wait_for(&format!("{style}.scroll_x >= 15.5")).await?;
    let right = scroll_offsets(tab).await?;
    tab.wheel(-160., 0.).await?;
    tab.wait_for(&format!("{style}.scroll_x <= 0.5")).await?;
    let horizontal = scroll_offsets(tab).await?;
    check!(
        right[0] - horizontal[0] >= 15.5
            && near(right[1], initial[1])
            && near(horizontal[1], initial[1]),
        "horizontal wheel reaches both RTL edges and preserves vertical position"
    );

    tab.wheel(0., 128.).await?;
    tab.wait_for(&format!("{style}.scroll_y > {}", horizontal[1] + 8.))
        .await?;
    tab.settle().await?;
    let vertical = scroll_offsets(tab).await?;
    check!(
        near(vertical[0], horizontal[0]),
        "vertical wheel preserves horizontal position"
    );

    let origin = (body.x, body.canvas[1] + body.b[1] + body.b[3] * 0.75);
    touch(tab, "touchStart", Some(origin)).await?;
    touch(tab, "touchMove", Some((origin.0, origin.1 - 35.))).await?;
    touch(tab, "touchMove", Some((origin.0, origin.1 - 100.))).await?;
    touch(tab, "touchEnd", None).await?;
    tab.wait_for(&format!("{style}.scroll_y > {}", vertical[1] + 8.))
        .await?;
    let dragged = scroll_offsets(tab).await?;
    ready(tab, "d.session.candidate?.[0] === '2024-02-24' && d.session.candidate?.[1] === '2024-03-05' && d.commits === 0").await?;

    let mut windows = Vec::new();
    for _ in 0..4 {
        let previous = range_state(tab).await?["session"]["mounted_months"].clone();
        tab.wheel(0., 1500.).await?;
        ready(
            tab,
            &format!(
                "JSON.stringify(d.session.mounted_months) !== {}",
                js(previous.to_string())
            ),
        )
        .await?;
        let mounted = range_state(tab).await?["session"]["mounted_months"].clone();
        let count = mounted.as_array().context("mounted month list")?.len();
        check!(
            (2..=5).contains(&count),
            "only nearby months are mounted: {mounted}"
        );
        windows.push(mounted);
    }
    check!(
        windows
            .last()
            .and_then(Value::as_array)
            .is_some_and(|months| !months.contains(&json!([2024, 2]))),
        "the bounded feed evicts a month far above the viewport"
    );
    for (id, original) in &pinned {
        check_eq!(
            bounds(tab, id).await?,
            *original,
            "header action stays pinned while the body scrolls"
        );
    }
    let input = tab.eval("window.rangeInputEvents").await?;
    let events = input.as_array().context("trusted input events")?;
    let trusted = |kind: &str, device: Option<&str>| {
        events.iter().any(|event| {
            event["kind"] == kind
                && event["trusted"] == true
                && device.is_none_or(|device| event["device"] == device)
        })
    };
    check!(
        trusted("wheel", None) && trusted("pointermove", Some("touch")),
        "Chrome received trusted wheel and touch motion"
    );
    focus_projected(tab, &part("cancel")).await?;
    tab.press("Enter").await?;
    closed(tab, OPEN, [START, END], 0).await?;
    let short = short_calendar(env, tab).await?;
    Ok(
        json!({"panel":panel,"body":body.b,"day":start,"band":band,"header":pinned,
        "offsets":{"initial":initial,"right":right,"left":horizontal,"vertical":vertical,"touch":dragged},
        "mounted_windows":windows,"input_events":input,"short":short}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab, url: &str) -> Result<()> {
    reload(tab, url).await?;
    let calendar = calendar_paint_and_ax(env, tab).await?;
    reload(tab, url).await?;
    let keyboard = calendar_keyboard(tab).await?;
    reload(tab, url).await?;
    let selection = selection(tab).await?;
    reload(tab, url).await?;
    let input = input_errors(env, tab).await?;
    reload(tab, url).await?;
    let composition = composition(tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "date range picker page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-date-range-picker.json",
        &json!({"browser":chrome.version,"calendar":calendar,"keyboard":keyboard,
            "selection":selection,"input":input,"composition":composition,"compact":compact,
            "errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 924, 2.).await?;
    let result = journey(env, chrome, &tab, url).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-date-range-picker-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
                "announcements":tab.eval("window.rangeAnnouncements").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-date-range-picker-failure.png"))
            .await;
    }
    result
}
