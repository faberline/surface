//! Material app bars through Chromium WebGPU paint, wheel and keyboard input.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

const BAR: &str = "app-bar-demo-bar";
const TITLE: &str = "app-bar-demo-bar-title";
const SUBTITLE: &str = "app-bar-demo-bar-subtitle";
const LEADING: &str = "app-bar-demo-bar-leading";
const ACTIONS: &str = "app-bar-demo-bar-actions";
const LEADING_BUTTON: &str = "app-bar-demo-leading";
const ACTION: &str = "app-bar-demo-action";
const DISABLED: &str = "app-bar-demo-disabled";
const SCROLL: &str = "app-bar-demo-scroll";
const FIRST_ROW: &str = "app-bar-demo-row-0";
const SEARCH: &str = "app-bar-demo-search-entry";
const SEARCH_VIEW: &str = "app-bar-demo-search-view";
const SEARCH_INPUT: &str = "app-bar-demo-search-input";
const SEARCH_CLOSE: &str = "app-bar-demo-search-close";
const SEARCH_CLEAR: &str = "app-bar-demo-search-clear";
const SEARCH_RESULT: &str = "app-bar-demo-search-result";

async fn bar(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["app_bar"].clone())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.app_bar)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, id: &str, condition: &str) -> Result<()> {
    click(tab, id).await?;
    ready(tab, condition).await
}

async fn choice(tab: &Tab, property: &str, option: &str) -> Result<()> {
    select(
        tab,
        &format!("app-bar-demo-{option}"),
        &format!("d.{property} === {}", js(option)),
    )
    .await
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await?;
    tab.wait_for(&format!("document.activeElement === {}", projected(id)))
        .await
}

async fn focus(tab: &Tab, id: &str) -> Result<()> {
    let element = projected(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    focused(tab, id).await
}

type Rect = [f64; 4];

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .as_slice()
        .try_into()
        .context("four control bounds")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn contains([x, y, w, h]: Rect, [cx, cy, cw, ch]: Rect) -> bool {
    cx >= x - 0.75 && cy >= y - 0.75 && cx + cw <= x + w + 0.75 && cy + ch <= y + h + 0.75
}

fn disjoint([x, y, w, h]: Rect, [cx, cy, cw, ch]: Rect) -> bool {
    x + w <= cx + 0.75 || cx + cw <= x + 0.75 || y + h <= cy + 0.75 || cy + ch <= y + 0.75
}

async fn expect_height(tab: &Tab, height: f64) -> Result<Rect> {
    let bounds = bounds(tab, BAR).await?;
    check!(
        near(bounds[3], height),
        "app bar height is {}, expected {height}",
        bounds[3]
    );
    Ok(bounds)
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
            return v?.width === {width} && v?.height === {height}
        }})()"
    ))
    .await?;
    tab.settle().await
}

struct Image {
    pixels: Vec<u8>,
    info: png::OutputInfo,
    channels: usize,
    scale: f64,
    canvas: [f64; 4],
}

impl Image {
    async fn capture(env: &Env, tab: &Tab, label: &str) -> Result<Self> {
        tab.settle().await?;
        let path = env.artifact(&format!("material-app-bar-{label}.png"));
        tab.screenshot(&path).await?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let info = reader.next_frame(&mut pixels)?;
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG color type {other:?}"),
        };
        let window_width = tab
            .eval("innerWidth")
            .await?
            .as_f64()
            .context("window width")?;
        let scale = f64::from(info.width) / window_width;
        let canvas = tab.canvas_box().await?;
        Ok(Self {
            pixels,
            info,
            channels,
            scale,
            canvas,
        })
    }

    fn pixel(&self, x: f64, y: f64) -> Result<[u8; 3]> {
        let x = ((x + self.canvas[0]) * self.scale).floor() as usize;
        let y = ((y + self.canvas[1]) * self.scale).floor() as usize;
        if x >= self.info.width as usize || y >= self.info.height as usize {
            bail!("WebGPU probe is outside the screenshot");
        }
        let offset = y * self.info.line_size + x * self.channels;
        Ok(self.pixels[offset..offset + 3].try_into()?)
    }

    fn ink_bounds(&self, rect: Rect, foreground: [u8; 3]) -> Result<(f64, f64)> {
        let [x, y, w, h] = rect;
        let mut first = None;
        let mut last = None;
        for column in x.ceil() as u32..(x + w).floor() as u32 {
            for row in y.ceil() as u32..(y + h).floor() as u32 {
                if similar(self.pixel(f64::from(column), f64::from(row))?, foreground) {
                    first.get_or_insert(column);
                    last = Some(column);
                }
            }
        }
        Ok((
            f64::from(first.context("no foreground pixels")?),
            f64::from(last.context("no foreground pixels")?),
        ))
    }
}

fn rgb(value: &Value) -> Result<[u8; 3]> {
    let [r, g, b, _]: [u8; 4] =
        serde_json::from_value(value.clone()).context("RGBA color token")?;
    Ok([r, g, b])
}

fn similar(a: [u8; 3], b: [u8; 3]) -> bool {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum::<u16>()
        <= 8
}

async fn surface(env: &Env, tab: &Tab, label: &str, token: &str) -> Result<[u8; 3]> {
    // The semantic tone changes before the short paint transition ends.
    tokio::time::sleep(Duration::from_millis(250)).await;
    let [x, y, _, h] = bounds(tab, BAR).await?;
    let image = Image::capture(env, tab, label).await?;
    let pixel = image.pixel(x + 3., y + h - 4.)?;
    let expected = rgb(&bar(tab).await?[token])?;
    check!(
        similar(pixel, expected),
        "WebGPU bar color is {pixel:?}, expected {expected:?}"
    );
    Ok(pixel)
}

async fn variants(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M56",
        "all top app bar variants use the intended dimensions and surface colors",
    );
    ready(
        tab,
        "d.kind === 'small' && d.behavior === 'pinned' && d.collapse === 0",
    )
    .await?;
    let small = expect_height(tab, 64.).await?;
    let light = surface(env, tab, "small-light", "surface").await?;

    choice(tab, "kind", "medium").await?;
    let medium = expect_height(tab, 136.).await?;
    let medium_title = bounds(tab, TITLE).await?;
    let medium_subtitle = bounds(tab, SUBTITLE).await?;
    check!(
        contains(medium, medium_title)
            && contains(medium, medium_subtitle)
            && medium_title[1] + medium_title[3] <= medium_subtitle[1] + 0.75,
        "medium title and subtitle fit in separate lines"
    );
    Image::capture(env, tab, "medium-light").await?;
    click(tab, "app-bar-demo-subtitle").await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(SUBTITLE)
    ))
    .await?;
    tab.settle().await?;
    let medium_only = expect_height(tab, 112.).await?;
    choice(tab, "kind", "large").await?;
    let large_only = expect_height(tab, 120.).await?;
    click(tab, "app-bar-demo-subtitle").await?;
    let large_title = bounds(tab, TITLE).await?;
    let large_subtitle = bounds(tab, SUBTITLE).await?;
    let large = expect_height(tab, 152.).await?;
    check!(
        contains(large, large_title)
            && contains(large, large_subtitle)
            && large_title[1] + large_title[3] <= large_subtitle[1] + 0.75,
        "large title and subtitle fit in separate lines"
    );
    click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    let dark = surface(env, tab, "large-dark", "surface").await?;
    check!(
        !similar(light, dark),
        "light and dark bars paint different colors"
    );
    click(tab, "theme").await?;
    until(tab, "!s.dark").await?;
    Ok(
        json!({"small":small,"medium":medium,"medium_title_only":medium_only,
        "large":large,"large_title_only":large_only,"light":light,"dark":dark}),
    )
}

async fn centered(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M57",
        "center title stays centered with asymmetric actions, including RTL and keyboard",
    );
    choice(tab, "kind", "center").await?;
    let mut layouts = Vec::new();
    for rtl in [false, true] {
        if rtl {
            select(tab, "app-bar-demo-rtl", "d.rtl").await?;
        }
        let bar = expect_height(tab, 64.).await?;
        let title = bounds(tab, TITLE).await?;
        let leading = bounds(tab, LEADING).await?;
        let actions = bounds(tab, ACTIONS).await?;
        let lead_control = bounds(tab, LEADING_BUTTON).await?;
        let action = bounds(tab, ACTION).await?;
        let disabled = bounds(tab, DISABLED).await?;
        check!(
            action[2] + disabled[2] > lead_control[2] + 8.,
            "actual leading and action controls are asymmetric"
        );
        check!(
            if rtl {
                leading[0] > actions[0]
            } else {
                leading[0] < actions[0]
            },
            "the leading and trailing sides mirror in RTL"
        );
        check!(
            contains(bar, title) && disjoint(title, leading) && disjoint(title, actions),
            "center title does not overlap the icons"
        );
        let export = dom(tab).await?;
        let title_node = find_node(&export, TITLE)?;
        check_eq!(title_node["text"], json!("工作區總覽"), "sample title");
        let foreground = rgb(&title_node["style"]["foreground"])?;
        let image = Image::capture(env, tab, if rtl { "center-rtl" } else { "center-ltr" }).await?;
        let (first, last) = image.ink_bounds(title, foreground)?;
        let ink_center = (first + last) / 2.;
        check!(
            (ink_center - (bar[0] + bar[2] / 2.)).abs() <= 4.,
            "painted title centers on the whole bar: {ink_center}"
        );
        layouts.push(
            json!({"rtl":rtl,"title":title,"leading":leading,"actions":actions,"ink":[first,last]}),
        );
    }

    tab.wait_for(&format!(
        "{}?.getAttribute('aria-disabled') === 'true'",
        projected(DISABLED)
    ))
    .await?;
    let before = bar(tab).await?["actions"]
        .as_u64()
        .context("action count")?;
    click(tab, DISABLED).await?;
    tab.settle().await?;
    check_eq!(
        bar(tab).await?["actions"],
        json!(before),
        "disabled button ignores pointer press"
    );
    focus(tab, LEADING_BUTTON).await?;
    tab.press("Tab").await?;
    focused(tab, ACTION).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.actions === {}", before + 1)).await?;
    tab.press("Tab").await?;
    tab.settle().await?;
    check!(
        state(tab).await?["focused"] != DISABLED,
        "disabled action is omitted from Tab order"
    );
    select(tab, "app-bar-demo-rtl", "!d.rtl").await?;
    Ok(json!(layouts))
}

async fn wheel(tab: &Tab, dy: f64) -> Result<()> {
    let scroll = point(tab, SCROLL).await?;
    tab.mouse_move(scroll.x, scroll.y).await?;
    // At the emulated DPR, CDP wheel units are physical; the app uses dp.
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("device scale")?;
    tab.wheel(0., dy * scale).await?;
    tab.settle().await
}

async fn offset(tab: &Tab, value: f64) -> Result<()> {
    ready(tab, &format!("Math.abs(d.body_offset - {value}) <= 0.75")).await
}

async fn relative_row(tab: &Tab) -> Result<f64> {
    Ok(bounds(tab, FIRST_ROW).await?[1] - bounds(tab, SCROLL).await?[1])
}

async fn expect_scroll(tab: &Tab, initial: f64, expected: f64) -> Result<Value> {
    offset(tab, expected).await?;
    let relative = relative_row(tab).await?;
    check!(
        near(initial - relative, expected),
        "real row layout moves by {expected} dp"
    );
    let export = dom(tab).await?;
    let node = find_node(&export, SCROLL)?;
    let exported = node["style"]["scroll_y"]
        .as_f64()
        .context("exported scroll offset")?;
    check!(
        near(exported, expected),
        "DOM scroll offset is {exported}, expected {expected}"
    );
    Ok(
        json!({"body_offset":exported,"row_relative_y":relative,"collapse":bar(tab).await?["collapse"]}),
    )
}

async fn scroll_modes(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M58",
        "pinned and enter-always bars forward exactly the unused wheel delta",
    );
    choice(tab, "kind", "large").await?;
    wheel(tab, -900.).await?;
    offset(tab, 0.).await?;
    let expanded = expect_height(tab, 152.).await?;
    let initial = relative_row(tab).await?;
    let normal = surface(env, tab, "pinned-top", "surface").await?;
    wheel(tab, 80.).await?;
    let pinned = expect_scroll(tab, initial, 80.).await?;
    check_eq!(
        bar(tab).await?["collapse"],
        json!(0.0),
        "pinned bar does not collapse"
    );
    expect_height(tab, 152.).await?;
    ready(tab, "d.scrolled").await?;
    let raised = surface(env, tab, "pinned-scrolled", "scrolled_surface").await?;
    check!(
        !similar(normal, raised),
        "scroll changes the painted tonal surface"
    );
    wheel(tab, -600.).await?;
    expect_scroll(tab, initial, 0.).await?;
    ready(tab, "!d.scrolled").await?;

    choice(tab, "behavior", "enter-always").await?;
    let initial = relative_row(tab).await?;
    let collapse_range = expanded[3] - 64.;
    wheel(tab, collapse_range + 84.).await?;
    ready(tab, "d.collapse === 1").await?;
    let entered = expect_scroll(tab, initial, 84.).await?;
    let collapsed = expect_height(tab, 64.).await?;
    let title = bounds(tab, TITLE).await?;
    let subtitle = bounds(tab, SUBTITLE).await?;
    check!(
        contains(collapsed, title) && contains(collapsed, subtitle) && disjoint(title, subtitle),
        "collapsed bar keeps its title and subtitle in view"
    );
    Image::capture(env, tab, "enter-always-collapsed").await?;
    wheel(tab, -24.).await?;
    ready(tab, "d.collapse > 0 && d.collapse < 1").await?;
    let entering = expect_scroll(tab, initial, 84.).await?;
    expect_height(tab, collapsed[3] + 24.).await?;
    wheel(tab, -900.).await?;
    expect_scroll(tab, initial, 0.).await?;
    ready(tab, "d.collapse === 0").await?;

    step(
        "M59",
        "exit-until-collapsed drains content before expanding and forwards the remainder",
    );
    choice(tab, "behavior", "exit-until-collapsed").await?;
    let initial = relative_row(tab).await?;
    wheel(tab, collapse_range + 84.).await?;
    ready(tab, "d.collapse === 1").await?;
    expect_scroll(tab, initial, 84.).await?;
    wheel(tab, -30.).await?;
    let body_first = expect_scroll(tab, initial, 54.).await?;
    check_eq!(
        bar(tab).await?["collapse"],
        json!(1.0),
        "bar stays collapsed while the body can scroll back"
    );
    expect_height(tab, 64.).await?;
    wheel(tab, -80.).await?;
    let remainder = expect_scroll(tab, initial, 0.).await?;
    ready(tab, "d.collapse > 0 && d.collapse < 1").await?;
    expect_height(tab, 90.).await?;
    Image::capture(env, tab, "exit-until-collapsed-return").await?;
    wheel(tab, -600.).await?;
    expect_scroll(tab, initial, 0.).await?;
    ready(tab, "d.collapse === 0").await?;
    expect_height(tab, 152.).await?;
    Ok(
        json!({"normal":normal,"raised":raised,"pinned":pinned,"enter_always":[entered,entering],
        "exit_until_collapsed":[body_first,remainder]}),
    )
}

async fn open_search(tab: &Tab) -> Result<()> {
    focus(tab, SEARCH).await?;
    tab.press("Enter").await?;
    ready(tab, "d.search_open").await?;
    until(tab, &format!("s.focused === {}", js(SEARCH_INPUT))).await?;
    tab.wait_for(&format!(
        "document.activeElement?.id === 'surface-text-input' && {}?.getAttribute('aria-modal') === 'true' && {} === null",
        projected(SEARCH_VIEW), projected(SEARCH)
    )).await
}

async fn search(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M60",
        "search owns focus, accepts actual text and a result, and restores its entry",
    );
    choice(tab, "kind", "search").await?;
    let entry = bounds(tab, SEARCH).await?;
    check!(
        contains(bounds(tab, BAR).await?, entry),
        "search entry fits the bar"
    );
    open_search(tab).await?;
    let initial = dom(tab).await?;
    check_eq!(
        find_node(&initial, SEARCH_RESULT)?["name"],
        json!("設計紀錄"),
        "initial search result"
    );
    tab.insert_text("找不到").await?;
    ready(tab, "d.query === '找不到'").await?;
    let empty = dom(tab).await?;
    check!(
        find_node(&empty, SEARCH_RESULT).is_err()
            && find_node(&empty, "app-bar-demo-search-empty").is_ok(),
        "a real input query can leave no matched results"
    );
    focus(tab, SEARCH_CLEAR).await?;
    tab.press("Enter").await?;
    ready(tab, "d.query === ''").await?;
    until(tab, &format!("s.focused === {}", js(SEARCH_INPUT))).await?;
    tab.wait_for("document.activeElement?.id === 'surface-text-input'")
        .await?;
    let query = "設計";
    tab.insert_text(query).await?;
    ready(tab, &format!("d.search_open && d.query === {}", js(query))).await?;
    let found = dom(tab).await?;
    check!(
        find_node(&found, SEARCH_RESULT).is_ok()
            && find_node(&found, "app-bar-demo-search-result-tests").is_err(),
        "only the matching result stays in the view"
    );
    Image::capture(env, tab, "search-input").await?;
    tab.press("Escape").await?;
    ready(tab, "!d.search_open").await?;
    focused(tab, SEARCH).await?;

    open_search(tab).await?;
    let before = bar(tab).await?["search_selections"]
        .as_u64()
        .context("search selection count")?;
    focus(tab, SEARCH_RESULT).await?;
    tab.press("Enter").await?;
    ready(tab, &format!("d.search_selections === {}", before + 1)).await?;
    ready(tab, "!d.search_open && d.selected === '設計紀錄'").await?;
    focused(tab, SEARCH).await?;
    open_search(tab).await?;
    focus(tab, SEARCH_CLOSE).await?;
    tab.press("Enter").await?;
    ready(tab, "!d.search_open").await?;
    focused(tab, SEARCH).await?;
    Ok(json!({"entry":entry,"query":query,"after_selection":bar(tab).await?}))
}

async fn compact_choice(tab: &Tab, property: &str, option: &str) -> Result<()> {
    let id = format!("app-bar-demo-{option}");
    let control = bounds(tab, &id).await?;
    let clip = bounds(tab, SCROLL).await?;
    if !contains(clip, control) {
        let control_center = control[1] + control[3] / 2.;
        let clip_center = clip[1] + clip[3] / 2.;
        wheel(tab, control_center - clip_center).await?;
    }
    check!(
        contains(bounds(tab, SCROLL).await?, bounds(tab, &id).await?),
        "the compact scroll view reveals {id}"
    );
    choice(tab, property, option).await
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M61",
        "320 dp dark RTL app bars fit controls and paint text at 200%",
    );
    select(tab, "app-bar-demo-scale", "d.text_scale === 2").await?;
    select(tab, "app-bar-demo-rtl", "d.rtl").await?;
    click(tab, "theme").await?;
    until(tab, "s.dark").await?;
    resize(tab, 320, 580).await?;
    compact_choice(tab, "behavior", "pinned").await?;
    let mut layouts = Vec::new();
    for kind in ["small", "center", "medium", "large"] {
        compact_choice(tab, "kind", kind).await?;
        let bar = bounds(tab, BAR).await?;
        let title = bounds(tab, TITLE).await?;
        let leading = bounds(tab, LEADING_BUTTON).await?;
        let action = bounds(tab, ACTION).await?;
        let disabled = bounds(tab, DISABLED).await?;
        for item in [title, leading, action, disabled] {
            check!(
                item[2] > 0. && item[3] > 0. && contains(bar, item),
                "{kind} control fits 320 dp: {item:?}"
            );
        }
        for control in [leading, action, disabled] {
            check!(
                control[2] >= 48. && control[3] >= 48.,
                "{kind} icon retains a 48 dp target"
            );
            check!(
                disjoint(title, control),
                "{kind} text and icon do not overlap"
            );
        }
        let export = dom(tab).await?;
        let title_node = find_node(&export, TITLE)?;
        let authored_height = title_node["style"]["line_height"]
            .as_f64()
            .context("title line height")?;
        check!(
            title[3] >= authored_height * 2. - 0.75,
            "{kind} provides vertical space for 200% title"
        );
        let image = Image::capture(env, tab, &format!("compact-dark-{kind}")).await?;
        let ink = image.ink_bounds(title, rgb(&title_node["style"]["foreground"])?)?;
        check!(
            ink.0 >= title[0] - 0.75 && ink.1 <= title[0] + title[2] + 0.75,
            "{kind} paints scaled CJK inside its title"
        );
        let subtitle = if matches!(kind, "medium" | "large") {
            let subtitle = bounds(tab, SUBTITLE).await?;
            check!(
                contains(bar, subtitle) && disjoint(title, subtitle),
                "scaled {kind} subtitle fits its own row"
            );
            Some(subtitle)
        } else {
            None
        };
        layouts.push(
            json!({"kind":kind,"bar":bar,"title":title,"subtitle":subtitle,"ink":[ink.0,ink.1]}),
        );
    }
    compact_choice(tab, "kind", "search").await?;
    check!(
        contains(bounds(tab, BAR).await?, bounds(tab, SEARCH).await?),
        "scaled search entry fits"
    );
    open_search(tab).await?;
    let view = bounds(tab, SEARCH_VIEW).await?;
    let canvas = tab.canvas_box().await?;
    check!(
        contains([0., 0., canvas[2], canvas[3]], view),
        "scaled search modal fits the viewport"
    );
    for id in [SEARCH_INPUT, SEARCH_CLOSE, SEARCH_RESULT] {
        check!(
            contains(view, bounds(tab, id).await?),
            "{id} fits the scaled search modal"
        );
    }
    Image::capture(env, tab, "compact-dark-search").await?;
    tab.press("Escape").await?;
    ready(tab, "!d.search_open").await?;
    focused(tab, SEARCH).await?;
    Ok(json!({"variants":layouts,"search_view":view}))
}

async fn contextual(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M62",
        "context actions replace the app bar and return focus to the selected row",
    );
    const ROW: &str = "app-bar-demo-context";
    resize(tab, 960, 760).await?;
    choice(tab, "kind", "small").await?;
    wheel(tab, -500.).await?;
    offset(tab, 0.).await?;
    let mut callbacks = Vec::new();
    for control in ["app-bar-demo-context-close", "app-bar-demo-context-action"] {
        focus(tab, ROW).await?;
        tab.press("Enter").await?;
        ready(tab, "d.contextual").await?;
        let export = dom(tab).await?;
        check_eq!(
            find_node(&export, TITLE)?["text"],
            json!("已選取 1 項"),
            "selected row changes the app bar title"
        );
        check!(
            find_node(&export, ACTION).is_err(),
            "normal actions leave the context bar"
        );
        if callbacks.is_empty() {
            Image::capture(env, tab, "contextual-selection").await?;
        }
        focus(tab, control).await?;
        tab.press("Enter").await?;
        ready(tab, "!d.contextual").await?;
        focused(tab, ROW).await?;
        let restored = dom(tab).await?;
        check_eq!(
            find_node(&restored, TITLE)?["text"],
            json!("工作區總覽"),
            "normal title returns"
        );
        check!(
            find_node(&restored, ACTION).is_ok(),
            "normal actions return"
        );
        callbacks.push(control);
    }
    Ok(json!({"callbacks":callbacks,"focus":ROW}))
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 760).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, "app-bar-demo-show").await?;
    let variants = variants(env, tab).await?;
    let centered = centered(env, tab).await?;
    let scroll = scroll_modes(env, tab).await?;
    let search = search(env, tab).await?;
    let compact = compact(env, tab).await?;
    let contextual = contextual(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "app bar page errors: {errors:?}");
    env.write_json(
        "browser-material-app-bar.json",
        &json!({
            "browser":chrome.version,"variants":variants,"centered":centered,
        "scroll":scroll,"search":search,"compact":compact,"contextual":contextual,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 804, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, &tab).await;
    if let Err(error) = &result {
        let dump = json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":dom(&tab).await.ok(),"active":tab.eval("document.activeElement?.outerHTML").await.ok(),
            "errors":tab.errors()});
        let _ = env.write_json("material-app-bar-failure.json", &dump);
        let _ = tab
            .screenshot(&env.artifact("material-app-bar-failure.png"))
            .await;
    }
    result
}
