//! Search-specific WebGPU alignment, editing, and forced colors.

use super::*;
use crate::material::material_time_input::focus_projected;
use crate::material::material_web_contrast::{contrast, media, palette};
use crate::material::material_web_contrast_png::Rgb;

const HINT: &str = "搜尋文件";

async fn node(tab: &Tab, id: &str) -> Result<Value> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})",
        js(id)
    ))
    .await
}

fn rgb(value: &Value) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(value.clone())
        .with_context(|| format!("expected an RGBA paint color: {value}"))?;
    check_eq!(a, 255, "Search ink or surface is opaque");
    Ok([r, g, b])
}

async fn caret(tab: &Tab) -> Result<Rect> {
    let values = tab.eval_f64s(&format!(
        "(() => {{const c={EDITOR}.getBoundingClientRect(), s=document.getElementById('surface').getBoundingClientRect(); return [c.x-s.x,c.y-s.y,c.width,c.height]}})()"
    )).await?;
    values
        .try_into()
        .map_err(|_| anyhow::anyhow!("caret proxy rect"))
}

async fn range(tab: &Tab, anchor: usize, cursor: usize, host: [usize; 2]) -> Result<()> {
    tab.wait_for(&format!(
        "(() => {{const input={EDITOR}, d=JSON.parse(window.gallery.export_dom()).text_inputs[{}];
            return d?.selection_anchor === {anchor} && d.selection_cursor === {cursor} &&
                input.selectionStart === {} && input.selectionEnd === {};}})()",
        js(INPUT),
        host[0],
        host[1]
    ))
    .await?;
    tab.settle().await
}

fn inset([x, y, w, h]: Rect, amount: f64) -> Rect {
    [x + amount, y + amount, w - 2. * amount, h - 2. * amount]
}

fn text_strip([x, y, w, h]: Rect) -> Rect {
    // The pill curves enter a rectangular 4dp inset at its corners. Sample
    // the central strip to isolate true glyph ink from the same-color ring.
    [x + 4., y + 16., w - 8., h - 32.]
}

fn ink_bounds(image: &CanvasPng, rect: Rect, color: Rgb) -> Result<(Rect, usize)> {
    let [x, y, w, h] = rect;
    let px = |dp: f64| (dp * 2.).round() as usize;
    let mut hit = 0;
    let (mut left, mut top, mut right, mut bottom) = (usize::MAX, usize::MAX, 0, 0);
    for y in px(y)..px(y + h) {
        for x in px(x)..px(x + w) {
            if image.pixel(x, y)? == color {
                hit += 1;
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    check!(
        hit > 20,
        "Search has a nontrivial painted glyph run in {rect:?}, {color:?}: {hit}"
    );
    Ok((
        [
            left as f64 / 2.,
            top as f64 / 2.,
            (right - left + 1) as f64 / 2.,
            (bottom - top + 1) as f64 / 2.,
        ],
        hit,
    ))
}

async fn single_editor(tab: &Tab) -> Result<Value> {
    let tree = ax_tree(tab).await?;
    let named: Vec<_> = tree
        .iter()
        .filter(|node| node["ignored"] == false && node["name"]["value"] == HINT)
        .collect();
    check_eq!(
        named.len(),
        1,
        "paint-only Search hint creates no second AX node with the editor's name"
    );
    let editor = ax(&tree, "textbox", HINT)?;
    check_eq!(
        ax_property(editor, "focused"),
        Some(&json!(true)),
        "the one accessible Search editor stays focused"
    );
    Ok(editor.clone())
}

fn assert_right(caret: Rect, input: Rect) -> Result<()> {
    let delta = input[0] + input[2] - caret[0];
    check!(
        near(delta, 6.) && near(caret[1] + caret[3] / 2., input[1] + input[3] / 2.),
        "caret proxy is on the right and centered inside its input: {caret:?} / {input:?}"
    );
    Ok(())
}

async fn clearance(env: &Env, tab: &Tab, label: &str, rtl: bool) -> Result<Value> {
    let [x, y, w, h] = bounds(tab, INPUT).await?;
    let c = caret(tab).await?;
    let bar = bounds(tab, BAR).await?;
    let ink = rgb(&node(tab, INPUT).await?["style"]["foreground"])?;
    let picture = paint(env, tab, label).await?;
    let fill = picture.sample(bar[0] + bar[2] / 2., bar[1] + 2.)?;
    let ring_x = if rtl { x + w - 0.75 } else { x + 0.75 };
    let gap_x = if rtl { c[0] + 2.5 } else { c[0] - 1. };
    let caret_x = c[0] + 0.25;
    let scan_y = y + h / 2.;
    let ring = picture.sample(ring_x, scan_y)?;
    let gap = picture.sample(gap_x, scan_y)?;
    let caret_ink = picture.sample(caret_x, scan_y)?;
    check_eq!(ring, ink, "focus stroke paints separate exact ink");
    check_eq!(caret_ink, ink, "visible one-dp caret paints exact ink");
    check_eq!(
        gap,
        fill,
        "bar background is visible between actual caret and focus stroke"
    );
    check!(
        gap != ring,
        "the gap separates the caret from the focus stroke"
    );
    check!(
        near(
            if rtl { x + w - c[0] } else { c[0] - x },
            if rtl { 6. } else { 4. }
        ),
        "real textarea caret origin respects the 4dp content inset: {c:?}, {:?}",
        [x, y, w, h]
    );
    Ok(
        json!({"input":[x,y,w,h],"caret":c,"scan_y":scan_y,"ring":[ring_x,ring],
              "gap":[gap_x,gap],"caret_ink":[caret_x,caret_ink],"background":fill}),
    )
}

async fn dark_rtl(tab: &Tab) -> Result<()> {
    for setting in ["theme", "rtl", "large-text"] {
        activate(tab, &format!("search-overlay-demo-{setting}")).await?;
    }
    until(
        tab,
        "s.dark && s.search_overlay.rtl && s.search_overlay.text_scale === 2",
    )
    .await
}

pub(super) async fn rtl(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    route(tab, url).await?;
    dark_rtl(tab).await?;
    resize(tab, 320, 400).await?;
    open(tab).await?;
    let empty_input = bounds(tab, INPUT).await?;
    let empty_caret = caret(tab).await?;
    assert_right(empty_caret, empty_input)?;
    let rtl_contained = clearance(env, tab, "caret-rtl-contained", true).await?;
    let empty_ax = single_editor(tab).await?;
    let hint_color = rgb(&node(tab, "search-doc-design-supporting").await?["style"]["foreground"])?;
    let fg = rgb(&node(tab, INPUT).await?["style"]["foreground"])?;
    check!(hint_color != fg, "the normal dark hint uses its own ink");
    check_eq!(editing(tab).await?["value"], "", "hint is not editor data");
    let empty = paint(env, tab, "rtl-empty-hint").await?;
    let (hint, hint_count) = ink_bounds(&empty, inset(empty_input, 4.), hint_color)?;
    check!(
        hint[0] >= empty_input[0] + empty_input[2] / 3.
            && empty_input[0] + empty_input[2] - (hint[0] + hint[2]) < 12.,
        "actual hint is painted next to the leading right edge: {hint:?}, {empty_input:?}"
    );

    tab.ime_compose("H", 1, 1).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing",
        js(INPUT)
    ))
    .await?;
    tab.settle().await?;
    single_editor(tab).await?;
    let empty_preedit = paint(env, tab, "rtl-empty-preedit").await?;
    check_eq!(
        empty_preedit.count_color(inset(empty_input, 4.), hint_color)?,
        0,
        "pending preedit suppresses all exact hint ink"
    );
    let mut preedit_rect = inset(empty_input, 4.);
    preedit_rect[0] += preedit_rect[2] / 2.;
    preedit_rect[2] /= 2.;
    let (empty_preedit_ink, _) = ink_bounds(&empty_preedit, preedit_rect, fg)?;
    check_eq!(
        editing(tab).await?["value"],
        "",
        "synthetic pending text never replaces the accepted query"
    );
    tab.ime_compose("", 0, 0).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === false",
        js(INPUT)
    ))
    .await?;

    set_query(tab, "設計").await?;
    let input = bounds(tab, INPUT).await?;
    let typed_caret = caret(tab).await?;
    assert_right(typed_caret, input)?;
    let typed = paint(env, tab, "rtl-typed-query").await?;
    let (typed_ink, _) = ink_bounds(&typed, text_strip(input), fg)?;
    check!(
        input[0] + input[2] - (typed_ink[0] + typed_ink[2]) < 12.
            && typed_ink[0] > input[0] + input[2] / 2.,
        "actual query glyphs sit on the right: {typed_ink:?}, {input:?}"
    );
    range(tab, 6, 6, [2, 2]).await?;
    tab.press("ArrowLeft").await?;
    range(tab, 3, 3, [1, 1]).await?;
    let middle = caret(tab).await?;
    tab.press("ArrowLeft").await?;
    range(tab, 0, 0, [0, 0]).await?;
    let start = caret(tab).await?;
    check!(typed_caret[0]-middle[0]>20. && middle[0]-start[0]>20.,
        "both visible logical advances have the measured RTL-aligned caret x: {start:?}/{middle:?}/{typed_caret:?}");
    let canvas = tab.canvas_box().await?;
    let y = canvas[1] + input[1] + input[3] / 2.;
    tab.click(canvas[0] + typed_caret[0] - 1., y).await?;
    edit_focus(tab).await?;
    range(tab, 6, 6, [2, 2]).await?;
    tab.mouse_move(canvas[0] + start[0] + 1., y).await?;
    tab.mouse_down().await?;
    for x in [canvas[0] + middle[0], canvas[0] + typed_caret[0] - 1.] {
        tab.cdp(
            "Input.dispatchMouseEvent",
            json!({"type":"mouseMoved","x":x,"y":y,"button":"left","buttons":1}),
        )
        .await?;
    }
    tab.cdp("Input.dispatchMouseEvent",json!({"type":"mouseReleased","x":canvas[0]+typed_caret[0]-1.,"y":y,"button":"left","buttons":0,"clickCount":1})).await?;
    range(tab, 0, 6, [0, 2]).await?;
    let actual_pointer = editing(tab).await?;
    let selection = paint(env, tab, "rtl-pointer-selection").await?;
    let focus = single_editor(tab).await?;
    tab.ime_compose("H", 0, 1).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.display_value === 'H'",
        js(INPUT)
    ))
    .await?;
    let editing_preedit = editing(tab).await?;
    check_eq!(
        editing_preedit["value"],
        "設計",
        "selected preedit keeps the committed query while replacing its paint"
    );
    let active = single_editor(tab).await?;
    let preedit = paint(env, tab, "rtl-selected-preedit").await?;
    let obsolete_left = [typed_ink[0] - 1., input[1] + 16., 20., input[3] - 32.];
    check_eq!(
        preedit.count_color(obsolete_left, fg)?,
        0,
        "selected preedit no longer paints the replaced query's left glyph"
    );
    click(tab, CLEAR).await?;
    check_eq!(
        editing(tab).await?,
        editing_preedit,
        "real Clear press during composition preserves and keeps the editor focused"
    );
    tab.ime_compose("", 0, 0).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === false",
        js(INPUT)
    ))
    .await?;
    click(tab, CLEAR).await?;
    until(
        tab,
        "s.search_overlay.query === '' && s.search_overlay.submissions === 0",
    )
    .await?;
    edit_focus(tab).await?;
    let cleared_caret = caret(tab).await?;
    assert_right(cleared_caret, bounds(tab, INPUT).await?)?;
    let cleared = paint(env, tab, "rtl-clear-restores-hint").await?;
    let (cleared_hint, cleared_count) = ink_bounds(&cleared, inset(empty_input, 4.), hint_color)?;
    check_eq!(
        (cleared_hint, cleared_count),
        (hint, hint_count),
        "Clear restores the same RTL hint paint"
    );
    // A preceding canvas pointer action keeps canvas focus ownership.
    tab.press("Escape").await?;
    until(tab, "!s.search_overlay.open && s.search_overlay.motion.progress === 0 && !s.search_overlay.motion.moving && s.focused === 'search-overlay-demo-opener'").await?;
    activate(tab, "search-overlay-demo-style").await?;
    open(tab).await?;
    let rtl_divided = clearance(env, tab, "caret-rtl-divided", true).await?;

    route(tab, url).await?;
    resize(tab, 960, 800).await?;
    open(tab).await?;
    let ltr_contained = clearance(env, tab, "caret-ltr-contained", false).await?;
    set_query(tab, "H").await?;
    let ltr_input = bounds(tab, INPUT).await?;
    let ltr_caret = caret(tab).await?;
    let ltr = paint(env, tab, "ltr-start-reference").await?;
    let ltr_fg = rgb(&node(tab, INPUT).await?["style"]["foreground"])?;
    let (ltr_run, _) = ink_bounds(&ltr, text_strip(ltr_input), ltr_fg)?;
    check!(near(ltr_caret[0]-ltr_input[0]-4.,11.640625) && ltr_run[0]<ltr_input[0]+14.,
        "LTR glyph advance is unchanged within the deliberate 4dp inset: {ltr_caret:?}, {ltr_run:?}, {ltr_input:?}");
    set_query(tab, "").await?;
    close(tab).await?;
    activate(tab, "search-overlay-demo-style").await?;
    open(tab).await?;
    let ltr_divided = clearance(env, tab, "caret-ltr-divided", false).await?;
    Ok(
        json!({"empty_input":empty_input,"empty_caret":empty_caret,"hint":hint,"hint_pixels":hint_count,
        "empty_ax":empty_ax,"empty_preedit_ink":empty_preedit_ink,"typed":typed_ink,
        "start":start,"middle":middle,"end":typed_caret,"pointer":actual_pointer,
        "selection_png":selection.dimensions(),"selection_ax":focus,"preedit":editing_preedit,
        "preedit_ax":active,"clear":cleared_caret,"ltr_input":ltr_input,"ltr_caret":ltr_caret,"ltr_ink":ltr_run,
        "caret_clearance":{"rtl_contained":rtl_contained,"rtl_divided":rtl_divided,
                          "ltr_contained":ltr_contained,"ltr_divided":ltr_divided}}),
    )
}

pub(super) async fn forced(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let mut variants = Vec::new();
    for compact in [false, true] {
        for divided in [false, true] {
            media(tab, false, false).await?;
            route(tab, url).await?;
            if compact {
                dark_rtl(tab).await?;
            }
            if divided {
                activate(tab, "search-overlay-demo-style").await?;
            }
            let (width, height) = if compact { (320, 400) } else { (960, 800) };
            resize(tab, width, height).await?;
            open(tab).await?;
            let before = contrast(tab, "normal").await?;
            let profile = format!(
                "{}-{}",
                if compact {
                    "full-rtl-dark"
                } else {
                    "docked-light"
                },
                if divided { "divided" } else { "contained" }
            );
            let normal = paint(env, tab, &format!("{profile}-normal")).await?;
            let input = bounds(tab, INPUT).await?;
            let bar = bounds(tab, BAR).await?;
            let dialog = bounds(tab, DIALOG).await?;
            let results = bounds(tab, RESULTS).await?;
            let before_color = normal.sample(bar[0] + bar[2] * 0.5, bar[1] + 3.)?;
            let before_editor = editing(tab).await?;

            media(tab, true, false).await?;
            let appearance = contrast(tab, "forced").await?;
            let css = palette(tab, &appearance).await?;
            let pairs = &css["pairs"];
            let fg = rgb(&pairs["canvas"]["foreground"])?;
            let bg = rgb(&pairs["canvas"]["background"])?;
            check!(fg != bg, "Canvas and CanvasText are distinct: {css}");
            check_eq!(
                bounds(tab, DIALOG).await?,
                dialog,
                "forced colors keep dialog placement and size"
            );
            check_eq!(
                bounds(tab, INPUT).await?,
                input,
                "forced colors keep editor geometry"
            );
            check_eq!(
                editing(tab).await?,
                before_editor,
                "forced colors preserve the editor buffer"
            );
            edit_focus(tab).await?;
            let forced = paint(env, tab, &format!("{profile}-forced")).await?;
            let fill = forced.sample(bar[0] + bar[2] * 0.5, bar[1] + 3.)?;
            check_eq!(fill, bg, "Canvas colors the actual search bar");
            check!(
                before_color != fill,
                "entering forced colors changes the real authored bar pixel"
            );
            let (hint, hint_pixels) = ink_bounds(&forced, text_strip(input), fg)?;
            single_editor(tab).await?;
            let [x, y, w, h] = input;
            let ring_top = forced.sample(x + w / 2., y + 0.75)?;
            let ring_bottom = forced.sample(x + w / 2., y + h - 0.75)?;
            check_eq!(ring_top, fg, "actual top focus edge uses CanvasText");
            check_eq!(ring_bottom, fg, "actual bottom focus edge uses CanvasText");
            let corner = forced.sample(x + 0.5, y + 0.5)?;
            if !divided {
                check_eq!(
                    corner,
                    bg,
                    "rounded Contained focus corner leaves Canvas visible"
                );
            }
            let detail = if divided {
                let r = bounds(tab, DIVIDER).await?;
                let actual = forced.sample(r[0] + r[2] / 2., r[1] + 0.5)?;
                check_eq!(actual, fg, "Divided rule paints the CSS system foreground");
                json!({"bounds":r,"divider":actual})
            } else {
                json!({"gap":forced.sample(bar[0]+bar[2]/2.,bar[1]+bar[3]+1.)?})
            };
            let row = bounds(tab, DESIGN).await?;
            check!(
                contains(results, row),
                "first result remains visibly inside the search scroll viewport"
            );
            let headline = bounds(tab, "search-doc-design-headline").await?;
            let result_ink = forced.count_color(headline, fg)?;
            check!(
                result_ink > 20,
                "search result actual glyphs use CanvasText: {result_ink}"
            );
            let outside = [f64::from(width) - 3., f64::from(height) - 3.];
            let outside_px = forced.sample(outside[0], outside[1])?;
            if compact {
                check_eq!(
                    outside_px,
                    bg,
                    "fullscreen panel paints Canvas to the corner"
                );
            } else {
                for i in 0..3 {
                    let expected = (u16::from(fg[i]) * 82 + u16::from(bg[i]) * 173 + 127) / 255;
                    check!((i16::from(outside_px[i])-expected as i16).abs()<=1,
                        "docked scrim uses independent CSS foreground at alpha 82 over Canvas: {fg:?}/{bg:?}/{outside_px:?}");
                }
            }
            let query = if compact { "設計" } else { "設" };
            set_query(tab, query).await?;
            let token = editing(tab).await?;
            let typed = paint(env, tab, &format!("{profile}-forced-typed")).await?;
            let (typed_ink, _) = ink_bounds(&typed, text_strip(bounds(tab, INPUT).await?), fg)?;
            media(tab, false, false).await?;
            let restoration = contrast(tab, "normal").await?;
            check_eq!(
                restoration["theme"],
                before["theme"],
                "leaving forced colors restores original theme palette"
            );
            check_eq!(
                editing(tab).await?,
                token,
                "leaving forced colors keeps query and exact selection"
            );
            edit_focus(tab).await?;
            focus_projected(tab, CLEAR).await?;
            tab.press("Enter").await?;
            until(tab, "s.search_overlay.query === ''").await?;
            edit_focus(tab).await?;
            let restore = paint(env, tab, &format!("{profile}-normal-restored")).await?;
            check_eq!(
                restore.sample(bar[0] + bar[2] * 0.5, bar[1] + 3.)?,
                before_color,
                "original bar pixel is restored"
            );
            close(tab).await?;
            variants.push(json!({"profile":profile,"browser_palette":css,"input":input,"dialog":dialog,
                "fill":fill,"ring":[ring_top,ring_bottom],"corner":corner,"hint":hint,"hint_pixels":hint_pixels,
                "result_ink_pixels":result_ink,"typed":typed_ink,"outside":outside_px,"detail":detail,
                "normal_color":before_color,"modal_preserved":true,"png":forced.dimensions()}));
        }
    }
    route(tab, url).await?;
    resize(tab, 600, 280).await?;
    open(tab).await?;
    let short = bounds(tab, DIALOG).await?;
    check_eq!(
        short,
        [0., 0., 600., 280.],
        "600dp with insufficient dock space falls back to full-screen"
    );
    set_query(tab, "設").await?;
    let draft = editing(tab).await?;
    resize(tab, 600, 720).await?;
    tab.wait_for("window.gallery.control_bounds('search-overlay-demo-dialog')[2] < 600")
        .await?;
    edit_focus(tab).await?;
    let fit = bounds(tab, DIALOG).await?;
    check!(
        fit[1] > 0. && fit[2] >= 360. && fit[3] >= 240.,
        "same 600dp viewport at usable height docks: {fit:?}"
    );
    check_eq!(
        editing(tab).await?,
        draft,
        "fit fallback retains real query and exact selection"
    );
    close(tab).await?;
    Ok(
        json!({"profiles":variants,"real_cdp_forced_colors":true,"fit":{"short":short,"docked":fit,"draft":draft}}),
    )
}
