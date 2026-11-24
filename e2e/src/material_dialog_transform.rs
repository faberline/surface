//! Measured FAB container transform through the actual headless Chrome host.

use super::*;

use super::motion::{freeze, preference, retired_ax, tick, visible_ax, CLOCK};

const ROUTE: &str = "dialog-demo-show-transform";
const FAB: &str = "dialog-demo-fab-open";
const LAYER: &str = "dialog-demo-edit-transform-layer";
const SAVE: &str = "dialog-demo-edit-action";
const VIEWPORT: Rect = [0., 0., 320., 400.];

type Pixel = [u8; 3];
type Point = [f64; 2];

async fn scene(tab: &Tab, url: &str, width: u32, height: u32, reduced: bool) -> Result<()> {
    preference(tab, reduced).await?;
    tab.viewport(960, 1844).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, ROUTE).await?;
    ready(tab, "d.visible && d.transform.preview && d.open === null").await?;
    activate(tab, "dialog-demo-theme").await?;
    activate(tab, "dialog-demo-rtl").await?;
    until(tab, "s.dark && s.dialog_demo.rtl").await?;
    resize(tab, width, height).await?;
    let fab = bounds(tab, FAB).await?;
    check_eq!(
        [fab[2], fab[3]],
        [56., 56.],
        "the actual FAB has its complete 56 dp target"
    );
    only_fab_launcher(tab).await?;
    ready(tab, "d.transform.progress === 0").await?;
    if !reduced {
        ready(tab, &format!(
        "d.transform.progress === 0 && d.transform.anchor !== null &&
        [d.transform.anchor.x,d.transform.anchor.y,d.transform.anchor.width,d.transform.anchor.height]
        .every((v,i)=>v===window.gallery.control_bounds({})[i])",
        js(FAB)
        )).await?;
    }
    freeze(tab).await
}

async fn only_fab_launcher(tab: &Tab) -> Result<()> {
    let tree = ax_tree(tab).await?;
    ax(&tree, "button", "新增行程")?;
    for (id, label) in [
        ("dialog-demo-basic-open", "基本對話框"),
        ("dialog-demo-alert-open", "重要提示"),
        ("dialog-demo-edit-open", "編輯行程"),
    ] {
        check_eq!(
            tab.eval(&format!("window.gallery.control_bounds({}).length", js(id)))
                .await?,
            0,
            "the dedicated preview has no overlapping legacy launcher: {id}"
        );
        check_eq!(
            tab.eval(&format!("{} === null", projected(id))).await?,
            true,
            "the legacy launcher has no projected DOM target: {id}"
        );
        check!(
            ax(&tree, "button", label).is_err(),
            "the legacy launcher is absent in Chrome AX: {label}"
        );
    }
    Ok(())
}

async fn facts(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["dialog_demo"]["transform"].take())
}

fn rect(value: &Value) -> Result<Rect> {
    fn number(value: &Value, key: &str) -> Result<f64> {
        value[key]
            .as_f64()
            .with_context(|| format!("transform geometry has {key}: {value}"))
    }
    Ok([
        number(value, "x")?,
        number(value, "y")?,
        number(value, "width")?,
        number(value, "height")?,
    ])
}

fn progress(pose: &Value) -> Result<f64> {
    pose["progress"]
        .as_f64()
        .context("Gallery transform progress")
}

async fn opening(tab: &Tab) -> Result<()> {
    activate(tab, FAB).await?;
    ready(tab, "d.open === 'edit' && d.presentation === 'fullscreen'").await?;
    editor_focus(tab).await
}

async fn live(tab: &Tab) -> Result<()> {
    editor_focus(tab).await?;
    visible_ax(tab, EDIT, "編輯行程", "儲存").await?;
    let tree = ax_tree(tab).await?;
    check_eq!(
        ax_property(ax(&tree, "textbox", "行程名稱")?, "focused"),
        Some(&json!(true)),
        "the actual Chrome editor owns focus"
    );
    check!(
        ax(&tree, "button", "新增行程").is_err(),
        "active transform suppresses the launcher in Chrome AX"
    );
    Ok(())
}

async fn retired(tab: &Tab) -> Result<()> {
    retired_ax(tab, EDIT, "編輯行程", SAVE, "儲存").await?;
    let tree = ax_tree(tab).await?;
    check!(
        ax(&tree, "textbox", "行程名稱").is_err(),
        "retained transform cannot leave a stale textbox"
    );
    ax(&tree, "button", "新增行程")?;
    tab.wait_for(&format!(
        "{INPUT}.hidden && {INPUT}.getClientRects().length === 0 && document.activeElement !== {INPUT}"
    )).await
}

fn sample(image: &Raster, [x, y]: Point) -> Result<Pixel> {
    image.sample(x, y)
}

fn color_difference(a: Pixel, b: Pixel) -> u32 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| u32::from(a.abs_diff(b)))
        .sum()
}

fn intermediate(anchor: Rect, pose: &Value) -> Result<Rect> {
    let p = progress(pose)?;
    let aperture = rect(&pose["container"])?;
    check!(
        (0.2..0.9).contains(&p),
        "real rAF advances to a clearly intermediate pose: {pose}"
    );
    let expected = std::array::from_fn::<_, 4, _>(|i| anchor[i] + (VIEWPORT[i] - anchor[i]) * p);
    check!(
        aperture
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 0.015),
        "the measured moving aperture interpolates from the real FAB: {aperture:?} / {expected:?}"
    );
    check!(
        aperture[2] > anchor[2]
            && aperture[2] < VIEWPORT[2]
            && aperture[3] > anchor[3]
            && aperture[3] < VIEWPORT[3],
        "the visible aperture is larger than the FAB and smaller than the viewport: {aperture:?}"
    );
    Ok(aperture)
}

fn aperture_points([x, y, w, h]: Rect) -> [Point; 3] {
    [
        [x + w * 0.6, y + h * 0.66],
        [x + 0.25, y + 0.25],
        [VIEWPORT[2] - 4., y + h * 0.66],
    ]
}

fn pixels_at(image: &Raster, points: [Point; 3]) -> Result<[Pixel; 3]> {
    Ok([
        sample(image, points[0])?,
        sample(image, points[1])?,
        sample(image, points[2])?,
    ])
}

fn painted_aperture(rest: &Raster, painted: &Raster, aperture: Rect) -> Result<Value> {
    let points = aperture_points(aperture);
    let base = pixels_at(rest, points)?;
    let actual = pixels_at(painted, points)?;
    check!(
        color_difference(base[0], actual[0]) >= 12,
        "the captured aperture interior visibly changes: {base:?} / {actual:?}"
    );
    check_eq!(
        actual[1],
        base[1],
        "an actual rounded corner pixel keeps the uncovered background"
    );
    check_eq!(
        actual[2],
        base[2],
        "paint outside the measured aperture remains the underlying page"
    );
    Ok(json!({"points":points,"rest":base,"frame":actual}))
}

async fn field_pixel(tab: &Tab, image: &Raster) -> Result<(Point, Pixel)> {
    let [x, y, _, height] = bounds(tab, EDITOR).await?;
    let point = [x + 8., y + height * 0.72];
    let [r,g,b,a]: [u8;4] = serde_json::from_value(tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {}).style.background",
        js(EDITOR)
    )).await?)?;
    check_eq!(a, 255, "the settled editor surface is opaque");
    let pixel = sample(image, point)?;
    check_eq!(
        pixel,
        [r, g, b],
        "the actual frame paints the settled editor field at its authored location"
    );
    Ok((point, pixel))
}

async fn normal(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    scene(tab, url, 320, 400, false).await?;
    let anchor = bounds(tab, FAB).await?;
    check_eq!(
        anchor,
        [24., 320., 56., 56.],
        "a real 56 dp RTL FAB at the lower logical end"
    );
    check_eq!(
        rect(&facts(tab).await?["anchor"])?,
        anchor,
        "fixture measures actual painted FAB bounds"
    );
    let origin_point = [anchor[0] + 12., anchor[1] + 28.];
    let rest = raster(env, tab, "transform-rest").await?;
    let rest_fab = sample(&rest, origin_point)?;
    opening(tab).await?;
    ready(tab, "d.transform.progress === 0 && !d.transform.retiring").await?;
    live(tab).await?;
    let start = facts(tab).await?;
    check_eq!(
        rect(&start["container"])?,
        anchor,
        "first live frame starts at the measured FAB"
    );
    check_eq!(
        bounds(tab, EDIT).await?,
        VIEWPORT,
        "the active modal's logical layout is already full-screen"
    );
    let first = raster(env, tab, "transform-start").await?;
    check_eq!(
        sample(&first, origin_point)?,
        rest_fab,
        "first frame keeps the actual FAB appearance at the source"
    );
    let far = [4., 390.];
    check_eq!(
        sample(&first, far)?,
        sample(&rest, far)?,
        "no flash outside the initial FAB"
    );

    tick(tab, 60).await?;
    let mid = facts(tab).await?;
    let aperture = intermediate(anchor, &mid)?;
    let entry = raster(env, tab, "transform-entering").await?;
    let entry_pixels = painted_aperture(&rest, &entry, aperture)?;
    tick(tab, 500).await?;
    ready(tab, "d.transform.progress === 1 && d.open === 'edit'").await?;
    check_eq!(
        rect(&facts(tab).await?["container"])?,
        VIEWPORT,
        "the measured settled container covers the viewport"
    );
    let full = raster(env, tab, "transform-open").await?;
    let (field_probe, field) = field_pixel(tab, &full).await?;
    check!(
        color_difference(field, sample(&rest, field_probe)?) >= 12,
        "settled frame covers the old page with the full-size editor"
    );

    tab.press("Escape").await?;
    ready(
        tab,
        "d.open === null && d.transform.retiring && d.transform.progress === 1",
    )
    .await?;
    focused(tab, FAB).await?;
    retired(tab).await?;
    let exit_start = raster(env, tab, "transform-exit-start").await?;
    check_eq!(
        sample(&exit_start, field_probe)?,
        field,
        "the upper paint remains visible after editor input and AX have retired"
    );

    tick(tab, 560).await?;
    let leaving = facts(tab).await?;
    let exit_rect = intermediate(anchor, &leaving)?;
    let exit = raster(env, tab, "transform-exiting").await?;
    let exit_pixels = painted_aperture(&rest, &exit, exit_rect)?;
    let trigger = point(tab, FAB).await?;
    let [cx, cy, ..] = trigger.canvas;
    check!(
        contains(exit_rect, [trigger.x - cx, trigger.y - cy, 1., 1.]),
        "the real reopening pointer lies inside the still-painted retiring layer"
    );
    tab.click(trigger.x, trigger.y).await?;
    ready(tab, "d.open === 'edit' && !d.transform.retiring").await?;
    live(tab).await?;
    let reverse = facts(tab).await?;
    check_eq!(
        rect(&reverse["container"])?,
        exit_rect,
        "same-frame reversal keeps the exact visible aperture"
    );
    check_eq!(
        progress(&reverse)?,
        progress(&leaving)?,
        "reversal starts with no progress jump"
    );
    let reversed = raster(env, tab, "transform-reversed").await?;
    check_eq!(
        sample(&reversed, aperture_points(exit_rect)[0])?,
        sample(&exit, aperture_points(exit_rect)[0])?,
        "twice-sampled painted aperture stays at the held reversal pose"
    );
    tick(tab, 650).await?;
    check!(
        progress(&facts(tab).await?)? > progress(&reverse)?,
        "reversal grows toward the full-screen endpoint"
    );
    tick(tab, 1100).await?;
    ready(tab, "d.transform.progress === 1").await?;

    tab.press("Escape").await?;
    ready(tab, "d.open === null && d.transform.retiring").await?;
    tick(tab, 1600).await?;
    ready(tab, "d.transform.progress === 0 && !d.transform.retiring").await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(LAYER)
    ))
    .await?;
    pointer_focus(tab, FAB).await?;
    retired(tab).await?;
    let closed = raster(env, tab, "transform-closed").await?;
    check_eq!(
        sample(&closed, far)?,
        sample(&rest, far)?,
        "after exit the retired panel completely leaves the original page"
    );
    Ok(
        json!({"anchor":anchor,"start":start,"entry":mid,"exit":leaving,"reversal":reverse,
        "entry_pixels":entry_pixels,"exit_pixels":exit_pixels,"origin_point":origin_point,"origin_pixel":rest_fab,
        "field_probe":field_probe,"field_color":field,"paint_retired_after_focus_return":true,"trusted_reverse":true,
        "legacy_launchers_absent":true}),
    )
}

async fn reduce(env: &Env, tab: &Tab, url: &str, normal: &Value) -> Result<Value> {
    scene(tab, url, 320, 400, true).await?;
    let resting_anchor_available = !facts(tab).await?["anchor"].is_null();
    opening(tab).await?;
    ready(tab, "d.transform.progress === 1 && !d.transform.retiring").await?;
    live(tab).await?;
    check_eq!(
        tab.eval(&format!("window.{CLOCK}")).await?,
        0,
        "Reduce Motion reaches open with no clock advance"
    );
    let opened = raster(env, tab, "transform-reduced-open").await?;
    let (field_probe, field) = field_pixel(tab, &opened).await?;
    check_eq!(
        json!(field_probe),
        normal["field_probe"],
        "same field location after Reduce Motion"
    );
    check_eq!(
        json!(field),
        normal["field_color"],
        "same exact endpoint paint after Reduce Motion"
    );
    tab.press("Escape").await?;
    ready(
        tab,
        "d.open === null && d.transform.progress === 0 && !d.transform.retiring",
    )
    .await?;
    focused(tab, FAB).await?;
    retired(tab).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(LAYER)
    ))
    .await?;
    check_eq!(
        tab.eval(&format!("window.{CLOCK}")).await?,
        0,
        "Reduce Motion clears retirement with no clock advance"
    );
    raster(env, tab, "transform-reduced-closed").await?;
    Ok(
        json!({"clock_ms":0,"field_probe":field_probe,"field_color":field,"exact_endpoints":true,
        "resting_anchor_available":resting_anchor_available}),
    )
}

async fn adaptive(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    scene(tab, url, 599, 500, false).await?;
    opening(tab).await?;
    ready(tab, "d.transform.progress === 0").await?;
    tick(tab, 60).await?;
    let starting = facts(tab).await?;
    check!(
        (0.2..0.9).contains(&progress(&starting)?),
        "start resizing with the FAB expansion in flight"
    );
    fill(tab, CHANGED).await?;
    let before = select_last_character(tab).await?;
    tab.ime_compose("ㄅ", 0, 1).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(EDITOR)
    ))
    .await?;
    tab.settle().await?;
    let composition = selection(tab).await?;
    check_eq!(
        composition["same"],
        true,
        "same real textarea owns active synthetic preedit"
    );
    raster(env, tab, "transform-adaptive-full-preedit").await?;

    resize(tab, 600, 500).await?;
    ready(tab,"d.open === 'edit' && d.presentation === 'basic' && d.transform.progress === 1 && d.draft === '秋日散步'").await?;
    editor_focus(tab).await?;
    let wide = bounds(tab, EDIT).await?;
    check!(
        wide[0] > 0. && wide[1] > 0. && wide[2] < 600.,
        "600 dp selects separately measured Basic layout: {wide:?}"
    );
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(LAYER)
    ))
    .await?;
    check_eq!(selection(tab).await?, composition, "progress-to-Basic resize keeps the real textarea, draft, preedit and directed selection intact");
    raster(env, tab, "transform-adaptive-basic-preedit").await?;

    resize(tab, 599, 500).await?;
    ready(tab,"d.open === 'edit' && d.presentation === 'fullscreen' && d.transform.progress === 1 && d.draft === '秋日散步'").await?;
    editor_focus(tab).await?;
    check_eq!(
        selection(tab).await?,
        composition,
        "the same still-active editor returns to the Full-screen layout"
    );
    tab.ime_compose("", 0, 0).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === false",
        js(EDITOR)
    ))
    .await?;
    tab.settle().await?;
    check_eq!(
        selection(tab).await?,
        before,
        "preedit cancellation preserves the original draft and backward range"
    );
    ready(
        tab,
        "d.saved === '週末行程' && d.saves === 0 && d.discards === 0",
    )
    .await?;
    Ok(
        json!({"widths":[599,600,599],"starting":starting,"basic_bounds":wide,"selection":before,"preedit":composition,
        "same_textarea":true,"draft":CHANGED,"saved":INITIAL}),
    )
}

async fn short_clearance(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let scroll_id = "dialog-demo-fab-preview-scroll";
    let saved_id = "dialog-demo-saved";
    scene(tab, url, 320, 240, true).await?;
    activate(tab, "dialog-demo-large-text").await?;
    ready(tab, "d.text_scale === 2 && d.rtl").await?;
    let fab = bounds(tab, FAB).await?;
    let scroller = bounds(tab, scroll_id).await?;
    check_eq!(
        scroller[1] + scroller[3] + 8.,
        fab[1],
        "preview content clip ends a full 8 dp above the real FAB"
    );
    let below = point(tab, scroll_id).await?;
    tab.mouse_move(below.x, below.y).await?;
    tab.wheel(0., -1000.).await?;
    tab.settle().await?;
    let initial = bounds(tab, saved_id).await?;
    check!(
        initial[1] + initial[3] > scroller[1] + scroller[3],
        "short large-text preview still has content below its initial clip: {initial:?}"
    );
    raster(env, tab, "transform-clearance-short-before").await?;
    tab.wheel(0., 1000.).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({})[1] < {}",
        js(saved_id),
        initial[1] - 2.
    ))
    .await?;
    tab.settle().await?;
    let shown = bounds(tab, saved_id).await?;
    check!(
        contains(scroller, shown),
        "real wheel reveals the complete saved status above the FAB: {shown:?}"
    );
    check_eq!(
        bounds(tab, FAB).await?,
        fab,
        "wheel leaves the entire FAB fixed"
    );
    check_eq!(
        bounds(tab, scroll_id).await?,
        scroller,
        "the preview clip also keeps its position"
    );
    let after = raster(env, tab, "transform-clearance-short-after").await?;
    let [r,g,b,_]: [u8; 4] = serde_json::from_value(
        tab.eval(&format!("JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {}).style.foreground",js(saved_id))).await?
    )?;
    let ink = after
        .colors(shown)?
        .get(&[r, g, b])
        .copied()
        .unwrap_or_default();
    check!(
        ink >= 20,
        "the actual WebGPU capture paints the now-available saved status: {ink} solid pixels"
    );

    click(tab, FAB).await?;
    ready(
        tab,
        "d.open === 'edit' && d.presentation === 'fullscreen' && d.transform.progress === 1",
    )
    .await?;
    live(tab).await?;
    tab.press("Escape").await?;
    ready(tab, "d.open === null && !d.transform.retiring").await?;
    pointer_focus(tab, FAB).await?;
    Ok(json!({"viewport":[320,240],"text_scale":2,"clip_gap":8,
        "scroller":scroller,"fab":fab,"saved_before":initial,"saved_after":shown,
        "saved_rgb":[r,g,b],"saved_solid_ink":ink,"physical_fab_opens":true}))
}

pub(super) async fn run(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let normal = normal(env, tab, url)
        .await
        .context("missing or misplaced Full-screen FAB container")?;
    let reduced = reduce(env, tab, url, &normal)
        .await
        .context("Full-screen Reduced Motion")?;
    let adaptive = adaptive(env, tab, url)
        .await
        .context("Full-screen to Basic responsive edit")?;
    let short = short_clearance(env, tab, url)
        .await
        .context("short FAB preview clearance and scrolling")?;
    preference(tab, false).await?;
    Ok(
        json!({"normal":normal,"reduced":reduced,"adaptive":adaptive,"short_clearance":short,"manual_performance_time":true}),
    )
}
