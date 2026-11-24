//! Looping numeric and text-choice Scroll Fields in Chrome and WebGPU.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    click,
    material_list_corners::{freeze, reduced_motion, tick},
    material_time_input::{
        activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
        projected, resize, Raster,
    },
    point, state, until,
};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const TEXTAREA: &str = "document.getElementById('surface-text-input')";
const CANVAS_FOCUS: &str = "document.activeElement === document.getElementById('surface')";

#[derive(Clone, Copy)]
struct Fixture {
    base: &'static str,
    open: &'static str,
    inspector: &'static str,
    label: &'static str,
    bounds: [u8; 2],
}

const NUMBER: Fixture = Fixture {
    base: "scroll-field-demo",
    open: "scroll-loop-numeric-show",
    inspector: "scroll_field",
    label: "數量",
    bounds: [2, 10],
};
const CHOICE: Fixture = Fixture {
    base: "scroll-choice-demo",
    open: "scroll-loop-choice-show",
    inspector: "scroll_choice",
    label: "地點",
    bounds: [1, 2],
};

type Rect = [f64; 4];
type Position = [f64; 2];

impl Fixture {
    fn part(self, suffix: &str) -> String {
        format!("{}-value-{suffix}", self.base)
    }

    fn button(self, suffix: &str) -> String {
        format!("{}-{suffix}", self.base)
    }

    async fn read(self, tab: &Tab) -> Result<Value> {
        Ok(state(tab).await?[self.inspector].take())
    }

    async fn ready(self, tab: &Tab, condition: &str) -> Result<()> {
        until(
            tab,
            &format!("(d => d && ({condition}))(s.{})", self.inspector),
        )
        .await?;
        tab.settle().await
    }

    async fn settled(self, tab: &Tab, condition: &str) -> Result<()> {
        self.ready(
            tab,
            &format!("!d.requests_frame && !d.dragging && ({condition})"),
        )
        .await
    }

    async fn phase(self, tab: &Tab) -> Result<f64> {
        self.read(tab).await?["offset"]
            .as_f64()
            .context("continuous wheel phase")
    }

    async fn choose(self, tab: &Tab, suffix: &str) -> Result<()> {
        activate(tab, &self.button(suffix)).await
    }

    async fn reload(self, tab: &Tab, url: &str) -> Result<()> {
        tab.viewport(960, 2044).await?;
        tab.goto(url).await?;
        crate::smoke::ready(tab, "gallery").await?;
        resize(tab, 960, 2000).await?;
        activate(tab, "section-0").await?;
        until(tab, "s.section === 0").await?;
        activate(tab, self.open).await?;
        self.ready(tab, "d.visible && d.looping && d.commits === 0")
            .await?;
        resize(tab, 960, 820).await?;
        self.ready(
            tab,
            "d.visible && d.looping && d.fits && d.mode === 'scroll'",
        )
        .await
    }

    async fn back(self, tab: &Tab) -> Result<()> {
        // Both new launchers must be visible in the original Gallery menu.
        resize(tab, 960, 2000).await?;
        self.choose(tab, "back").await?;
        focused(tab, self.open).await
    }

    async fn editor_focus(self, tab: &Tab) -> Result<()> {
        until(tab, &format!("s.focused === {}", js(self.part("editor")))).await?;
        tab.wait_for(&format!(
            "{TEXTAREA} !== null && document.activeElement === {TEXTAREA}"
        ))
        .await?;
        tab.settle().await
    }

    async fn edit(self, tab: &Tab) -> Result<()> {
        focus_projected(tab, &self.part("adjustable")).await?;
        tab.press("Enter").await?;
        self.editor_focus(tab).await
    }

    async fn replace(self, tab: &Tab, raw: &str) -> Result<()> {
        tab.shortcut("a").await?;
        tab.insert_text(raw).await?;
        self.ready(tab, &format!("d.mode === 'input' && d.raw === {}", js(raw)))
            .await
    }

    async fn spin(self, tab: &Tab, ordinal: u8, value_text: &str) -> Result<Value> {
        let dom = tab.eval(&format!(r#"(() => {{
            const node = {};
            if (!node) return null;
            return {{
                name:node.getAttribute('aria-label'), role:node.getAttribute('role'),
                min:Number(node.getAttribute('aria-valuemin')), max:Number(node.getAttribute('aria-valuemax')),
                now:Number(node.getAttribute('aria-valuenow')), text:node.getAttribute('aria-valuetext'),
                descendants:node.querySelectorAll('[tabindex], [data-surface-action]').length,
                wheel_count:document.querySelectorAll('#surface-accessibility [role="spinbutton"]').length,
            }};
        }})()"#, projected(&self.part("adjustable")))).await?;
        let [min, max] = self.bounds;
        check!(
            dom["role"] == "spinbutton" && dom["name"] == self.label
                && dom["min"] == min && dom["max"] == max && dom["now"] == ordinal
                && dom["text"] == value_text && dom["wheel_count"] == 1 && dom["descendants"] == 0,
            "one Chrome spinbutton reports exactly its real bounded scale and localized value: {dom}"
        );
        let tree = ax_tree(tab).await?;
        let node = ax(&tree, "spinbutton", self.label)?;
        let value = &node["value"]["value"];
        let description = &node["description"]["value"];
        let ax_number = |key| ax_property(node, key).and_then(Value::as_f64);
        let number_matches = value.as_f64() == Some(f64::from(ordinal));
        let projected_value = if self.inspector == CHOICE.inspector {
            description
                .as_str()
                .is_some_and(|text| text.contains("可輸入部分地名"))
                && (value == value_text
                    || (number_matches
                        && description
                            .as_str()
                            .is_some_and(|text| text.contains(value_text))))
        } else {
            number_matches
        };
        check!(
            ax_number("valuemin") == Some(f64::from(min))
                && ax_number("valuemax") == Some(f64::from(max))
                && ax_property(node, "focusable") == Some(&json!(true))
                && projected_value,
            "the actual Chrome accessibility tree exposes the scale and value: {node}"
        );
        Ok(json!({"dom":dom,"chrome_value":value,"chrome_description":description}))
    }
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-scroll-loop-fields-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn between(value: f64, [low, high]: [f64; 2]) -> Result<()> {
    check!(
        low < value && value < high,
        "the painted wheel phase {value} remains inside true signed interval ({low}, {high})"
    );
    Ok(())
}

async fn numeric_keyboard(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M217",
        "ArrowUp and ArrowDown wrap the real numeric bounds using continuous signed motion",
    );
    reduced_motion(tab, false).await?;
    NUMBER
        .ready(
            tab,
            "d.committed === 10 && d.offset === 4 && !d.reduced_motion",
        )
        .await?;
    focus_projected(tab, &NUMBER.part("adjustable")).await?;
    let start = NUMBER.spin(tab, 10, "10 件").await?;
    freeze(tab).await?;
    tab.press("ArrowUp").await?;
    NUMBER
        .ready(
            tab,
            "d.committed === 2 && d.commits === 1 && d.requests_frame",
        )
        .await?;
    let first = NUMBER.spin(tab, 2, "2 件").await?;
    let up_start = NUMBER.phase(tab).await?;
    check!(
        (up_start - 4.).abs() < 0.02,
        "first frame stays at the former last value: {up_start}"
    );
    tick(tab, 38).await?;
    let up = NUMBER.phase(tab).await?;
    between(up, [4., 5.]).context("numeric Increase seaming")?;
    capture(env, tab, "numeric-forward-seam").await?;
    tick(tab, 700).await?;
    NUMBER
        .settled(
            tab,
            "d.committed === 2 && d.offset === 0 && d.commits === 1",
        )
        .await?;

    tab.press("ArrowDown").await?;
    NUMBER
        .ready(
            tab,
            "d.committed === 10 && d.commits === 2 && d.requests_frame",
        )
        .await?;
    let last = NUMBER.spin(tab, 10, "10 件").await?;
    let down_start = NUMBER.phase(tab).await?;
    check!(
        down_start.abs() < 0.02,
        "first frame stays at the former first value: {down_start}"
    );
    tick(tab, 738).await?;
    let down = NUMBER.phase(tab).await?;
    between(down, [-1., 0.]).context("numeric Decrease seaming")?;
    capture(env, tab, "numeric-reverse-seam").await?;
    tick(tab, 1400).await?;
    NUMBER
        .settled(
            tab,
            "d.committed === 10 && d.offset === 4 && d.commits === 2",
        )
        .await?;

    reduced_motion(tab, true).await?;
    NUMBER.ready(tab, "d.reduced_motion").await?;
    tab.press("Home").await?;
    NUMBER
        .settled(
            tab,
            "d.committed === 2 && d.offset === 0 && d.commits === 3",
        )
        .await?;
    tab.press("End").await?;
    NUMBER
        .settled(
            tab,
            "d.committed === 10 && d.offset === 4 && d.commits === 4",
        )
        .await?;
    Ok(
        json!({"ax":[start,first,last],"forward":[up_start,up,0.],"reverse":[down_start,down,4.],
        "boundary_values":[10,2,10],"home_end":[2,10],"commits":4}),
    )
}

const TRACE: &str = r#"
    (() => {
        const canvas = document.getElementById('surface');
        window.scrollLoopContacts = [];
        for (const type of ['pointerdown','pointerup','pointercancel','wheel']) {
            canvas.addEventListener(type, event => {
                if (event.isTrusted) window.scrollLoopContacts.push({
                    type, device:event.pointerType ?? null, id:event.pointerId ?? null,
                    x:event.clientX, y:event.clientY,
                });
            }, {capture:true,passive:true});
        }
    })()
"#;

async fn touch(tab: &Tab, kind: &str, at: Option<Position>) -> Result<()> {
    let points = at.map_or_else(|| json!([]), |[x, y]| json!([{"x":x,"y":y,"id":1}]));
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":kind,"touchPoints":points}),
    )
    .await?;
    tab.settle().await
}

async fn mouse(tab: &Tab, kind: &str, [x, y]: Position, held: bool) -> Result<()> {
    tab.cdp("Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,"button":"left","buttons":u8::from(held),"clickCount":1,"pointerType":"mouse"}),
    ).await?;
    tab.settle().await
}

fn trusted(contacts: &Value, kind: &str, device: Option<&str>) -> bool {
    contacts.as_array().is_some_and(|trace| {
        trace
            .iter()
            .any(|event| event["type"] == kind && event["device"].as_str() == device)
    })
}

fn rgb(data: &Value, key: &str) -> Result<[u8; 3]> {
    let [r, g, b, a]: [u8; 4] = serde_json::from_value(data["palette"][key].clone())
        .with_context(|| format!("Scroll Field paint token {key}"))?;
    check_eq!(a, 255, "the sampled color token is opaque");
    Ok([r, g, b])
}

async fn numeric_drag(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M218",
        "a real mouse drag crosses two numeric seams, commits once and a canceled touch rolls back",
    );
    resize(tab, 960, 1200).await?;
    reduced_motion(tab, false).await?;
    NUMBER
        .ready(
            tab,
            "!d.reduced_motion && d.committed === 10 && d.offset === 4",
        )
        .await?;
    tab.eval(TRACE).await?;
    freeze(tab).await?;
    let owner = point(tab, &NUMBER.part("adjustable")).await?;
    let pitch = NUMBER.read(tab).await?["row_height"]
        .as_f64()
        .context("numeric row pitch")?;
    let origin = [owner.x, owner.y];
    let end = [owner.x, owner.y + 6.23 * pitch];
    check!(
        end[1] < owner.canvas[1] + owner.canvas[3],
        "a full lap and another row fit in the canvas"
    );
    tab.mouse_move(owner.x, owner.y).await?;
    mouse(tab, "mousePressed", origin, true).await?;
    mouse(tab, "mouseMoved", end, true).await?;
    NUMBER
        .ready(
            tab,
            "d.dragging && d.preview === 2 && d.committed === 10 && d.commits === 0",
        )
        .await?;
    let held = NUMBER.phase(tab).await?;
    check!(
        (held - 10.23).abs() < 0.03,
        "held phase retains all 6.23 traveled rows across the seams: {held}"
    );
    let aria_held = NUMBER.spin(tab, 10, "10 件").await?;
    let held_image = capture(env, tab, "numeric-multi-lap").await?;
    let face = bounds(tab, &NUMBER.part("face")).await?;
    let ink = rgb(&NUMBER.read(tab).await?, "center")?;
    let held_ink = held_image.colors(face)?.get(&ink).copied().unwrap_or(0);
    check!(
        held_ink > 8,
        "the held lap paints visible preview ink in the face"
    );

    mouse(tab, "mouseReleased", end, false).await?;
    NUMBER
        .ready(
            tab,
            "d.committed === 2 && d.commits === 1 && d.requests_frame && !d.dragging",
        )
        .await?;
    let release = NUMBER.phase(tab).await?;
    check!(
        (release - held).abs() < 0.02,
        "acceptance keeps the current lap and visual offset: {release}/{held}"
    );
    tick(tab, 38).await?;
    let middle = NUMBER.phase(tab).await?;
    between(middle, [10., release]).context("release remains in the same lap")?;
    capture(env, tab, "numeric-lap-settling").await?;
    tick(tab, 700).await?;
    NUMBER
        .settled(
            tab,
            "d.offset === 0 && d.committed === 2 && d.commits === 1",
        )
        .await?;
    let accepted = NUMBER.spin(tab, 2, "2 件").await?;

    touch(tab, "touchStart", Some(origin)).await?;
    touch(tab, "touchMove", Some(end)).await?;
    NUMBER
        .ready(
            tab,
            "d.dragging && d.preview === 4 && d.committed === 2 && d.commits === 1",
        )
        .await?;
    let cancel_before = NUMBER.phase(tab).await?;
    check!(
        (cancel_before - 6.23).abs() < 0.03,
        "the canceled gesture really crosses a full cycle: {cancel_before}"
    );
    touch(tab, "touchCancel", None).await?;
    tick(tab, 1400).await?;
    NUMBER
        .settled(
            tab,
            "d.offset === 0 && d.committed === 2 && d.commits === 1",
        )
        .await?;
    let contacts = tab.eval("window.scrollLoopContacts").await?;
    check!(
        trusted(&contacts, "pointerup", Some("mouse"))
            && trusted(&contacts, "pointercancel", Some("touch")),
        "the browser delivered both the trusted mouse release and touch cancellation"
    );
    Ok(
        json!({"pitch":pitch,"held":held,"settling":[release,middle,0.],"held_ink":held_ink,
        "committed":2,"commits":1,"canceled_phase":cancel_before,"contacts":contacts,"ax_held":aria_held,"ax":accepted}),
    )
}

async fn outside_points(
    fixture: Fixture,
    tab: &Tab,
    face: Rect,
) -> Result<([Rect; 2], [Position; 2])> {
    let upper = bounds(tab, &fixture.part("above-target")).await?;
    let lower = bounds(tab, &fixture.part("below-target")).await?;
    let face_bottom = face[1] + face[3];
    let lower_bottom = lower[1] + lower[3];
    check!(upper[3] >= 48. && lower[3] >= 48. && face[1] - upper[1] >= 4. && lower_bottom - face_bottom >= 4.,
        "both full 48 dp neighbors protrude beyond the painted face: {upper:?}, {lower:?}, {face:?}");
    let points = [
        [upper[0] + upper[2] / 2., (upper[1] + face[1]) / 2.],
        [lower[0] + lower[2] / 2., (lower_bottom + face_bottom) / 2.],
    ];
    Ok(([upper, lower], points))
}

async fn screen(tab: &Tab, [x, y]: Position) -> Result<Position> {
    let [left, top, _, _] = tab.canvas_box().await?;
    Ok([x + left, y + top])
}

async fn numeric_taps(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M219",
        "48 dp targets outside the face wrap both ways and wheel input scrolls the page",
    );
    reduced_motion(tab, true).await?;
    NUMBER.choose(tab, "compact").await?;
    NUMBER
        .ready(tab, "d.reduced_motion && d.face_height === 120")
        .await?;
    let face = bounds(tab, &NUMBER.part("face")).await?;
    check!(
        near(face[3], 120.),
        "the actual painted numeric face is 120 dp high: {face:?}"
    );
    let (zones, [upper, lower]) = outside_points(NUMBER, tab, face).await?;
    tab.eval(TRACE).await?;
    let [x, y] = screen(tab, upper).await?;
    tab.click(x, y).await?;
    NUMBER
        .settled(
            tab,
            "d.committed === 2 && d.offset === 0 && d.commits === 1",
        )
        .await?;
    touch(tab, "touchStart", Some(screen(tab, lower).await?)).await?;
    touch(tab, "touchEnd", None).await?;
    NUMBER
        .settled(
            tab,
            "d.committed === 10 && d.offset === 4 && d.commits === 2",
        )
        .await?;
    capture(env, tab, "numeric-cropped-targets").await?;

    let scroll = |data: &Value| {
        data["scroll_top"]
            .as_f64()
            .context("ancestor scroll offset")
    };
    let before = scroll(&NUMBER.read(tab).await?)?;
    let center = point(tab, &NUMBER.part("adjustable")).await?;
    tab.mouse_move(center.x, center.y).await?;
    tab.wheel(0., 96.).await?;
    NUMBER
        .ready(
            tab,
            &format!("d.scroll_top > {before} && d.committed === 10 && d.commits === 2"),
        )
        .await?;
    let after = scroll(&NUMBER.read(tab).await?)?;
    let contacts = tab.eval("window.scrollLoopContacts").await?;
    check!(
        trusted(&contacts, "pointerup", Some("mouse"))
            && trusted(&contacts, "pointerup", Some("touch"))
            && trusted(&contacts, "wheel", None),
        "both cropped taps and the page wheel arrived as trusted input"
    );
    NUMBER.back(tab).await?;
    Ok(
        json!({"face":face,"zones":zones,"outside":[upper,lower],"tap_values":[2,10],"commits":2,
        "page_scroll":[before,after],"contacts":contacts,"back_focus":NUMBER.open}),
    )
}

fn intersection([x, y, w, h]: Rect, [a, b, c, d]: Rect) -> Rect {
    let left = x.max(a);
    let top = y.max(b);
    [
        left,
        top,
        (x + w).min(a + c) - left,
        (y + h).min(b + d) - top,
    ]
}

async fn choice_direction(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M220",
        "both neighbors of a two-item list paint the same name but travel in the chosen direction",
    );
    reduced_motion(tab, false).await?;
    CHOICE.choose(tab, "compact").await?;
    CHOICE.ready(tab, "!d.reduced_motion && d.face_height === 120 && d.committed === 'banqiao' && d.ordinal === 1 && d.offset === 0").await?;
    let initial = CHOICE.spin(tab, 1, "板橋").await?;
    let face = bounds(tab, &CHOICE.part("face")).await?;
    let (_, [upper, lower]) = outside_points(CHOICE, tab, face).await?;
    let [above_id, below_id] = [CHOICE.part("above"), CHOICE.part("below")];
    let labels = tab
        .eval(&format!(
            r#"(() => {{
        const nodes = JSON.parse(window.gallery.export_dom()).dom.nodes;
        return [{}, {}].map(id => nodes.find(node => node.stable_id === id)?.text);
    }})()"#,
            js(&above_id),
            js(&below_id)
        ))
        .await?;
    check_eq!(
        labels,
        json!(["台北 車站", "台北 車站"]),
        "each physical side has its own copy of the other choice"
    );
    let [above, below] = [bounds(tab, &above_id).await?, bounds(tab, &below_id).await?];
    check!(above[1] < face[1] && below[1] + below[3] > face[1] + face[3],
        "the two independent paint slots straddle opposite crop boundaries: {above:?}, {below:?}, {face:?}");
    let image = capture(env, tab, "choice-two-peeks").await?;
    let color = rgb(&CHOICE.read(tab).await?, "neighbor")?;
    let counts = [
        image
            .colors(intersection(above, face))?
            .get(&color)
            .copied()
            .unwrap_or(0),
        image
            .colors(intersection(below, face))?
            .get(&color)
            .copied()
            .unwrap_or(0),
    ];
    check!(
        counts.iter().all(|count| *count > 8),
        "the real PNG contains both cropped neighbors: {counts:?}"
    );

    freeze(tab).await?;
    let [x, y] = screen(tab, upper).await?;
    tab.click(x, y).await?;
    CHOICE.ready(tab, "d.committed === 'taipei-station' && d.ordinal === 2 && d.commits === 1 && d.requests_frame").await?;
    tick(tab, 38).await?;
    let forward = CHOICE.phase(tab).await?;
    between(forward, [0., 1.]).context("upper two-item selected physical direction")?;
    capture(env, tab, "choice-forward").await?;
    tick(tab, 700).await?;
    CHOICE
        .settled(tab, "d.offset === 1 && d.commits === 1")
        .await?;
    let first = CHOICE.spin(tab, 2, "台北 車站").await?;

    let [x, y] = screen(tab, lower).await?;
    tab.click(x, y).await?;
    CHOICE
        .ready(
            tab,
            "d.committed === 'banqiao' && d.ordinal === 1 && d.commits === 2 && d.requests_frame",
        )
        .await?;
    tick(tab, 1400).await?;
    CHOICE
        .settled(tab, "d.offset === 0 && d.commits === 2")
        .await?;
    touch(tab, "touchStart", Some([x, y])).await?;
    touch(tab, "touchEnd", None).await?;
    CHOICE.ready(tab, "d.committed === 'taipei-station' && d.ordinal === 2 && d.commits === 3 && d.requests_frame").await?;
    tick(tab, 1438).await?;
    let reverse = CHOICE.phase(tab).await?;
    between(reverse, [-1., 0.]).context("lower two-item selected physical direction")?;
    capture(env, tab, "choice-reverse").await?;
    tick(tab, 2100).await?;
    CHOICE
        .settled(tab, "d.offset === 1 && d.commits === 3")
        .await?;
    let last = CHOICE.spin(tab, 2, "台北 車站").await?;
    Ok(
        json!({"peek_labels":labels,"peek_ink":counts,"peek_slots":[above,below],"face":face,
        "upper_phase":forward,"lower_phase":reverse,"accepted_id":"taipei-station","commits":3,"ax":[initial,first,last]}),
    )
}

async fn choice_catalog(tab: &Tab) -> Result<Value> {
    step("M221", "catalog changes retire the held route without changing the accepted choice or losing the typed draft");
    reduced_motion(tab, false).await?;
    CHOICE
        .ready(
            tab,
            "d.committed === 'banqiao' && d.ordinal === 1 && d.offset === 0",
        )
        .await?;
    check_eq!(
        CHOICE.read(tab).await?["order"],
        json!(["banqiao", "taipei-station"]),
        "the initial authored order has two stable IDs"
    );
    tab.eval(TRACE).await?;
    freeze(tab).await?;
    focus_projected(tab, &CHOICE.button("reorder")).await?;
    let owner = point(tab, &CHOICE.part("adjustable")).await?;
    let pitch = CHOICE.read(tab).await?["row_height"]
        .as_f64()
        .context("choice row pitch")?;
    touch(tab, "touchStart", Some([owner.x, owner.y])).await?;
    touch(tab, "touchMove", Some([owner.x, owner.y + pitch * 1.23])).await?;
    CHOICE.ready(tab, "d.dragging && d.preview === 'taipei-station' && d.committed === 'banqiao' && d.commits === 0").await?;
    let held = CHOICE.phase(tab).await?;
    check!(
        (held - 1.23).abs() < 0.03,
        "old catalog really previews a different choice: {held}"
    );
    until(tab, "s.focused === 'scroll-choice-demo-reorder'").await?;
    tab.press("Enter").await?;
    CHOICE
        .settled(
            tab,
            "d.committed === 'banqiao' && d.ordinal === 2 && d.offset === 1 && d.commits === 0",
        )
        .await?;
    touch(tab, "touchEnd", None).await?;
    tick(tab, 700).await?;
    CHOICE
        .settled(
            tab,
            "d.committed === 'banqiao' && d.ordinal === 2 && d.offset === 1 && d.commits === 0",
        )
        .await?;
    let reversed = CHOICE.read(tab).await?["order"].clone();
    check_eq!(
        reversed,
        json!(["taipei-station", "banqiao"]),
        "the replacement really reversed the order"
    );
    let reordered = CHOICE.spin(tab, 2, "板橋").await?;
    let contacts = tab.eval("window.scrollLoopContacts").await?;
    check!(
        trusted(&contacts, "pointerup", Some("touch")),
        "the stale end actually reached Chrome after replacement"
    );

    CHOICE.edit(tab).await?;
    CHOICE.replace(tab, "板").await?;
    CHOICE
        .ready(tab, "d.suggestion === '板橋' && d.commits === 0")
        .await?;
    CHOICE.choose(tab, "relabel").await?;
    CHOICE
        .ready(
            tab,
            "d.mode === 'input' && d.raw === '板' && d.suggestion === '板橋站' && d.commits === 0",
        )
        .await?;
    click(tab, &CHOICE.part("editor")).await?;
    CHOICE.editor_focus(tab).await?;
    tab.press("Escape").await?;
    CHOICE.settled(tab, "d.mode === 'scroll' && d.committed === 'banqiao' && d.ordinal === 2 && d.raw === '板橋站' && d.commits === 0").await?;
    let renamed = CHOICE.spin(tab, 2, "板橋站").await?;
    reduced_motion(tab, true).await?;
    CHOICE.ready(tab, "d.reduced_motion").await?;
    focus_projected(tab, &CHOICE.part("adjustable")).await?;
    tab.press("ArrowUp").await?;
    CHOICE.settled(tab, "d.committed === 'taipei-station' && d.ordinal === 1 && d.offset === 0 && d.commits === 1").await?;
    let accepted = CHOICE.spin(tab, 1, "台北 車站").await?;
    CHOICE.back(tab).await?;
    Ok(
        json!({"held":held,"order":reversed,"stale_release_ignored":true,"external_commits":0,
        "id_retained":"banqiao","rematched_suggestion":"板橋站","ax":[reordered,renamed,accepted],"contacts":contacts,
        "wrapped_id":"taipei-station","commits":1,"back_focus":CHOICE.open}),
    )
}

async fn compact(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    step("M222", "explicit input stays bounded and dark RTL 200 percent fallback preserves live composition through resize");
    NUMBER.edit(tab).await?;
    NUMBER.replace(tab, "12").await?;
    tab.press("Enter").await?;
    NUMBER.ready(tab, "d.mode === 'input' && d.raw === '12' && d.error === 'OutOfRange' && d.committed === 10 && d.commits === 0").await?;
    NUMBER.editor_focus(tab).await?;
    let error = tab
        .eval_str(&format!("{}.textContent", projected(&NUMBER.part("error"))))
        .await?;
    let invalid = tab
        .eval(&format!("{TEXTAREA}.getAttribute('aria-invalid')"))
        .await?;
    check!(
        !error.trim().is_empty() && invalid == "true",
        "typed 12 stays invalid instead of looping through the lower bound: {error}"
    );
    capture(env, tab, "numeric-out-of-range").await?;
    NUMBER.replace(tab, "２").await?;
    tab.press("Enter").await?;
    NUMBER.settled(tab, "d.mode === 'scroll' && !d.error && d.committed === 2 && d.offset === 0 && d.commits === 1").await?;
    focused(tab, &NUMBER.part("adjustable")).await?;
    let bounded = NUMBER.spin(tab, 2, "2 件").await?;

    CHOICE.reload(tab, url).await?;
    for option in ["theme", "rtl", "large-text"] {
        CHOICE.choose(tab, option).await?;
    }
    resize(tab, 320, 820).await?;
    until(tab, "s.dark === true").await?;
    CHOICE.ready(tab, "d.rtl && d.text_scale === 2 && !d.fits && d.mode === 'input' && d.committed === 'banqiao'").await?;
    let editor_id = CHOICE.part("editor");
    click(tab, &editor_id).await?;
    CHOICE.editor_focus(tab).await?;
    let editor = bounds(tab, &editor_id).await?;
    check!(
        contains([0., 0., 320., 820.], editor) && editor[2] >= 48. && editor[3] >= 48.,
        "the dark RTL 200 percent editor remains inside the measured canvas: {editor:?}"
    );
    let only_editor = tab
        .eval(&format!(
            "{} === null",
            projected(&CHOICE.part("adjustable"))
        ))
        .await?;
    check_eq!(
        only_editor,
        json!(true),
        "the compact path exposes the editor without a hidden second spinbutton"
    );
    let dark = capture(env, tab, "compact-dark-rtl").await?;
    let ink = rgb(&CHOICE.read(tab).await?, "editing")?;
    let ink_count = dark.colors(editor)?.get(&ink).copied().unwrap_or(0);
    check!(
        ink_count > 16,
        "the enlarged dark editor paints its font color in real WebGPU pixels"
    );

    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(&editor_id)
    );
    tab.shortcut("a").await?;
    tab.ime_compose("台北車站", 4, 4).await?;
    tab.wait_for(&composing).await?;
    CHOICE
        .ready(
            tab,
            "d.raw === '板橋' && d.committed === 'banqiao' && d.commits === 0",
        )
        .await?;
    resize(tab, 960, 820).await?;
    CHOICE
        .ready(
            tab,
            "d.fits && d.mode === 'input' && d.raw === '板橋' && d.commits === 0",
        )
        .await?;
    CHOICE.editor_focus(tab).await?;
    tab.wait_for(&composing).await?;
    capture(env, tab, "resized-ime-preedit").await?;
    tab.press("Enter").await?;
    CHOICE
        .ready(
            tab,
            "d.mode === 'input' && d.committed === 'banqiao' && d.commits === 0",
        )
        .await?;
    if CHOICE.read(tab).await?["raw"] != "台北車站" {
        tab.insert_text("台北車站").await?;
    }
    let draft = "d.mode === 'input' && d.raw === '台北車站' && d.committed === 'banqiao' && d.commits === 0";
    tab.wait_for(&format!("!({composing})")).await?;
    CHOICE.ready(tab, draft).await?;
    resize(tab, 320, 820).await?;
    CHOICE.ready(tab, draft).await?;
    CHOICE.editor_focus(tab).await?;
    resize(tab, 960, 820).await?;
    CHOICE.ready(tab, draft).await?;
    CHOICE.editor_focus(tab).await?;
    tab.press("Enter").await?;
    CHOICE.settled(tab, "d.mode === 'scroll' && d.fits && d.committed === 'taipei-station' && d.ordinal === 2 && d.offset === 1 && d.commits === 1").await?;
    until(tab, "s.focused === 'scroll-choice-demo-value-adjustable'").await?;
    tab.wait_for(CANVAS_FOCUS).await?;
    let choice = CHOICE.spin(tab, 2, "台北 車站").await?;
    capture(env, tab, "wide-rtl-choice").await?;
    resize(tab, 320, 820).await?;
    CHOICE.ready(tab, "!d.fits && d.mode === 'input' && d.committed === 'taipei-station' && d.raw === '台北' && d.commits === 1").await?;
    CHOICE.editor_focus(tab).await?;
    Ok(
        json!({"numeric":{"invalid_input":"12","visible_error":error,"committed":2,"commits":1,"ax":bounded},
        "choice":{"viewport":[320,820],"rtl":true,"text_scale":2,"editor":editor,"ink_count":ink_count,
        "preedit_survived_resize":true,"raw":"台北車站","first_enter_commits":0,"id":"taipei-station","commits":1,
        "restored_editor_focus":true,"ax":choice}}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    NUMBER.reload(tab, url).await?;
    let keyboard = numeric_keyboard(env, tab).await?;
    NUMBER.reload(tab, url).await?;
    let drag = numeric_drag(env, tab).await?;
    NUMBER.reload(tab, url).await?;
    let taps = numeric_taps(env, tab).await?;
    CHOICE.reload(tab, url).await?;
    let direction = choice_direction(env, tab).await?;
    CHOICE.reload(tab, url).await?;
    let catalog = choice_catalog(tab).await?;
    NUMBER.reload(tab, url).await?;
    let compact = compact(env, tab, url).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "looping Scroll Field page errors: {errors:?}"
    );
    env.write_json("browser-material-scroll-loop-fields.json", &json!({
        "browser":chrome.version,"keyboard":keyboard,"drag":drag,"taps":taps,"direction":direction,
        "catalog":catalog,"compact":compact,"errors":errors,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-scroll-loop-fields-failure.json",
            &json!({
                "error":format!("{error:#}"),"state":state(&tab).await.ok(),"errors":tab.errors(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),
                "contacts":tab.eval("window.scrollLoopContacts ?? []").await.ok(),
            }),
        );
        let _ = tab
            .screenshot(&env.artifact("material-scroll-loop-fields-failure.png"))
            .await;
    }
    result.context("Material 3 looping Scroll Fields")
}
