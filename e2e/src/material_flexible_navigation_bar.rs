//! Focused browser proof for the vertical compact Expressive navigation bar.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{state, until};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "navigation-bar-demo-show";
const BACK: &str = "navigation-bar-demo-back";
const PREVIEW: &str = "navigation-bar-demo-preview";
const BAR: &str = "navigation-bar-demo-bar";
const SCROLL: &str = "navigation-bar-demo-scroll";
const PAGE_TITLE: &str = "navigation-bar-demo-page-title";
const LAST_ENTRY: &str = "navigation-bar-demo-entry-2";
const BEFORE_BAR: &str = "navigation-bar-demo-count-5";
const HOME: &str = "navigation-bar-demo-home";
const DISABLED: &str = "navigation-bar-demo-explore";
const SAVED: &str = "navigation-bar-demo-saved";
const UPDATES: &str = "navigation-bar-demo-updates";
const SETTINGS: &str = "navigation-bar-demo-settings";
const IDS: [&str; 5] = [HOME, DISABLED, SAVED, UPDATES, SETTINGS];
const NAMES: [&str; 5] = ["首頁", "探索路線", "收藏的地點", "最新消息", "設定"];
const ENABLED: [&str; 4] = [HOME, SAVED, UPDATES, SETTINGS];
const STEM: &str = "material-flexible-navigation-bar";
type Rect = [f64; 4];
type Rgb = [u8; 3];

fn bottom(rect: Rect) -> f64 {
    rect[1] + rect[3]
}

#[derive(Clone, Debug, PartialEq)]
struct Destination {
    id: &'static str,
    hit: Rect,
    pill: Rect,
    label: Rect,
}

impl Destination {
    fn receipt(&self) -> Value {
        json!({"id":self.id,"hit":self.hit,"pill":self.pill,"label":self.label})
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Geometry {
    preview: Rect,
    bar: Rect,
    scroll: Rect,
    destinations: Vec<Destination>,
}

impl Geometry {
    fn receipt(&self) -> Value {
        json!({"preview":self.preview,"bar":self.bar,"scroll":self.scroll,
               "destinations": self.destinations.iter().map(Destination::receipt).collect::<Vec<_>>()})
    }
}

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    activate(tab, "section-3").await?;
    until(tab, "s.section === 3").await?;
    activate(tab, SHOW).await?;
    until(tab, "s.navigation_bar.visible").await?;
    focused(tab, BACK).await?;
    resize(tab, 960, 800).await
}

async fn geometry(
    tab: &Tab,
    count: usize,
    width: f64,
    rtl: bool,
    enlarged: bool,
) -> Result<Geometry> {
    let preview = bounds(tab, PREVIEW).await?;
    let bar = bounds(tab, BAR).await?;
    let scroll = bounds(tab, SCROLL).await?;
    check_eq!(
        bar[2],
        width,
        "the expressive bar fills the measured preview width"
    );
    check_eq!(preview[2], width, "the preview width agrees with the bar");
    check_eq!(
        bottom(bar),
        bottom(preview),
        "the bar stays on the bottom edge"
    );
    check_eq!(
        bottom(scroll),
        bar[1],
        "the independent scroll clip ends at the navigation bar"
    );
    if enlarged {
        check!(bar[3] > 64., "200% type grows the bar");
    } else {
        check_eq!(bar[3], 64., "the compact vertical layout starts at 64 dp");
    }
    let cell = width / count as f64;
    let mut destinations = Vec::with_capacity(count);
    for (index, id) in IDS[..count].iter().copied().enumerate() {
        let hit = bounds(tab, id).await?;
        let pill = bounds(tab, &format!("{id}-indicator")).await?;
        let label = bounds(tab, &format!("{id}-label")).await?;
        let physical = if rtl { count - 1 - index } else { index };
        check!(
            near(hit[0], bar[0] + cell * physical as f64)
                && near(hit[2], cell)
                && hit[1] == bar[1]
                && hit[3] == bar[3]
                && hit[2] >= 48.
                && hit[3] >= 48.,
            "destination {id} fills one equally wide hit cell in visual order: {hit:?}"
        );
        check_eq!(
            [pill[2], pill[3]],
            [56., 32.],
            "{id} preserves pill geometry"
        );
        check!(
            contains(hit, pill) && contains(hit, label),
            "{id} keeps the pill and complete label within its own bounds"
        );
        check!(
            near(pill[0] + pill[2] / 2., hit[0] + hit[2] / 2.)
                && near(label[0] + label[2] / 2., hit[0] + hit[2] / 2.)
                && near(pill[1] - hit[1], 6.)
                && near(label[1] - bottom(pill), 4.)
                && bottom(hit) - bottom(label) >= 6.,
            "the icon and label have the prescribed vertical stack inside {id}"
        );
        destinations.push(Destination {
            id,
            hit,
            pill,
            label,
        });
    }
    for excluded in &IDS[count..] {
        check_eq!(
            tab.eval(&format!("{} === null", projected(excluded)))
                .await?,
            true,
            "the resized bar retires the removed destination"
        );
    }
    Ok(Geometry {
        preview,
        bar,
        scroll,
        destinations,
    })
}

fn token(state: &Value, key: &str) -> Result<Rgb> {
    let channels = state["navigation_bar"]["palette"][key].clone();
    let rgba: [u8; 4] =
        serde_json::from_value(channels).context("actual navigation color token")?;
    check_eq!(rgba[3], 255, "palette {key} is opaque");
    Ok([rgba[0], rgba[1], rgba[2]])
}

async fn paint(
    env: &Env,
    tab: &Tab,
    stage: &str,
    geometry: &Geometry,
    selected: &str,
) -> Result<(Raster, Value)> {
    tab.settle().await?;
    let palette = state(tab).await?;
    let destination = geometry
        .destinations
        .iter()
        .find(|slot| slot.id == selected)
        .context("selected cell")?;
    let path = env.artifact(&format!("{STEM}-{stage}.png"));
    tab.screenshot(&path).await?;
    let raster = Raster::read(tab, path).await?;
    let surface_x = destination.hit[0] + destination.hit[2] / 2.;
    let y = geometry.bar[1];
    let [px, py, _, _] = destination.pill;
    let surface = token(&palette, "surface_container")?;
    let fill = token(&palette, "secondary_container")?;
    let icon = token(&palette, "on_secondary_container")?;
    let surface_pixel = raster.sample(surface_x, y + 2.)?;
    let pill_pixel = raster.sample(px + 7., py + 16.)?;
    let corner_pixel = raster.sample(px, py)?;
    check_eq!(
        surface_pixel,
        surface,
        "the authored theme colors reach the WebGPU surface pixels"
    );
    check_eq!(
        pill_pixel,
        fill,
        "the active pill has visible Secondary Container paint"
    );
    check_eq!(
        corner_pixel,
        surface,
        "the selected pill leaves its rounded outer corner clear"
    );
    let icon_pixels = raster
        .colors([px + 16., py + 4., 24., 24.])?
        .get(&icon)
        .copied()
        .unwrap_or_default();
    check!(
        icon_pixels > 24,
        "the actual selected vector paints its On Secondary Container ink"
    );
    let inactive = geometry
        .destinations
        .iter()
        .find(|slot| slot.id != selected && slot.id != DISABLED)
        .context("inactive enabled destination")?;
    let [ix, iy, _, _] = inactive.pill;
    check_eq!(
        raster.sample(ix + 7., iy + 16.)?,
        surface,
        "the inactive item leaves its pill area clear"
    );
    Ok((
        raster,
        json!({"stage":stage,"surface":surface_pixel,"pill":pill_pixel,
        "corner":corner_pixel,"icon_ink":icon,"icon_pixels":icon_pixels}),
    ))
}

async fn counts_and_colors(env: &Env, tab: &Tab) -> Result<Value> {
    let mut layouts = Vec::new();
    for count in [3, 4, 5] {
        let control = format!("navigation-bar-demo-count-{count}");
        activate(tab, &control).await?;
        until(tab, &format!("s.navigation_bar.count === {count}")).await?;
        let actual = geometry(tab, count, 480., false, false).await?;
        check_eq!(
            [actual.preview[0], actual.preview[1]],
            [240., 0.],
            "wide preview is centered"
        );
        check_eq!(
            bottom(actual.bar),
            800.,
            "the preview and bar fill the 800 dp test canvas"
        );
        layouts.push(actual);
    }
    let layout = layouts.last().context("five-destination layout")?;
    let (_, light) = paint(env, tab, "light", layout, HOME).await?;
    activate(tab, "navigation-bar-demo-theme").await?;
    until(tab, "s.dark").await?;
    check_eq!(
        geometry(tab, 5, 480., false, false).await?,
        *layout,
        "changing theme preserves geometry"
    );
    let (_, dark) = paint(env, tab, "dark", layout, HOME).await?;
    check!(
        light["surface"] != dark["surface"] && light["pill"] != dark["pill"],
        "the decoded light and dark images use distinct Surface and pill colors"
    );
    Ok(
        json!({"layouts":layouts.iter().map(Geometry::receipt).collect::<Vec<_>>(),
        "light":light,"dark":dark}),
    )
}

pub(super) async fn selection(tab: &Tab, id: &str, page: &str, activations: usize) -> Result<()> {
    until(
        tab,
        &format!(
            "s.navigation_bar.selected === {id:?} && s.navigation_bar.page === {page:?} &&
         s.navigation_bar.activations === {activations}"
        ),
    )
    .await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-current') === 'page' &&
         document.querySelectorAll('#surface-accessibility [aria-current=page]').length === 1",
        projected(id)
    ))
    .await
}

pub(super) async fn projection_and_tab(tab: &Tab) -> Result<Value> {
    selection(tab, HOME, "首頁", 0).await?;
    let tree = ax_tree(tab).await?;
    let bar = ax(&tree, "navigation", "主要導覽")?;
    let children = bar["childIds"]
        .as_array()
        .context("AX navigation children")?;
    for (id, name) in IDS.into_iter().zip(NAMES) {
        let button = ax(&tree, "button", name)?;
        check!(
            children.contains(&button["nodeId"]),
            "{name} is in the real AX navigation landmark"
        );
        check!(
            ax_property(button, "pressed").is_none() && ax_property(button, "selected").is_none(),
            "{name} is an action for a current page, with no toggle state"
        );
        if id == DISABLED {
            check_eq!(
                ax_property(button, "disabled"),
                Some(&json!(true)),
                "the AX button is disabled"
            );
            check_eq!(
                tab.eval(&format!(
                    "{}?.hasAttribute('data-surface-action')",
                    projected(id)
                ))
                .await?,
                false,
                "disabled navigation has no projected press action"
            );
        }
    }
    focus_projected(tab, BEFORE_BAR).await?;
    for id in ENABLED {
        tab.press("Tab").await?;
        focused(tab, id).await?;
        selection(tab, HOME, "首頁", 0).await?;
    }
    for id in ENABLED[..3].iter().rev().copied() {
        tab.press("Shift+Tab").await?;
        focused(tab, id).await?;
        selection(tab, HOME, "首頁", 0).await?;
    }
    let now = ax_tree(tab).await?;
    check_eq!(
        ax_property(ax(&now, "button", "首頁")?, "focused"),
        Some(&json!(true)),
        "real browser AX retains the focused Home"
    );
    Ok(
        json!({"landmark":"主要導覽","roles":NAMES,"disabled":DISABLED,"current":HOME,
        "tab_order":ENABLED,"shift_tab_order":[UPDATES,SAVED,HOME],"preactivations":0}),
    )
}

async fn visible_heading(tab: &Tab, name: &str) -> Result<Rect> {
    let title = bounds(tab, PAGE_TITLE).await?;
    let rendered_clip: Rect = serde_json::from_value(
        tab.eval(&format!(
            "(() => {{ const snapshot = JSON.parse(window.gallery.export_dom());
            const index = snapshot.dom.nodes.findIndex(node => node.stable_id === {PAGE_TITLE:?});
            const r = snapshot.layout.nodes[index].clip;
            return [r.x, r.y, r.width, r.height]; }})()"
        ))
        .await?,
    )
    .context("the page heading has a real inherited clip")?;
    check!(
        contains(rendered_clip, title) && bottom(title) <= bounds(tab, BAR).await?[1],
        "the complete title remains visible above the anchored bar: {title:?}"
    );
    let tree = ax_tree(tab).await?;
    ax(&tree, "heading", name)?;
    Ok(title)
}

async fn screen(tab: &Tab, id: &str, edge: bool) -> Result<(f64, f64)> {
    let [x, y, w, h] = bounds(tab, id).await?;
    let [left, top, _, _] = tab.canvas_box().await?;
    Ok(if edge {
        (left + x + 6., top + y + h - 5.)
    } else {
        (left + x + w / 2., top + y + h / 2.)
    })
}

pub(super) async fn activate_with_keys_and_contact(tab: &Tab) -> Result<Value> {
    focused(tab, HOME).await?;
    tab.press("Tab").await?;
    focused(tab, SAVED).await?;
    selection(tab, HOME, "首頁", 0).await?;
    tab.press("Enter").await?;
    selection(tab, SAVED, "收藏", 1).await?;
    focused(tab, SAVED).await?;
    let saved_heading = visible_heading(tab, "收藏").await?;
    tab.press("Tab").await?;
    focused(tab, UPDATES).await?;
    selection(tab, SAVED, "收藏", 1).await?;
    tab.press("Space").await?;
    selection(tab, UPDATES, "最新消息", 2).await?;
    focused(tab, UPDATES).await?;
    let updates_heading = visible_heading(tab, "最新消息").await?;

    let (x, y) = screen(tab, DISABLED, false).await?;
    tab.click(x, y).await?;
    tab.settle().await?;
    selection(tab, UPDATES, "最新消息", 2).await?;
    check!(
        state(tab).await?["focused"] != DISABLED,
        "disabled cell never takes input focus"
    );
    check_eq!(
        tab.eval("document.activeElement === document.querySelector('canvas')")
            .await?,
        true,
        "the disabled pointer attempt actually used the canvas path"
    );

    let (x, y) = screen(tab, SETTINGS, true).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":x,"y":y,"id":1}]}),
    )
    .await?;
    tab.settle().await?;
    selection(tab, UPDATES, "最新消息", 2).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchEnd","touchPoints":[]}),
    )
    .await?;
    tab.settle().await?;
    selection(tab, SETTINGS, "設定", 3).await?;
    let settings_heading = visible_heading(tab, "設定").await?;
    Ok(
        json!({"enter":SAVED,"space":UPDATES,"disabled_pointer":DISABLED,
        "contact_target":SETTINGS,"activations":3,
        "visible_titles":[saved_heading,updates_heading,settings_heading]}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    activate(tab, "navigation-bar-demo-rtl").await?;
    activate(tab, "navigation-bar-demo-large-text").await?;
    until(
        tab,
        "s.dark && s.navigation_bar.rtl && s.navigation_bar.text_scale === 2",
    )
    .await?;
    resize(tab, 320, 480).await?;
    until(tab, "s.navigation_bar.preview_width === 320").await?;
    focus_projected(tab, HOME).await?;
    for id in [SAVED, UPDATES] {
        tab.press("Tab").await?;
        focused(tab, id).await?;
        selection(tab, SETTINGS, "設定", 3).await?;
    }
    tab.press("Space").await?;
    selection(tab, UPDATES, "最新消息", 4).await?;
    focused(tab, UPDATES).await?;
    let heading = visible_heading(tab, "最新消息").await?;
    tab.press("Tab").await?;
    focused(tab, SETTINGS).await?;
    selection(tab, UPDATES, "最新消息", 4).await?;
    let before = geometry(tab, 5, 320., true, true).await?;
    let entry_before = bounds(tab, LAST_ENTRY).await?;
    check!(
        !contains(before.scroll, entry_before),
        "the last item starts outside or cut by the content clip"
    );
    let selected = &before.destinations[3];
    let [x, y, w, h] = selected.label;
    check!(
        h >= 64.,
        "the selected CJK label wraps to at least two 32 dp lines"
    );
    let (raster, initial_paint) = paint(env, tab, "rtl-large", &before, UPDATES).await?;
    let selected_text = token(&state(tab).await?, "secondary")?;
    let [focus_x, focus_y, _, _] = before.destinations[4].hit;
    let focused_outline = raster.sample(focus_x + 1., focus_y + 2.)?;
    check_eq!(
        focused_outline,
        selected_text,
        "the actual Settings focus outline uses Secondary ink"
    );
    let mut lines = Vec::new();
    for line in 0..2 {
        let sample = raster.colors([x, y + 32. * f64::from(line), w, 32.])?;
        let ink = sample.get(&selected_text).copied().unwrap_or_default();
        check!(
            ink > 32,
            "wrapped selected text paints its label color on line {}",
            line + 1
        );
        lines.push(ink);
    }

    let (wheel_x, wheel_y) = screen(tab, SCROLL, false).await?;
    tab.mouse_move(wheel_x, wheel_y).await?;
    tab.wheel(0., 800.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({LAST_ENTRY:?})[1] < {}",
        entry_before[1]
    ))
    .await?;
    tab.settle().await?;
    let after = geometry(tab, 5, 320., true, true).await?;
    check_eq!(
        before,
        after,
        "wheel never moves the bar, hit cells, icons or labels"
    );
    let entry_after = bounds(tab, LAST_ENTRY).await?;
    check!(
        contains(after.scroll, entry_after),
        "the actual wheel reveals the whole last entry"
    );
    focused(tab, SETTINGS).await?;
    selection(tab, UPDATES, "最新消息", 4).await?;
    check_eq!(
        visible_heading(tab, "最新消息").await?,
        heading,
        "the current page title stays fixed while content scrolls"
    );
    let (_, final_paint) = paint(env, tab, "rtl-large-scrolled", &after, UPDATES).await?;
    activate(tab, BACK).await?;
    until(tab, "!s.navigation_bar.visible").await?;
    focused(tab, SHOW).await?;
    Ok(
        json!({"geometry":after.receipt(),"text_scale":2,"rtl":true,"selected":UPDATES,
        "focused_after_wheel":SETTINGS,"restored_focus":SHOW,"heading":heading,
        "entry_before":entry_before,"entry_after":entry_after,"selected_line_pixels":lines,
        "selected_label_rgb":selected_text,"focused_outline_rgb":focused_outline,
        "initial_paint":initial_paint,"final_paint":final_paint,"activations":4}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step("M270", "compact navigation fits three, four and five equal cells with light and dark WebGPU paint");
        route(&tab, url).await.context("M270 route")?;
        report["reduced_motion"] = tab.eval("matchMedia('(prefers-reduced-motion: reduce)').matches").await?;
        check_eq!(report["reduced_motion"], true, "the static color checks use the requested motion preference");
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter();if(!a)return null;const {isFallbackAdapter,vendor,architecture}=a.info;return {isFallbackAdapter,vendor,architecture};})()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(), "WebGPU reports actual adapter metadata");
        report["counts_and_colors"] = counts_and_colors(env, &tab).await.context("M270")?;
        step("M271", "real navigation semantics expose the current page and let Tab explore without selecting");
        report["accessibility"] = projection_and_tab(&tab).await.context("M271")?;
        step("M272", "Enter, Space and touch each choose one destination, while disabled pointer input stays inert");
        report["actions"] = activate_with_keys_and_contact(&tab).await.context("M272")?;
        step("M273", "320 dp RTL and 200% type paint complete wrapped labels while wheel leaves the bottom and focus pinned");
        report["compact"] = compact(env, &tab).await.context("M273")?;
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }.await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact(&format!("{STEM}-failure.png")))
            .await;
    }
    std::fs::write(
        env.artifact(&format!("browser-{STEM}.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    result
}
