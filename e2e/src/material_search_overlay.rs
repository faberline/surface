use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focused, near, projected, resize,
};
use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

#[path = "material_search_overlay_motion.rs"]
mod motion;
#[path = "material_search_overlay_rtl.rs"]
mod rtl;
use super::material_web_contrast_png::{CanvasPng, Rect};

const INPUT: &str = "search-overlay-demo-input";
const OPENER: &str = "search-overlay-demo-opener";
const ANCHOR: &str = "search-overlay-demo-anchor";
const DIALOG: &str = "search-overlay-demo-dialog";
const BAR: &str = "search-overlay-demo-bar";
const RESULTS: &str = "search-overlay-demo-results";
const DIVIDER: &str = "search-overlay-demo-divider";
const BACK: &str = "search-overlay-demo-back";
const CLEAR: &str = "search-overlay-demo-clear";
const DESIGN: &str = "search-doc-design";
const REVIEW: &str = "search-doc-review";
const ARCHIVE: &str = "search-doc-archive";
const EDITOR: &str = "document.getElementById('surface-text-input')";

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1844).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    activate(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    activate(tab, "search-overlay-demo-show").await?;
    tab.wait_for(&format!("{} !== null", projected(OPENER)))
        .await
}

async fn open(tab: &Tab) -> Result<()> {
    activate(tab, OPENER).await?;
    until(tab, "s.search_overlay.open && s.search_overlay.motion.progress === 1 && !s.search_overlay.motion.moving").await?;
    edit_focus(tab).await
}

async fn edit_focus(tab: &Tab) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(INPUT))).await?;
    tab.wait_for(&format!(
        "document.activeElement === {EDITOR} && {EDITOR}.getAttribute('aria-label') === '搜尋文件'"
    ))
    .await?;
    tab.settle().await
}

async fn close(tab: &Tab) -> Result<()> {
    tab.press("Escape").await?;
    until(tab, "!s.search_overlay.open && s.search_overlay.motion.progress === 0 && !s.search_overlay.motion.moving").await?;
    focused(tab, OPENER).await
}

async fn docked_ready(tab: &Tab, width: u32) -> Result<()> {
    tab.wait_for(&format!(
        "(() => {{ const r = window.gallery.control_bounds({}); return r.length === 4 && r[1] > 0 && r[2] >= 360 && r[2] < {width}; }})()",
        js(DIALOG)
    )).await?;
    tab.settle().await
}

fn inside(outer: Rect, inner: Rect, label: &str) -> Result<()> {
    check!(
        contains(outer, inner),
        "{label}: outer={outer:?}, inner={inner:?}"
    );
    Ok(())
}

async fn paint(env: &Env, tab: &Tab, name: &str) -> Result<CanvasPng> {
    CanvasPng::capture(env, tab, &format!("search-{name}")).await
}

async fn geometry(env: &Env, tab: &Tab, url: &str, compact: bool) -> Result<Value> {
    route(tab, url).await?;
    let (width, height) = if compact { (320, 400) } else { (960, 800) };
    if compact {
        for suffix in ["theme", "rtl", "large-text"] {
            activate(tab, &format!("search-overlay-demo-{suffix}")).await?;
        }
        until(
            tab,
            "s.dark && s.search_overlay.rtl && s.search_overlay.text_scale === 2",
        )
        .await?;
    }
    resize(tab, width, height).await?;
    let viewport = [0., 0., width.into(), height.into()];
    let page = paint(
        env,
        tab,
        if compact {
            "compact-rest"
        } else {
            "docked-rest"
        },
    )
    .await?;
    let probe = [f64::from(width) - 5., f64::from(height) - 5.];
    let behind = page.sample(probe[0], probe[1])?;
    let mut profiles = Vec::new();
    for divided in [false, true] {
        if divided {
            activate(tab, "search-overlay-demo-style").await?;
            until(tab, "s.search_overlay.style === 'Divided'").await?;
        }
        let anchor = bounds(tab, ANCHOR).await?;
        let opener = bounds(tab, OPENER).await?;
        if !divided {
            check!(
                near(opener[0] - anchor[0], 24.),
                "resting Contained has 24 dp side inset: {opener:?}, {anchor:?}"
            );
        }
        open(tab).await?;
        let dialog = bounds(tab, DIALOG).await?;
        let bar = bounds(tab, BAR).await?;
        let results = bounds(tab, RESULTS).await?;
        let input = bounds(tab, INPUT).await?;
        let back = bounds(tab, BACK).await?;
        for (rect, id) in [(bar, BAR), (results, RESULTS), (input, INPUT), (back, BACK)] {
            inside(dialog, rect, id)?;
        }
        check!(
            near(back[2], 48.) && near(back[3], 48.) && near(input[3], 48.),
            "stable 48 dp editor and back hit targets: {input:?}, {back:?}"
        );
        if compact {
            check_eq!(dialog, viewport, "compact dialog fills the viewport");
            check!(
                back[0] > input[0],
                "RTL puts Back on physical right: {back:?}, {input:?}"
            );
            check!(
                near(bar[3], if divided { 72. } else { 56. }),
                "compact divided/contained exact top row height: {bar:?}"
            );
        } else {
            inside(viewport, dialog, DIALOG)?;
            check!(
                (360. ..=720.).contains(&dialog[2])
                    && dialog[3] >= 240.
                    && dialog[3] <= f64::from(height) * 2. / 3.
                    && near(dialog[1], anchor[1]),
                "docked bounded to anchor and standard min/max: {dialog:?}, {anchor:?}"
            );
            check!(
                near(dialog[0] - anchor[0], if divided { 0. } else { 12. }) && near(bar[3], 56.),
                "docked fills anchor except 12 dp Contained inset: {bar:?}, {dialog:?}, {anchor:?}"
            );
        }
        let variant = if divided { "divided" } else { "contained" };
        let png = paint(
            env,
            tab,
            &format!(
                "{}-{variant}",
                if compact {
                    "compact-dark-rtl"
                } else {
                    "docked-light"
                }
            ),
        )
        .await?;
        let overlay_pixel = png.sample(probe[0], probe[1])?;
        if !compact {
            check!(
                behind.iter().zip(overlay_pixel).all(|(a, b)| *a > b),
                "real docked scrim darkens background: {behind:?} -> {overlay_pixel:?}"
            );
        }
        let detail = if divided {
            let line = bounds(tab, DIVIDER).await?;
            let ink = png.sample(line[0] + line[2] * 0.6, line[1] + 0.5)?;
            let above = png.sample(line[0] + line[2] * 0.6, line[1] - 3.)?;
            check!(
                near(line[3], 1.) && ink != above,
                "painted 1 dp divider separates surfaces: {line:?}, {ink:?}, {above:?}"
            );
            json!({"divider":line,"pixel":ink,"above_pixel":above})
        } else {
            let x = bar[0] + bar[2] * 0.65;
            let center = png.sample(x, bar[1] + bar[3] - 3.)?;
            let gap = png.sample(x, bar[1] + bar[3] + 1.)?;
            check!(
                compact || near(results[1] - bar[1] - bar[3], 6.),
                "Contained has a 2 dp gap plus results inset: {bar:?}, {results:?}"
            );
            check!(
                center != gap,
                "rounded pill paint and gap have different RGB: {center:?}, {gap:?}"
            );
            json!({"pill_pixel":center,"gap_pixel":gap})
        };
        profiles.push(json!({"style":variant,"dimensions":png.dimensions(),"anchor":anchor,"dialog":dialog,"bar":bar,"results":results,"input":input,"back":back,"outside_before":behind,"outside_after":overlay_pixel,"detail":detail}));
        close(tab).await?;
    }
    Ok(json!({"profiles":profiles,"errors":tab.errors()}))
}

async fn editing(tab: &Tab) -> Result<Value> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]",
        js(INPUT)
    ))
    .await
}

async fn browser_editor(tab: &Tab) -> Result<Value> {
    tab.eval(&format!(
        "(() => {{ const e={EDITOR}; return {{same:e===window.__searchRetainedEditor,value:e.value,start:e.selectionStart,end:e.selectionEnd,direction:e.selectionDirection}}; }})()"
    )).await
}

async fn set_query(tab: &Tab, value: &str) -> Result<()> {
    edit_focus(tab).await?;
    tab.shortcut("a").await?;
    if value.is_empty() {
        tab.press("Backspace").await?;
    } else {
        tab.insert_text(value).await?;
    }
    until(tab, &format!("s.search_overlay.query === {}", js(value))).await?;
    tab.settle().await
}

async fn adaptive(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, 599, 720).await?;
    open(tab).await?;
    check_eq!(
        bounds(tab, DIALOG).await?,
        [0., 0., 599., 720.],
        "599 dp opens Full-screen"
    );
    let row = bounds(tab, DESIGN).await?;
    let results = bounds(tab, RESULTS).await?;
    let p = point(tab, RESULTS).await?;
    tab.mouse_move(p.x, p.y).await?;
    tab.wheel(0., 110.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({})[1] < {}",
        js(DESIGN),
        row[1] - 48.
    ))
    .await?;
    let offset = results[1] - bounds(tab, DESIGN).await?[1];
    resize(tab, 600, 720).await?;
    docked_ready(tab, 600).await?;
    let docked = bounds(tab, DIALOG).await?;
    check!(
        docked[0] > 0. && docked[1] > 0. && docked[2] < 600.,
        "600 dp opens Docked at real anchor: {docked:?}"
    );
    let offset_after = bounds(tab, RESULTS).await?[1] - bounds(tab, DESIGN).await?[1];
    check!(
        near(offset_after, offset),
        "physical wheel scroll survives 599→600: {offset} -> {offset_after}"
    );
    resize(tab, 599, 720).await?;
    set_query(tab, "設計").await?;
    tab.press("Shift+ArrowLeft").await?;
    tab.wait_for(&format!(
        "{EDITOR}.selectionStart === 1 && {EDITOR}.selectionEnd === 2"
    ))
    .await?;
    tab.settle().await?;
    tab.eval(&format!("window.__searchRetainedEditor={EDITOR}; true"))
        .await?;
    let browser_before = browser_editor(tab).await?;
    let model_before = editing(tab).await?;
    check!(
        model_before.is_object(),
        "Runtime exposes actual text editor snapshot: {model_before}"
    );
    resize(tab, 600, 720).await?;
    docked_ready(tab, 600).await?;
    edit_focus(tab).await?;
    check_eq!(
        browser_editor(tab).await?,
        browser_before,
        "same DOM textarea and native selection direction survive 599→600"
    );
    check_eq!(
        editing(tab).await?,
        model_before,
        "Runtime editor and selection survive 599→600"
    );

    resize(tab, 599, 720).await?;
    tab.eval(&format!(
        "(() => {{const e={EDITOR}; window.__searchEditTrace = [];
         for (const n of ['keydown','beforeinput','input','compositionstart','compositionupdate','compositionend'])
            e.addEventListener(n,event => window.__searchEditTrace.push({{type:n,key:event.key,data:event.data,kind:event.inputType,isComposing:event.isComposing,value:e.value,start:e.selectionStart,end:e.selectionEnd}}));
         return true;}})()"
    )).await?;
    tab.ime_compose("ㄐㄧ", 0, 2).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(INPUT)
    ))
    .await?;
    tab.settle().await?;
    let composing = editing(tab).await?;
    let browser_composing = browser_editor(tab).await?;
    resize(tab, 600, 720).await?;
    docked_ready(tab, 600).await?;
    edit_focus(tab).await?;
    check_eq!(
        editing(tab).await?,
        composing,
        "active synthetic preedit survives layout switch"
    );
    check_eq!(
        browser_editor(tab).await?,
        browser_composing,
        "same browser editor value/selection survive active preedit resize"
    );
    let png = paint(env, tab, "resize-pending-composition").await?;
    tab.press("Enter").await?;
    until(
        tab,
        "s.search_overlay.submissions === 0 && s.search_overlay.open",
    )
    .await?;
    check_eq!(
        browser_editor(tab).await?,
        browser_composing,
        "trusted Enter does not put a newline in the single-line composition or consume its selection"
    );
    tab.insert_text("定").await?;
    until(tab, "s.search_overlay.query === '設定'").await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === false",
        js(INPUT)
    ))
    .await?;
    tab.settle().await?;
    check_eq!(
        state(tab).await?["search_overlay"]["submissions"],
        0,
        "synthetic commit only edits query"
    );
    tab.press("Enter").await?;
    until(
        tab,
        "s.search_overlay.submissions === 1 && s.search_overlay.open",
    )
    .await?;
    let web_trace = tab.eval("window.__searchEditTrace").await?;
    close(tab).await?;
    Ok(
        json!({"docked":docked,"scroll_before":offset,"scroll_after":offset_after,"selection":browser_before,"runtime":model_before,"preedit":composing,"browser_preedit":browser_composing,"png":png.dimensions(),"web_trace":web_trace}),
    )
}

async fn interaction(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, 960, 800).await?;
    open(tab).await?;
    let tree = ax_tree(tab).await?;
    let modal = ax(&tree, "dialog", "搜尋工作區")?;
    check_eq!(
        ax_property(modal, "modal"),
        Some(&json!(true)),
        "Chrome AX marks expanded Search as modal"
    );
    let editor = ax(&tree, "textbox", "搜尋文件")?;
    check_eq!(
        ax_property(editor, "focused"),
        Some(&json!(true)),
        "Chrome AX exposes named and focused native editor"
    );
    let disabled = ax(&tree, "button", "設計封存")?;
    check_eq!(
        ax_property(disabled, "disabled"),
        Some(&json!(true)),
        "Chrome AX exposes disabled result"
    );
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-disabled')",
            projected(ARCHIVE)
        ))
        .await?,
        "true",
        "disabled result stays exposed"
    );
    check_eq!(
        tab.eval(&format!("{} === null", projected(OPENER))).await?,
        true,
        "modal hides opener from Web accessibility"
    );
    edit_focus(tab).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, BACK).await?;
    tab.press("Tab").await?;
    edit_focus(tab).await?;
    set_query(tab, "設計").await?;
    tab.press("Tab").await?;
    focused(tab, CLEAR).await?;
    tab.press("Tab").await?;
    focused(tab, DESIGN).await?;
    tab.press("ArrowDown").await?;
    focused(tab, REVIEW).await?;
    tab.press("Tab").await?;
    focused(tab, BACK).await?;
    tab.press("Enter").await?;
    until(tab, "!s.search_overlay.open").await?;
    focused(tab, OPENER).await?;

    open(tab).await?;
    tab.press("Tab").await?;
    focused(tab, CLEAR).await?;
    tab.press("Tab").await?;
    focused(tab, DESIGN).await?;
    tab.press("ArrowDown").await?;
    focused(tab, REVIEW).await?;
    tab.press("Enter").await?;
    until(tab,"!s.search_overlay.open && s.search_overlay.selected === 'search-doc-review' && s.search_overlay.submissions === 0").await?;
    focused(tab, OPENER).await?;

    open(tab).await?;
    set_query(tab, "無匹配").await?;
    let status = projected("search-overlay-demo-status");
    tab.wait_for(&format!("({status}?.textContent.includes('找不到相符文件') === true) || Array.from(document.querySelectorAll('[aria-live]')).some(n => n.textContent.includes('找不到相符文件'))")).await?;
    bounds(tab, "search-overlay-demo-empty").await?;
    paint(env, tab, "empty-chrome-ax").await?;
    tab.press("Tab").await?;
    focused(tab, CLEAR).await?;
    tab.press("Enter").await?;
    until(
        tab,
        "s.search_overlay.query === '' && s.search_overlay.result_ids.length === 12",
    )
    .await?;
    edit_focus(tab).await?;
    let anchor = point(tab, "search-overlay-demo-style").await?;
    tab.click(anchor.x, anchor.y).await?;
    until(
        tab,
        "!s.search_overlay.open && s.search_overlay.style === 'Contained'",
    )
    .await?;
    until(tab, &format!("s.focused === {}", js(OPENER))).await?;
    open(tab).await?;
    click(tab, ARCHIVE).await?;
    check_eq!(
        state(tab).await?["search_overlay"]["selected"],
        REVIEW,
        "actual disabled result click does not replace selected ID"
    );
    check_eq!(
        state(tab).await?["search_overlay"]["open"],
        true,
        "disabled click keeps modal open"
    );
    click(tab, BACK).await?;
    until(tab, "!s.search_overlay.open").await?;
    until(tab, &format!("s.focused === {}", js(OPENER))).await?;
    Ok(
        json!({"dialog":modal,"textbox":editor,"disabled":disabled,"stable_result":"search-doc-review","empty_status":"找不到相符文件"}),
    )
}

async fn navigation(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    resize(tab, 960, 800).await?;
    open(tab).await?;
    tab.press("ArrowDown").await?;
    focused(tab, DESIGN).await?;
    check_eq!(
        state(tab).await?["search_overlay"]["submissions"],
        0,
        "plain ArrowDown moves directly to first enabled history item without submitting"
    );
    close(tab).await?;
    open(tab).await?;
    set_query(tab, "封存").await?;
    until(tab,"s.search_overlay.result_ids.length === 1 && s.search_overlay.result_ids[0] === 'search-doc-archive'").await?;
    tab.press("ArrowDown").await?;
    edit_focus(tab).await?;
    set_query(tab, "無匹配").await?;
    tab.press("ArrowDown").await?;
    edit_focus(tab).await?;
    set_query(tab, "設計").await?;
    tab.press("Shift+ArrowDown").await?;
    edit_focus(tab).await?;
    tab.ime_compose("ㄕ", 0, 1).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing",
        js(INPUT)
    ))
    .await?;
    tab.press("ArrowDown").await?;
    edit_focus(tab).await?;
    let preedit = editing(tab).await?;
    check_eq!(
        preedit["composing"],
        true,
        "trusted Down does not move focus during synthetic preedit"
    );
    tab.ime_compose("", 0, 0).await?;
    set_query(tab, "H").await?;
    let input = bounds(tab, INPUT).await?;
    let colors = state(tab).await?["display_contrast"]["theme"].clone();
    let rgba = colors["on_surface"]
        .as_array()
        .context("Search ink color")?;
    let ink: [u8; 3] = [
        rgba[0].as_u64().context("red")?.try_into()?,
        rgba[1].as_u64().context("green")?.try_into()?,
        rgba[2].as_u64().context("blue")?.try_into()?,
    ];
    let caret=tab.eval_f64s(&format!(
        "(() => {{ const caret={EDITOR}.getBoundingClientRect(), canvas=document.querySelector('canvas').getBoundingClientRect(); return [caret.x-canvas.x,caret.y-canvas.y,caret.width,caret.height] }})()"
    )).await?;
    let png = paint(env, tab, "live-centered-editor").await?;
    let left = ((input[0] + 3.) * 2.).ceil() as usize;
    let right = ((caret[0] - 1.) * 2.).floor() as usize;
    let top = ((input[1] + 3.) * 2.).ceil() as usize;
    let bottom = ((input[1] + input[3] - 3.) * 2.).floor() as usize;
    let mut ys = Vec::new();
    for y in top..bottom {
        for x in left..right {
            if png.pixel(x, y)? == ink {
                ys.push(y);
            }
        }
    }
    let first = ys.first().context("painted H upper ink edge")?;
    let last = ys.last().context("painted H lower ink edge")?;
    let ink_midpoint = (*first as f64 + *last as f64) / 4.;
    let center = input[1] + input[3] / 2.;
    check!(
        ys.len() >= 20 && (ink_midpoint - center).abs() <= 5.,
        "real glyph ink vertically centered inside stable editor: {ink_midpoint}/{center}, {} pixels",
        ys.len()
    );
    check!(
        (caret[1] + caret[3] / 2. - center).abs() <= 1.,
        "browser caret proxy agrees with centered hit box: {caret:?}, {input:?}"
    );
    close(tab).await?;
    Ok(
        json!({"direct_first":DESIGN,"disabled_only_held":true,"empty_results_held":true,"modified_held":true,"preedit":preedit,"glyph_ink":[first,last],"glyph_pixels":ys.len(),"glyph_center":ink_midpoint,"input":input,"browser_caret":caret,"dimensions":png.dimensions()}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 844, 2.).await?;
    let mut result = json!({"browser":chrome.version});
    let run: Result<()> = async {
        step("M248","docked Contained and Divided styles draw real light scrim, gap and divider");
        result["docked"] = geometry(env, &tab, url, false).await.context("M248")?;
        let adapter = tab.eval("(async()=>{const a=await navigator.gpu?.requestAdapter(); if(!a)return null; const {isFallbackAdapter,vendor,architecture}=a.info; return {isFallbackAdapter,vendor,architecture};})()").await?;
        check_eq!(adapter["isFallbackAdapter"],false,"actual non-fallback WebGPU adapter");
        result["adapter"] = adapter;
        step("M249","320 dp Dark RTL 200% Full-screen Contained and Divided preserve the 48 dp input/navigation controls");
        result["compact"] = geometry(env,&tab,url,true).await.context("M249")?;
        step("M250","real wheel, native selection and synthetic preedit survive the 599→600 dp breakpoint");
        result["adaptive"] = adaptive(env,&tab,url).await.context("M250")?;
        step("M251","Chrome AX, trusted Tab, real disabled pointer, result ID and scrim dismissal");
        result["interaction"] = interaction(env,&tab,url).await.context("M251")?;
        step("M252","ArrowDown jumps to first enabled result only, and Canvas glyph and textarea caret center");
        result["navigation"] = navigation(env,&tab,url).await.context("M252")?;
        step("M253","RTL actual glyphs, pointer selection, composition and separated focus and caret");
        result["rtl"]=rtl::rtl(env,&tab,url).await.context("M253")?;
        step("M254","four Search style/presentation combinations use real Chrome system colors");
        result["forced"]=rtl::forced(env,&tab,url).await.context("M254")?;
        step("M255","real Search entry and exit move through pixels; Reduce Motion snaps to both endpoints");
        result["motion"]=motion::run(env,&tab,url).await.context("M255")?;
        check!(tab.errors().is_empty(),"no page errors: {:?}",tab.errors());
        Ok(())
    }.await;
    result["errors"] = json!(tab.errors());
    if let Err(error) = &run {
        result["failure"] = json!(format!("{error:#}"));
        result["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        result["dom_at_failure"] = tab
            .eval_json("window.gallery.export_dom()")
            .await
            .unwrap_or(Value::Null);
        result["web_input_at_failure"] = tab.eval(&format!("(() => {{const e={EDITOR}; return {{trace:window.__searchEditTrace, value:e?.value,start:e?.selectionStart,end:e?.selectionEnd,focus:document.activeElement?.id}};}})()")).await.unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-search-overlay-failure.png"))
            .await;
    }
    env.write_json("browser-material-search-overlay.json", &result)?;
    run
}
