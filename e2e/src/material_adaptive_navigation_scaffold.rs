use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click, focus,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    state, until,
};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 4] = ["M366", "M367", "M368", "M369"];
const SHOW: &str = "adaptive-navigation-demo-show";
const BACK: &str = "adaptive-navigation-demo-back";
const NAV: &str = "adaptive-navigation-demo-bar";
const SCROLL: &str = "adaptive-navigation-demo-bar-scroll";
const PAGE: &str = "adaptive-navigation-demo-page";
const EDITOR: &str = "adaptive-navigation-demo-draft";
const HOME: &str = "adaptive-navigation-demo-home";
const SAVED: &str = "adaptive-navigation-demo-saved";
const UPDATES: &str = "adaptive-navigation-demo-updates";
const SETTINGS: &str = "adaptive-navigation-demo-settings";
const STEM: &str = "material-adaptive-navigation";
const TEXT: &str = "draft";
type Rect = [f64; 4];

fn right([x, _, width, _]: Rect) -> f64 {
    x + width
}
fn bottom([_, y, _, height]: Rect) -> f64 {
    y + height
}

async fn current(tab: &Tab, id: &str, activations: usize) -> Result<()> {
    until(tab, &format!(
        "s.adaptive_navigation.selected === {id:?} && s.adaptive_navigation.activations === {activations}"
    )).await?;
    tab.wait_for(&format!(
        "{}?.getAttribute('aria-current') === 'page' &&
         document.querySelectorAll('#surface-accessibility [aria-current=page]').length === 1",
        projected(id)
    ))
    .await
}

async fn layout(tab: &Tab, width: u32, height: u32, rtl: bool) -> Result<(Rect, Rect)> {
    resize(tab, width, height).await?;
    until(tab, &format!(
        "s.adaptive_navigation.viewport[0] === {width} && s.adaptive_navigation.viewport[1] === {height}"
    )).await?;
    let nav = bounds(tab, NAV).await?;
    let page = bounds(tab, PAGE).await?;
    if width < 840 {
        check!(
            near(nav[0], 0.)
                && near(nav[2], f64::from(width))
                && near(bottom(nav), f64::from(height))
                && near(page[2], f64::from(width))
                && near(bottom(page), nav[1]),
            "page occupies the full space above the bottom bar: {page:?}, {nav:?}"
        );
    } else {
        check!(
            near(nav[1], 0.) && near(nav[2], 96.) && near(nav[3], f64::from(height))
                && near(page[1], 0.) && near(page[3], f64::from(height))
                && if rtl {
                    near(page[0], 0.) && near(right(page), nav[0]) && near(right(nav), f64::from(width))
                } else {
                    near(nav[0], 0.) && near(right(nav), page[0]) && near(right(page), f64::from(width))
                },
            "the 96 dp rail stays at logical start with an independent full-height page: {page:?}, {nav:?}"
        );
    }
    let tree = ax_tree(tab).await?;
    let _ = ax(&tree, "navigation", "主要導覽")?;
    let disabled = ax(&tree, "button", "探索路線")?;
    check_eq!(
        ax_property(disabled, "disabled"),
        Some(&json!(true)),
        "the browser accessibility tree exposes the disabled destination"
    );
    check_eq!(
        tab.eval("document.querySelectorAll('#surface-accessibility [role=navigation]').length")
            .await?,
        json!(1),
        "the page has one navigation landmark after replacement"
    );
    Ok((nav, page))
}

async fn screenshot(env: &Env, tab: &Tab, name: &str, nav: Rect, rail: bool) -> Result<Value> {
    tab.settle().await?;
    let state = state(tab).await?;
    let fixture = &state["adaptive_navigation"];
    let colors = &fixture["palette"];
    let current = fixture["selected"]
        .as_str()
        .context("read current destination")?;
    let current_box = bounds(tab, current).await?;
    check!(
        contains(nav, current_box),
        "the current destination is fully visible for pixel sampling"
    );
    let color = |key| -> Result<[u8; 3]> {
        let [r, g, b, a]: [u8; 4] = serde_json::from_value(colors[key].clone())
            .context("the Gallery palette supplies an RGBA color")?;
        check_eq!(a, 255, "the sampled navigation token is opaque");
        Ok([r, g, b])
    };
    let path = env.artifact(&format!("{STEM}-{name}.png"));
    tab.screenshot(&path).await?;
    let raster = Raster::read(tab, path).await?;
    let fill = color(if rail { "rail" } else { "bar" })?;
    let selected = color("selected")?;
    check_eq!(
        raster.sample(nav[0] + 3., nav[1] + 2.)?,
        fill,
        "navigation surface paints its token"
    );
    let selected_pixels = raster
        .colors(current_box)?
        .get(&selected)
        .copied()
        .unwrap_or_default();
    check!(
        selected_pixels > 20,
        "the visible current page has its selected indicator"
    );
    Ok(
        json!({"image":format!("{STEM}-{name}.png"),"bounds":nav,"surface":fill,
              "selected":selected,"selected_pixels":selected_pixels,"current_bounds":current_box}),
    )
}

async fn bars(env: &Env, tab: &Tab) -> Result<Value> {
    let (compact, _) = layout(tab, 599, 620, false).await?;
    focus_projected(tab, HOME).await?;
    tab.press("Tab").await?;
    focused(tab, SAVED).await?;
    current(tab, HOME, 0).await?;
    tab.press("Enter").await?;
    current(tab, SAVED, 1).await?;
    focused(tab, SAVED).await?;
    let [_, _, width, height] = bounds(tab, &format!("{SAVED}-indicator")).await?;
    check_eq!(
        [width, height],
        [56., 32.],
        "the original vertical active pill is present"
    );
    focus_projected(tab, UPDATES).await?;
    let compact = screenshot(env, tab, "compact", compact, false).await?;
    focus_projected(tab, SAVED).await?;
    let (medium, _) = layout(tab, 600, 620, false).await?;
    focused(tab, SAVED).await?;
    current(tab, SAVED, 1).await?;
    let hit = bounds(tab, SAVED).await?;
    let pill = bounds(tab, &format!("{SAVED}-indicator")).await?;
    check!(
        near(hit[2], 112.) && near(pill[3], 40.) && pill[2] > 56.,
        "the same destination uses the horizontal pill"
    );
    focus_projected(tab, UPDATES).await?;
    let medium = screenshot(env, tab, "medium", medium, false).await?;
    focus_projected(tab, SAVED).await?;
    Ok(json!({"compact":compact,"medium":medium,"current":SAVED,"focus":SAVED}))
}

async fn rail(env: &Env, tab: &Tab) -> Result<Value> {
    layout(tab, 839, 620, false).await?;
    focused(tab, SAVED).await?;
    let (rail, page) = layout(tab, 840, 620, false).await?;
    focused(tab, SAVED).await?;
    current(tab, SAVED, 1).await?;
    tab.press("ArrowDown").await?;
    focused(tab, UPDATES).await?;
    current(tab, SAVED, 1).await?;
    tab.press("Space").await?;
    current(tab, UPDATES, 2).await?;
    focused(tab, UPDATES).await?;
    tab.press("ArrowDown").await?;
    focused(tab, SETTINGS).await?;
    current(tab, UPDATES, 2).await?;
    let paint = screenshot(env, tab, "rail", rail, true).await?;
    Ok(json!({"paint":paint,"page":page,"current":UPDATES,"focus":SETTINGS}))
}

async fn editor(tab: &Tab) -> Result<Value> {
    focus(tab, EDITOR, "surface-text-input").await?;
    let read = format!(
        r#"(() => {{
        const s=JSON.parse(window.gallery.export_dom()).text_inputs[{EDITOR:?}];
        const t=document.getElementById('surface-text-input');
        return {{browser:[t?.value,t?.selectionStart,t?.selectionEnd,t?.selectionDirection],
                 runtime:[s?.selection_anchor,s?.selection_cursor], value:s?.value}};
    }})()"#
    );
    let browser = json!([TEXT, 3, 5, "backward"]);
    let runtime = json!([5, 3]);
    let retained = format!(
        "(() => {{ const editor = {read};
            return editor.value === {TEXT:?}
                && JSON.stringify(editor.browser) === JSON.stringify({browser})
                && JSON.stringify(editor.runtime) === JSON.stringify({runtime});
        }})()"
    );
    if let Err(error) = tab.wait_for(&retained).await {
        let actual = tab.eval(&read).await?;
        return Err(error.context(format!("observed editor value and selection: {actual}")));
    }
    tab.eval(&read).await
}

async fn draft(tab: &Tab) -> Result<Value> {
    click(tab, EDITOR).await?;
    focus(tab, EDITOR, "surface-text-input").await?;
    tab.insert_text(TEXT).await?;
    until(tab, &format!("s.adaptive_navigation.draft === {TEXT:?}")).await?;
    for _ in 0..2 {
        tab.press("Shift+ArrowLeft").await?;
    }
    let before = editor(tab).await?;
    for width in [839, 320, 840] {
        layout(tab, width, 620, false).await?;
        check_eq!(
            editor(tab).await?,
            before,
            "the same focused draft and backward range survive resize"
        );
        current(tab, UPDATES, 2).await?;
    }
    Ok(before)
}

async fn compact_height(env: &Env, tab: &Tab) -> Result<Value> {
    for control in ["theme", "rtl", "large"] {
        activate(tab, &format!("adaptive-navigation-demo-{control}")).await?;
    }
    until(
        tab,
        "s.dark && s.adaptive_navigation.rtl && s.adaptive_navigation.text_scale === 2",
    )
    .await?;
    let (rail, page) = layout(tab, 840, 300, true).await?;
    let clip = bounds(tab, SCROLL).await?;
    let last = bounds(tab, SETTINGS).await?;
    check!(
        !contains(clip, last),
        "the last destination initially needs scroll reveal"
    );
    focus_projected(tab, HOME).await?;
    let scroll_y = |id| {
        format!(
            "JSON.parse(window.gallery.export_dom()).dom.nodes
                .find(node => node.stable_id === {id:?})?.style.scroll_y"
        )
    };
    let page_scroll = scroll_y(PAGE);
    let rail_scroll = scroll_y(SCROLL);
    let page_before = tab.eval(&page_scroll).await?;
    let rail_before = tab
        .eval(&rail_scroll)
        .await?
        .as_f64()
        .context("read the rail's initial scroll offset")?;
    let editor_before = bounds(tab, EDITOR).await?;
    for target in [SAVED, UPDATES, SETTINGS] {
        tab.press("ArrowDown").await?;
        focused(tab, target).await?;
        current(tab, UPDATES, 2).await?;
    }
    let revealed = bounds(tab, SETTINGS).await?;
    let rail_after = tab
        .eval(&rail_scroll)
        .await?
        .as_f64()
        .context("read the rail's final scroll offset")?;
    check!(
        contains(clip, revealed) && revealed[1] < last[1] && rail_after > rail_before,
        "keyboard focus reveals the bottom inside the rail clip"
    );
    check_eq!(
        (
            bounds(tab, PAGE).await?,
            bounds(tab, EDITOR).await?,
            tab.eval(&page_scroll).await?
        ),
        (page, editor_before, page_before.clone()),
        "rail scrolling preserves the page position, its editor and its scroll offset"
    );
    until(tab, &format!("s.adaptive_navigation.draft === {TEXT:?}")).await?;
    let image = screenshot(env, tab, "dark-rtl-short-rail", rail, true).await?;
    Ok(json!({
        "paint":image, "before":last, "revealed":revealed, "clip":clip, "focus":SETTINGS,
        "rail_scroll":[rail_before,rail_after], "page":page, "page_scroll":page_before,
        "editor":editor_before
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2444, 2.).await?;
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    tab.goto(url).await?;
    crate::smoke::ready(&tab, "gallery").await?;
    resize(&tab, 960, 2400).await?;
    activate(&tab, "section-3").await?;
    until(&tab, "s.section === 3").await?;
    activate(&tab, SHOW).await?;
    focused(&tab, BACK).await?;
    step(
        STEPS[0],
        "real keys and the current page survive the compact to medium bar",
    );
    let bars = bars(env, &tab).await?;
    step(
        STEPS[1],
        "839 to 840 dp moves navigation to a leading rail with arrow focus",
    );
    let rail = rail(env, &tab).await?;
    step(
        STEPS[2],
        "the same live draft, keyboard focus and backward selection survive all three layouts",
    );
    let draft = draft(&tab).await?;
    step(
        STEPS[3],
        "dark RTL short rail reveals focused destinations without scrolling the page",
    );
    let dark = compact_height(env, &tab).await?;
    env.write_json(
        &format!("{STEM}.json"),
        &json!({"bars":bars,"rail":rail,"draft":draft,"dark":dark}),
    )
}
