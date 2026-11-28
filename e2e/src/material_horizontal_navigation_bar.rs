//! Render the medium Expressive bar through the Gallery Runtime.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use super::{
    material_flexible_navigation_bar::{
        activate_with_keys_and_contact, projection_and_tab, selection,
    },
    material_time_input::{
        activate, bounds, contains, focus_projected, focused, near, resize, Raster,
    },
    state, until,
};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 3] = ["M363", "M364", "M365"];
const SHOW: &str = "navigation-bar-medium-demo-show";
const BACK: &str = "navigation-bar-demo-back";
const PREVIEW: &str = "navigation-bar-demo-preview";
const BAR: &str = "navigation-bar-demo-bar";
const SCROLL: &str = "navigation-bar-demo-scroll";
const LAST_ENTRY: &str = "navigation-bar-demo-entry-2";
const HOME: &str = "navigation-bar-demo-home";
const SAVED: &str = "navigation-bar-demo-saved";
const UPDATES: &str = "navigation-bar-demo-updates";
const SETTINGS: &str = "navigation-bar-demo-settings";
const IDS: [&str; 5] = [
    HOME,
    "navigation-bar-demo-explore",
    SAVED,
    UPDATES,
    SETTINGS,
];
const STEM: &str = "material-horizontal-navigation-bar";
const CELL: f64 = 112.;
type Rect = [f64; 4];
type Rgb = [u8; 3];

#[derive(Debug, PartialEq, Serialize)]
struct Destination {
    id: &'static str,
    hit: Rect,
    pill: Rect,
    icon: Rect,
    label: Rect,
}

#[derive(Debug, PartialEq, Serialize)]
struct Layout {
    bar: Rect,
    scroll: Rect,
    destinations: Vec<Destination>,
}

fn right(rect: Rect) -> f64 {
    rect[0] + rect[2]
}

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

async fn medium(tab: &Tab, width: u32, height: u32) -> Result<()> {
    resize(tab, width, height).await?;
    until(
        tab,
        &format!("s.navigation_bar.horizontal && s.navigation_bar.preview_width === {width}"),
    )
    .await
}

async fn geometry(tab: &Tab, count: usize, width: f64, rtl: bool, large: bool) -> Result<Layout> {
    let preview = bounds(tab, PREVIEW).await?;
    let bar = bounds(tab, BAR).await?;
    let scroll = bounds(tab, SCROLL).await?;
    check_eq!(bar[2], width, "the medium bar fills the window preview");
    check_eq!(bottom(bar), bottom(preview), "the bar is anchored");
    check_eq!(bottom(scroll), bar[1], "the page scrolls above the bar");
    check!(
        if large { bar[3] > 64. } else { bar[3] == 64. },
        "the bar height follows the tallest label"
    );
    let start = bar[0] + (width - CELL * count as f64) / 2.;
    let mut destinations = Vec::with_capacity(count);
    for (index, id) in IDS[..count].iter().copied().enumerate() {
        let hit = bounds(tab, id).await?;
        let pill = bounds(tab, &format!("{id}-indicator")).await?;
        let label = bounds(tab, &format!("{id}-label")).await?;
        let visual = if rtl { count - 1 - index } else { index };
        check!(
            near(hit[0], start + CELL * visual as f64)
                && near(hit[1], bar[1])
                && near(hit[2], CELL)
                && near(hit[3], bar[3]),
            "{id} keeps one full fixed-width hit target in visual order"
        );
        check!(
            contains(hit, pill)
                && contains(pill, label)
                && near(pill[0] + pill[2] / 2., hit[0] + CELL / 2.)
                && near(pill[1] + pill[3] / 2., hit[1] + hit[3] / 2.)
                && near(pill[2], 60. + label[2])
                && label[2] <= 52.
                && near(pill[3], 24_f64.max(label[3]) + 16.),
            "{id} keeps its centered pill, insets and complete measured label"
        );
        let icon = [
            if rtl {
                right(pill) - 40.
            } else {
                pill[0] + 16.
            },
            pill[1] + (pill[3] - 24.) / 2.,
            24.,
            24.,
        ];
        let gap = if rtl {
            icon[0] - right(label)
        } else {
            label[0] - right(icon)
        };
        check!(
            near(gap, 4.) && contains(pill, icon),
            "{id} mirrors the leading icon and gap"
        );
        destinations.push(Destination {
            id,
            hit,
            pill,
            icon,
            label,
        });
    }
    Ok(Layout {
        bar,
        scroll,
        destinations,
    })
}

fn palette(model: &Value, name: &str) -> Result<Rgb> {
    let [red, green, blue, alpha]: [u8; 4] =
        serde_json::from_value(model["navigation_bar"]["palette"][name].clone())
            .with_context(|| format!("navigation palette {name}"))?;
    check_eq!(alpha, 255, "sampled navigation tokens are opaque");
    Ok([red, green, blue])
}

async fn paint(
    env: &Env,
    tab: &Tab,
    tag: &str,
    layout: &Layout,
    id: &str,
    focus_id: Option<&str>,
) -> Result<Value> {
    tab.settle().await?;
    let model = state(tab).await?;
    let surface = palette(&model, "surface_container")?;
    let fill = palette(&model, "secondary_container")?;
    let ink = palette(&model, "on_secondary_container")?;
    let target = layout
        .destinations
        .iter()
        .find(|target| target.id == id)
        .context("the current item belongs to the bar")?;
    let image = env.artifact(&format!("{STEM}-{tag}.png"));
    tab.screenshot(&image).await?;
    let raster = Raster::read(tab, image).await?;
    let [x, y, _, height] = target.pill;
    check_eq!(
        raster.sample(layout.bar[0] + 3., y + height / 2.)?,
        surface,
        "outer space shows the bar token"
    );
    check_eq!(
        raster.sample(x + 6., y + height / 2.)?,
        fill,
        "the selected pill paints its paired face"
    );
    check_eq!(
        raster.sample(x, y)?,
        surface,
        "the pill leaves its rounded corner clear"
    );
    let icon_ink = raster
        .colors(target.icon)?
        .get(&ink)
        .copied()
        .unwrap_or_default();
    let label_ink = raster
        .colors(target.label)?
        .get(&ink)
        .copied()
        .unwrap_or_default();
    check!(
        icon_ink > 24 && label_ink > 16,
        "actual selected icon and label share OnSecondaryContainer"
    );
    let focus = if let Some(focus_id) = focus_id {
        let [left, top, width, _] = layout
            .destinations
            .iter()
            .find(|item| item.id == focus_id)
            .context("the focused destination remains in the bar")?
            .hit;
        let focus_ink = palette(&model, "secondary")?;
        for depth in [0.75, 1.75, 2.75] {
            check_eq!(
                raster.sample(left + width / 2., top + depth)?,
                focus_ink,
                "the adjacent focused cell paints its 3 dp Secondary ring"
            );
        }
        Some(focus_ink)
    } else {
        None
    };
    Ok(
        json!({"image":format!("{STEM}-{tag}.png"),"surface":surface,"indicator":fill,
              "ink":ink,"icon_pixels":icon_ink,"label_pixels":label_ink,"focus_ink":focus}),
    )
}

async fn initial_layouts(env: &Env, tab: &Tab) -> Result<Value> {
    let mut layouts = Vec::new();
    for width in [600_u32, 839] {
        medium(tab, width, 760).await?;
        for count in [3, 4, 5] {
            activate(tab, &format!("navigation-bar-demo-count-{count}")).await?;
            until(tab, &format!("s.navigation_bar.count === {count}")).await?;
            layouts.push(geometry(tab, count, f64::from(width), false, false).await?);
        }
    }
    let light = paint(env, tab, "light", &layouts[5], HOME, None).await?;
    Ok(json!({"layouts":layouts,"paint":light}))
}

async fn check_enabled_order(tab: &Tab) -> Result<()> {
    focused(tab, SAVED).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, HOME).await?;
    tab.press("Tab").await?;
    focused(tab, SAVED).await?;
    tab.press("Tab").await?;
    focused(tab, UPDATES).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, SAVED).await?;
    selection(tab, SETTINGS, "設定", 3).await
}

async fn navigation(tab: &Tab) -> Result<Value> {
    let before = geometry(tab, 5, 839., false, false).await?;
    let ax = projection_and_tab(tab).await?;
    let actions = activate_with_keys_and_contact(tab).await?;
    let after = geometry(tab, 5, 839., false, false).await?;
    check_eq!(
        before,
        after,
        "selection preserves every measured destination rectangle"
    );
    let hit = bounds(tab, HOME).await?;
    let [left, top, _, _] = tab.canvas_box().await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({
            "type":"touchStart","touchPoints":[{"x":left+hit[0]+4.,"y":top+hit[1]+5.,"id":1}]
        }),
    )
    .await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchCancel","touchPoints":[]}),
    )
    .await?;
    selection(tab, SETTINGS, "設定", 3).await?;
    medium(tab, 600, 760).await?;
    focus_projected(tab, SAVED).await?;
    selection(tab, SETTINGS, "設定", 3).await?;
    check_enabled_order(tab).await?;
    resize(tab, 599, 760).await?;
    until(
        tab,
        "!s.navigation_bar.horizontal && s.navigation_bar.preview_width === 599",
    )
    .await?;
    selection(tab, SETTINGS, "設定", 3).await?;
    focused(tab, SAVED).await?;
    check_enabled_order(tab).await?;
    let compact = bounds(tab, &format!("{SAVED}-indicator")).await?;
    check_eq!(
        [compact[2], compact[3]],
        [56., 32.],
        "599 dp uses the existing vertical pill"
    );
    medium(tab, 600, 760).await?;
    selection(tab, SETTINGS, "設定", 3).await?;
    focused(tab, SAVED).await?;
    check_enabled_order(tab).await?;
    geometry(tab, 5, 600., false, false).await?;
    Ok(
        json!({"ax":ax,"actions":actions,"touch_cancelled":true,"focus":SAVED,"widths":[839,600,599,600]}),
    )
}

async fn enlarged_rtl(env: &Env, tab: &Tab) -> Result<Value> {
    for control in ["theme", "rtl", "large-text"] {
        activate(tab, &format!("navigation-bar-demo-{control}")).await?;
    }
    until(
        tab,
        "s.dark && s.navigation_bar.rtl && s.navigation_bar.text_scale === 2",
    )
    .await?;
    medium(tab, 720, 350).await?;
    focus_projected(tab, UPDATES).await?;
    tab.press("Space").await?;
    selection(tab, UPDATES, "最新消息", 4).await?;
    focused(tab, UPDATES).await?;
    tab.press("Tab").await?;
    focused(tab, SETTINGS).await?;
    let before = geometry(tab, 5, 720., true, true).await?;
    check!(
        before.destinations[3].label[3] >= 64.,
        "the four-character name wraps at 200% text size"
    );
    let initial = paint(env, tab, "rtl-large", &before, UPDATES, Some(SETTINGS)).await?;
    let old_page = bounds(tab, LAST_ENTRY).await?;
    let [x, y, w, h] = before.scroll;
    let [left, top, _, _] = tab.canvas_box().await?;
    tab.mouse_move(left + x + w / 2., top + y + h / 2.).await?;
    tab.wheel(0., 800.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({LAST_ENTRY:?})[1] < {}",
        old_page[1]
    ))
    .await?;
    let after = geometry(tab, 5, 720., true, true).await?;
    check_eq!(
        before,
        after,
        "a real page scroll keeps the medium bar and its targets fixed"
    );
    focused(tab, SETTINGS).await?;
    selection(tab, UPDATES, "最新消息", 4).await?;
    let scrolled = paint(
        env,
        tab,
        "rtl-large-scrolled",
        &after,
        UPDATES,
        Some(SETTINGS),
    )
    .await?;
    Ok(
        json!({"layout":after,"initial":initial,"scrolled":scrolled,"current":UPDATES,"focus":SETTINGS}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    tab.goto(url).await?;
    crate::smoke::ready(&tab, "gallery").await?;
    resize(&tab, 960, 1800).await?;
    activate(&tab, "section-3").await?;
    until(&tab, "s.section === 3").await?;
    activate(&tab, SHOW).await?;
    focused(&tab, BACK).await?;
    step(
        STEPS[0],
        "600–839 dp centers fixed-width horizontal targets and pairs the icon and label ink",
    );
    let layout = initial_layouts(env, &tab).await?;
    step(
        STEPS[1],
        "real keyboard, touch and current page survive the 599 to 600 dp handoff",
    );
    let interaction = navigation(&tab).await?;
    step(
        STEPS[2],
        "dark RTL 200% wraps labels and keeps the focused bar pinned during page scroll",
    );
    let rtl = enlarged_rtl(env, &tab).await?;
    activate(&tab, BACK).await?;
    focused(&tab, SHOW).await?;
    let proof = json!({"layout":layout,"interaction":interaction,"rtl":rtl});
    std::fs::write(
        env.artifact(&format!("{STEM}.json")),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    Ok(())
}
