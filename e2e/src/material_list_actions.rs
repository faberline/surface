//! Material 3 multi-action Lists in headless Chrome WebGPU.

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

const GROUP: &str = "list-actions";
const SCROLL: &str = "list-actions-scroll";
const BEFORE: &str = "list-actions-demo-before";
const AFTER: &str = "list-actions-demo-after";
const A: &str = "list-action-a";
const A_PIN: &str = "list-action-a-pin";
const A_MORE: &str = "list-action-a-more";
const B: &str = "list-action-b";
const B_PIN: &str = "list-action-b-pin";
const B_MORE: &str = "list-action-b-more";
const C: &str = "list-action-c";
const C_PIN: &str = "list-action-c-pin";
const C_ARCHIVE: &str = "list-action-c-archive";
const C_MORE: &str = "list-action-c-more";
const F: &str = "list-action-f";
const F_MORE: &str = "list-action-f-more";
const LONG: &str = "長文字會在窄畫面自動換行";
const SUPPORT: &str = "放大文字後仍可完整閱讀";

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["list_actions"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.list_actions)")).await?;
    tab.settle().await
}

async fn open(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1644).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1600).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "list-actions-demo-show").await?;
    ready(
        tab,
        "d.visible && d.variant === 'segmented' && d.activations === 0
         && d.last === null && !d.rtl && d.text_scale === 1",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("list-actions-demo-{name}")).await
}

async fn enter_list(tab: &Tab, first: &str) -> Result<()> {
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, first).await
}

async fn follow(tab: &Tab, steps: &[(&str, &str)]) -> Result<()> {
    for (key, id) in steps {
        tab.press(key).await?;
        focused(tab, id).await?;
    }
    Ok(())
}

async fn activated(tab: &Tab, count: u8, id: &str) -> Result<()> {
    ready(
        tab,
        &format!("d.activations === {count} && d.last === {}", js(id)),
    )
    .await
}

async fn checked(tab: &Tab, id: &str, value: bool) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.actions.find(action => action.id === {})?.checked === {value}",
            js(id)
        ),
    )
    .await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-list-actions-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn button(
    tab: &Tab,
    id: &str,
    row: &str,
    name: &str,
    disabled: bool,
    pressed: Option<bool>,
) -> Result<Value> {
    let proxy = projected(id);
    let dom = tab
        .eval(&format!(
            "(() => {{
              const b = {proxy};
              return b && {{
                role:b.getAttribute('role'), name:b.getAttribute('aria-label'),
                pressed:b.getAttribute('aria-pressed'), checked:b.getAttribute('aria-checked'),
                selected:b.getAttribute('aria-selected'), disabled:b.getAttribute('aria-disabled'),
                target:b.getAttribute('data-surface-action'), tabindex:b.getAttribute('tabindex'),
                row:b.closest('[role=listitem]')?.dataset.surfaceId,
                parent_button:b.parentElement?.closest('[role=button]')?.dataset.surfaceId ?? null,
                nested_actions:b.querySelectorAll('[role=button],[data-surface-action],[tabindex]').length,
              }};
            }})()"
        ))
        .await?;
    let expected = pressed.map(|value| if value { "true" } else { "false" });
    check!(
        dom["role"] == "button"
            && dom["name"] == name
            && dom["row"] == row
            && dom["parent_button"].is_null()
            && dom["nested_actions"] == 0
            && dom["checked"].is_null()
            && dom["selected"].is_null()
            && dom["pressed"].as_str() == expected
            && dom["disabled"].as_str() == disabled.then_some("true")
            && dom["target"].as_str() == (!disabled).then_some("press")
            && dom["tabindex"].as_str() == (!disabled).then_some("-1"),
        "each named button owns just its own available action within {row}: {id}: {dom}"
    );
    let tree = ax_tree(tab).await?;
    let exposed = ax(&tree, "button", name)?;
    let pressed_ax = ax_property(exposed, "pressed");
    if let Some(pressed) = pressed {
        check!(
            pressed_ax == Some(&json!(pressed)) || pressed_ax.and_then(Value::as_str) == expected,
            "Chrome AX exposes the toggle as pressed={pressed}: {exposed}"
        );
    }
    if disabled {
        check_eq!(
            ax_property(exposed, "disabled"),
            Some(&json!(true)),
            "Chrome AX exposes the independently disabled button"
        );
    }
    Ok(json!({"dom":dom,"description":exposed["description"]["value"],"chrome_pressed":pressed_ax}))
}

async fn semantics(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M172",
        "one named list exposes sibling buttons with independent disabled and toggle states",
    );
    let group = projected(GROUP);
    let scope = tab.eval(&format!(
        "(() => {{
            const list={group};
            const row=list?.querySelector('[data-surface-id=\"list-action-a-item\"]');
            return list && row && {{
              list_role:list.getAttribute('role'), label:list.getAttribute('aria-label'),
              list_multi:list.getAttribute('aria-multiselectable'),
              row_role:row.getAttribute('role'), row_action:row.getAttribute('data-surface-action'),
              row_tab:row.getAttribute('tabindex'), row_selected:row.getAttribute('aria-selected'),
              siblings:[...row.querySelectorAll('[role=button]')].map(button=>button.dataset.surfaceId),
              options:list.querySelectorAll('[role=option],[role=listbox]').length,
            }};
         }})()"
    )).await?;
    check!(
        scope["list_role"] == "list"
            && scope["label"] == "書籤動作"
            && scope["list_multi"].is_null()
            && scope["row_role"] == "listitem"
            && scope["row_action"].is_null()
            && scope["row_tab"].is_null()
            && scope["row_selected"].is_null()
            && scope["options"] == 0
            && scope["siblings"] == json!([A, A_PIN, A_MORE]),
        "list and row are semantic owners for independent controls: {scope}"
    );
    let tree = ax_tree(tab).await?;
    ax(&tree, "list", "書籤動作")?;
    button(tab, A, "list-action-a-item", "山", false, None).await?;
    let pin = button(
        tab,
        A_PIN,
        "list-action-a-item",
        "固定山",
        false,
        Some(true),
    )
    .await?;
    button(
        tab,
        A_MORE,
        "list-action-a-item",
        "更多山的選項",
        false,
        None,
    )
    .await?;
    button(tab, B, "list-action-b-item", "海", true, None).await?;
    let b_pin = button(
        tab,
        B_PIN,
        "list-action-b-item",
        "固定海",
        false,
        Some(false),
    )
    .await?;
    button(
        tab,
        B_MORE,
        "list-action-b-item",
        "更多海的選項",
        true,
        None,
    )
    .await?;
    let name = button(tab, C, "list-action-c-item", LONG, false, None).await?;
    check!(
        name["description"]
            .as_str()
            .is_some_and(|text| text.contains(SUPPORT)),
        "the long primary button exposes the separate supporting text: {name}"
    );
    capture(env, tab, "siblings-light").await?;
    Ok(json!({"scope":scope,"a_pin":pin,"b_pin":b_pin,"long_primary":name}))
}

fn no_overlap(a: Rect, b: Rect) -> bool {
    a[0].max(b[0]) >= (a[0] + a[2]).min(b[0] + b[2]) - 0.25
        || a[1].max(b[1]) >= (a[1] + a[3]).min(b[1] + b[3]) - 0.25
}

async fn targets(tab: &Tab, row: &str, ids: &[&str]) -> Result<Vec<Rect>> {
    let row = bounds(tab, row).await?;
    let mut results: Vec<Rect> = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let target = bounds(tab, id).await?;
        let inline = results
            .first()
            .is_some_and(|primary| target[1] < primary[1] + primary[3] - 0.75);
        let expected_height = if inline { row[3] } else { 48. };
        check!(
            contains(row, target) && target[2] >= 47.5 && target[3] >= 47.5
                && (index == 0 || (near(target[2], 48.) && near(target[3], expected_height))),
            "supplementary targets are 48 dp wide and match the row height inline or 48 dp in a strip:
             {id} {target:?} {row:?}"
        );
        check!(
            results.iter().all(|other| no_overlap(*other, target)),
            "the targets are disjoint: {id} {target:?} previous {results:?}"
        );
        results.push(target);
    }
    Ok(results)
}

async fn color(tab: &Tab, role: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(data(tab).await?["colors"][role].clone())
        .with_context(|| format!("multi-action list color {role}"))?;
    check_eq!(a, 255, "the sampled palette token is opaque");
    Ok([r, g, b])
}

fn pixel_matches(image: &Raster, sample: Rect, expected: Rgb) -> Result<usize> {
    Ok(image
        .colors(sample)?
        .into_iter()
        .filter(|(color, _)| color.iter().zip(expected).all(|(x, y)| x.abs_diff(y) <= 5))
        .map(|(_, pixels)| pixels)
        .sum())
}

async fn geometry(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M173",
        "all three styles preserve 48 dp wide supplementary targets and the segmented gap",
    );
    let mut profiles = Vec::new();
    for (variant, gap) in [("segmented", 2.), ("standard", 0.), ("baseline", 0.)] {
        preset(tab, variant).await?;
        ready(tab, &format!("d.variant === {}", js(variant))).await?;
        let a = targets(tab, "list-action-a-item", &[A, A_PIN, A_MORE]).await?;
        let b = targets(tab, "list-action-b-item", &[B, B_PIN, B_MORE]).await?;
        let c = targets(tab, "list-action-c-item", &[C, C_PIN, C_ARCHIVE, C_MORE]).await?;
        let first = bounds(tab, "list-action-a-item").await?;
        let second = bounds(tab, "list-action-b-item").await?;
        check!(
            near(second[1] - first[1] - first[3], gap)
                && near(first[3], 56.)
                && near(second[3], 56.)
                && a[0][0] < a[1][0]
                && a[1][0] < a[2][0]
                && c[0][0] < c[1][0]
                && c[1][0] < c[2][0]
                && c[2][0] < c[3][0],
            "{variant} keeps wide authored placement, minimum row heights and the correct gap"
        );
        let image = capture(env, tab, &format!("{variant}-targets")).await?;
        let checked_ink = color(tab, "primary").await?;
        let unchecked_ink = color(tab, "on_surface_variant").await?;
        let checked_icon = bounds(tab, "list-action-a-pin-icon").await?;
        let unchecked_icon = bounds(tab, "list-action-b-pin-icon").await?;
        let icon_size = if variant == "baseline" { 24. } else { 20. };
        let checked_pixels = pixel_matches(&image, checked_icon, checked_ink)?;
        let unchecked_pixels = pixel_matches(&image, unchecked_icon, unchecked_ink)?;
        check!(
            [checked_icon, unchecked_icon]
                .iter()
                .all(|icon| near(icon[2], icon_size) && near(icon[3], icon_size))
                && checked_ink != unchecked_ink
                && checked_pixels >= 16
                && unchecked_pixels >= 16,
            "{variant} paints distinct pressed and unpressed pin ink in {icon_size} dp glyphs:
             checked {checked_pixels}, unchecked {unchecked_pixels}"
        );
        let mut gap_pixels = None;
        if gap > 0. {
            let background = color(tab, "surface_container").await?;
            let sample = [first[0] + 50., first[1] + first[3] + 0.5, 12., 1.];
            let pixels = pixel_matches(&image, sample, background)?;
            check!(
                pixels >= 36,
                "the 2 dp segmented gap displays the scene surface"
            );
            gap_pixels = Some(pixels);
        }
        profiles.push(json!({"variant":variant,"gap":gap,"a":a,"b":b,"c":c,
            "checked_ink_pixels":checked_pixels,"unchecked_ink_pixels":unchecked_pixels,
            "gap_pixels":gap_pixels}));
    }
    Ok(json!(profiles))
}

async fn keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M174",
        "four arrows follow authored order, skip disabled actions and wrap through clipped rows",
    );
    let initial = bounds(tab, F_MORE).await?;
    let clip = bounds(tab, SCROLL).await?;
    check!(
        initial[1] >= clip[1] + clip[3],
        "the final supplementary control begins fully clipped"
    );
    check_eq!(
        tab.eval(&format!("{} === null", projected(F_MORE))).await?,
        json!(true),
        "the fully clipped last action has no keyboard proxy"
    );
    enter_list(tab, A).await?;
    follow(
        tab,
        &[
            ("ArrowRight", A_PIN),
            ("ArrowDown", A_MORE),
            ("ArrowRight", B_PIN),
            ("ArrowDown", C),
            ("ArrowLeft", B_PIN),
            ("ArrowUp", A_MORE),
            ("ArrowLeft", A_PIN),
            ("ArrowUp", A),
            ("ArrowLeft", F_MORE),
        ],
    )
    .await?;
    let revealed = bounds(tab, F_MORE).await?;
    check!(
        revealed[1] < initial[1] && contains(bounds(tab, SCROLL).await?, revealed),
        "wrapping reveals the full final action: {revealed:?}"
    );
    button(
        tab,
        F_MORE,
        "list-action-f-item",
        "更多森林的選項",
        false,
        None,
    )
    .await?;
    follow(
        tab,
        &[("ArrowRight", A), ("ArrowUp", F_MORE), ("ArrowDown", A)],
    )
    .await?;
    ready(tab, "d.activations === 0 && d.last === null").await?;
    checked(tab, A_PIN, true).await?;
    checked(tab, B_PIN, false).await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, A).await?;
    follow(tab, &[("ArrowRight", A_PIN)]).await?;
    tab.press("Enter").await?;
    activated(tab, 1, A_PIN).await?;
    checked(tab, A_PIN, false).await?;
    button(
        tab,
        A_PIN,
        "list-action-a-item",
        "固定山",
        false,
        Some(false),
    )
    .await?;
    follow(tab, &[("ArrowDown", A_MORE), ("ArrowRight", B_PIN)]).await?;
    tab.press("Space").await?;
    activated(tab, 2, B_PIN).await?;
    checked(tab, B_PIN, true).await?;
    checked(tab, A_PIN, false).await?;
    capture(env, tab, "keyboard-independent-focus").await?;
    Ok(
        json!({"first":A,"reverse_entry":A,"last_initial":initial,"last_revealed":revealed,
        "enter":A_PIN,"space":B_PIN,"activations":2}),
    )
}

async fn mouse(tab: &Tab, kind: &str, (x, y): (f64, f64), held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,"button":if kind == "mouseMoved" { "none" } else { "left" },
            "buttons":u8::from(held),"clickCount":1}),
    )
    .await?;
    tab.settle().await
}

async fn touch(tab: &Tab, id: &str, cancel: bool) -> Result<()> {
    let target = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"id":1,"x":target.x,"y":target.y}]}),
    )
    .await?;
    tab.settle().await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":if cancel { "touchCancel" } else { "touchEnd" },"touchPoints":[]}),
    )
    .await?;
    tab.settle().await
}

async fn pointer(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M175",
        "trusted pointer and touch activate one sibling with canceled, crossed and disabled input inert",
    );
    tab.eval(
        "(() => {
           window.listActionContacts=[];
           for (const kind of ['pointerdown','pointerup','pointercancel']) {
               document.querySelector('canvas').addEventListener(kind,event => {
                   if (event.isTrusted)
                       window.listActionContacts.push({kind,type:event.pointerType});
               },{capture:true,passive:true});
           }
        })()",
    )
    .await?;
    click(tab, A).await?;
    activated(tab, 1, A).await?;
    checked(tab, A_PIN, true).await?;
    click(tab, "list-action-a-pin-icon").await?;
    activated(tab, 2, A_PIN).await?;
    checked(tab, A_PIN, false).await?;

    let start = point(tab, A).await?;
    let end = point(tab, A_MORE).await?;
    mouse(tab, "mouseMoved", (start.x, start.y), false).await?;
    mouse(tab, "mousePressed", (start.x, start.y), true).await?;
    activated(tab, 2, A_PIN).await?;
    mouse(tab, "mouseMoved", (end.x, end.y), true).await?;
    mouse(tab, "mouseReleased", (end.x, end.y), false).await?;
    activated(tab, 2, A_PIN).await?;
    click(tab, B).await?;
    click(tab, B_MORE).await?;
    let parent = point(tab, "list-action-a-item").await?;
    let gap = (parent.x, parent.canvas[1] + parent.b[1] + parent.b[3] + 1.);
    tab.click(gap.0, gap.1).await?;
    mouse(tab, "mouseMoved", (start.x, start.y), false).await?;
    mouse(tab, "mousePressed", (start.x, start.y), true).await?;
    mouse(tab, "mouseMoved", gap, true).await?;
    mouse(tab, "mouseReleased", gap, false).await?;
    activated(tab, 2, A_PIN).await?;

    touch(tab, B_PIN, true).await?;
    activated(tab, 2, A_PIN).await?;
    checked(tab, B_PIN, false).await?;
    touch(tab, "list-action-a-more-icon", false).await?;
    activated(tab, 3, A_MORE).await?;
    touch(tab, B_PIN, false).await?;
    activated(tab, 4, B_PIN).await?;
    checked(tab, B_PIN, true).await?;
    checked(tab, A_PIN, false).await?;

    let events = tab.eval("window.listActionContacts").await?;
    let real = events.as_array().context("trusted pointer log")?;
    for (device, kind) in [
        ("mouse", "pointerdown"),
        ("mouse", "pointerup"),
        ("touch", "pointerdown"),
        ("touch", "pointerup"),
        ("touch", "pointercancel"),
    ] {
        check!(
            real.iter()
                .any(|event| event["type"] == device && event["kind"] == kind),
            "a trusted {device} {kind} reached the canvas"
        );
    }
    capture(env, tab, "mouse-and-touch-targets").await?;
    Ok(
        json!({"accepted":[A,A_PIN,A_MORE,B_PIN],"actual_count":4,"crossed_release_inert":true,
        "disabled_and_gap_inert":true,"touch_cancel_inert":true,"events":events}),
    )
}

async fn dark_rtl_large(tab: &Tab) -> Result<()> {
    for name in ["theme", "rtl", "large-text"] {
        preset(tab, name).await?;
    }
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2").await
}

async fn enter_c(tab: &Tab) -> Result<()> {
    enter_list(tab, A).await?;
    follow(
        tab,
        &[
            ("ArrowRight", A_PIN),
            ("ArrowRight", A_MORE),
            ("ArrowRight", B_PIN),
            ("ArrowRight", C),
        ],
    )
    .await
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M176",
        "dark RTL at 200 percent moves supplementary actions below the complete long primary label",
    );
    dark_rtl_large(tab).await?;
    resize(tab, 320, 900).await?;
    ready(tab, "d.available_width === 296").await?;
    enter_c(tab).await?;
    let c = targets(tab, "list-action-c-item", &[C, C_PIN, C_ARCHIVE, C_MORE]).await?;
    let row = bounds(tab, "list-action-c-item").await?;
    let headline = bounds(tab, "list-action-c-headline").await?;
    let support = bounds(tab, "list-action-c-supporting").await?;
    let leading = bounds(tab, "list-action-c-leading").await?;
    let viewport = [0., 0., 320., 900.];
    check!(
        contains(viewport, row)
            && [headline, support, leading].into_iter().all(|slot| contains(c[0], slot))
            && headline[3] > 55.
            && leading[0] >= headline[0] + headline[2] - 0.75
            && support[1] >= headline[1] + headline[3] - 0.75,
        "the primary exposes all wrapped text and the RTL leading icon: {row:?}, {headline:?}, {support:?}, {leading:?}"
    );
    check!(
        c[1..]
            .iter()
            .all(|target| target[1] >= c[0][1] + c[0][3] - 0.75)
            && c[1][0] > c[2][0]
            && c[2][0] > c[3][0],
        "all three controls reflow below the row text and mirror physically in RTL: {c:?}"
    );
    let named = button(tab, C, "list-action-c-item", LONG, false, None).await?;
    check!(
        named["description"]
            .as_str()
            .is_some_and(|text| text.contains(SUPPORT)),
        "the complete large primary name keeps its supporting description"
    );
    follow(
        tab,
        &[
            ("ArrowRight", C_PIN),
            ("ArrowDown", C_ARCHIVE),
            ("ArrowRight", C_MORE),
            ("ArrowLeft", C_ARCHIVE),
        ],
    )
    .await?;
    ready(tab, "d.activations === 0 && d.last === null").await?;
    tab.press("Enter").await?;
    activated(tab, 1, C_ARCHIVE).await?;
    checked(tab, C_PIN, false).await?;
    let clip = bounds(tab, SCROLL).await?;
    let revealed = targets(tab, "list-action-c-item", &[C, C_PIN, C_ARCHIVE, C_MORE]).await?;
    check!(
        revealed.iter().all(|target| contains(clip, *target)),
        "the scroller reveals all the text and supplementary actions together: {revealed:?} in {clip:?}"
    );
    capture(env, tab, "compact-dark-rtl-reflow").await?;
    Ok(
        json!({"row":row,"targets":c,"headline":headline,"support":support,
        "chrome_primary":named,"rtl_arrow_action":C_ARCHIVE,"revealed_targets":revealed}),
    )
}

async fn clip_of(tab: &Tab, id: &str) -> Result<Rect> {
    let info = data(tab).await?;
    let node = info["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["id"] == id))
        .with_context(|| format!("action inspector clip for {id}"))?;
    serde_json::from_value(node["clip"].clone()).context("actual inherited clip")
}

async fn resize_focus(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M177",
        "the same supplementary action stays focused across reflow and short resize",
    );
    dark_rtl_large(tab).await?;
    enter_c(tab).await?;
    follow(
        tab,
        &[
            ("ArrowDown", C_PIN),
            ("ArrowRight", C_ARCHIVE),
            ("ArrowDown", C_MORE),
        ],
    )
    .await?;
    let wide = bounds(tab, C_MORE).await?;
    let wide_primary = bounds(tab, C).await?;
    check!(
        wide[1] < wide_primary[1] + wide_primary[3] - 0.75,
        "the action starts inline with the wide primary target"
    );
    resize(tab, 320, 900).await?;
    focused(tab, C_MORE).await?;
    let narrow = bounds(tab, C_MORE).await?;
    let compact_primary = bounds(tab, C).await?;
    check!(
        narrow[1] >= compact_primary[1] + compact_primary[3] - 0.75
            && contains(clip_of(tab, C_MORE).await?, narrow),
        "the same focused control moves into the supplementary strip"
    );
    resize(tab, 320, 240).await?;
    focused(tab, C_MORE).await?;
    let short = bounds(tab, C_MORE).await?;
    check!(
        contains([0., 0., 320., 240.], short)
            && contains(clip_of(tab, C_MORE).await?, short)
            && near(short[2], 48.)
            && near(short[3], 48.),
        "the whole focused supplementary action stays visible after the short resize: {short:?}"
    );
    tab.press("Space").await?;
    activated(tab, 1, C_MORE).await?;
    capture(env, tab, "short-dark-rtl-retained-focus").await?;
    resize(tab, 960, 820).await?;
    focused(tab, C_MORE).await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    preset(tab, "reorder").await?;
    ready(tab, "d.actions[0]?.id === 'list-action-f'").await?;
    enter_list(tab, F).await?;
    follow(tab, &[("ArrowRight", F_MORE)]).await?;
    preset(tab, "remove").await?;
    ready(
        tab,
        "d.actions.every(action => action.id !== 'list-action-f-more')",
    )
    .await?;
    enter_list(tab, F).await?;
    follow(tab, &[("ArrowRight", "list-action-e")]).await?;
    activated(tab, 1, C_MORE).await?;
    Ok(
        json!({"id":C_MORE,"wide":wide,"narrow":narrow,"short":short,
        "checked_no_new_activation":true,"reordered_entry":F,"after_removal":"list-action-e"}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    open(tab, url).await?;
    let semantics = semantics(env, tab).await?;
    open(tab, url).await?;
    let geometry = geometry(env, tab).await?;
    open(tab, url).await?;
    let keyboard = keyboard(env, tab).await?;
    open(tab, url).await?;
    let pointer = pointer(env, tab).await?;
    open(tab, url).await?;
    let compact = compact(env, tab).await?;
    open(tab, url).await?;
    let resize_focus = resize_focus(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material multi-action list page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-list-actions.json",
        &json!({"browser":chrome.version,"semantics":semantics,"geometry":geometry,
            "keyboard":keyboard,"pointer":pointer,"compact":compact,"resize_focus":resize_focus,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1644, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-list-actions-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-list-actions-failure.png"))
            .await;
    }
    result.context("Material 3 multi-action list")
}
