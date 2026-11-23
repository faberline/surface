//! Material 3 list reorder with trusted input on Chrome's WebGPU host.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

const LIST: &str = "list-reorder-items";
const SCROLL: &str = "list-reorder-scroll";
const STATUS: &str = "list-reorder-demo-status";
const EXTERNAL: &str = "list-reorder-demo-external";
const LIFT: &str = "list-reorder-items-lift";
const DROP: &str = "list-reorder-items-drop";
const A: &str = "list-reorder-a";
const B: &str = "list-reorder-b";
const C: &str = "list-reorder-c";
const D: &str = "list-reorder-d";
const E: &str = "list-reorder-e";
const I: &str = "list-reorder-i";
const FIXED: &str = "list-reorder-fixed";
const J: &str = "list-reorder-j";
const K: &str = "list-reorder-k";
const ORDER: &[&str] = &[
    A,
    B,
    C,
    D,
    E,
    "list-reorder-f",
    "list-reorder-g",
    "list-reorder-h",
    I,
    FIXED,
    J,
    K,
];

type Rect = [f64; 4];
type Rgb = [u8; 3];
type Position = (f64, f64);

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["list_reorder"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.list_reorder)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1844).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1800).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "list-reorder-demo-show").await?;
    ready(
        tab,
        "d.visible && d.variant === 'segmented' && !d.rtl && d.text_scale === 1 &&
        d.moves === 0 && d.activations === 0 && d.external_changes === 0 && d.last_move === null",
    )
    .await?;
    resize(tab, 960, 820).await?;
    check_eq!(
        data(tab).await?["order"],
        json!(ORDER),
        "the authored order and fixed barrier"
    );
    Ok(())
}

async fn preset(tab: &Tab, name: &str) -> Result<()> {
    activate(tab, &format!("list-reorder-demo-{name}")).await
}

fn part(item: &str, suffix: &str) -> String {
    format!("{item}-{suffix}")
}

async fn center(tab: &Tab, id: &str) -> Result<Position> {
    let anchor = point(tab, id).await?;
    Ok((anchor.x, anchor.y))
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-list-reorder-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn node(tab: &Tab, id: &str) -> Result<Value> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {}) ?? null",
        js(id),
    ))
    .await
}

async fn preview(tab: &Tab, present: bool) -> Result<()> {
    let n = if present { 4 } else { 0 };
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === {n} && window.gallery.control_bounds({}).length === {n}",
        js(LIFT), js(DROP),
    )).await?;
    tab.settle().await
}

fn moved(order: &[&str], item: &str, before: &str) -> Result<Value> {
    let mut order: Vec<_> = order.iter().copied().filter(|id| *id != item).collect();
    let destination = order
        .iter()
        .position(|id| *id == before)
        .context("expected destination in fixture")?;
    order.insert(destination, item);
    Ok(json!(order))
}

async fn movement(tab: &Tab, count: u8, item: &str, before: &str, order: &Value) -> Result<()> {
    ready(
        tab,
        &format!(
            "d.moves === {count} && d.last_move?.item === {} && d.last_move?.before === {}",
            js(item),
            js(before)
        ),
    )
    .await?;
    check_eq!(
        data(tab).await?["order"],
        *order,
        "exact accepted stable-ID order"
    );
    preview(tab, false).await
}

async fn unchanged(tab: &Tab, order: &[&str], count: u8, revision: &Value) -> Result<()> {
    tab.settle().await?;
    let current = data(tab).await?;
    check_eq!(
        current["order"],
        json!(order),
        "blocked or held input preserves hosted order"
    );
    check_eq!(
        current["moves"],
        json!(count),
        "blocked or held input emits no move"
    );
    check_eq!(
        current["revision"],
        *revision,
        "blocked or held input preserves the revision"
    );
    Ok(())
}

async fn status(tab: &Tab, contents: &str, announced: bool) -> Result<()> {
    let element = projected(STATUS);
    tab.wait_for(&format!(
        "{element}?.getAttribute('role') === 'status' && {element}?.textContent === {}",
        js(contents),
    ))
    .await?;
    if announced {
        tab.wait_for(&format!(
            "[...document.querySelectorAll('#surface-accessibility [aria-live=polite]')].some(node => node.textContent.includes({}))",
            js(contents),
        )).await?;
    }
    Ok(())
}

async fn button(tab: &Tab, id: &str, row: &str, name: &str, disabled: bool) -> Result<()> {
    let target = projected(id);
    let shell = projected(&part(row, "item"));
    let attributes = tab
        .eval(&format!(
            "(() => {{
          const b={target}, row={shell};
          return b && {{
            role:b.getAttribute('role'), name:b.getAttribute('aria-label'),
            disabled:b.getAttribute('aria-disabled'), action:b.getAttribute('data-surface-action'),
            tab:b.getAttribute('tabindex'), selected:b.getAttribute('aria-selected'),
            inside:!!row?.contains(b), nested:!!b.parentElement?.closest('[role=button]'),
          }};
        }})()"
        ))
        .await?;
    check!(
        attributes["role"] == "button"
            && attributes["name"] == name
            && attributes["disabled"].as_str() == disabled.then_some("true")
            && attributes["action"].as_str() == (!disabled).then_some("press")
            && attributes["tab"].as_str() == (!disabled).then_some("-1")
            && attributes["selected"].is_null()
            && attributes["inside"] == true
            && attributes["nested"] == false,
        "{id} is an independent named action in an ordinary list item: {attributes}"
    );
    let tree = ax_tree(tab).await?;
    let button = ax(&tree, "button", name)?;
    if disabled {
        check_eq!(
            ax_property(button, "disabled"),
            Some(&json!(true)),
            "Chrome exposes the blocked move"
        );
    }
    Ok(())
}

async fn passive(tab: &Tab, id: &str) -> Result<()> {
    let proxy = projected(id);
    check_eq!(
        tab.eval(&format!("(() => {{const n={proxy}; return !n || (!n.matches('[role=button],[tabindex],[data-surface-action]') && !n.querySelector('[tabindex],[data-surface-action]'));}})()")).await?,
        json!(true),
        format!("{id} adds no keyboard or semantic action")
    );
    Ok(())
}

async fn scope(tab: &Tab) -> Result<Value> {
    let group = projected(LIST);
    let row = projected(&part(A, "item"));
    let semantics = tab.eval(&format!(
        "(() => {{
          const l={group}, r={row};
          return l && r && {{
            role:l.getAttribute('role'), name:l.getAttribute('aria-label'), row:r.getAttribute('role'),
            row_action:r.getAttribute('data-surface-action'), row_tab:r.getAttribute('tabindex'),
            controls:[...r.querySelectorAll('[role=button]')].map(b=>b.dataset.surfaceId),
            nested_options:l.querySelectorAll('[role=listbox],[role=option]').length,
          }};
        }})()"
    )).await?;
    check!(
        semantics["role"] == "list"
            && semantics["name"] == "旅程資料順序"
            && semantics["row"] == "listitem"
            && semantics["row_action"].is_null()
            && semantics["row_tab"].is_null()
            && semantics["controls"]
                == json!([A, part(A, "up").as_str(), part(A, "down").as_str()])
            && semantics["nested_options"] == 0,
        "Chrome exposes an ordinary list with three peer buttons per movable item: {semantics}"
    );
    ax(&ax_tree(tab).await?, "list", "旅程資料順序")?;
    button(tab, A, A, "筆記", false).await?;
    button(tab, &part(A, "up"), A, "上移筆記", true).await?;
    button(tab, &part(A, "down"), A, "下移筆記", false).await?;
    passive(tab, &part(A, "grip")).await?;
    Ok(semantics)
}

async fn key(tab: &Tab, name: &str, down: bool, repeat: bool) -> Result<()> {
    let key = crate::keys::key(name)?;
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({
            "type":if down { "keyDown" } else { "keyUp" },"key":key.key,"code":key.code,
            "windowsVirtualKeyCode":key.key_code,"modifiers":0,"location":key.location,
            "text":if down { key.text.as_str() } else { "" },"autoRepeat":repeat,
        }),
    )
    .await?;
    Ok(())
}

#[derive(Clone, Copy)]
enum Device {
    Mouse,
    Touch,
}

impl Device {
    fn name(self) -> &'static str {
        match self {
            Self::Mouse => "mouse",
            Self::Touch => "touch",
        }
    }

    async fn press(self, tab: &Tab, (x, y): Position) -> Result<()> {
        match self {
            Self::Mouse => {
                tab.mouse_move(x, y).await?;
                mouse(tab, "mousePressed", (x, y), true).await
            }
            Self::Touch => touch(tab, "touchStart", Some((x, y))).await,
        }?;
        tab.settle().await
    }

    async fn move_to(self, tab: &Tab, at: Position) -> Result<()> {
        match self {
            Self::Mouse => mouse(tab, "mouseMoved", at, true).await,
            Self::Touch => touch(tab, "touchMove", Some(at)).await,
        }
    }

    async fn release(self, tab: &Tab, at: Position) -> Result<()> {
        match self {
            Self::Mouse => mouse(tab, "mouseReleased", at, false).await,
            Self::Touch => touch(tab, "touchEnd", None).await,
        }
    }
}

async fn mouse(tab: &Tab, kind: &str, (x, y): Position, held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,"button":"left",
            "buttons":u8::from(held),"clickCount":1}),
    )
    .await?;
    Ok(())
}

async fn touch(tab: &Tab, kind: &str, at: Option<Position>) -> Result<()> {
    let points: Vec<_> = at
        .into_iter()
        .map(|(x, y)| json!({"id":1,"x":x,"y":y}))
        .collect();
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    Ok(())
}

async fn tap(tab: &Tab, id: &str) -> Result<()> {
    let at = center(tab, id).await?;
    Device::Touch.press(tab, at).await?;
    Device::Touch.release(tab, at).await?;
    tab.settle().await
}

const TRACE: &str = r#"
    (() => {
        const canvas = document.getElementById('surface');
        window.listReorderTrace = [];
        for (const kind of ['pointerdown', 'gotpointercapture', 'pointermove',
                            'lostpointercapture', 'pointerup', 'pointercancel']) {
            document.addEventListener(kind, event => {
                if (event.target !== canvas) return;
                window.listReorderTrace.push({kind, id:event.pointerId, device:event.pointerType,
                    trusted:event.isTrusted, buttons:event.buttons, x:event.clientX, y:event.clientY});
            }, {capture:true, passive:true});
        }
    })()
"#;

async fn clear_trace(tab: &Tab) -> Result<()> {
    tab.eval("window.listReorderTrace.length = 0").await?;
    Ok(())
}

async fn trace(tab: &Tab, device: Device) -> Result<Vec<Value>> {
    let events: Vec<Value> = serde_json::from_value(tab.eval("window.listReorderTrace").await?)?;
    Ok(events
        .into_iter()
        .filter(|event| event["device"] == device.name() && event["trusted"] == true)
        .collect())
}

fn move_events(events: &[Value]) -> usize {
    events
        .iter()
        .filter(|event| event["kind"] == "pointermove")
        .count()
}

async fn toward_c(tab: &Tab, device: Device) -> Result<Position> {
    let start = center(tab, &part(B, "grip")).await?;
    let target = point(tab, &part(C, "item")).await?;
    // Cross C's midpoint while leaving the destination line unobscured.
    let destination = (start.0, target.y + 3.);
    device.press(tab, start).await?;
    device.move_to(tab, destination).await?;
    preview(tab, true).await?;
    Ok(destination)
}

async fn commands(tab: &Tab) -> Result<Value> {
    step(
        "M182",
        "keyboard, mouse and touch use separate move buttons and honor a fixed barrier",
    );
    let semantics = scope(tab).await?;
    status(tab, "尚未調整順序", false).await?;
    let initial = data(tab).await?;
    let a_up = part(A, "up");
    let a_down = part(A, "down");
    click(tab, &a_up).await?;
    unchanged(tab, ORDER, 0, &initial["revision"]).await?;
    status(tab, "尚未調整順序", false).await?;

    focus_projected(tab, A).await?;
    tab.press("ArrowDown").await?;
    focused(tab, &a_down).await?;
    let reordered = moved(ORDER, A, C)?;
    key(tab, "Enter", true, false).await?;
    movement(tab, 1, A, C, &reordered).await?;
    focused(tab, &a_down).await?;
    let one_move = data(tab).await?;
    for _ in 0..2 {
        key(tab, "Enter", true, true).await?;
        tab.settle().await?;
        check_eq!(
            data(tab).await?,
            one_move,
            "a held Enter cannot move the row again"
        );
    }
    key(tab, "Enter", false, false).await?;
    status(tab, "已將筆記移到第 2 項", true).await?;
    focus_projected(tab, &a_up).await?;
    tab.press("Space").await?;
    movement(tab, 2, A, B, &json!(ORDER)).await?;
    focused(tab, &a_down).await?;
    status(tab, "已將筆記移到第 1 項", true).await?;

    click(tab, &a_down).await?;
    movement(tab, 3, A, C, &reordered).await?;
    tap(tab, &a_up).await?;
    movement(tab, 4, A, B, &json!(ORDER)).await?;
    click(tab, A).await?;
    ready(
        tab,
        &format!("d.activations === 1 && d.last_activation === {}", js(A)),
    )
    .await?;
    status(tab, "已開啟筆記", true).await?;
    let before_wheel = data(tab).await?;
    unchanged(tab, ORDER, 4, &before_wheel["revision"]).await?;

    let (x, y) = center(tab, SCROLL).await?;
    tab.mouse_move(x, y).await?;
    tab.wheel(0., 3000.).await?;
    ready(tab, "d.scroll_y > 500").await?;
    let scrolled = data(tab).await?;
    button(tab, FIXED, FIXED, "固定分隔", true).await?;
    button(tab, &part(I, "down"), I, "下移清單", true).await?;
    button(tab, &part(J, "up"), J, "上移票券", true).await?;
    button(tab, &part(J, "down"), J, "下移票券", false).await?;
    click(tab, &part(J, "up")).await?;
    unchanged(tab, ORDER, 4, &scrolled["revision"]).await?;
    status(tab, "已開啟筆記", false).await?;
    let start = center(tab, &part(J, "grip")).await?;
    let across = center(tab, &part(FIXED, "item")).await?;
    Device::Mouse.press(tab, start).await?;
    Device::Mouse.move_to(tab, (start.0, across.1)).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 4 && window.gallery.control_bounds({}).length === 0",
        js(LIFT),
        js(DROP),
    )).await?;
    unchanged(tab, ORDER, 4, &scrolled["revision"]).await?;
    Device::Mouse.release(tab, (start.0, across.1)).await?;
    preview(tab, false).await?;
    unchanged(tab, ORDER, 4, &scrolled["revision"]).await?;
    status(tab, "已開啟筆記", false).await?;
    check_eq!(
        data(tab).await?["activations"],
        json!(1),
        "reorder controls never invoke a primary command"
    );
    Ok(
        json!({"semantics":semantics,"moves":4,"primary_activations":1,"fixed":FIXED,
        "blocked":[part(I,"down"),part(J,"up")],"wheel_scroll_y":scrolled["scroll_y"],"live_status":true}),
    )
}

async fn palette(tab: &Tab, key: &str) -> Result<[u8; 4]> {
    serde_json::from_value(data(tab).await?["colors"][key].clone())
        .with_context(|| format!("Gallery color {key}"))
}

fn rgb([red, green, blue, _]: [u8; 4]) -> Rgb {
    [red, green, blue]
}

fn matches(color: Rgb, expected: Rgb) -> bool {
    color
        .into_iter()
        .zip(expected)
        .all(|(a, b)| a.abs_diff(b) <= 5)
}

fn pixel_count(image: &Raster, sample: Rect, color: Rgb) -> Result<usize> {
    Ok(image
        .colors(sample)?
        .into_iter()
        .filter(|(sample, _)| matches(*sample, color))
        .map(|(_, count)| count)
        .sum())
}

fn swatch(image: &Raster, sample: Rect) -> Result<Rgb> {
    image
        .colors(sample)?
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(pixel, _)| pixel)
        .context("painted PNG swatch")
}

fn blend(fill: Rgb, ink: Rgb, alpha: u8, srgb_attachment: bool) -> Rgb {
    let to_space = |n: u8| {
        let value = f64::from(n) / 255.;
        if !srgb_attachment {
            value
        } else if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let alpha = f64::from(alpha) / 255.;
    std::array::from_fn(|i| {
        let mixed = to_space(fill[i]) * (1. - alpha) + to_space(ink[i]) * alpha;
        let output = if !srgb_attachment {
            mixed
        } else if mixed <= 0.0031308 {
            mixed * 12.92
        } else {
            1.055 * mixed.powf(1. / 2.4) - 0.055
        };
        (output * 255.).round() as u8
    })
}

async fn paint(env: &Env, tab: &Tab, baseline: bool) -> Result<Value> {
    let (fill_role, ink_role, opacity, radius, name) = if baseline {
        ("surface", "on_surface", 41, 0., "baseline-held")
    } else {
        (
            "tertiary_container",
            "on_tertiary_container",
            26,
            16.,
            "expressive-held",
        )
    };
    let fill = palette(tab, fill_role).await?;
    let ink = palette(tab, ink_role).await?;
    let target = palette(tab, "surface_container_low").await?;
    let shadow = rgb(palette(tab, "shadow").await?);
    let lift = node(tab, LIFT).await?;
    let tint = tab
        .eval(&format!(
            "JSON.parse(window.gallery.export_dom()).dom.nodes[{}].style.background",
            lift["first_child"].as_u64().context("lift state layer")?,
        ))
        .await?;
    check_eq!(
        lift["style"]["background"],
        json!(fill),
        "held row surface uses its Material token"
    );
    check_eq!(
        tint,
        json!([ink[0], ink[1], ink[2], opacity]),
        "held row uses the exact dragged state layer"
    );
    let lift_style = &lift["style"];
    let authored = &lift_style["corners"];
    let resolved = if authored.is_null() {
        vec![lift_style["radius"].as_f64().unwrap_or(0.); 4]
    } else {
        ["top_left", "top_right", "bottom_right", "bottom_left"]
            .into_iter()
            .map(|key| authored[key].as_f64())
            .collect::<Option<Vec<_>>>()
            .context("lift corners")?
    };
    check_eq!(
        resolved,
        vec![radius; 4],
        "held row keeps its Material shape"
    );
    let layers = lift["box_shadows"]
        .as_array()
        .context("held row shadow layers")?;
    let actual = layers
        .iter()
        .map(|layer| ["offset_y", "blur", "spread"].map(|field| layer[field].as_f64()))
        .collect::<Vec<_>>();
    let expected = if baseline {
        [[6., 10., 4.], [2., 3., 0.]]
    } else {
        [[2., 6., 2.], [1., 2., 0.]]
    }
    .map(|layer| layer.map(Some));
    check_eq!(
        actual.as_slice(),
        expected.as_slice(),
        "the two held-row shadow layers use the selected elevation"
    );
    check!(
        layers.iter().all(|layer| {
            serde_json::from_value::<[u8; 4]>(layer["color"].clone())
                .ok()
                .map(rgb)
                == Some(shadow)
        }),
        "held-row shadows use the theme Shadow color"
    );
    let drop = node(tab, DROP).await?;
    check_eq!(
        drop["style"]["background"],
        json!(target),
        "drop line uses Surface Container Low"
    );

    let image = capture(env, tab, name).await?;
    let [x, y, width, _] = bounds(tab, LIFT).await?;
    let tinted = swatch(&image, [x + width / 2., y + 7., 10., 3.])?;
    check!(
        [false, true]
            .into_iter()
            .any(|srgb| matches(tinted, blend(rgb(fill), rgb(ink), opacity, srgb))),
        "WebGPU paints the tinted lift, {name}: {tinted:?}, base {fill:?}, ink {ink:?}"
    );
    let title_ink = pixel_count(
        &image,
        bounds(tab, &part(LIFT, "headline")).await?,
        rgb(ink),
    )?;
    check!(
        title_ink >= 16,
        "held row paints its exact headline color: {title_ink} pixels"
    );
    let [x, y, width, height] = bounds(tab, DROP).await?;
    let line = swatch(&image, [x + width / 2., y + height / 2. - 0.5, 10., 1.])?;
    check!(
        matches(line, rgb(target)),
        "WebGPU paints the destination token: {line:?}"
    );
    Ok(
        json!({"fill":fill,"ink":ink,"opacity":opacity,"corners":resolved,"shadow_layers":actual,
        "tinted_pixel":tinted,"drop_pixel":line,"title_ink_pixels":title_ink}),
    )
}

async fn rows(tab: &Tab) -> Result<[Rect; 3]> {
    Ok([
        bounds(tab, &part(A, "item")).await?,
        bounds(tab, &part(B, "item")).await?,
        bounds(tab, &part(C, "item")).await?,
    ])
}

async fn drag_profiles(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M183",
        "mouse drag previews unequal-height rows with Material lift and destination paint",
    );
    let mut profiles = Vec::new();
    for baseline in [false, true] {
        if baseline {
            reload(tab, url).await?;
            preset(tab, "baseline").await?;
            ready(tab, "d.variant === 'baseline'").await?;
        }
        focus_projected(tab, A).await?;
        let initial = data(tab).await?;
        let before = rows(tab).await?;
        check_eq!(
            [before[0][3], before[1][3], before[2][3]],
            [56., 72., 88.],
            "row geometry uses the three documented minimum heights"
        );
        let end = toward_c(tab, Device::Mouse).await?;
        unchanged(tab, ORDER, 0, &initial["revision"]).await?;
        check_eq!(
            rows(tab).await?,
            before,
            "preview paint never changes the real row geometry"
        );
        until(tab, &format!("s.focused === {}", js(A))).await?;
        passive(tab, LIFT).await?;
        passive(tab, DROP).await?;
        let result = paint(env, tab, baseline).await?;
        Device::Mouse.release(tab, end).await?;
        movement(tab, 1, B, D, &moved(ORDER, B, D)?).await?;
        status(tab, "已將山徑移到第 3 項", true).await?;
        until(tab, &format!("s.focused === {}", js(A))).await?;
        let after = data(tab).await?;
        check_eq!(
            after["revision"],
            json!(1),
            "one accepted drag advances the revision once"
        );
        check_eq!(
            after["activations"],
            json!(0),
            "drag does not invoke a primary command"
        );
        profiles.push(json!({"variant":after["variant"],"rows":before,"paint":result,"last_move":after["last_move"]}));
    }
    Ok(json!(profiles))
}

async fn freeze(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            const start = performance.now();
            window.listReorderClock = 0;
            Object.defineProperty(performance, 'now', {
                configurable: true, value: () => start + window.listReorderClock,
            });
        })()",
    )
    .await?;
    tab.settle().await
}

#[derive(Debug, Deserialize, Serialize)]
struct Frame {
    scroll: f64,
    clip: Rect,
    rows: Vec<Rect>,
}

async fn frame(tab: &Tab) -> Result<Frame> {
    let row_ids: Vec<_> = ORDER.iter().map(|id| part(id, "item")).collect();
    serde_json::from_value(
        tab.eval(&format!(
            "(() => {{
            const bounds = id => Array.from(window.gallery.control_bounds(id));
            return {{scroll:JSON.parse(window.gallery.inspect()).list_reorder.scroll_y,
                clip:bounds({}),rows:{}.map(id=>bounds(id))}};
        }})()",
            js(SCROLL),
            json!(row_ids),
        ))
        .await?,
    )
    .context("rendered reorder geometry")
}

fn edge_anchor(frame: &Frame, y: f64, minimum: usize) -> Option<usize> {
    // Find a newly visible row just beyond the stationary pointer. Its top
    // must be below the release, while its predecessor's midpoint is above.
    let bottom = frame.clip[1] + frame.clip[3];
    let barrier = ORDER.iter().position(|id| *id == FIXED)?;
    (minimum..barrier).find(|&index| {
        let row_y = frame.rows[index][1];
        let previous = frame.rows[index - 1];
        row_y > y + 0.5 && row_y < bottom - 0.5 && previous[1] + previous[3] / 2. < y - 0.5
    })
}

async fn edge_drop(env: &Env, tab: &Tab, device: Device) -> Result<Value> {
    focus_projected(tab, EXTERNAL).await?;
    tab.eval(TRACE).await?;
    freeze(tab).await?;
    let canvas = tab.canvas_box().await?;
    let initial = frame(tab).await?;
    let clip_end = initial.clip[1] + initial.clip[3];
    let release_y = clip_end - 8.;
    let start = center(tab, &part(A, "grip")).await?;
    let release = (start.0, canvas[1] + release_y);
    let last_visible = initial
        .rows
        .iter()
        .rposition(|row| row[1] < clip_end)
        .context("initial visible rows")?;
    device.press(tab, start).await?;
    device.move_to(tab, release).await?;
    preview(tab, true).await?;
    let before = trace(tab, device).await?;
    check!(
        before
            .iter()
            .any(|event| event["kind"] == "pointermove" && event["buttons"] == 1),
        "the last held move was a trusted event: {before:?}"
    );
    let mut sample = None;
    for offset in 1..=180 {
        tab.eval(&format!("window.listReorderClock = {}", offset * 8))
            .await?;
        tab.settle().await?;
        let current = frame(tab).await?;
        if current.scroll <= initial.scroll + 90. {
            continue;
        }
        if let Some(index) = edge_anchor(&current, release_y, last_visible + 2) {
            sample = Some((current, index));
            break;
        }
    }
    let (latest, index) =
        sample.context("edge scrolling never showed the later anchor at the release point")?;
    check!(
        latest.scroll > initial.scroll + 90.,
        "the browser has presented the later scroll offset: {latest:?}"
    );
    let target = ORDER[index];
    unchanged(tab, ORDER, 0, &json!(0)).await?;
    let during = trace(tab, device).await?;
    check_eq!(
        move_events(&during),
        move_events(&before),
        "edge scrolling received no subsequent pointer move"
    );
    // Keep the timer frozen and release at the same contact position.
    device.release(tab, release).await?;
    let expected = moved(ORDER, A, target)?;
    movement(tab, 1, A, target, &expected).await?;
    status(tab, &format!("已將筆記移到第 {index} 項"), true).await?;
    let events = trace(tab, device).await?;
    check_eq!(
        move_events(&events),
        move_events(&before),
        "the release uses the last rendered scroll offset without another move event"
    );
    check_eq!(
        events
            .iter()
            .filter(|event| event["kind"] == "pointerup")
            .count(),
        1,
        "exactly one trusted release"
    );
    let final_state = data(tab).await?;
    check_eq!(
        final_state["activations"],
        json!(0),
        "edge drag never invokes the primary button"
    );
    check_eq!(final_state["revision"], json!(1), "edge drop commits once");
    if matches!(device, Device::Touch) {
        capture(env, tab, "edge-release").await?;
    }
    Ok(
        json!({"device":device.name(),"last_visible_before_scroll":ORDER[last_visible],"before":target,
        "release_y":release_y,"initial":initial,"latest":latest,"order":expected,"events":events,
        "moves_at_release":move_events(&before),"actual_request":final_state["last_move"]}),
    )
}

async fn edge(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step(
        "M184",
        "mouse and touch release use the rendered edge-scroll layout without another move",
    );
    let mouse = edge_drop(env, tab, Device::Mouse).await?;
    reload(tab, url).await?;
    let touch = edge_drop(env, tab, Device::Touch).await?;
    Ok(json!([mouse, touch]))
}

async fn canceled(tab: &Tab, order: &[&str], revision: u8, message: &str) -> Result<()> {
    preview(tab, false).await?;
    unchanged(tab, order, 0, &json!(revision)).await?;
    let current = data(tab).await?;
    check!(
        current["last_move"].is_null() && current["activations"] == 0,
        "cancellation leaves no move or primary result"
    );
    status(tab, message, false).await
}

async fn cancels(tab: &Tab) -> Result<Value> {
    step(
        "M185",
        "Escape, trusted capture loss, touch cancel and an external revision clear held paint",
    );
    tab.eval(TRACE).await?;
    focus_projected(tab, B).await?;
    let end = toward_c(tab, Device::Mouse).await?;
    tab.press("Escape").await?;
    canceled(tab, ORDER, 0, "尚未調整順序").await?;
    Device::Mouse.release(tab, end).await?;
    canceled(tab, ORDER, 0, "尚未調整順序").await?;
    until(tab, &format!("s.focused === {}", js(B))).await?;

    clear_trace(tab).await?;
    let end = toward_c(tab, Device::Mouse).await?;
    let events = trace(tab, Device::Mouse).await?;
    let owner = events
        .iter()
        .find(|event| event["kind"] == "pointerdown")
        .and_then(|event| event["id"].as_i64())
        .context("trusted captured pointer")?;
    tab.wait_for(&format!(
        "document.getElementById('surface').hasPointerCapture({owner}) &&
        window.listReorderTrace.some(e => e.id === {owner} && e.kind === 'gotpointercapture' && e.trusted)"
    )).await?;
    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    Device::Mouse.release(tab, end).await?;
    canceled(tab, ORDER, 0, "尚未調整順序").await?;
    let loss = trace(tab, Device::Mouse).await?;
    let ordered: Vec<_> = loss
        .iter()
        .filter(|event| event["id"] == owner)
        .filter_map(|event| event["kind"].as_str())
        .filter(|kind| *kind != "pointermove")
        .collect();
    check_eq!(
        ordered,
        vec![
            "pointerdown",
            "gotpointercapture",
            "lostpointercapture",
            "pointerup"
        ],
        "the browser's unexpected trusted capture loss precedes release"
    );

    clear_trace(tab).await?;
    toward_c(tab, Device::Touch).await?;
    touch(tab, "touchCancel", None).await?;
    canceled(tab, ORDER, 0, "尚未調整順序").await?;
    let touch_events = trace(tab, Device::Touch).await?;
    check!(
        touch_events
            .iter()
            .any(|event| event["kind"] == "pointercancel"),
        "a trusted touch cancel reached the actual canvas"
    );

    focus_projected(tab, EXTERNAL).await?;
    let end = toward_c(tab, Device::Mouse).await?;
    until(tab, &format!("s.focused === {}", js(EXTERNAL))).await?;
    tab.press("Enter").await?;
    ready(tab, "d.external_changes === 1").await?;
    let mut external_order = ORDER.to_vec();
    let n = external_order.len();
    external_order.swap(n - 2, n - 1);
    canceled(tab, &external_order, 1, "已更新清單順序").await?;
    Device::Mouse.release(tab, end).await?;
    canceled(tab, &external_order, 1, "已更新清單順序").await?;
    status(tab, "已更新清單順序", true).await?;

    let end = toward_c(tab, Device::Mouse).await?;
    Device::Mouse.release(tab, end).await?;
    let accepted = moved(&external_order, B, D)?;
    movement(tab, 1, B, D, &accepted).await?;
    check_eq!(
        data(tab).await?["revision"],
        json!(2),
        "only the external reorder and fresh drop advance revision"
    );
    status(tab, "已將山徑移到第 3 項", true).await?;
    Ok(
        json!({"escape":true,"lost_capture":loss,"touch_cancel":touch_events,"external_order":external_order,
        "fresh_order":accepted,"fresh_moves":1,"final_revision":2}),
    )
}

fn disjoint(a: Rect, b: Rect) -> bool {
    a[0] + a[2] <= b[0] + 0.75
        || b[0] + b[2] <= a[0] + 0.75
        || a[1] + a[3] <= b[1] + 0.75
        || b[1] + b[3] <= a[1] + 0.75
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M186",
        "at 320 dp RTL with 200 percent text each keyboard, mouse and touch target stays reachable",
    );
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":"reduce"}]}),
    )
    .await?;
    tab.wait_for("matchMedia('(prefers-reduced-motion: reduce)').matches")
        .await?;
    for setting in ["theme", "rtl", "large-text"] {
        preset(tab, setting).await?;
    }
    resize(tab, 320, 900).await?;
    ready(tab, "d.rtl && d.text_scale === 2").await?;
    until(tab, "s.dark").await?;
    focus_projected(tab, A).await?;
    let c_up = part(C, "up");
    let c_down = part(C, "down");
    let c_grip = part(C, "grip");
    for target in [
        part(A, "down").as_str(),
        B,
        part(B, "up").as_str(),
        part(B, "down").as_str(),
        C,
        &c_up,
        &c_down,
    ] {
        tab.press("ArrowDown").await?;
        focused(tab, target).await?;
    }
    let primary = bounds(tab, C).await?;
    let support = bounds(tab, &part(C, "supporting")).await?;
    let shell = bounds(tab, &part(C, "item")).await?;
    let viewport = bounds(tab, SCROLL).await?;
    let targets = [
        primary,
        bounds(tab, &c_up).await?,
        bounds(tab, &c_down).await?,
        bounds(tab, &c_grip).await?,
    ];
    check!(
        contains(primary,support) && support[3] > 48.
            && targets.iter().all(|&target| contains(shell,target) && contains(viewport,target)
                && target[0] >= 0. && target[0]+target[2]<=320. && target[2]>=48. && target[3]>=48.),
        "the full large label and every hit target fit the named scroller: {targets:?}, support {support:?}, clip {viewport:?}"
    );
    check!(
        targets[1..].iter().all(|target| near(target[2],48.) && near(target[3],48.)
            && target[1] >= primary[1] + primary[3] - 0.75)
            && targets[1][0] > targets[2][0] && targets[2][0] > targets[3][0]
            && targets.iter().enumerate().all(|(i,&shape)| targets[i+1..].iter().all(|&other| disjoint(shape,other))),
        "the three separate 48 dp controls wrap below the text and mirror their placement: {targets:?}"
    );
    button(tab, C, C, "地圖", false).await?;
    button(tab, &c_up, C, "上移地圖", false).await?;
    button(tab, &c_down, C, "下移地圖", false).await?;
    passive(tab, &c_grip).await?;
    let image = capture(env, tab, "compact-dark-rtl").await?;
    let support_ink = pixel_count(
        &image,
        support,
        rgb(palette(tab, "on_surface_variant").await?),
    )?;
    check!(
        support_ink >= 16,
        "the actual WebGPU screenshot shows the wrapped supporting label: {support_ink} pixels"
    );

    let expected = moved(ORDER, C, E)?;
    tab.press("Space").await?;
    movement(tab, 1, C, E, &expected).await?;
    focused(tab, &c_down).await?;
    status(tab, "已將地圖移到第 4 項", true).await?;
    click(tab, &c_up).await?;
    movement(tab, 2, C, D, &json!(ORDER)).await?;
    tap(tab, &c_grip).await?;
    unchanged(tab, ORDER, 2, &json!(2)).await?;
    preview(tab, false).await?;
    tap(tab, &c_down).await?;
    movement(tab, 3, C, E, &expected).await?;
    status(tab, "已將地圖移到第 4 項", true).await?;
    check_eq!(
        data(tab).await?["activations"],
        json!(0),
        "the compact grip and move buttons do not activate the primary"
    );
    Ok(
        json!({"viewport":[320,900],"rtl":true,"text_scale":2,"reduce_motion":true,
        "text":support,"ink_pixels":support_ink,"targets":targets,"scroller":viewport,"moves":3,"order":expected}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let controls = commands(tab).await?;
    reload(tab, url).await?;
    let profiles = drag_profiles(env, tab, url).await?;
    reload(tab, url).await?;
    let edge = edge(env, tab, url).await?;
    reload(tab, url).await?;
    let cancellation = cancels(tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material list reorder browser errors: {errors:?}"
    );
    env.write_json(
        "browser-material-list-reorder.json",
        &json!({
            "browser":chrome.version,"commands":controls,"profiles":profiles,"edge":edge,
            "cancellation":cancellation,"compact":compact,"errors":errors,
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1844, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json("material-list-reorder-failure.json", &json!({
            "error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors(),
            "pointer_trace":tab.eval("window.listReorderTrace ?? null").await.ok(),
            "pointer_captures":tab.eval("Array.from(new Set((window.listReorderTrace ?? []).map(e=>e.id))).map(id=>[id,document.getElementById('surface')?.hasPointerCapture(id)])").await.ok(),
        }));
        let _ = tab
            .screenshot(&env.artifact("material-list-reorder-failure.png"))
            .await;
    }
    result.context("Material 3 list reorder")
}
