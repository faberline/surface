//! The floating and anchored Material 3 FAB menu in headless Chrome.

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs::File, io::BufReader};

const TRIGGER: &str = "fab-menu-demo-trigger";
const ANCHOR: &str = "fab-menu-demo";
const FACE: &str = "fab-menu-demo-trigger-face";
const ICON: &str = "fab-menu-demo-trigger-icon";
const PANEL: &str = "fab-menu-demo-menu";
const LIST: &str = "fab-menu-demo-list";
const OUTSIDE: &str = "fab-menu-demo-outside";
const OPENER_NAME: &str = "新增項目";
const CLOSER_NAME: &str = "關閉動作選單";
const MENU_NAME: &str = "動作選單";
const LABELS: [&str; 6] = [
    "筆記",
    "任務",
    "會議紀錄",
    "匯入專案參考資料",
    "提醒",
    "工作區捷徑",
];
const SIZES: [(&str, f64, f64, f64); 3] = [
    ("regular", 56., 16., 24.),
    ("medium", 80., 20., 28.),
    ("large", 96., 28., 36.),
];
const PALETTES: [&str; 3] = ["primary", "secondary", "tertiary"];
const ENABLED: [usize; 5] = [0, 1, 3, 4, 5];

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn item(index: usize) -> String {
    format!("fab-menu-demo-item-{index}")
}

fn part(index: usize, component: &str) -> String {
    format!("{}-{component}", item(index))
}

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["fab_menu"].clone())
}

async fn ready(tab: &Tab, predicate: &str) -> Result<()> {
    until(tab, &format!("(d => {predicate})(s.fab_menu)")).await?;
    tab.settle().await
}

async fn select(tab: &Tab, suffix: &str, predicate: &str) -> Result<()> {
    click(tab, &format!("fab-menu-demo-{suffix}")).await?;
    ready(tab, predicate).await
}

async fn choose(tab: &Tab, suffix: &str, value: &str) -> Result<()> {
    select(
        tab,
        &format!("{suffix}-{value}"),
        &format!("d.{suffix} === {}", js(value)),
    )
    .await
}

async fn theme(tab: &Tab, dark: bool) -> Result<()> {
    if (state(tab).await?["dark"] == true) != dark {
        click(tab, "fab-menu-demo-theme").await?;
    }
    until(tab, &format!("s.dark === {dark}")).await?;
    tab.settle().await
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

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 1100, 880).await?;
    click(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    click(tab, "fab-menu-demo-show").await?;
    bounds(tab, TRIGGER).await?;
    tab.settle().await
}

async fn bounds(tab: &Tab, id: &str) -> Result<Rect> {
    point(tab, id)
        .await?
        .b
        .as_slice()
        .try_into()
        .with_context(|| format!("expected four coordinates for {id}"))
}

async fn dom(tab: &Tab) -> Result<Value> {
    tab.eval_json("window.gallery.export_dom()").await
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.75
}

fn same(a: Rect, b: Rect) -> bool {
    a.into_iter().zip(b).all(|(a, b)| near(a, b))
}

fn bottom(r: Rect) -> f64 {
    r[1] + r[3]
}

fn end(r: Rect) -> f64 {
    r[0] + r[2]
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner[0] >= outer[0] - 0.75
        && inner[1] >= outer[1] - 0.75
        && end(inner) <= end(outer) + 0.75
        && bottom(inner) <= bottom(outer) + 0.75
}

fn target(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

async fn runtime_focus(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    runtime_focus(tab, id).await?;
    tab.wait_for(&format!("document.activeElement === {}", target(id)))
        .await
}

async fn focus(tab: &Tab, id: &str) -> Result<()> {
    let node = target(id);
    tab.wait_for(&format!("{node} !== null")).await?;
    tab.eval(&format!("{node}.focus()")).await?;
    focused(tab, id).await
}

async fn absent(tab: &Tab, id: &str) -> Result<()> {
    tab.wait_for(&format!("{} === null", target(id))).await
}

async fn accessibility(tab: &Tab, role: &str, label: &str) -> Result<Value> {
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let mut nodes = tree["nodes"]
        .as_array()
        .context("Chrome AX nodes")?
        .iter()
        .filter(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"] == label
        });
    let first = nodes
        .next()
        .with_context(|| format!("Chrome AX has a named {role} {label}"))?;
    check!(
        nodes.next().is_none(),
        "Chrome AX has a single {role} named {label}"
    );
    Ok(first.clone())
}

fn ax_property<'a>(node: &'a Value, property: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|p| p["name"] == property)
        .map(|p| &p["value"]["value"])
}

async fn expanded(tab: &Tab, is_open: bool, web: bool) -> Result<()> {
    let node = target(TRIGGER);
    let popup = if web {
        format!(" && {node}.getAttribute('aria-haspopup') === 'menu'")
    } else {
        String::new()
    };
    tab.wait_for(&format!(
        "{node}?.getAttribute('aria-expanded') === '{is_open}'{popup}"
    ))
    .await
}

async fn open_key(tab: &Tab, first: &str) -> Result<()> {
    focus(tab, TRIGGER).await?;
    tab.press("Enter").await?;
    expanded(tab, true, false).await?;
    focused(tab, first).await
}

async fn close_key(tab: &Tab) -> Result<()> {
    tab.press("Escape").await?;
    expanded(tab, false, false).await?;
    focused(tab, TRIGGER).await
}

async fn reduced(tab: &Tab, enabled: bool) -> Result<()> {
    let value = if enabled { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":value}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {value})"))
    ))
    .await?;
    ready(tab, &format!("d.reduced_motion === {enabled}")).await
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    // Only elapsed time is controlled. The input and paint are real Chrome frames.
    tab.eval("(() => { const start = Math.ceil(performance.now()); window.fabMenuTime = 0;
          Object.defineProperty(performance,'now',{configurable:true,value:() => start + window.fabMenuTime}); })()").await?;
    Ok(())
}

async fn advance(tab: &Tab, time: u32) -> Result<()> {
    tab.eval(&format!("window.fabMenuTime = {time}")).await?;
    tab.settle().await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-fab-menu-{name}.png")))
        .await
}

struct Image {
    data: Vec<u8>,
    info: png::OutputInfo,
    channels: usize,
    scale: f64,
    canvas: Rect,
}

impl Image {
    async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Self> {
        capture(env, tab, name).await?;
        let decoder = png::Decoder::new(BufReader::new(File::open(
            env.artifact(&format!("material-fab-menu-{name}.png")),
        )?));
        let mut reader = decoder.read_info()?;
        let mut data = vec![0; reader.output_buffer_size().context("PNG buffer size")?];
        let info = reader.next_frame(&mut data)?;
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG type {other:?}"),
        };
        let width = tab
            .eval("innerWidth")
            .await?
            .as_f64()
            .context("Chrome viewport width")?;
        Ok(Self {
            data,
            channels,
            scale: f64::from(info.width) / width,
            info,
            canvas: tab.canvas_box().await?,
        })
    }

    fn at(&self, x: f64, y: f64) -> Result<Rgb> {
        let x = ((x + self.canvas[0]) * self.scale).floor() as usize;
        let y = ((y + self.canvas[1]) * self.scale).floor() as usize;
        if x >= self.info.width as usize || y >= self.info.height as usize {
            bail!("pixel outside Chrome screenshot: {x},{y}")
        }
        let i = y * self.info.line_size + x * self.channels;
        Ok(self.data[i..i + 3].try_into()?)
    }

    fn ink_in(&self, [x, y, width, height]: Rect, ink: Rgb) -> Result<usize> {
        let mut count = 0;
        for row in 0..height.floor() as u32 {
            for column in 0..width.floor() as u32 {
                let pixel = self.at(x + f64::from(column) + 0.5, y + f64::from(row) + 0.5)?;
                count += usize::from(difference(pixel, ink) <= 28);
            }
        }
        Ok(count)
    }
}

fn rgb(value: &Value) -> Result<Rgb> {
    let [r, g, b, _]: [u8; 4] = serde_json::from_value(value.clone()).context("RGBA color")?;
    Ok([r, g, b])
}

fn difference(a: Rgb, b: Rgb) -> u16 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn has_corner(value: &Value, radius: f64) -> bool {
    ["top_left", "top_right", "bottom_left", "bottom_right"]
        .into_iter()
        .all(|side| value[side].as_f64().is_some_and(|r| near(r, radius)))
}

async fn touch_enabled(tab: &Tab, enabled: bool) -> Result<()> {
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":enabled,"maxTouchPoints":1}),
    )
    .await?;
    Ok(())
}

async fn tap(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":p.x,"y":p.y,"id":1}]}),
    )
    .await?;
    tab.settle().await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchEnd","touchPoints":[]}),
    )
    .await?;
    tab.settle().await
}

async fn presentation(tab: &Tab, mode: &str) -> Result<()> {
    select(tab, mode, &format!("d.presentation === {}", js(mode))).await
}

async fn item_count(tab: &Tab, n: usize) -> Result<()> {
    select(
        tab,
        if n == 2 { "count-two" } else { "count-six" },
        &format!("d.count === {n}"),
    )
    .await
}

async fn actions(tab: &Tab) -> Result<[u64; 6]> {
    serde_json::from_value(data(tab).await?["actions"].clone())
        .context("six FAB menu action counts")
}

async fn disclosure(tab: &Tab, is_open: bool) -> Result<Value> {
    expanded(tab, is_open, true).await?;
    let node = accessibility(tab, "button", OPENER_NAME).await?;
    check_eq!(
        ax_property(&node, "expanded"),
        Some(&json!(is_open)),
        "Chrome exposes the menu button expanded state"
    );
    check_eq!(
        ax_property(&node, "hasPopup"),
        Some(&json!("menu")),
        "Chrome exposes a menu popup on the FAB"
    );
    Ok(node)
}

async fn web_geometry(env: &Env, tab: &Tab) -> Result<Value> {
    step("M106","Web keeps one stationary menu button, flips the baseline menu, and paints the surface in light and dark");
    ready(tab, "d.presentation === 'web' && !d.open && d.count === 2").await?;
    disclosure(tab, false).await?;
    let anchor = bounds(tab, FACE).await?;
    open_key(tab, &item(0)).await?;
    let button_ax = disclosure(tab, true).await?;
    let menu_ax = accessibility(tab, "menu", MENU_NAME).await?;
    accessibility(tab, "menuitem", LABELS[0]).await?;
    accessibility(tab, "menuitem", LABELS[1]).await?;
    check!(
        same(anchor, bounds(tab, FACE).await?),
        "Web menu keeps the closed FAB pose"
    );
    let above = bounds(tab, PANEL).await?;
    check!(
        contains([0., 0., 1100., 880.], above) && near(anchor[1] - bottom(above), 4.),
        "near the bottom, the actual menu flips above and leaves a 4 dp gap"
    );
    let style = dom(tab).await?;
    let light_color = rgb(&find_node(&style, PANEL)?["style"]["background"])?;
    let light = Image::capture(env, tab, "web-light-above").await?;
    let light_pixel = light.at(above[0] + above[2] / 2., above[1] + 4.)?;
    check!(
        difference(light_pixel, light_color) <= 10,
        "light baseline popup paint matches its surface fill"
    );
    close_key(tab).await?;
    absent(tab, PANEL).await?;

    theme(tab, true).await?;
    select(tab, "top", "d.top").await?;
    let top_anchor = bounds(tab, FACE).await?;
    open_key(tab, &item(0)).await?;
    let below = bounds(tab, PANEL).await?;
    check!(
        contains([0., 0., 1100., 880.], below) && near(below[1] - bottom(top_anchor), 4.),
        "near the top, actual menu opens below with the same 4 dp gap"
    );
    let style = dom(tab).await?;
    let dark_color = rgb(&find_node(&style, PANEL)?["style"]["background"])?;
    let dark = Image::capture(env, tab, "web-dark-below").await?;
    let dark_pixel = dark.at(below[0] + below[2] / 2., below[1] + 4.)?;
    check!(
        difference(dark_pixel, dark_color) <= 10 && difference(light_pixel, dark_pixel) > 120,
        "dark baseline menu uses a distinctly darker painted surface"
    );
    close_key(tab).await?;
    Ok(
        json!({"above":above,"below":below,"anchors":[anchor,top_anchor],
        "light":light_pixel,"dark":dark_pixel,"trigger_ax":button_ax,"menu_ax":menu_ax}),
    )
}

async fn web_interaction(env: &Env, tab: &Tab) -> Result<Value> {
    step("M107","Web arrows skip disabled items and dismissal restores or advances focus across keyboard and outside input");
    item_count(tab, 6).await?;
    let initial = actions(tab).await?;
    focus(tab, TRIGGER).await?;
    tab.press("Shift+Tab").await?;
    tab.settle().await?;
    let previous = state(tab).await?["focused"]
        .as_str()
        .context("closed Tab predecessor")?
        .to_owned();
    focused(tab, &previous).await?;
    focus(tab, TRIGGER).await?;
    tab.press("ArrowDown").await?;
    expanded(tab, true, true).await?;
    focused(tab, &item(0)).await?;
    let disabled = target(&item(2));
    tab.wait_for(&format!(
        "{disabled}?.getAttribute('aria-disabled') === 'true'"
    ))
    .await?;
    for (key, index) in [
        ("ArrowDown", 1),
        ("ArrowDown", 3),
        ("ArrowUp", 1),
        ("End", 5),
        ("Home", 0),
    ] {
        tab.press(key).await?;
        focused(tab, &item(index)).await?;
    }
    tab.press("Tab").await?;
    expanded(tab, false, true).await?;
    focused(tab, OUTSIDE).await?;
    absent(tab, PANEL).await?;
    check_eq!(
        actions(tab).await?,
        initial,
        "forward Tab closes without selecting a menu item"
    );

    open_key(tab, &item(0)).await?;
    tab.press("Shift+Tab").await?;
    expanded(tab, false, true).await?;
    focused(tab, &previous).await?;
    absent(tab, PANEL).await?;
    check_eq!(
        actions(tab).await?,
        initial,
        "backward Tab continues to the original previous control"
    );

    open_key(tab, &item(0)).await?;
    tab.press("Enter").await?;
    expanded(tab, false, true).await?;
    focused(tab, TRIGGER).await?;
    let mut expected = initial;
    expected[0] += 1;
    check_eq!(
        actions(tab).await?,
        expected,
        "keyboard selects the named action exactly once"
    );
    let outside_before = data(tab).await?["outside_actions"].clone();

    click(tab, TRIGGER).await?;
    expanded(tab, true, true).await?;
    runtime_focus(tab, &item(0)).await?;
    absent(tab, OUTSIDE).await?;
    click(tab, OUTSIDE).await?;
    expanded(tab, false, true).await?;
    runtime_focus(tab, TRIGGER).await?;
    check_eq!(
        data(tab).await?["outside_actions"].clone(),
        outside_before,
        "outside mouse input closes before routing a covered action"
    );

    open_key(tab, &item(0)).await?;
    touch_enabled(tab, true).await?;
    tab.eval("document.addEventListener('pointerdown', event => {
       if (event.target?.id === 'surface' && event.pointerType === 'touch') window.fabMenuTrusted = event.isTrusted;
    },{capture:true,passive:true})").await?;
    tap(tab, &item(1)).await?;
    expanded(tab, false, true).await?;
    runtime_focus(tab, TRIGGER).await?;
    expected[1] += 1;
    check_eq!(
        actions(tab).await?,
        expected,
        "trusted touch selects the second action once"
    );
    check_eq!(
        tab.eval("window.fabMenuTrusted").await?,
        json!(true),
        "Chrome delivered a trusted touch event"
    );
    touch_enabled(tab, false).await?;

    select(tab, "all-disabled", "d.all_disabled").await?;
    open_key(tab, TRIGGER).await?;
    for i in 0..6 {
        let node = target(&item(i));
        tab.wait_for(&format!("{node}?.getAttribute('aria-disabled') === 'true'"))
            .await?;
    }
    capture(env, tab, "web-all-disabled").await?;
    tab.press("Enter").await?;
    expanded(tab, false, true).await?;
    focused(tab, TRIGGER).await?;
    check_eq!(
        actions(tab).await?,
        expected,
        "an all-disabled menu retains a usable close trigger"
    );
    Ok(
        json!({"tab_next":OUTSIDE,"tab_previous":previous,"actions":expected,
        "outside_actions":outside_before,"trusted_touch":true}),
    )
}

async fn floating_geometry(env: &Env, tab: &Tab) -> Result<Value> {
    step("M108","Floating FAB preserves its top trailing anchor, uses exact tokens, and paints three palettes in both themes");
    presentation(tab, "floating").await?;
    reduced(tab, true).await?;
    let regular = bounds(tab, FACE).await?;
    check!(
        near(end(regular), 1084.) && near(bottom(regular), 864.),
        "a window below 1200 dp uses the 16 dp original slot"
    );
    resize(tab, 1280, 880).await?;
    let mut sizes = Vec::new();
    for (name, size, radius, glyph) in SIZES {
        choose(tab, "size", name).await?;
        let fixed = bounds(tab, ANCHOR).await?;
        let closed = bounds(tab, FACE).await?;
        let icon = bounds(tab, ICON).await?;
        let export = dom(tab).await?;
        let shape = &find_node(&export, FACE)?["style"]["corners"];
        check!(
            near(closed[2], size)
                && near(closed[3], size)
                && near(icon[2], glyph)
                && has_corner(shape, radius),
            "{name} matches the expressive trigger size, corner and icon"
        );
        check!(
            same(fixed, closed) && near(end(closed), 1256.) && near(bottom(closed), 856.),
            "{name} uses the original 24 dp wide-window slot"
        );
        open_key(tab, TRIGGER).await?;
        let live = bounds(tab, FACE).await?;
        let closer = bounds(tab, ICON).await?;
        let export = dom(tab).await?;
        let shape = &find_node(&export, FACE)?["style"]["corners"];
        check!(
            near(live[2], 56.)
                && near(live[3], 56.)
                && has_corner(shape, 28.)
                && near(closer[2], 20.),
            "native close is a 56 dp circle with a 20 dp icon"
        );
        check!(
            near(live[1], closed[1])
                && near(end(live), end(closed))
                && same(fixed, bounds(tab, ANCHOR).await?),
            "the close face holds the original top trailing corner throughout the transition"
        );
        let top = bounds(tab, &part(0, "face")).await?;
        let lower = bounds(tab, &part(1, "face")).await?;
        check!(
            near(live[1] - bottom(lower), 8.)
                && near(lower[1] - bottom(top), 4.)
                && near(end(top), end(live))
                && near(end(lower), end(live)),
            "the floating stack follows its 8 dp anchor gap and 4 dp row gap"
        );
        let row_icon = bounds(tab, &part(0, "icon")).await?;
        let title = bounds(tab, &part(0, "label")).await?;
        check!(
            top[2] >= 56.
                && near(top[3], 56.)
                && near(row_icon[2], 24.)
                && contains(top, row_icon)
                && contains(top, title)
                && near(title[0] - end(row_icon), 8.),
            "the pill uses a 56 dp target, a 24 dp glyph, and an 8 dp glyph-label gap"
        );
        sizes.push(
            json!({"size":name,"closed":closed,"close":live,"icon":closer,"rows":[top,lower]}),
        );
        close_key(tab).await?;
    }
    item_count(tab, 6).await?;
    let mut palette_samples = Vec::new();
    for dark in [false, true] {
        theme(tab, dark).await?;
        let mut closer_colors = Vec::new();
        let mut item_colors = Vec::new();
        for name in PALETTES {
            choose(tab, "palette", name).await?;
            open_key(tab, TRIGGER).await?;
            tab.press("Tab").await?;
            focused(tab, &item(0)).await?;
            let close = bounds(tab, FACE).await?;
            let pill = bounds(tab, &part(1, "face")).await?;
            let long = bounds(tab, &part(3, "face")).await?;
            check!(
                long[2] > pill[2] + 35. && near(end(long), end(pill)),
                "a longer item hugs its label along the same trailing edge"
            );
            let export = dom(tab).await?;
            let close_fill = rgb(&find_node(&export, FACE)?["style"]["background"])?;
            let pill_fill = rgb(&find_node(&export, &part(1, "face"))?["style"]["background"])?;
            let image = Image::capture(
                env,
                tab,
                &format!("floating-{}-{name}", if dark { "dark" } else { "light" }),
            )
            .await?;
            let painted_close = image.at(close[0] + close[2] / 2., close[1] + 7.)?;
            let outer_circle = image.at(close[0] + 3., close[1] + 3.)?;
            let painted_pill = image.at(pill[0] + pill[2] / 2., pill[1] + 6.)?;
            let outer_pill = image.at(pill[0] + 3., pill[1] + 3.)?;
            check!(
                difference(painted_close, close_fill) <= 10
                    && difference(outer_circle, close_fill) > 14
                    && difference(painted_pill, pill_fill) <= 10
                    && difference(outer_pill, pill_fill) > 14,
                "close circle and floating pill paint their distinct fills and rounded contours"
            );
            check!(
                !closer_colors.contains(&painted_close) && !item_colors.contains(&painted_pill),
                "all three close and pill fills distinguish their named palettes in each theme"
            );
            closer_colors.push(painted_close);
            item_colors.push(painted_pill);
            palette_samples.push(json!({"palette":name,"dark":dark,"close":painted_close,"pill":painted_pill,
                "close_corner":outer_circle,"pill_corner":outer_pill,"short_width":pill[2],"long_width":long[2]}));
            close_key(tab).await?;
        }
    }
    reduced(tab, false).await?;
    Ok(json!({"sizes":sizes,"palettes":palette_samples}))
}

async fn floating_interaction(env: &Env, tab: &Tab) -> Result<Value> {
    step("M109","Floating mode keeps the stable close first, follows authored Button order, and consumes outside touch");
    presentation(tab, "floating").await?;
    reduced(tab, true).await?;
    item_count(tab, 6).await?;
    expanded(tab, false, false).await?;
    accessibility(tab, "button", OPENER_NAME).await?;
    open_key(tab, TRIGGER).await?;
    let close_ax = accessibility(tab, "button", CLOSER_NAME).await?;
    check_eq!(
        ax_property(&close_ax, "expanded"),
        Some(&json!(true)),
        "floating close retains the disclosure state"
    );
    absent(tab, PANEL).await?;
    absent(tab, OUTSIDE).await?;
    for label in LABELS {
        accessibility(tab, "button", label).await?;
    }
    let disabled = target(&item(2));
    tab.wait_for(&format!(
        "{disabled}?.getAttribute('aria-disabled') === 'true'"
    ))
    .await?;
    for i in ENABLED {
        tab.press("Tab").await?;
        focused(tab, &item(i)).await?;
    }
    tab.press("Tab").await?;
    focused(tab, TRIGGER).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &item(5)).await?;
    capture(env, tab, "floating-keyboard-order").await?;
    let mut expected = actions(tab).await?;
    tab.press("Space").await?;
    expanded(tab, false, false).await?;
    focused(tab, TRIGGER).await?;
    expected[5] += 1;
    check_eq!(
        actions(tab).await?,
        expected,
        "keyboard activates the last floating Button exactly once"
    );
    absent(tab, &item(5)).await?;

    open_key(tab, TRIGGER).await?;
    click(tab, &item(2)).await?;
    expanded(tab, true, false).await?;
    check_eq!(
        actions(tab).await?,
        expected,
        "disabled floating row never runs its action"
    );
    click(tab, &item(3)).await?;
    expanded(tab, false, false).await?;
    runtime_focus(tab, TRIGGER).await?;
    expected[3] += 1;
    check_eq!(
        actions(tab).await?,
        expected,
        "real mouse release on a pill selects only that action"
    );

    open_key(tab, TRIGGER).await?;
    let before = data(tab).await?["outside_actions"].clone();
    touch_enabled(tab, true).await?;
    tab.eval("document.addEventListener('pointerdown', event => {
        if (event.target?.id === 'surface' && event.pointerType === 'touch') window.floatingTrusted = event.isTrusted;
    },{capture:true,passive:true})").await?;
    tap(tab, OUTSIDE).await?;
    expanded(tab, false, false).await?;
    runtime_focus(tab, TRIGGER).await?;
    check_eq!(
        data(tab).await?["outside_actions"].clone(),
        before,
        "outside touch does not activate the covered control"
    );
    check_eq!(
        tab.eval("window.floatingTrusted").await?,
        json!(true),
        "the dismissing touch came from Chrome input"
    );
    touch_enabled(tab, false).await?;

    open_key(tab, TRIGGER).await?;
    tab.press("Enter").await?;
    expanded(tab, false, false).await?;
    focused(tab, TRIGGER).await?;
    check_eq!(
        actions(tab).await?,
        expected,
        "close activation never selects a pill"
    );
    select(tab, "disable-trigger", "d.disabled").await?;
    click(tab, TRIGGER).await?;
    ready(tab, "!d.open").await?;
    let node = target(TRIGGER);
    tab.wait_for(&format!("{node}?.getAttribute('aria-disabled') === 'true'"))
        .await?;
    check_eq!(
        actions(tab).await?,
        expected,
        "disabled FAB cannot reopen the menu"
    );
    reduced(tab, false).await?;
    Ok(
        json!({"close_ax":close_ax,"focus_order":ENABLED.map(item),"actions":expected,
        "outside_actions":before,"trusted_touch":true}),
    )
}

async fn scroll_offset(tab: &Tab, id: &str) -> Result<f64> {
    find_node(&dom(tab).await?, id)?["style"]["scroll_y"]
        .as_f64()
        .context("list scroll offset")
}

async fn compact_setup(tab: &Tab, mode: &str) -> Result<()> {
    presentation(tab, mode).await?;
    theme(tab, true).await?;
    select(tab, "rtl", "d.rtl").await?;
    select(tab, "large-text", "d.text_scale === 2").await?;
    choose(tab, "size", "large").await?;
    item_count(tab, 6).await?;
    reduced(tab, true).await?;
    resize(tab, 320, 360).await
}

async fn compact(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step("M110","both 320 dp dark RTL menus keep a long 200 percent label, scrolling, and the focused target inside the viewport");
    let viewport = [0., 0., 320., 360.];
    compact_setup(tab, "floating").await?;
    let closed = bounds(tab, FACE).await?;
    check!(
        near(closed[0], 16.) && near(bottom(closed), 344.) && contains(viewport, closed),
        "compact RTL anchors the closed face to the left with a 16 dp margin"
    );
    open_key(tab, TRIGGER).await?;
    let pinned = bounds(tab, FACE).await?;
    let list = bounds(tab, LIST).await?;
    check!(
        near(pinned[0], closed[0])
            && near(pinned[1], closed[1])
            && contains(viewport, list)
            && near(pinned[1] - bottom(list), 3.),
        "the mirrored close holds its corner below the padded list"
    );
    let nearest = bounds(tab, &part(5, "face")).await?;
    check!(
        contains(list, nearest) && near(pinned[1] - bottom(nearest), 8.),
        "the short list starts with the nearest pill fully visible above the close"
    );
    capture(env, tab, "floating-dark-rtl-initial-nearest").await?;
    for i in [0, 1, 3] {
        tab.press("Tab").await?;
        focused(tab, &item(i)).await?;
    }
    let long = bounds(tab, &part(3, "face")).await?;
    let icon = bounds(tab, &part(3, "icon")).await?;
    let label = bounds(tab, &part(3, "label")).await?;
    let ring = bounds(tab, &part(3, "focus-ring")).await?;
    let export = dom(tab).await?;
    check_eq!(
        find_node(&export, &part(3, "label"))?["text"].clone(),
        json!(LABELS[3]),
        "floating pill keeps the full CJK label"
    );
    check!(long[3]>56. && near(icon[2],24.) && contains(long,icon) && contains(long,label)
        && near(icon[0]-end(label),8.) && contains(list,ring) && bottom(ring)<pinned[1],
        "the mirrored glyph and wrapped label stay inside the pill, and the whole ring stays above the pinned close");
    let image = Image::capture(env, tab, "floating-dark-rtl-200-focused").await?;
    let ring_style = &find_node(&export, &part(3, "focus-ring"))?["style"];
    let ring_color = rgb(&ring_style["border_color"])?;
    let ring_pixel = image.at(ring[0] + ring[2] / 2., ring[1] + 1.5)?;
    check!(
        difference(ring_pixel, ring_color) <= 10,
        "Chrome paints the complete floating focus outline"
    );
    let ink = rgb(&find_node(&export, &part(3, "label"))?["style"]["foreground"])?;
    let [x, y, w, h] = label;
    let lines = [
        image.ink_in([x, y, w, h / 2.], ink)?,
        image.ink_in([x, y + h / 2., w, h / 2.], ink)?,
    ];
    check!(
        lines.into_iter().all(|count| count > 40),
        "each wrapped line of Chinese text paints inside its rounded pill"
    );
    let scroll_mid = scroll_offset(tab, LIST).await?;
    for i in [4, 5] {
        tab.press("Tab").await?;
        focused(tab, &item(i)).await?;
    }
    let scroll_end = scroll_offset(tab, LIST).await?;
    let last_ring = bounds(tab, &part(5, "focus-ring")).await?;
    check!(
        scroll_end > scroll_mid
            && contains(list, last_ring)
            && same(pinned, bounds(tab, FACE).await?),
        "Tab scrolls the real clipped list to the last action while the close remains fixed"
    );
    close_key(tab).await?;

    reload(tab, url).await?;
    compact_setup(tab, "web").await?;
    let trigger = bounds(tab, FACE).await?;
    open_key(tab, &item(0)).await?;
    let popup = bounds(tab, PANEL).await?;
    check!(contains(viewport,popup) && near(trigger[1]-bottom(popup),4.) && same(trigger,bounds(tab,FACE).await?),
        "the compact Web popup clamps to the viewport while retaining its stationary anchor and 4 dp gap");
    for i in [1, 3] {
        tab.press("ArrowDown").await?;
        focused(tab, &item(i)).await?;
    }
    let web_row = bounds(tab, &item(3)).await?;
    let web_export = dom(tab).await?;
    check_eq!(
        find_node(&web_export, &item(3))?["name"].clone(),
        json!(LABELS[3]),
        "the Web menu keeps the full Chinese accessible name"
    );
    check!(
        contains(popup, web_row) && web_row[3] >= 48.,
        "the Web popup scrolls the full focused action into its clip"
    );
    capture(env, tab, "web-dark-rtl-200-focused").await?;
    let web_scroll = scroll_offset(tab, PANEL).await?;
    check!(
        web_scroll > 0.,
        "the compact Web menu scrolled to reveal the focused action"
    );
    close_key(tab).await?;
    reduced(tab, false).await?;
    Ok(
        json!({"floating":{"trigger":closed,"close":pinned,"viewport":list,"long_face":long,"long_label":label,
        "long_icon":icon,"initial_nearest_face":nearest,"focus_ring":ring,"focus_ring_pixel":ring_pixel,"label_line_ink":lines,
        "scroll":[scroll_mid,scroll_end],"last_focus_ring":last_ring},
        "web":{"trigger":trigger,"popup":popup,"focused_row":web_row,"scroll":web_scroll}}),
    )
}

fn metric(value: &Value, item_index: usize, property: &str) -> Result<f64> {
    value["item_motion"][item_index][property]
        .as_f64()
        .with_context(|| format!("FAB menu row {item_index} {property}"))
}

fn progress(value: &Value) -> Result<f64> {
    value["progress"]
        .as_f64()
        .context("FAB menu morph progress")
}

async fn motion(env: &Env, tab: &Tab) -> Result<Value> {
    step("M111","real floating entry and exit stagger in opposite orders, reverse mid-flight, and snap under Reduce Motion");
    presentation(tab, "floating").await?;
    choose(tab, "size", "large").await?;
    item_count(tab, 6).await?;
    reduced(tab, false).await?;
    let closed = bounds(tab, FACE).await?;
    pin_clock(tab).await?;
    open_key(tab, TRIGGER).await?;
    advance(tab, 70).await?;
    ready(tab, "d.progress > 0 && d.progress < 1").await?;
    let entering = data(tab).await?;
    let mid_face = bounds(tab, FACE).await?;
    check!(
        mid_face[2] > 56.
            && mid_face[2] < 96.
            && near(mid_face[1], closed[1])
            && near(end(mid_face), end(closed)),
        "the morph changes the actual face size around the invariant top trailing corner"
    );
    check!(
        metric(&entering, 5, "width")? > metric(&entering, 0, "width")?
            && metric(&entering, 5, "opacity")? > metric(&entering, 0, "opacity")?,
        "during entrance the nearest pill is wider and more opaque than the first authored pill"
    );
    capture(env, tab, "floating-enter-stagger").await?;

    advance(tab, 1500).await?;
    ready(tab, "d.progress >= 0.999 && d.visible_items === d.count").await?;
    let rest = data(tab).await?;
    let old_top = point(tab, &item(0)).await?;
    close_key(tab).await?;
    absent(tab, &item(0)).await?;
    advance(tab, 1550).await?;
    ready(tab, "d.progress > 0 && d.progress < 1").await?;
    let exiting = data(tab).await?;
    check!(
        metric(&exiting, 0, "width")? < metric(&exiting, 5, "width")?
            && metric(&exiting, 0, "opacity")? < metric(&exiting, 5, "opacity")?,
        "exit starts with the farthest item while the nearest item remains painted"
    );
    capture(env, tab, "floating-exit-stagger").await?;
    let initial = actions(tab).await?;
    tab.click(old_top.x, old_top.y).await?;
    check_eq!(
        actions(tab).await?,
        initial,
        "clicking the former row during exit never activates it"
    );

    open_key(tab, TRIGGER).await?;
    let retarget = data(tab).await?;
    check!(
        (progress(&retarget)? - progress(&exiting)?).abs() < 0.01
            && (metric(&retarget, 5, "width")? - metric(&exiting, 5, "width")?).abs() < 0.01,
        "reopening before exit completes retains the exact current pose"
    );
    advance(tab, 1640).await?;
    ready(tab, &format!("d.progress > {}", progress(&retarget)?)).await?;
    let reversing = data(tab).await?;
    capture(env, tab, "floating-reversed").await?;
    advance(tab, 3000).await?;
    ready(tab, "d.progress >= 0.999 && d.visible_items === d.count").await?;
    close_key(tab).await?;
    advance(tab, 4500).await?;
    ready(tab, "d.progress <= 0.001 && d.visible_items === 0").await?;
    check!(
        same(closed, bounds(tab, FACE).await?),
        "full exit returns to the original 96 dp face"
    );
    reduced(tab, true).await?;
    open_key(tab, TRIGGER).await?;
    ready(tab, "d.progress === 1 && d.visible_items === d.count").await?;
    check!(
        near(bounds(tab, FACE).await?[2], 56.),
        "Reduce Motion reaches the close circle without time advancing"
    );
    close_key(tab).await?;
    ready(tab, "d.progress === 0 && d.visible_items === 0").await?;
    absent(tab, &item(5)).await?;
    check!(
        same(closed, bounds(tab, FACE).await?),
        "Reduce Motion removes pills and restores the square immediately"
    );
    tab.eval("delete performance.now").await?;
    reduced(tab, false).await?;
    Ok(
        json!({"closed":closed,"partial_face":mid_face,"entering":entering,"rest":rest,
        "exiting":exiting,"retarget":retarget,"reversing":reversing,"reduced_motion_endpoints":[0,1]}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let web_geometry = web_geometry(env, tab).await?;
    reload(tab, url).await?;
    let web_interaction = web_interaction(env, tab).await?;
    reload(tab, url).await?;
    let floating_geometry = floating_geometry(env, tab).await?;
    reload(tab, url).await?;
    let floating_interaction = floating_interaction(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab, url).await?;
    reload(tab, url).await?;
    let motion = motion(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "FAB menu page errors: {errors:?}");
    env.write_json("browser-material-fab-menu.json",&json!({"browser":chrome.version,
        "web_geometry":web_geometry,"web_interaction":web_interaction,"floating_geometry":floating_geometry,
        "floating_interaction":floating_interaction,"compact":compact,"motion":motion,"errors":errors}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(1100, 924, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _=env.write_json("material-fab-menu-failure.json",&json!({"error":format!("{error:#}"),
            "state":state(&tab).await.ok(),"dom":dom(&tab).await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}));
        let _ = tab
            .screenshot(&env.artifact("material-fab-menu-failure.png"))
            .await;
    }
    result
}
