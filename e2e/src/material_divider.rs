//! Decorative divider paint and keyboard traversal on headless Chrome WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_tree, bounds, contains, focus_projected, focused, projected, resize, Raster,
};
use super::until;
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const SHOW: &str = "divider-demo-show";
const BACK: &str = "divider-demo-back";
const THEME: &str = "divider-demo-theme";
const RTL: &str = "divider-demo-rtl";
const LARGE_TEXT: &str = "divider-demo-large-text";
const WEIGHT: &str = "divider-demo-weight";

type Rect = [f64; 4];

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1004).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 960).await?;
    activate(tab, "section-3").await?;
    until(tab, "s.section === 3").await?;
    activate(tab, SHOW).await?;
    focused(tab, BACK).await
}

fn ink(
    raster: &Raster,
    rect: Rect,
    vertical: bool,
    thickness: u32,
    expected: [u8; 3],
) -> Result<Value> {
    let [x, y, width, height] = rect;
    // At DPR 2 the square quad can antialias its last two physical pixels
    // at a corner. Stay three pixels from each end and keep every depth.
    let end_inset = 1.5;
    for offset in 0..thickness {
        let depth = f64::from(offset) + 0.5;
        let points = if vertical {
            [
                [x + depth, y + end_inset],
                [x + depth, y + height / 2.],
                [x + depth, y + height - end_inset],
            ]
        } else {
            [
                [x + end_inset, y + depth],
                [x + width / 2., y + depth],
                [x + width - end_inset, y + depth],
            ]
        };
        for [x, y] in points {
            check_eq!(raster.sample(x, y)?, expected, "PNG uses outline variant");
        }
    }
    let adjacent = if vertical {
        [
            [x - 0.5, y + height / 2.],
            [x + width + 0.5, y + height / 2.],
        ]
    } else {
        [
            [x + width / 2., y - 0.5],
            [x + width / 2., y + height + 0.5],
        ]
    };
    for [x, y] in adjacent {
        check!(
            raster.sample(x, y)? != expected,
            "line has no extra ink outside its measured thickness"
        );
    }
    Ok(json!({"bounds":rect,"rgb":expected}))
}

async fn profile(env: &Env, tab: &Tab, compact: bool, heavy: bool) -> Result<Value> {
    let width = if compact { 320 } else { 960 };
    let height = 800;
    let thickness = if heavy { 8 } else { 1 };
    resize(tab, width, height).await?;
    focus_projected(tab, BACK).await?;
    for control in [THEME, RTL, LARGE_TEXT, WEIGHT] {
        tab.press("Tab").await?;
        focused(tab, control).await?;
    }
    focus_projected(tab, BACK).await?;
    let full = bounds(tab, "divider-demo-full").await?;
    let inset = bounds(tab, "divider-demo-inset").await?;
    let middle = bounds(tab, "divider-demo-middle").await?;
    let vertical = bounds(tab, "divider-demo-vertical").await?;
    let exposed = ax_tree(tab).await?;
    for label in ["全寬", "單側內縮", "兩側內縮", "垂直"] {
        ax(&exposed, "heading", label)?;
    }
    check!(
        !exposed
            .iter()
            .any(|node| node["ignored"] == false && node["role"]["value"] == "separator"),
        "Chrome does not announce decorative lines"
    );
    for id in [
        "divider-demo-full",
        "divider-demo-inset",
        "divider-demo-middle",
        "divider-demo-vertical",
    ] {
        check_eq!(
            tab.eval(&format!("{} === null", projected(id))).await?,
            true,
            "the DOM accessibility proxy omits {id}"
        );
    }
    for rect in [full, inset, middle, vertical] {
        check!(
            contains([0., 0., f64::from(width), f64::from(height)], rect),
            "{rect:?} lies within the viewport"
        );
    }
    check_eq!(
        [full[3], inset[3], middle[3], vertical[2], vertical[3]],
        [
            f64::from(thickness),
            f64::from(thickness),
            f64::from(thickness),
            f64::from(thickness),
            64.
        ],
        "all divider layouts use the selected thickness"
    );
    check_eq!(
        [
            inset[0] - full[0],
            full[2] - inset[2],
            middle[0] - full[0],
            full[2] - middle[2]
        ],
        [if compact { 0. } else { 16. }, 16., 16., 32.],
        "only the single leading inset mirrors"
    );

    let name = match (compact, heavy) {
        (true, true) => "compact-dark-rtl-200-heavy",
        (true, false) => "compact-dark-rtl-200",
        (false, true) => "wide-light-ltr-heavy",
        (false, false) => "wide-light-ltr",
    };
    let path = env.artifact(&format!("material-divider-{name}.png"));
    tab.screenshot(&path).await?;
    let raster = Raster::read(tab, path).await?;
    let palette = if compact {
        [73, 69, 78]
    } else {
        [202, 196, 207]
    };
    Ok(json!({
        "profile":name,"viewport":[width,height],"accessible_groups":true,"decorative_focus":false,
        "thickness_dp":thickness,
        "full":ink(&raster,full,false,thickness,palette)?,"inset":ink(&raster,inset,false,thickness,palette)?,
        "middle":ink(&raster,middle,false,thickness,palette)?,"vertical":ink(&raster,vertical,true,thickness,palette)?,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1004, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step("M264", "wide light dividers paint the named layouts without adding focus or AX stops");
        route(&tab, url).await.context("M264")?;
        report["adapter"] = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter();if(!a)return null;const {isFallbackAdapter,vendor,architecture}=a.info;return {isFallbackAdapter,vendor,architecture};})()").await?;
        check!(
            report["adapter"]["isFallbackAdapter"].is_boolean(),
            "WebGPU adapter reports fallback status: {}",
            report["adapter"]
        );
        report["wide"] = profile(env, &tab, false, false).await.context("M264")?;
        step("M265", "320 dp dark RTL and 200 percent text keep the four regular divider layouts visible");
        for control in [THEME, RTL, LARGE_TEXT] {
            activate(&tab, control).await?;
        }
        until(&tab, "s.dark").await?;
        report["compact"] = profile(env, &tab, true, false).await.context("M265")?;
        step("M266", "Heavy dividers paint eight dp in every layout before and after reflow");
        activate(&tab, WEIGHT).await?;
        report["compact_heavy"] = profile(env, &tab, true, true).await.context("M266")?;
        resize(&tab, 960, 800).await?;
        for control in [THEME, RTL, LARGE_TEXT] {
            activate(&tab, control).await?;
        }
        until(&tab, "!s.dark").await?;
        report["wide_heavy"] = profile(env, &tab, false, true).await.context("M266")?;
        activate(&tab, WEIGHT).await?;
        check_eq!(bounds(&tab, "divider-demo-full").await?[3], 1., "returns to regular thickness");
        resize(&tab, 960, 960).await?;
        activate(&tab, BACK).await?;
        focused(&tab, SHOW).await?;
        report["focus_restored"] = json!(true);
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }.await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = super::state(&tab).await.unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-divider-failure.png"))
            .await;
    }
    env.write_json("browser-material-divider.json", &report)?;
    result
}
