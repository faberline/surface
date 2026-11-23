//! Material 3 list artwork, accessibility and painted pixels in headless Chrome.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const LIST: &str = "list-media-items";
const SCROLL: &str = "list-media-scroll";
const EARTH: &str = "list-media-earth";
const POSTER: &str = "list-media-poster";
const PROBE: &str = "list-media-probe";
const TEXT: &str = "list-media-text";
const DISABLED: &str = "list-media-disabled";
const DETAIL: &str = "list-media-detail";
const DETAIL_BACK: &str = "list-media-detail-back";
const DETAIL_IMAGE: &str = "list-media-detail-image";
const EARTH_NAME: &str = "藍色彈珠";
const POSTER_NAME: &str = "紐西蘭上空";
const POSTER_SUPPORT: &str = "國際太空站飛越紐西蘭南北兩島";
const GREEN: Rgb = [19, 190, 82];
const BLUE: Rgb = [27, 91, 238];
const RED: Rgb = [239, 23, 64];
const YELLOW: Rgb = [250, 205, 41];
const GUARDS: [Rgb; 2] = [[231, 32, 176], [255, 167, 16]];

type Rect = [f64; 4];
type Rgb = [u8; 3];

fn part(id: &str, suffix: &str) -> String {
    format!("{id}-{suffix}")
}

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["list_media"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.list_media)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "list-media-demo-show").await?;
    ready(
        tab,
        "d.visible && d.variant === 'segmented' && d.mode === 'passive' && d.wide &&
        !d.rtl && d.text_scale === 1 && d.activations === 0 && d.selection_changes === 0 &&
        d.selected.length === 0 && d.details === null",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("list-media-demo-{name}")).await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let file = env.artifact(&format!("material-list-media-{name}.png"));
    tab.screenshot(&file).await?;
    Raster::read(tab, file).await
}

async fn box_of(tab: &Tab, id: &str) -> Result<Rect> {
    bounds(tab, &part(id, "media")).await
}

fn near_rgb(a: Rgb, b: Rgb) -> bool {
    a.into_iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 5)
}

fn swatch(image: &Raster, rect: Rect) -> Result<Rgb> {
    image
        .colors(rect)?
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .context("WebGPU PNG sample")
}

fn sample(rect: Rect, u: f64, v: f64) -> Rect {
    [
        rect[0] + u * rect[2] - 1.,
        rect[1] + v * rect[3] - 1.,
        2.,
        2.,
    ]
}

async fn token(tab: &Tab, key: &str) -> Result<Rgb> {
    let [red, green, blue, alpha]: [u8; 4] =
        serde_json::from_value(data(tab).await?["colors"][key].clone())
            .with_context(|| format!("opaque Gallery color {key}"))?;
    check_eq!(alpha, 255, "gallery background color is opaque");
    Ok([red, green, blue])
}

fn visible_photo(image: &Raster, rect: Rect, label: &str) -> Result<usize> {
    let region = [
        rect[0] + rect[2] * 0.18,
        rect[1] + rect[3] * 0.2,
        rect[2] * 0.64,
        rect[3] * 0.6,
    ];
    let colors = image.colors(region)?;
    let visible_colors = colors
        .iter()
        .filter(|(rgb, _)| rgb.iter().copied().max().unwrap_or_default() > 20)
        .count();
    check!(
        visible_colors >= 64,
        "{label} shows photographic detail in the actual PNG: {visible_colors} colors"
    );
    Ok(visible_colors)
}

async fn list_semantics(tab: &Tab, selected: bool) -> Result<Value> {
    let expected = if selected { "listbox" } else { "list" };
    let element = projected(LIST);
    let attrs = tab
        .eval(&format!(
            "(() => {{const n={element}; return n && {{role:n.getAttribute('role'),
          label:n.getAttribute('aria-label'), multi:n.getAttribute('aria-multiselectable'),
          media_elements:n.querySelectorAll('img,[role=img],video').length}};}})()"
        ))
        .await?;
    check!(
        attrs["role"] == expected
            && attrs["label"] == "太空影像"
            && attrs["multi"].as_str() == selected.then_some("true")
            && attrs["media_elements"] == 0,
        "one accessible list owns the rows and the images remain decorative: {attrs}"
    );
    ax(&ax_tree(tab).await?, expected, "太空影像")?;
    Ok(attrs)
}

async fn passive_artwork(tab: &Tab, id: &str) -> Result<()> {
    let target = projected(id);
    check_eq!(
        tab.eval(&format!("(() => {{const n={target}; return !n || (!n.matches('[tabindex],[data-surface-action],[role=img],[role=button]') &&
           !n.querySelector('[tabindex],[data-surface-action],[role=img]'));}})()")).await?,
        json!(true),
        format!("{id} is a decoration without a projected action or Tab stop")
    );
    Ok(())
}

async fn measurement(tab: &Tab, id: &str) -> Result<Value> {
    let artwork = box_of(tab, id).await?;
    let row = bounds(tab, id).await?;
    let label = bounds(tab, &part(id, "headline")).await?;
    check!(
        contains(row, artwork) && contains(row, label),
        "{id} encloses the media and a distinct label: {row:?} {artwork:?} {label:?}"
    );
    Ok(json!({"artwork":artwork,"row":row,"label":label,
        "inset":artwork[0]-row[0],"label_gap":label[0]-artwork[0]-artwork[2]}))
}

async fn profiles(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M187",
        "real photo and poster paint at the preferred wide and portrait sizes in a passive list",
    );
    let semantics = list_semantics(tab, false).await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "StaticText", EARTH_NAME)?;
    ax(&tree, "StaticText", POSTER_NAME)?;
    for id in [EARTH, POSTER, PROBE] {
        passive_artwork(tab, &part(id, "media")).await?;
        check_eq!(
            tab.eval(&format!(
                "{}?.getAttribute('tabindex') ?? null",
                projected(id)
            ))
            .await?,
            Value::Null,
            format!("passive row {id} has no Tab stop")
        );
    }
    check_eq!(
        tab.eval(&format!(
            "window.gallery.control_bounds({}).length",
            js(part(TEXT, "media"))
        ))
        .await?,
        json!(0),
        "the mixed text-only row has no media slot"
    );
    let wide_photo = box_of(tab, EARTH).await?;
    let wide_poster = box_of(tab, POSTER).await?;
    check!(
        near(wide_photo[2], 80.)
            && near(wide_photo[3], 80.)
            && near(wide_poster[2], 114.)
            && near(wide_poster[3], 64.),
        "the wide scene uses an 80 dp photo and 114×64 dp poster"
    );
    let wide = capture(env, tab, "wide-light").await?;
    let color_counts = [
        visible_photo(&wide, wide_photo, "Blue Marble")?,
        visible_photo(&wide, wide_poster, "ISS movie poster")?,
    ];
    let wide_profiles = [
        measurement(tab, EARTH).await?,
        measurement(tab, POSTER).await?,
    ];

    resize(tab, 600, 820).await?;
    ready(tab, "!d.wide").await?;
    let small_photo = box_of(tab, EARTH).await?;
    let small_poster = box_of(tab, POSTER).await?;
    check!(
        near(small_photo[2], 56.)
            && near(small_photo[3], 56.)
            && near(small_poster[2], 100.)
            && near(small_poster[3], 56.),
        "the portrait view uses 56×56 and 100×56 dp artwork"
    );
    let portrait = capture(env, tab, "portrait-light").await?;
    visible_photo(&portrait, small_photo, "portrait photo")?;
    visible_photo(&portrait, small_poster, "portrait poster")?;
    preset(tab, "baseline").await?;
    ready(tab, "d.variant === 'baseline'").await?;
    let baseline_photo = measurement(tab, EARTH).await?;
    let baseline_poster = measurement(tab, POSTER).await?;
    check!(baseline_photo["inset"] == 16. && baseline_photo["label_gap"] == 16.
        && baseline_poster["inset"] == 0. && baseline_poster["label_gap"] == 16.,
        "Baseline keeps 16 dp on stills and only passive posters sit flush: {baseline_photo} {baseline_poster}");
    Ok(
        json!({"semantics":semantics,"wide":wide_profiles,"portrait":[small_photo,small_poster],
        "baseline":[baseline_photo,baseline_poster],"wide_photo_colors":color_counts}),
    )
}

fn alpha_mix(ink: Rgb, back: Rgb, linear: bool) -> Rgb {
    let unpack = |value: u8| {
        let value = f64::from(value) / 255.;
        if !linear {
            value
        } else if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let alpha = 128. / 255.;
    std::array::from_fn(|i| {
        let mixed = unpack(ink[i]) * alpha + unpack(back[i]) * (1. - alpha);
        let value = if !linear {
            mixed
        } else if mixed <= 0.0031308 {
            12.92 * mixed
        } else {
            1.055 * mixed.powf(1. / 2.4) - 0.055
        };
        (value * 255.).round() as u8
    })
}

async fn dom_wire(tab: &Tab) -> Result<Value> {
    let exported = tab.eval_json("window.gallery.export_dom()").await?;
    let id = part(PROBE, "media");
    let nodes = exported["dom"]["nodes"]
        .as_array()
        .context("DOM export nodes")?;
    let index = nodes
        .iter()
        .position(|node| node["stable_id"] == id)
        .context("raster slot remains in DOM export")?;
    let node = &nodes[index];
    let layout = &exported["layout"]["nodes"][index]["bounds"];
    let json = serde_json::to_string(&exported)?;
    check!(
        exported["version"] == 1
            && exported["dom"]["version"] == 1
            && node["role"] == "Group"
            && layout["width"] == 56.
            && layout["height"] == 56.
            && ![
                "\"raster_image\"",
                "\"raster\"",
                "\"rgba\"",
                "\"pixels\"",
                "\"source_uv\""
            ]
            .into_iter()
            .any(|key| json.contains(key))
            && json.len() < 180_000,
        "the legacy JSON keeps the passive slot and its bounds without the raster resource"
    );
    Ok(
        json!({"version":exported["version"],"slot":id,"bounds":layout,"json_bytes":json.len(),"pixel_payload":false}),
    )
}

async fn crop(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M188",
        "actual GPU pixels prove center crop, orientation, alpha, corner masks and list clipping",
    );
    resize(tab, 600, 820).await?;
    ready(tab, "!d.wide").await?;
    let wire = dom_wire(tab).await?;
    let mut variants = Vec::new();
    for style in ["baseline", "standard", "segmented"] {
        preset(tab, style).await?;
        ready(tab, &format!("d.variant === {}", js(style))).await?;
        let rect = box_of(tab, PROBE).await?;
        let slot = js(part(PROBE, "media"));
        let radii = tab.eval(&format!(
            "(() => {{const s=JSON.parse(window.gallery.export_dom()).dom.nodes.find(n=>n.stable_id==={slot}).style;
                return ['top_left','top_right','bottom_right','bottom_left'].map(k=>s.corners?.[k] ?? s.radius);
             }})()"
        )).await?;
        let radius = if style == "baseline" { 0 } else { 8 };
        check_eq!(
            radii,
            json!(vec![radius; 4]),
            "the authored style sets the exact media radius"
        );
        let image = capture(env, tab, &format!("crop-{style}")).await?;
        let [x, y, w, h] = rect;
        let pixels = [
            swatch(&image, sample(rect, 0.24, 0.24))?,
            swatch(&image, sample(rect, 0.76, 0.24))?,
            swatch(&image, sample(rect, 0.24, 0.76))?,
            swatch(&image, sample(rect, 0.76, 0.76))?,
        ];
        let background = swatch(&image, [x - 4., y + h * 0.76 - 1., 2., 2.])?;
        check!(near_rgb(pixels[0],GREEN) && near_rgb(pixels[1],BLUE)
            && near_rgb(pixels[3],YELLOW) &&
            [false,true].into_iter().any(|linear| near_rgb(pixels[2],alpha_mix(RED,background,linear))),
            "center crop retains the opaque quadrant order and half-alpha composites onto the row: {pixels:?}, background {background:?}");
        let unexpected = image
            .colors([x + 2., y + 2., w - 4., h - 4.])?
            .into_iter()
            .filter(|(color, _)| GUARDS.iter().any(|&guard| near_rgb(*color, guard)))
            .map(|(_, count)| count)
            .sum::<usize>();
        check_eq!(unexpected, 0, "cropping excludes both outside color bands");
        let corner = swatch(&image, [x + 1.25, y + 1.25, 0.5, 0.5])?;
        let corner_background = swatch(&image, [x - 4., y + 1.25, 1., 0.5])?;
        let expected_corner = if style == "baseline" {
            GREEN
        } else {
            corner_background
        };
        check!(near_rgb(corner,expected_corner),
            "{style} uses the correct square or rounded corner in actual paint: {corner:?}, expected {expected_corner:?}");
        if style != "baseline" {
            check!(
                near_rgb(swatch(&image, [x + 6., y + 3., 1., 1.])?, GREEN),
                "the rounded edge still shows pixels in the interior"
            );
        }
        variants.push(json!({"style":style,"rect":rect,"quadrants":pixels,"alpha_background":background,
            "corner":corner,"corner_background":corner_background,"radius":radius,"guard_pixels":unexpected}));
    }

    let probe = box_of(tab, PROBE).await?;
    let height = (probe[1] + 0.5 * probe[3] + 12.).round() as u32;
    resize(tab, 600, height).await?;
    let clip = bounds(tab, SCROLL).await?;
    let [x, y, w, h] = box_of(tab, PROBE).await?;
    let bottom = clip[1] + clip[3];
    check!(bottom > y+8. && bottom < y+h-8. && bottom+4. < f64::from(height),
        "the viewport cuts through the diagnostic raster and leaves space for an outside pixel sample");
    let clipped = capture(env, tab, "clipped-portrait").await?;
    let inside = swatch(&clipped, [x + 0.75 * w, y + 8., 2., 2.])?;
    let outside = swatch(&clipped, [x + 0.75 * w, bottom + 3., 2., 2.])?;
    check!(near_rgb(inside,BLUE) && near_rgb(outside,token(tab,"surface_container").await?),
        "the inner clip includes real artwork above the edge and prevents paint beyond it: {inside:?} {outside:?}");
    let [cx, cy, _, _] = tab.canvas_box().await?;
    tab.mouse_move(cx + x + w / 2., cy + y + 10.).await?;
    let device_scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("Chrome DPR")?;
    tab.wheel(0., 36. * device_scale).await?;
    ready(tab, "d.scroll_y >= 35").await?;
    let revealed = box_of(tab, PROBE).await?;
    check!(
        contains(bounds(tab, SCROLL).await?, revealed),
        "a real wheel over decorative media scrolls and reveals the whole raster: {revealed:?}"
    );
    Ok(
        json!({"variants":variants,"clipping":{"initial":[x,y,w,h],"clip":clip,"outside":outside,
        "inside":inside,"revealed":revealed,"scroll_y":data(tab).await?["scroll_y"]},"wire":wire}),
    )
}

async fn interactive(
    tab: &Tab,
    id: &str,
    name: &str,
    selection: Option<bool>,
    disabled: bool,
) -> Result<Value> {
    let target = projected(id);
    let element = tab
        .eval(&format!(
            "(() => {{const n={target}; return n && {{
          role:n.getAttribute('role'), name:n.getAttribute('aria-label'),
          selected:n.getAttribute('aria-selected'), checked:n.getAttribute('aria-checked'),
          disabled:n.getAttribute('aria-disabled'), tab:n.getAttribute('tabindex'),
          action:n.getAttribute('data-surface-action'),
          nested:n.querySelectorAll('[tabindex],[data-surface-action]').length,
        }};}})()",
        ))
        .await?;
    let role = if selection.is_some() {
        "option"
    } else {
        "button"
    };
    let selected_attribute = selection.map(|value| if value { "true" } else { "false" });
    check!(
        element["role"] == role
            && element["name"] == name
            && element["selected"].as_str() == selected_attribute
            && element["checked"].is_null()
            && element["disabled"].as_str() == disabled.then_some("true")
            && element["tab"].as_str() == (!disabled).then_some("-1")
            && element["action"].as_str() == (!disabled).then_some("press")
            && element["nested"] == 0,
        "one complete row owns the named command or selection: {id} {element}"
    );
    let tree = ax_tree(tab).await?;
    let node = ax(&tree, role, name)?;
    if let Some(selected) = selection {
        check_eq!(
            ax_property(node, "selected"),
            Some(&json!(selected)),
            "Chrome reports the independent selection"
        );
    }
    if disabled {
        check_eq!(
            ax_property(node, "disabled"),
            Some(&json!(true)),
            "Chrome exposes the row as disabled"
        );
    }
    Ok(json!({"attributes":element,"description":node["description"]["value"]}))
}

async fn tap(tab: &Tab, id: &str) -> Result<()> {
    let at = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({
            "type":"touchStart","touchPoints":[{"id":1,"x":at.x,"y":at.y}],
        }),
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

async fn reveal_disabled(tab: &Tab) -> Result<()> {
    let at = point(tab, SCROLL).await?;
    tab.mouse_move(at.x, at.y).await?;
    tab.wheel(0., 1200.).await?;
    ready(tab, "d.scroll_y > 30").await?;
    let clip = bounds(tab, SCROLL).await?;
    let media = box_of(tab, DISABLED).await?;
    check!(
        contains(clip, media),
        "the disabled image is actually reachable with pointer input: {clip:?} {media:?}"
    );
    Ok(())
}

async fn detail(tab: &Tab, id: &str, name: &str, count: u8) -> Result<Rect> {
    ready(
        tab,
        &format!(
            "d.details === {} && d.activations === {count} && d.selection_changes === 0",
            js(id)
        ),
    )
    .await?;
    let proxy = projected(DETAIL);
    check_eq!(
        tab.eval(&format!("{proxy}?.getAttribute('aria-modal')"))
            .await?,
        json!("true"),
        "the detail surface owns the accessible modal scope"
    );
    ax(&ax_tree(tab).await?, "dialog", name)?;
    check_eq!(
        tab.eval(&format!("{} === null", projected(LIST))).await?,
        json!(true),
        "the background media list leaves the accessible scope"
    );
    passive_artwork(tab, DETAIL_IMAGE).await?;
    bounds(tab, DETAIL_IMAGE).await
}

async fn enter_key(tab: &Tab, kind: &str, repeat: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({"type":kind,"key":"Enter","code":"Enter","windowsVirtualKeyCode":13,
            "location":0,"modifiers":0,"autoRepeat":repeat}),
    )
    .await?;
    Ok(())
}

async fn actions(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M189",
        "keyboard and a click on the poster open one named detail each and return focus",
    );
    preset(tab, "baseline").await?;
    preset(tab, "action").await?;
    ready(tab, "d.mode === 'action' && d.variant === 'baseline'").await?;
    list_semantics(tab, false).await?;
    let baseline = measurement(tab, POSTER).await?;
    check!(
        baseline["inset"] == 3. && baseline["label_gap"] == 16.,
        "interactive Baseline posters leave room for the inward focus stroke"
    );
    let poster_button = interactive(tab, POSTER, POSTER_NAME, None, false).await?;
    check!(
        poster_button["description"]
            .as_str()
            .is_some_and(|description| description.contains(POSTER_SUPPORT)),
        "the video row keeps its caption in Chrome AX"
    );
    passive_artwork(tab, &part(POSTER, "media")).await?;

    tab.eval(
        "(() => {
            window.mediaEnterKeys = [];
            document.addEventListener('keydown', event => {
                if (event.key !== 'Enter' || !event.isTrusted) return;
                window.mediaEnterKeys.push({id:event.target?.dataset.surfaceId,repeat:event.repeat});
            }, {capture:true,passive:true});
        })()",
    ).await?;
    focus_projected(tab, EARTH).await?;
    enter_key(tab, "keyDown", false).await?;
    let photo = detail(tab, EARTH, EARTH_NAME, 1).await?;
    focused(tab, DETAIL_BACK).await?;
    for _ in 0..2 {
        enter_key(tab, "keyDown", true).await?;
        tab.settle().await?;
        let held = data(tab).await?;
        check!(
            held["details"] == EARTH && held["activations"] == 1,
            "holding the opener's Enter leaves the focused detail open"
        );
    }
    enter_key(tab, "keyUp", false).await?;
    let trusted_keys = tab.eval("window.mediaEnterKeys").await?;
    check_eq!(
        trusted_keys,
        json!([
            {"id":EARTH,"repeat":false},
            {"id":DETAIL_BACK,"repeat":true},
            {"id":DETAIL_BACK,"repeat":true},
        ]),
        "trusted repeated Enter reached the new Back target without dismissing the detail"
    );
    tab.press("Escape").await?;
    ready(tab, "d.details === null && d.activations === 1").await?;
    focused(tab, EARTH).await?;

    focus_projected(tab, POSTER).await?;
    click(tab, &part(POSTER, "media")).await?;
    let poster = detail(tab, POSTER, POSTER_NAME, 2).await?;
    let caption = projected("list-media-detail-text");
    let record = tab.eval(&format!("{caption}?.textContent")).await?;
    check!(
        record
            .as_str()
            .is_some_and(|value| value.contains("ISS042-E-170757")),
        "the opened video detail identifies the actual poster"
    );
    let image = capture(env, tab, "detail").await?;
    let photo_colors = visible_photo(&image, poster, "opened poster detail")?;
    click(tab, DETAIL_BACK).await?;
    ready(tab, "d.details === null && d.activations === 2").await?;
    until(tab, &format!("s.focused === {}", js(POSTER))).await?;
    reveal_disabled(tab).await?;
    interactive(tab, DISABLED, "封存影像", None, true).await?;
    click(tab, &part(DISABLED, "media")).await?;
    tab.settle().await?;
    let final_state = data(tab).await?;
    check!(
        final_state["activations"] == 2 && final_state["details"].is_null(),
        "a physical click on disabled art has no action"
    );
    Ok(
        json!({"baseline":baseline,"photo_hero":photo,"poster_hero":poster,
        "poster_colors":photo_colors,"poster_record":record,"activations":2,
        "restored_keyboard":EARTH,"restored_pointer":POSTER,"disabled_blocked":true,"trusted_keys":trusted_keys}),
    )
}

async fn selected(tab: &Tab, ids: &[&str], count: u8) -> Result<()> {
    ready(
        tab,
        &format!("d.selection_changes === {count} && d.activations === 0 && d.details === null"),
    )
    .await?;
    check_eq!(
        data(tab).await?["selected"],
        json!(ids),
        "the exact independent selection set"
    );
    Ok(())
}

async fn selections(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M190",
        "media, checkbox and keyboard share one independent action per enabled row",
    );
    preset(tab, "select").await?;
    ready(tab, "d.mode === 'select'").await?;
    let semantics = list_semantics(tab, true).await?;
    selected(tab, &[], 0).await?;
    interactive(tab, EARTH, EARTH_NAME, Some(false), false).await?;
    interactive(tab, POSTER, POSTER_NAME, Some(false), false).await?;
    passive_artwork(tab, &part(EARTH, "mark")).await?;
    passive_artwork(tab, &part(POSTER, "mark")).await?;

    focus_projected(tab, EARTH).await?;
    tab.press("Enter").await?;
    selected(tab, &[EARTH], 1).await?;
    tab.press("ArrowDown").await?;
    focused(tab, POSTER).await?;
    selected(tab, &[EARTH], 1).await?;
    tab.press("Space").await?;
    selected(tab, &[EARTH, POSTER], 2).await?;
    interactive(tab, EARTH, EARTH_NAME, Some(true), false).await?;
    interactive(tab, POSTER, POSTER_NAME, Some(true), false).await?;
    capture(env, tab, "selection-light").await?;

    click(tab, &part(EARTH, "media")).await?;
    selected(tab, &[POSTER], 3).await?;
    interactive(tab, EARTH, EARTH_NAME, Some(false), false).await?;
    tap(tab, &part(POSTER, "mark")).await?;
    selected(tab, &[], 4).await?;
    interactive(tab, POSTER, POSTER_NAME, Some(false), false).await?;
    reveal_disabled(tab).await?;
    interactive(tab, DISABLED, "封存影像", Some(false), true).await?;
    tap(tab, &part(DISABLED, "media")).await?;
    selected(tab, &[], 4).await?;
    Ok(
        json!({"semantics":semantics,"keyboard":[EARTH,POSTER],"mouse_media":EARTH,
        "touch_mark":POSTER,"changes":4,"disabled_blocked":true}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step("M191","320 dp dark RTL at 200 percent keeps a full media label, mirrored checkbox and one hit target");
    for name in ["theme", "rtl", "large-text", "select"] {
        preset(tab, name).await?;
    }
    resize(tab, 320, 900).await?;
    until(tab, "s.dark === true").await?;
    ready(
        tab,
        "d.rtl && d.text_scale === 2 && !d.wide && d.mode === 'select'",
    )
    .await?;
    focus_projected(tab, EARTH).await?;
    tab.press("ArrowDown").await?;
    focused(tab, POSTER).await?;
    selected(tab, &[], 0).await?;
    let poster = box_of(tab, POSTER).await?;
    let support = bounds(tab, &part(POSTER, "supporting")).await?;
    let title = bounds(tab, &part(POSTER, "headline")).await?;
    let mark = bounds(tab, &part(POSTER, "mark")).await?;
    let row = bounds(tab, POSTER).await?;
    let clip = bounds(tab, SCROLL).await?;
    check!(contains(clip,row) && contains([0.,0.,320.,900.],row)
        && [poster,support,title,mark].into_iter().all(|element| contains(row,element))
        && row[2] >= 48. && row[3] >= 48. && support[3] > 48.,
        "the whole large row and wrapped text stay visible on the 320 dp surface: {row:?}, {clip:?}, {support:?}");
    check!(near(poster[3],poster[2]*0.56) && poster[2] <= 100. && support[2] > poster[2]
        && poster[0] >= support[0]+support[2]-0.75 && mark[0]+mark[2] < support[0]
        && near(mark[2],18.) && near(mark[3],18.),
        "RTL mirrors media and checkbox, and the copy remains wider than the poster: {poster:?} {support:?} {mark:?}");
    let item = interactive(tab, POSTER, POSTER_NAME, Some(false), false).await?;
    check!(
        item["description"]
            .as_str()
            .is_some_and(|value| value.contains(POSTER_SUPPORT)),
        "Chrome retains the complete wrapped supporting text"
    );
    let image = capture(env, tab, "compact-dark-rtl").await?;
    let poster_colors = visible_photo(&image, poster, "compact ISS poster")?;
    let support_color = token(tab, "on_surface_variant").await?;
    let ink = image
        .colors(support)?
        .into_iter()
        .filter(|(color, _)| near_rgb(*color, support_color))
        .map(|(_, count)| count)
        .sum::<usize>();
    check!(
        ink >= 32,
        "the wrapped 200% caption paints its true foreground in Chrome: {ink}"
    );
    tap(tab, &part(POSTER, "media")).await?;
    selected(tab, &[POSTER], 1).await?;
    interactive(tab, POSTER, POSTER_NAME, Some(true), false).await?;
    Ok(
        json!({"viewport":[320,900],"text_scale":2,"rtl":true,"row":row,"clip":clip,
        "poster":poster,"support":support,"mark":mark,"poster_colors":poster_colors,"support_ink":ink}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let profiles = profiles(env, tab).await?;
    reload(tab, url).await?;
    let crop = crop(env, tab).await?;
    reload(tab, url).await?;
    let actions = actions(env, tab).await?;
    reload(tab, url).await?;
    let selection = selections(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material list media browser errors: {errors:?}"
    );
    env.write_json(
        "browser-material-list-media.json",
        &json!({
            "browser":chrome.version,"profiles":profiles,"crop":crop,"actions":actions,
            "selection":selection,"compact":compact,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json("material-list-media-failure.json",&json!({
            "error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors(),
        }));
        let _ = tab
            .screenshot(&env.artifact("material-list-media-failure.png"))
            .await;
    }
    result.context("Material 3 media list")
}
