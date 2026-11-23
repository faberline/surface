//! Controlled Material list multiple selection in Chrome WebGPU.

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

const GROUP: &str = "lists-items";
const SCROLL: &str = "lists-scroll";
const BEFORE: &str = "lists-demo-before";
const AFTER: &str = "lists-demo-after";
const A: &str = "lists-a";
const B: &str = "lists-b";
const C: &str = "lists-c";
const D: &str = "lists-d";
const F: &str = "lists-f";
const LONG: &str = "長文字會在窄畫面自動換行";

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["lists"].take())
}

async fn ready(tab: &Tab, expression: &str) -> Result<()> {
    until(tab, &format!("(d => {expression})(s.lists)")).await?;
    tab.settle().await
}

async fn open(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1644).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1600).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "lists-demo-show").await?;
    ready(
        tab,
        "d.visible && d.variant === 'segmented' && d.mode === 'select' && d.selected === 'lists-c'
         && d.activations === 0 && !d.rtl && d.text_scale === 1",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("lists-demo-{name}")).await
}

async fn selection(tab: &Tab, ids: &[&str], activations: u8) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.mode === 'multi' && d.activations === {activations}
             && JSON.stringify(d.selected_ids) === {}",
            js(json!(ids).to_string())
        ),
    )
    .await
}

async fn open_multi(tab: &Tab, url: &str) -> Result<()> {
    open(tab, url).await?;
    preset(tab, "multi").await?;
    selection(tab, &[C, F], 0).await
}

async fn enter_from_before(tab: &Tab, id: &str) -> Result<()> {
    focus_projected(tab, BEFORE).await?;
    tab.press("Tab").await?;
    focused(tab, id).await
}

fn mark(id: &str) -> String {
    format!("{id}-mark")
}

async fn node(tab: &Tab, id: &str) -> Result<Value> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})",
        js(id)
    ))
    .await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-lists-multi-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn listbox(tab: &Tab, multi: bool) -> Result<Value> {
    let group = projected(GROUP);
    let projected_group = tab.eval(&format!(
        "(() => {{
           const element = {group};
           return element && {{
               role:element.getAttribute('role'), name:element.getAttribute('aria-label'),
               multi:element.getAttribute('aria-multiselectable'),
               options:[...element.querySelectorAll('[role=option]')].map(option => option.dataset.surfaceId),
               nested_checkboxes:element.querySelectorAll('[role=checkbox],input[type=checkbox]').length,
           }};
        }})()"
    )).await?;
    let expected = multi.then_some("true");
    check!(
        projected_group["role"] == "listbox"
            && projected_group["name"] == "書籤"
            && projected_group["multi"].as_str() == expected
            && projected_group["nested_checkboxes"] == 0,
        "the named listbox uses its mode-specific ARIA state without nested checkboxes: {projected_group}"
    );
    let tree = ax_tree(tab).await?;
    let exposed = ax(&tree, "listbox", "書籤")?;
    check_eq!(
        ax_property(exposed, "multiselectable") == Some(&json!(true)),
        multi,
        "Chrome AX reflects the list's selection mode"
    );
    Ok(projected_group)
}

async fn option(tab: &Tab, id: &str, label: &str, selected: bool, disabled: bool) -> Result<Value> {
    let element = projected(id);
    let ornament = projected(&mark(id));
    let value = tab.eval(&format!(
        "(() => {{
            const option={element};
            const mark={ornament};
            return option && {{
              name:option.getAttribute('aria-label'), role:option.getAttribute('role'),
              selected:option.getAttribute('aria-selected'), checked:option.getAttribute('aria-checked'),
              disabled:option.getAttribute('aria-disabled'), tabindex:option.getAttribute('tabindex'),
              press:option.getAttribute('data-surface-action'),
              nested:option.querySelectorAll('[tabindex],[data-surface-action],button,input').length,
              mark_press:mark?.getAttribute('data-surface-action') ?? null,
              mark_tab:mark?.getAttribute('tabindex') ?? null,
            }};
         }})()"
    )).await?;
    let selected_value = if selected { "true" } else { "false" };
    check!(
        value["role"] == "option"
            && value["name"] == label
            && value["checked"].is_null()
            && value["selected"] == selected_value
            && value["nested"] == 0
            && value["mark_press"].is_null()
            && value["mark_tab"].is_null()
            && value["press"].as_str() == (!disabled).then_some("press")
            && value["tabindex"].as_str() == (!disabled).then_some("-1"),
        "the full named row owns one option and the checkbox paint is passive: {id} {value}"
    );
    let tree = ax_tree(tab).await?;
    let exposed = ax(&tree, "option", label)?;
    check_eq!(
        ax_property(exposed, "selected"),
        Some(&json!(selected)),
        format!("Chrome AX exposes the selected state of {label}")
    );
    if disabled {
        check!(
            value["disabled"] == "true" && ax_property(exposed, "disabled") == Some(&json!(true)),
            "Chrome AX names the disabled row without offering its action: {value}"
        );
    }
    Ok(json!({"dom":value,"chrome_name":exposed["name"]["value"],
        "chrome_selected":ax_property(exposed,"selected"),"description":exposed["description"]["value"]}))
}

async fn semantics(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M167",
        "one named listbox exposes multiple selected options and keeps single selection intact",
    );
    open(tab, url).await?;
    let single = listbox(tab, false).await?;
    option(tab, C, "河", true, false).await?;
    preset(tab, "multi").await?;
    selection(tab, &[C, F], 0).await?;
    let multi = listbox(tab, true).await?;
    option(tab, A, "山", false, false).await?;
    option(tab, B, "海", false, true).await?;
    let river = option(tab, C, "河", true, false).await?;
    check!(
        river["description"]
            .as_str()
            .is_some_and(|text| text.contains("河岸的步道")),
        "the selected row keeps its full name and a separate supporting description"
    );
    enter_from_before(tab, C).await?;
    tab.press("End").await?;
    focused(tab, F).await?;
    let forest = option(tab, F, "森林", true, false).await?;
    selection(tab, &[C, F], 0).await?;
    capture(env, tab, "selected-listbox").await?;
    preset(tab, "select").await?;
    ready(
        tab,
        "d.mode === 'select' && d.selected === 'lists-c' && d.activations === 0",
    )
    .await?;
    listbox(tab, false).await?;
    enter_from_before(tab, C).await?;
    option(tab, C, "河", true, false).await?;
    Ok(
        json!({"single":single,"multi":multi,"initial_selection":[C,F],"selected_river":river,
        "selected_forest":forest,"single_retained":true}),
    )
}

async fn held_activation(tab: &Tab, name: &str, ids: &[&str], activations: u8) -> Result<()> {
    let key = crate::keys::key(name)?;
    for repeat in [false, true, true] {
        tab.cdp(
            "Input.dispatchKeyEvent",
            json!({
                "type":"keyDown","key":key.key,"code":key.code,"windowsVirtualKeyCode":key.key_code,
                "modifiers":0,"location":key.location,"text":key.text,"unmodifiedText":key.text,
                "autoRepeat":repeat,
            }),
        )
        .await?;
        if repeat {
            tab.settle().await?;
            let current = data(tab).await?;
            check!(
                current["selected_ids"] == json!(ids) && current["activations"] == activations,
                "holding {name} does not send another row toggle: {current}"
            );
        } else {
            selection(tab, ids, activations).await?;
        }
    }
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({"type":"keyUp","key":key.key,"code":key.code,"windowsVirtualKeyCode":key.key_code,
            "modifiers":0,"location":key.location}),
    )
    .await?;
    selection(tab, ids, activations).await
}

async fn keyboard(tab: &Tab) -> Result<Value> {
    step(
        "M168",
        "keyboard wraps and skips disabled rows, and held Space and Enter toggle once each",
    );
    tab.eval(
        "(() => {
            window.multiListKeys = [];
            document.addEventListener('keydown', event => {
                const id = event.target?.dataset?.surfaceId;
                if (id?.startsWith('lists-') && event.isTrusted)
                    window.multiListKeys.push({id,key:event.key,repeat:event.repeat});
            }, {capture:true,passive:true});
        })()",
    )
    .await?;
    let before = bounds(tab, F).await?;
    let scroller = bounds(tab, SCROLL).await?;
    check!(
        before[1] >= scroller[1] + scroller[3],
        "the later checked row begins fully clipped: {before:?}"
    );
    enter_from_before(tab, C).await?;
    tab.press("ArrowUp").await?;
    focused(tab, A).await?;
    tab.press("ArrowUp").await?;
    focused(tab, F).await?;
    let revealed = bounds(tab, F).await?;
    check!(
        contains(bounds(tab, SCROLL).await?, revealed) && revealed[1] < before[1],
        "wrapping to the last option reveals it without toggling: {revealed:?}"
    );
    tab.press("ArrowDown").await?;
    focused(tab, A).await?;
    selection(tab, &[C, F], 0).await?;
    held_activation(tab, "Enter", &[A, C, F], 1).await?;
    option(tab, A, "山", true, false).await?;
    held_activation(tab, "Space", &[C, F], 2).await?;
    option(tab, A, "山", false, false).await?;
    tab.press("End").await?;
    focused(tab, F).await?;
    tab.press("Space").await?;
    selection(tab, &[C], 3).await?;
    option(tab, F, "森林", false, false).await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, C).await?;
    tab.press("Enter").await?;
    selection(tab, &[], 4).await?;
    tab.press("Tab").await?;
    focused(tab, AFTER).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, A).await?;
    let events = tab.eval("window.multiListKeys").await?;
    let keys = events.as_array().context("trusted list key event log")?;
    for (id, key, repeats) in [(A, "Enter", 2), (A, " ", 2), (F, " ", 0), (C, "Enter", 0)] {
        check_eq!(
            keys.iter()
                .filter(|event| event["id"] == id
                    && event["key"] == key
                    && event["repeat"] == false)
                .count(),
            1,
            format!("one trusted initial {key:?} reached {id}")
        );
        check_eq!(
            keys.iter()
                .filter(|event| event["id"] == id && event["key"] == key && event["repeat"] == true)
                .count(),
            repeats,
            format!("trusted repeated {key:?} events still reached {id}")
        );
    }

    preset(tab, "before").await?;
    selection(tab, &[C, F], 0).await?;
    preset(tab, "reorder").await?;
    selection(tab, &[F, C], 0).await?;
    enter_from_before(tab, F).await?;
    preset(tab, "remove").await?;
    selection(tab, &[C], 0).await?;
    enter_from_before(tab, C).await?;
    Ok(
        json!({"first_checked":C,"wrapped_row":revealed,"empty_entry":A,
        "reordered_entry":F,"after_hide":[C],"keys":events,"keyboard_activations":4,
        "repeated_keydowns":4}),
    )
}

async fn tap(tab: &Tab, id: &str) -> Result<()> {
    let target = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"id":1,"x":target.x,"y":target.y}]}),
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

async fn pointer_selection(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M169",
        "the checkbox and row text share one touch target, with disabled input inert",
    );
    tab.eval(
        "(() => {
            window.multiListContacts = [];
            for (const kind of ['pointerdown','pointerup']) {
                document.querySelector('canvas').addEventListener(kind, event => {
                    if (event.isTrusted) window.multiListContacts.push({kind,type:event.pointerType});
                }, {capture:true,passive:true});
            }
        })()",
    )
    .await?;
    click(tab, &mark(A)).await?;
    selection(tab, &[A, C, F], 1).await?;
    ready(tab, "d.last === 'lists-a'").await?;
    option(tab, A, "山", true, false).await?;
    click(tab, &mark(B)).await?;
    tap(tab, "lists-b-headline").await?;
    selection(tab, &[A, C, F], 1).await?;
    option(tab, B, "海", false, true).await?;
    tap(tab, "lists-c-headline").await?;
    selection(tab, &[A, F], 2).await?;
    ready(tab, "d.last === 'lists-c'").await?;
    option(tab, C, "河", false, false).await?;
    let events = tab.eval("window.multiListContacts").await?;
    let contacts = events
        .as_array()
        .context("trusted list pointer event log")?;
    for device in ["mouse", "touch"] {
        for phase in ["pointerdown", "pointerup"] {
            check_eq!(
                contacts
                    .iter()
                    .filter(|event| event["type"] == device && event["kind"] == phase)
                    .count(),
                2,
                format!("both trusted {device} events reached canvas for {phase}")
            );
        }
    }
    capture(env, tab, "mouse-and-touch").await?;
    Ok(json!({"mouse_checkbox":A,"touch_text":C,"disabled_inert":B,
        "selection":[A,F],"activations":2,"contacts":events}))
}

async fn color(tab: &Tab, role: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(data(tab).await?["colors"][role].clone())
        .with_context(|| format!("list color token {role}"))?;
    check_eq!(a, 255, "the sampled palette token is opaque");
    Ok([r, g, b])
}

fn close_color(a: Rgb, b: Rgb) -> bool {
    a.into_iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 5)
}

fn color_pixels(image: &Raster, sample: Rect, expected: Rgb) -> Result<usize> {
    Ok(image
        .colors(sample)?
        .into_iter()
        .filter(|(color, _)| close_color(*color, expected))
        .map(|(_, pixels)| pixels)
        .sum())
}

fn inset([x, y, w, h]: Rect, size: f64) -> Rect {
    [x + size, y + size, w - size * 2., h - size * 2.]
}

async fn checkbox_rect(tab: &Tab, id: &str) -> Result<Rect> {
    let box_rect = bounds(tab, &mark(id)).await?;
    let row = bounds(tab, id).await?;
    check!(
        near(box_rect[2], 18.) && near(box_rect[3], 18.) && contains(row, box_rect)
            && row[2] >= 47.5 && row[3] >= 47.5,
        "the passive checkbox reserves 18 dp inside an independently sized row: {id} {box_rect:?} {row:?}"
    );
    Ok(box_rect)
}

async fn geometry(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M170",
        "all three list styles reserve stable checkbox geometry and paint both states",
    );
    let mut profiles = Vec::new();
    for (variant, gap) in [("segmented", 2.), ("standard", 0.), ("baseline", 0.)] {
        preset(tab, variant).await?;
        ready(tab, &format!("d.variant === {}", js(variant))).await?;
        selection(tab, &[C, F], 0).await?;
        let a = bounds(tab, A).await?;
        let a_mark = checkbox_rect(tab, A).await?;
        checkbox_rect(tab, B).await?;
        let c_mark = checkbox_rect(tab, C).await?;
        let b = bounds(tab, B).await?;
        let c = bounds(tab, C).await?;
        let d = bounds(tab, D).await?;
        check!(
            near(a[3], 56.)
                && near(c[3], 72.)
                && near(d[3], 88.)
                && near(b[1] - a[1] - a[3], gap)
                && near(c[1] - b[1] - b[3], gap),
            "adding the checkbox preserves the {variant} row heights and gap"
        );
        let unchecked = node(tab, &mark(A)).await?;
        let checked = node(tab, &mark(C)).await?;
        let outline = color(tab, "on_surface_variant").await?;
        let primary = color(tab, "primary").await?;
        let tick = color(tab, "on_primary").await?;
        let [r, g, b] = outline;
        check!(
            unchecked["style"]["border_width"].as_f64() == Some(2.)
                && unchecked["style"]["border_color"] == json!([r, g, b, 255])
                && unchecked["fill_contour"].is_null()
                && checked["fill_contour"]["points"].is_array(),
            "unchecked and checked marks preserve their outline and vector check tokens"
        );
        let image = capture(env, tab, &format!("{variant}-checkboxes")).await?;
        let [x, y, _, _] = a_mark;
        let row_fill = color(
            tab,
            if variant == "segmented" {
                "surface"
            } else {
                "surface_container"
            },
        )
        .await?;
        let outline_pixels = color_pixels(&image, [x + 5., y + 0.5, 8., 1.], outline)?;
        let empty_pixels = color_pixels(&image, inset(a_mark, 5.), row_fill)?;
        let fill_pixels = color_pixels(&image, c_mark, primary)?;
        let tick_pixels = color_pixels(&image, inset(c_mark, 3.), tick)?;
        check!(
            outline_pixels >= 20 && empty_pixels >= 200 && fill_pixels >= 160 && tick_pixels >= 8,
            "{variant} actual WebGPU pixels show an empty outline and a filled visual check:
             outline={outline_pixels}, empty={empty_pixels}, fill={fill_pixels}, tick={tick_pixels}"
        );
        profiles.push(json!({"variant":variant,"unchecked_rect":a_mark,"checked_rect":c_mark,
            "row_heights":[a[3],c[3],d[3]],"gap":gap,
            "paint_pixels":{"outline":outline_pixels,"empty":empty_pixels,"fill":fill_pixels,"tick":tick_pixels}}));
    }
    let row = bounds(tab, A).await?;
    let mark_before = checkbox_rect(tab, A).await?;
    click(tab, &mark(A)).await?;
    selection(tab, &[A, C, F], 1).await?;
    check_eq!(
        bounds(tab, A).await?,
        row,
        "selection leaves the whole-row target unchanged"
    );
    check_eq!(
        checkbox_rect(tab, A).await?,
        mark_before,
        "selection leaves the passive slot unchanged"
    );
    Ok(json!(profiles))
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M171",
        "dark RTL at 200 percent keeps full names, mirrored checkbox paint and short keyboard access",
    );
    for setting in ["theme", "rtl", "large-text"] {
        preset(tab, setting).await?;
    }
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2").await?;
    resize(tab, 320, 900).await?;
    enter_from_before(tab, C).await?;
    tab.press("ArrowUp").await?;
    focused(tab, A).await?;
    let first = bounds(tab, A).await?;
    let outline = checkbox_rect(tab, A).await?;
    let trailing = bounds(tab, "lists-a-trailing").await?;
    let headline = bounds(tab, "lists-a-headline").await?;
    let leading = bounds(tab, "lists-a-leading").await?;
    check!(
        [outline, trailing, headline, leading]
            .into_iter()
            .all(|slot| contains(first, slot))
            && outline[0] + outline[2] <= trailing[0] + 0.75
            && trailing[0] + trailing[2] <= headline[0] + 0.75
            && headline[0] + headline[2] <= leading[0] + 0.75,
        "the leading icon, full label, trailing text and checkbox mirror without overlap"
    );
    capture(env, tab, "compact-rtl-slots").await?;
    tab.press("ArrowDown").await?;
    focused(tab, C).await?;
    tab.press("ArrowDown").await?;
    focused(tab, D).await?;
    let long = bounds(tab, D).await?;
    let title = bounds(tab, "lists-d-headline").await?;
    let support = bounds(tab, "lists-d-supporting").await?;
    let overline = bounds(tab, "lists-d-overline").await?;
    checkbox_rect(tab, D).await?;
    check!(
        contains([0., 0., 320., 900.], long)
            && contains(bounds(tab, SCROLL).await?, long)
            && [title, support, overline]
                .into_iter()
                .all(|part| contains(long, part))
            && title[3] > 55.
            && title[1] >= overline[1] + overline[3] - 0.75
            && support[1] >= title[1] + title[3] - 0.75,
        "the full long headline, overline and support wrap inside the revealed option"
    );
    check_eq!(
        node(tab, "lists-d-headline").await?["text"],
        json!(LONG),
        "paint retains the complete heading"
    );
    tab.press("Space").await?;
    selection(tab, &[C, D, F], 1).await?;
    let named = option(tab, D, LONG, true, false).await?;
    check!(
        named["description"]
            .as_str()
            .is_some_and(|text| text.contains("放大文字後仍可完整閱讀")),
        "Chrome exposes the complete support with the long option name"
    );
    capture(env, tab, "compact-dark-rtl-large-text").await?;
    focus_projected(tab, BEFORE).await?;
    resize(tab, 320, 240).await?;
    tab.press("Tab").await?;
    focused(tab, C).await?;
    let short_state = data(tab).await?;
    let row = short_state["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == C))
        .context("short selected option actual clip")?;
    let rect: Rect = serde_json::from_value(row["bounds"].clone())?;
    let clip: Rect = serde_json::from_value(row["clip"].clone())?;
    let box_rect = checkbox_rect(tab, C).await?;
    let top = rect[1].max(clip[1]).max(0.);
    let bottom = (rect[1] + rect[3]).min(clip[1] + clip[3]).min(240.);
    check!(
        bottom - top >= 47.5 && contains([0.,0.,320.,240.], box_rect) && contains(clip, box_rect),
        "the short scene reveals a full checkbox and at least one accessible target: {rect:?} {clip:?}"
    );
    let short_selected = option(tab, C, "河", true, false).await?;
    capture(env, tab, "short-dark-rtl-selected").await?;
    tab.press("Enter").await?;
    selection(tab, &[D, F], 2).await?;
    option(tab, C, "河", false, false).await?;
    Ok(
        json!({"rtl_slots":[outline,trailing,headline,leading],"full_row":long,"name":named,
        "short":{"row":rect,"clip":clip,"visible_height":bottom-top,"selected":short_selected,
        "remaining":[D,F]}}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    let semantics = semantics(env, tab, url).await?;
    open_multi(tab, url).await?;
    let keyboard = keyboard(tab).await?;
    open_multi(tab, url).await?;
    let pointer = pointer_selection(env, tab).await?;
    open_multi(tab, url).await?;
    let geometry = geometry(env, tab).await?;
    open_multi(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material multi-select lists page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-lists-multi.json",
        &json!({"browser":chrome.version,"semantics":semantics,"keyboard":keyboard,
            "pointer":pointer,"geometry":geometry,"compact":compact,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1644, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-lists-multi-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-lists-multi-failure.png"))
            .await;
    }
    result.context("Material 3 Lists multiple selection")
}
