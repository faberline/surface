//! Bounded Material List press ripples measured in headless WebGPU screenshots.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    material_list_corners::{
        distance, freeze, leave_rows, palette, pixel, preset, ready, reduced_motion, reload, tick,
        Scene,
    },
    material_time_input::{bounds, contains, focus_projected, focused, Raster},
    state,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

type Rect = [f64; 4];
type Rgb = [u8; 3];
type Point = [f64; 2];

const ROW: &str = "lists-c";
const POSTER: &str = "list-media-poster";
const PRESS_ALPHA: f64 = 26. / 255.;
const TRACE: &str = r#"
    (() => {
        window.listRippleContacts = [];
        const canvas = document.getElementById('surface');
        for (const type of ['pointerdown', 'pointerup', 'pointercancel']) {
            canvas.addEventListener(type, event => {
                if (event.isTrusted) window.listRippleContacts.push({
                    phase: type, device: event.pointerType,
                });
            }, { capture: true, passive: true });
        }
    })()
"#;

// Stop only frame delivery while making the quick tap. Input remains real and
// goes through the normal browser and Runtime. Flush the pending callbacks
// into real Chrome animation frames after the down and up have both arrived.
const HOLD_FRAMES: &str = r#"
    (() => {
        const request = window.requestAnimationFrame;
        const cancel = window.cancelAnimationFrame;
        const pending = new Map();
        let id = -1;
        window.listRippleFrames = { request, cancel, pending };
        window.requestAnimationFrame = callback => {
            const token = id--;
            pending.set(token, callback);
            return token;
        };
        window.cancelAnimationFrame = token => {
            if (!pending.delete(token) && token >= 0) cancel.call(window, token);
        };
        return new Promise(resolve => request.call(window, () => request.call(window, resolve)));
    })()
"#;

const RELEASE_FRAMES: &str = r#"
    (() => {
        const { request, cancel, pending } = window.listRippleFrames;
        window.requestAnimationFrame = request;
        window.cancelAnimationFrame = cancel;
        for (const callback of pending.values()) request.call(window, callback);
        return pending.size;
    })()
"#;

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-list-ripple-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn sample(image: &Raster, [x, y]: Point) -> Result<Rgb> {
    pixel(image, [x, y, 0.5, 0.5])
}

fn blend(under: Rgb, over: Rgb, fraction: f64) -> Rgb {
    std::array::from_fn(|n| {
        (f64::from(under[n]) * (1. - fraction) + f64::from(over[n]) * fraction).round() as u8
    })
}

async fn screen(tab: &Tab, [x, y]: Point) -> Result<Point> {
    let [left, top, _, _] = tab.canvas_box().await?;
    Ok([left + x, top + y])
}

async fn move_to(tab: &Tab, at: Point) -> Result<()> {
    let [x, y] = screen(tab, at).await?;
    tab.mouse_move(x, y).await?;
    tab.settle().await
}

async fn touch(tab: &Tab, kind: &str, at: Option<Point>) -> Result<()> {
    let points = match at {
        Some(at) => {
            let [x, y] = screen(tab, at).await?;
            json!([{"x":x, "y":y, "id":1}])
        }
        None => json!([]),
    };
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind, "touchPoints":points}),
    )
    .await?;
    Ok(())
}

async fn pressed(tab: &Tab, id: Option<&str>) -> Result<()> {
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).interaction.pressed === {}",
        id.map_or_else(|| "null".into(), js),
    ))
    .await?;
    tab.settle().await
}

async fn baseline_lists(tab: &Tab, variant: &str) -> Result<()> {
    let scene = Scene::Lists;
    preset(tab, scene, variant).await?;
    preset(tab, scene, "action").await?;
    ready(
        tab,
        scene,
        &format!("d.variant === {variant:?} && d.mode === 'action'"),
    )
    .await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await?;
    freeze(tab).await
}

async fn ordinary(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M196",
        "the bounded pointer ripple grows from the real off-center down point and fades once",
    );
    let scene = Scene::Lists;
    baseline_lists(tab, "segmented").await?;
    let row @ [x, y, w, _] = bounds(tab, ROW).await?;
    let origin = [x + 176., y + 24.];
    let far = [x + w - 30., y + 24.];
    let gap = [x + w / 2., y - 1.];
    let corner = [x + 0.5, y + 0.5];
    let surface = palette(tab, scene, "surface").await?;
    let gutter = palette(tab, scene, "surface_container").await?;
    let ink = palette(tab, scene, "on_surface").await?;
    let expected = blend(surface, ink, PRESS_ALPHA);
    let rest = capture(env, tab, "rest").await?;
    check_eq!(
        sample(&rest, origin)?,
        surface,
        "near spot is plain row fill"
    );
    check_eq!(sample(&rest, far)?, surface, "far spot is plain row fill");
    check_eq!(
        sample(&rest, gap)?,
        gutter,
        "the sample lies in a real segmented gap"
    );
    move_to(tab, origin).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(ROW)).await?;
    let first = capture(env, tab, "held-first").await?;
    check_eq!(
        sample(&first, origin)?,
        surface,
        "the first observing frame has no bright or full-row flash"
    );
    check_eq!(
        sample(&first, far)?,
        surface,
        "the first frame leaves the opposite end untouched"
    );

    tick(tab, 46).await?;
    let early = capture(env, tab, "off-center-early").await?;
    let early_near = sample(&early, origin)?;
    let early_far = sample(&early, far)?;
    check!(
        distance(surface, early_near) >= 12 && distance(early_near, expected) >= 9,
        "the physical down origin shows partial press opacity at 46ms: {early_near:?}"
    );
    check_eq!(
        early_far,
        surface,
        "the first wave has not reached the distant side"
    );

    tick(tab, 75).await?;
    let local_full = capture(env, tab, "local-full-alpha").await?;
    let near_at_75 = sample(&local_full, origin)?;
    check!(
        distance(near_at_75, expected) <= 8 && sample(&local_full, far)? == surface,
        "alpha reaches 10 percent at 75ms while the radius is still growing"
    );

    tick(tab, 225).await?;
    let full = capture(env, tab, "bounded-expanded").await?;
    let full_near = sample(&full, origin)?;
    let full_far = sample(&full, far)?;
    check!(
        distance(full_near, expected) <= 8 && distance(full_far, expected) <= 8,
        "both ends reach the tokenized 10 percent press, without a second full-row layer"
    );
    check_eq!(
        sample(&full, gap)?,
        gutter,
        "the adjacent gap has no ripple pixels"
    );
    check!(
        distance(sample(&full, corner)?, gutter) <= 5,
        "the expanded wave does not occupy the clipped rounded corner"
    );
    ready(tab, scene, "d.activations === 0").await?;

    tab.mouse_up().await?;
    ready(tab, scene, "d.activations === 1 && d.last === 'lists-c'").await?;
    leave_rows(tab).await?;
    tick(tab, 300).await?;
    let fading = capture(env, tab, "release-midpoint").await?;
    let midway = sample(&fading, origin)?;
    check!(
        distance(midway, blend(expected, ink, PRESS_ALPHA / 2.)) <= 8,
        "the accepted release has half press alpha over retained focus after 75ms: {midway:?}"
    );
    tick(tab, 376).await?;
    let clear = capture(env, tab, "release-settled").await?;
    check_eq!(
        sample(&clear, origin)?,
        expected,
        "only the normal focus tint remains after 150ms"
    );
    check_eq!(
        sample(&clear, far)?,
        expected,
        "the far side also returns to the uniform focus tint"
    );
    check_eq!(
        bounds(tab, ROW).await?,
        row,
        "ripple never changes row layout or hit geometry"
    );
    ready(tab, scene, "d.activations === 1").await?;
    Ok(
        json!({"row":row,"origin":origin,"times_ms":[0,46,75,225,300,376],"rest":surface,
        "early":[early_near,early_far],"near_at_75":near_at_75,"full":[full_near,full_far],"fade":midway,"gap":gutter,
        "endpoint":expected,"released_underlay":sample(&clear,origin)?,"activations":1}),
    )
}

async fn trusted_trace(tab: &Tab) -> Result<Value> {
    tab.eval("window.listRippleContacts").await
}

async fn contacts(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M197",
        "a coalesced trusted tap paints from its own origin and canceled contacts retire",
    );
    let scene = Scene::Lists;
    baseline_lists(tab, "segmented").await?;
    let [x, y, w, _] = bounds(tab, ROW).await?;
    let origin = [x + w - 160., y + 20.];
    let far = [x + 155., y + 20.];
    let surface = palette(tab, scene, "surface").await?;
    let ink = palette(tab, scene, "on_surface").await?;
    let focused_fill = blend(surface, ink, PRESS_ALPHA);
    tab.eval(TRACE).await?;
    tab.eval(HOLD_FRAMES).await?;
    let quick = async {
        touch(tab, "touchStart", Some(origin)).await?;
        touch(tab, "touchEnd", None).await
    }
    .await;
    let queued = tab.eval(RELEASE_FRAMES).await?;
    quick?;
    let queued = queued
        .as_u64()
        .context("number of buffered browser frames")?;
    check!(queued > 0, "the tap came before a pending presentation");
    ready(tab, scene, "d.activations === 1 && d.last === 'lists-c'").await?;
    pressed(tab, None).await?;
    let coalesced = capture(env, tab, "coalesced-touch").await?;
    let local = sample(&coalesced, origin)?;
    let opposite = sample(&coalesced, far)?;
    check!(
        distance(local, blend(focused_fill, ink, PRESS_ALPHA)) <= 8,
        "terminal Released starts one local pulse at the real touch point: {local:?}"
    );
    check_eq!(
        opposite,
        focused_fill,
        "the opposite side keeps only the normal focus tint"
    );
    tick(tab, 376).await?;
    let settled = capture(env, tab, "coalesced-settled").await?;
    check_eq!(
        sample(&settled, origin)?,
        focused_fill,
        "the coalesced pulse fades to the normal focus tint"
    );

    touch(tab, "touchStart", Some(origin)).await?;
    pressed(tab, Some(ROW)).await?;
    tick(tab, 436).await?;
    let held = capture(env, tab, "before-touch-cancel").await?;
    check!(
        distance(sample(&held, origin)?, surface) >= 15,
        "the second accepted down re-arms the wave"
    );
    touch(tab, "touchCancel", None).await?;
    pressed(tab, None).await?;
    let canceled = capture(env, tab, "after-touch-cancel").await?;
    check_eq!(
        sample(&canceled, origin)?,
        focused_fill,
        "touchcancel immediately leaves only the normal focus tint"
    );
    ready(tab, scene, "d.activations === 1").await?;

    move_to(tab, origin).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(ROW)).await?;
    tick(tab, 500).await?;
    let held = capture(env, tab, "before-outside-release").await?;
    check!(
        distance(sample(&held, origin)?, surface) >= 15,
        "a new mouse contact works after touch cancellation"
    );
    let [outside_x, outside_y] = screen(tab, [1., 1.]).await?;
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({
            "type":"mouseMoved","x":outside_x,"y":outside_y,"button":"none","buttons":1
        }),
    )
    .await?;
    tab.cdp("Input.dispatchMouseEvent", json!({
        "type":"mouseReleased","x":outside_x,"y":outside_y,"button":"left","buttons":0,"clickCount":1
    })).await?;
    pressed(tab, None).await?;
    let outside = capture(env, tab, "after-outside-release").await?;
    check_eq!(
        sample(&outside, origin)?,
        focused_fill,
        "outside release leaves only the normal focus tint"
    );
    ready(tab, scene, "d.activations === 1").await?;
    let events = trusted_trace(tab).await?;
    check_eq!(
        events,
        json!([
            {"phase":"pointerdown","device":"touch"},
            {"phase":"pointerup","device":"touch"},
            {"phase":"pointerdown","device":"touch"},
            {"phase":"pointercancel","device":"touch"},
            {"phase":"pointerdown","device":"mouse"},
            {"phase":"pointerup","device":"mouse"}
        ]),
        "quick, canceled, and rejected interactions were real Chrome pointer events"
    );
    Ok(
        json!({"origin":origin,"buffered_frames":queued,"coalesced":[local,opposite],
        "after_touch_cancel":sample(&canceled,origin)?,"after_outside_release":sample(&outside,origin)?,
        "rest":surface,"focused_underlay":focused_fill,"activations":1,"events":events}),
    )
}

async fn select_media(tab: &Tab, variant: &str) -> Result<()> {
    let scene = Scene::Media;
    preset(tab, scene, variant).await?;
    preset(tab, scene, "select").await?;
    ready(
        tab,
        scene,
        &format!("d.variant === {variant:?} && d.mode === 'select' && d.selected.length === 0"),
    )
    .await?;
    reduced_motion(tab, false).await?;
    leave_rows(tab).await
}

async fn media(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M198",
        "a real touch tints the poster below its text, check and focus outline",
    );
    let scene = Scene::Media;
    select_media(tab, "segmented").await?;
    freeze(tab).await?;
    let row @ [x, y, w, _] = bounds(tab, POSTER).await?;
    let artwork @ [mx, my, mw, mh] = bounds(tab, "list-media-poster-media").await?;
    let origin = [mx + mw * 0.35, my + mh * 0.38];
    let far = [x + w - 56., y + 20.];
    let title = bounds(tab, "list-media-poster-headline").await?;
    let marker = bounds(tab, "list-media-poster-mark").await?;
    check!(
        contains(row, artwork) && contains(row, title) && contains(row, marker),
        "the real photo, text, and check stay nested in their row"
    );
    let rest = capture(env, tab, "media-rest").await?;
    let photo = sample(&rest, origin)?;
    touch(tab, "touchStart", Some(origin)).await?;
    pressed(tab, Some(POSTER)).await?;
    tick(tab, 46).await?;
    let early = capture(env, tab, "media-off-center").await?;
    let tinted = sample(&early, origin)?;
    check!(
        distance(tinted, photo) >= 10 && distance(tinted, photo) <= 65,
        "the accepted wave paints above actual NASA photo pixels: {photo:?} → {tinted:?}"
    );
    check_eq!(
        sample(&early, far)?,
        sample(&rest, far)?,
        "the opposite row side has not changed"
    );
    touch(tab, "touchEnd", None).await?;
    ready(tab, scene, "d.selection_changes === 1 && d.selected.length === 1 && d.selected[0] === 'list-media-poster'").await?;
    // The real touch retained focus on the row. Let the selected style
    // finish animating before comparing its text and focus with the wave.
    check_eq!(
        state(tab).await?["focused"],
        json!(POSTER),
        "the touch gives the actual selection row focus"
    );
    tick(tab, 301).await?;
    let live = capture(env, tab, "media-focus").await?;
    tick(tab, 401).await?;
    let clear = capture(env, tab, "media-focus-settled").await?;
    let lip = [x + 10., y + (artwork[1] - y) / 2.];
    let focus = [x + w * 0.5, y + 1.5];
    let focus_ink = palette(tab, Scene::Lists, "secondary").await?;
    let text_ink = palette(tab, Scene::Lists, "on_secondary_container").await?;
    let check_ink = palette(tab, Scene::Lists, "on_primary").await?;
    let exact_ink = |image: &Raster, area: Rect, ink: Rgb| -> Result<usize> {
        Ok(image.colors(area)?.get(&ink).copied().unwrap_or(0))
    };
    let text_pixels = [
        exact_ink(&live, title, text_ink)?,
        exact_ink(&clear, title, text_ink)?,
    ];
    let mark_pixels = [
        exact_ink(&live, marker, check_ink)?,
        exact_ink(&clear, marker, check_ink)?,
    ];
    let focus_pixels = [sample(&live, focus)?, sample(&clear, focus)?];
    let in_fade = sample(&live, origin)?;
    check!(
        distance(in_fade, sample(&clear, origin)?) >= 9
            && distance(sample(&live, lip)?, sample(&clear, lip)?) >= 9,
        "a released wave still paints above the photo and row fill while focus is present"
    );
    check!(
        focus_pixels
            .iter()
            .all(|pixel| distance(*pixel, focus_ink) <= 5),
        "the same, full-strength focus ink stays above the pulse"
    );
    check!(
        text_pixels[0] >= 6 && text_pixels[0] == text_pixels[1],
        "the ripple does not tint opaque headline ink: {text_pixels:?}"
    );
    check!(
        mark_pixels[0] >= 4 && mark_pixels[0] == mark_pixels[1],
        "the ripple does not tint the selected check: {mark_pixels:?}"
    );
    check_eq!(
        sample(&clear, origin)?,
        photo,
        "the original raster returns after fade"
    );
    ready(tab, scene, "d.selection_changes === 1").await?;
    Ok(
        json!({"row":row,"artwork":artwork,"title":title,"marker":marker,"origin":origin,
        "photo":[photo,tinted,in_fade],"focus":focus_pixels,"text_ink_pixels":text_pixels,
        "check_ink_pixels":mark_pixels,"selection_changes":1}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M199",
        "320 dp dark RTL retains a physical right-side origin and clips the expanded wave",
    );
    let scene = Scene::Media;
    select_media(tab, "standard").await?;
    for setting in ["theme", "rtl", "large-text"] {
        preset(tab, scene, setting).await?;
    }
    super::material_time_input::resize(tab, 320, 900).await?;
    ready(
        tab,
        scene,
        "d.rtl && d.text_scale === 2 && d.mode === 'select'",
    )
    .await?;
    focus_projected(tab, "list-media-earth").await?;
    tab.press("ArrowDown").await?;
    focused(tab, POSTER).await?;
    let clip = bounds(tab, "list-media-scroll").await?;
    move_to(tab, [clip[0] + clip[2] / 2., clip[1] + clip[3] / 2.]).await?;
    let before = state(tab).await?["list_media"]["scroll_y"]
        .as_f64()
        .context("initial media scroll")?;
    tab.wheel(0., 28.).await?;
    ready(tab, scene, &format!("d.scroll_y > {before}")).await?;
    leave_rows(tab).await?;
    freeze(tab).await?;
    let row @ [x, y, w, h] = bounds(tab, POSTER).await?;
    check!(
        y < clip[1] && y + h > clip[1] + 65.,
        "the same actual row crosses a physical scroller clip: {row:?}, {clip:?}"
    );
    let origin = [x + w - 27., clip[1] + 30.];
    let far = [x + 30., clip[1] + 30.];
    let beyond = [x + w / 2., clip[1] - 1.];
    let outside = [x - 1., clip[1] + 40.];
    let rest = capture(env, tab, "compact-before").await?;
    // The intrinsic row bounds are unchanged after its clipped physical
    // point is touched. Compare against the transparent first held frame.
    touch(tab, "touchStart", Some(origin)).await?;
    pressed(tab, Some(POSTER)).await?;
    let first = capture(env, tab, "compact-first").await?;
    tick(tab, 39).await?;
    let early = capture(env, tab, "compact-dark-rtl").await?;
    let before_origin = sample(&first, origin)?;
    let after_origin = sample(&early, origin)?;
    let far_before = sample(&first, far)?;
    let far_early = sample(&early, far)?;
    check!(
        distance(before_origin, after_origin) >= 12,
        "the wave starts on the physical right where the touch lands"
    );
    check_eq!(
        far_early,
        far_before,
        "RTL does not mirror the origin to the opposite side"
    );
    tick(tab, 225).await?;
    let full = capture(env, tab, "compact-expanded").await?;
    check!(
        distance(sample(&full, far)?, far_before) >= 15,
        "full radius finally reaches the physical left"
    );
    for (sentinel, name) in [(beyond, "vertical"), (outside, "horizontal")] {
        check_eq!(
            sample(&full, sentinel)?,
            sample(&rest, sentinel)?,
            format!("the expanded wave is bounded by the actual {name} clip")
        );
    }
    touch(tab, "touchCancel", None).await?;
    pressed(tab, None).await?;
    ready(
        tab,
        scene,
        "d.selection_changes === 0 && d.selected.length === 0",
    )
    .await?;
    let current = state(tab).await?["list_media"]["scroll_y"].clone();
    Ok(
        json!({"viewport":[320,900],"rtl":true,"text_scale":2,"row":row,"clip":clip,
        "scroll":[json!(before),current],"origin":origin,"near":[before_origin,after_origin],
        "far":[far_before,far_early,sample(&full,far)?],"outside_samples":[sample(&full,beyond)?,sample(&full,outside)?]}),
    )
}

async fn reduced(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M200",
        "Reduce Motion changes a held Baseline wave into one static press immediately",
    );
    let scene = Scene::Lists;
    baseline_lists(tab, "baseline").await?;
    let [x, y, w, _] = bounds(tab, ROW).await?;
    let origin = [x + 168., y + 20.];
    let far = [x + w - 34., y + 20.];
    let surface = palette(tab, scene, "surface_container").await?;
    let ink = palette(tab, scene, "on_surface").await?;
    let token = blend(surface, ink, PRESS_ALPHA);
    move_to(tab, origin).await?;
    tab.mouse_down().await?;
    pressed(tab, Some(ROW)).await?;
    tick(tab, 42).await?;
    let partial = capture(env, tab, "reduced-before").await?;
    let near = sample(&partial, origin)?;
    let opposite = sample(&partial, far)?;
    check!(
        distance(near, surface) >= 10 && opposite == surface,
        "the held Baseline row first shows a moving bounded wave"
    );
    reduced_motion(tab, true).await?;
    let first = capture(env, tab, "reduced-static").await?;
    let static_pixels = [sample(&first, origin)?, sample(&first, far)?];
    check!(
        static_pixels
            .iter()
            .all(|pixel| distance(*pixel, token) <= 8),
        "the same frozen-time frame becomes the static 10 percent press"
    );
    check_eq!(
        tab.eval("window.listCornersClock").await?,
        json!(42),
        "the preference snaps without advancing time"
    );
    tick(tab, 390).await?;
    let later = capture(env, tab, "reduced-held").await?;
    check_eq!(
        [sample(&later, origin)?, sample(&later, far)?],
        static_pixels,
        "the reduced-motion press remains constant while held"
    );
    tab.mouse_up().await?;
    leave_rows(tab).await?;
    ready(tab, scene, "d.activations === 1 && d.last === 'lists-c'").await?;
    let released = capture(env, tab, "reduced-released").await?;
    check_eq!(
        sample(&released, origin)?,
        token,
        "release under Reduce Motion leaves only the normal focus tint"
    );
    Ok(
        json!({"before":[near,opposite],"after":static_pixels,"token":token,
        "preference_time_ms":42,"later_time_ms":390,"activations":1,"rest":surface,
        "released_underlay":sample(&released,origin)?}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url, Scene::Lists).await?;
    let ripple = ordinary(env, tab).await?;
    reload(tab, url, Scene::Lists).await?;
    let contacts = contacts(env, tab).await?;
    reload(tab, url, Scene::Media).await?;
    let media = media(env, tab).await?;
    reload(tab, url, Scene::Media).await?;
    let compact = compact(env, tab).await?;
    reload(tab, url, Scene::Lists).await?;
    let reduced = reduced(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "List ripple browser errors: {errors:?}");
    env.write_json("browser-material-list-ripple.json", &json!({"browser":chrome.version,
        "ripple":ripple,"contacts":contacts,"media":media,"compact":compact,"reduced":reduced,"errors":errors}))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-list-ripple-failure.json",
            &json!({
                "error":format!("{error:#}"), "state":state(&tab).await.ok(), "errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-list-ripple-failure.png"))
            .await;
    }
    result.context("Material 3 bounded List ripples")
}
