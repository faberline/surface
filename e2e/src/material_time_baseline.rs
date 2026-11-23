//! Baseline Material Time Input and Dial on the real Chrome WebGPU host.

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

const INPUT: &str = "time-input-modal";
const DIAL: &str = "time-dial";
const FACE: &str = "time-dial-dial-face";
const HOUR: &str = "time-dial-hour-selector";
const MINUTE: &str = "time-dial-minute-selector";
const USE_INPUT: &str = "time-dial-use-input";
const USE_DIAL: &str = "time-dial-use-dial";

type Rect = [f64; 4];
type Rgb = [u8; 3];
type Position = (f64, f64);

#[derive(Clone, Copy)]
struct Scene {
    key: &'static str,
    prefix: &'static str,
    show: &'static str,
    opener: &'static str,
    outside: &'static str,
    first: &'static str,
    draft: &'static str,
    saved: &'static str,
    commits: &'static str,
}

const INPUT_SCENE: Scene = Scene {
    key: "time_input",
    prefix: "time-input",
    show: "time-input-baseline-demo-show",
    opener: "time-input-demo-modal-open",
    outside: "time-input-demo-outside",
    first: "time-input-modal-hour-input",
    draft: "modal",
    saved: "modal_committed",
    commits: "modal_commits",
};
const DIAL_SCENE: Scene = Scene {
    key: "time_dial",
    prefix: "time-dial",
    show: "time-dial-baseline-demo-show",
    opener: "time-dial-demo-open",
    outside: "time-dial-demo-outside",
    first: HOUR,
    draft: "draft",
    saved: "saved",
    commits: "commits",
};

impl Scene {
    async fn ready(self, tab: &Tab, expression: &str) -> Result<()> {
        until(tab, &format!("(d => {expression})(s.{})", self.key)).await?;
        tab.settle().await
    }

    async fn reload(self, tab: &Tab, url: &str) -> Result<()> {
        tab.viewport(960, 1444).await?;
        tab.goto(url).await?;
        crate::smoke::ready(tab, "gallery").await?;
        resize(tab, 960, 1400).await?;
        activate(tab, "section-2").await?;
        until(tab, "s.section === 2").await?;
        activate(tab, self.show).await?;
        self.ready(
            tab,
            &format!(
                "d.visible && d.variant === 'Baseline' && d.{} === null && d.{} === '09:07' && d.{} === 0",
                self.draft, self.saved, self.commits
            ),
        )
        .await
    }

    async fn setting(self, tab: &Tab, suffix: &str) -> Result<()> {
        activate(tab, &format!("{}-demo-{suffix}", self.prefix)).await
    }

    async fn open(self, tab: &Tab) -> Result<()> {
        activate(tab, self.opener).await?;
        self.ready(tab, &format!("d.{} !== null", self.draft))
            .await?;
        focused(tab, self.first).await
    }

    async fn closed(self, tab: &Tab, saved: &str, commits: u8) -> Result<()> {
        self.ready(
            tab,
            &format!(
                "d.{} === null && d.{} === {} && d.{} === {commits} && d.outside_actions === 0",
                self.draft,
                self.saved,
                js(saved),
                self.commits
            ),
        )
        .await?;
        until(tab, &format!("s.focused === {}", js(self.opener))).await
    }
}

fn part(base: &str, name: &str) -> String {
    format!("{base}-{name}")
}

fn center([x, y, w, h]: Rect) -> Rect {
    [x + w / 2., y + h / 2., 0., 0.]
}

fn overlap([ax, ay, aw, ah]: Rect, [bx, by, bw, bh]: Rect) -> bool {
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

async fn size(tab: &Tab, id: &str, w: f64, h: f64) -> Result<Rect> {
    let rect = bounds(tab, id).await?;
    check!(
        near(rect[2], w) && near(rect[3], h),
        "{id} measures {w} × {h} dp: {rect:?}"
    );
    Ok(rect)
}

async fn node(tab: &Tab, id: &str) -> Result<Value> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes.find(n => n.stable_id === {})",
        js(id)
    ))
    .await
}

async fn shell(tab: &Tab, id: &str, viewport: Rect) -> Result<Rect> {
    let panel = bounds(tab, id).await?;
    let authored = node(tab, id).await?;
    let padding = authored["style"]["padding"]
        .as_f64()
        .context("Baseline panel padding")?;
    check!(
        contains(viewport, panel)
            && if viewport[3] < 320. {
                (0. ..=24.).contains(&padding)
            } else {
                near(padding, 24.)
            },
        "the Baseline panel fits with its normal or adaptive inset: {panel:?}, {padding} dp"
    );
    let shadows = authored["box_shadows"]
        .as_array()
        .context("the Baseline modal authors its two elevation layers")?;
    check_eq!(
        shadows.len(),
        2,
        "Baseline elevation has both shadow layers"
    );
    for (layer, (offset, blur, spread)) in shadows.iter().zip([(4., 8., 3.), (1., 3., 0.)]) {
        check!(
            layer["offset_y"] == offset
                && layer["blur"] == blur
                && layer["spread"] == spread
                && layer["color"][3].as_u64().is_some_and(|alpha| alpha > 0),
            "the modal uses the Material Level 3 shadow layers: {shadows:?}"
        );
    }
    Ok(panel)
}

async fn token(tab: &Tab, name: &str) -> Result<Rgb> {
    let [r, g, b, a]: [u8; 4] =
        serde_json::from_value(state(tab).await?["baseline_time_colors"][name].clone())?;
    check_eq!(a, 255, format!("the Gallery {name} color is opaque"));
    Ok([r, g, b])
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    let path = env.artifact(&format!("material-time-baseline-{name}.png"));
    tab.settle().await?;
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

fn expect_paint(image: &Raster, rect: Rect, color: Rgb, what: &str) -> Result<usize> {
    let colors = image.colors(rect)?;
    let total: usize = colors.values().sum();
    let matches: usize = colors
        .iter()
        .filter(|(sample, _)| sample.iter().zip(color).all(|(a, b)| a.abs_diff(b) <= 4))
        .map(|(_, count)| *count)
        .sum();
    check!(
        matches * 4 >= total * 3,
        "{what} paints {color:?} in the actual PNG: {rect:?}, {matches}/{total} pixels"
    );
    Ok(matches)
}

fn field_fill([x, y, w, _]: Rect) -> Rect {
    [x + w - 13., y + 7., 5., 3.]
}

fn top_stroke([x, y, w, _]: Rect) -> Rect {
    [x + w / 2. - 4., y + 0.5, 8., 1.]
}

fn panel_fill([x, y, w, _]: Rect) -> Rect {
    [x + w / 2. - 3., y + 10., 6., 4.]
}

async fn checked(tab: &Tab, id: &str, selected: bool) -> Result<String> {
    let label = tab
        .eval_str(&format!("{}?.getAttribute('aria-label')", projected(id)))
        .await?;
    let tree = ax_tree(tab).await?;
    let option = ax(&tree, "radio", &label)?;
    check_eq!(
        ax_property(option, "checked"),
        Some(&json!(selected.to_string())),
        format!("Chrome reports the checked state of {id}")
    );
    Ok(label)
}

async fn absent(tab: &Tab, id: &str) -> Result<()> {
    check_eq!(
        tab.eval(&format!(
            "window.gallery.control_bounds({}).length === 0 && {} === null",
            js(id),
            projected(id)
        ))
        .await?,
        json!(true),
        format!("{id} is absent from both layout and projected accessibility")
    );
    Ok(())
}

async fn period_geometry(
    tab: &Tab,
    base: &str,
    panel: Rect,
    paint_size: (f64, f64),
) -> Result<Value> {
    let paint = size(tab, &part(base, "period-paint"), paint_size.0, paint_size.1).await?;
    let am = bounds(tab, &part(base, "am")).await?;
    let pm = bounds(tab, &part(base, "pm")).await?;
    check!(
        [am, pm]
            .into_iter()
            .all(|target| contains(panel, target) && target[2] >= 47.5 && target[3] >= 47.5)
            && !overlap(am, pm)
            && contains(panel, paint)
            && overlap(am, paint)
            && overlap(pm, paint),
        "each period owns a disjoint 48 dp target over its painted group: {paint:?} {am:?} {pm:?}"
    );
    Ok(json!({"paint":paint,"am":am,"pm":pm}))
}

async fn standalone_input(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M155",
        "Baseline Input measures its editors and period targets and paints the active field",
    );
    resize(tab, 960, 820).await?;
    let mut profiles = Vec::new();
    for (dark, name) in [(false, "input-light-12h"), (true, "input-dark-24h")] {
        if dark {
            INPUT_SCENE.setting(tab, "theme").await?;
            INPUT_SCENE.setting(tab, "format").await?;
        }
        until(tab, &format!("s.dark === {dark}")).await?;
        INPUT_SCENE.open(tab).await?;
        INPUT_SCENE
            .ready(
                tab,
                if dark {
                    "d.inline === null && d.format === 'TwentyFourHour' && d.modal.candidate === '09:07'"
                } else {
                    "d.inline === null && d.format === 'TwelveHour' && d.modal.candidate === '09:07'"
                },
            )
            .await?;
        let panel = shell(tab, INPUT, [0., 0., 960., 820.]).await?;
        let title = bounds(tab, &part(INPUT, "title")).await?;
        check!(
            near(title[0] - panel[0], 24.) && near(title[1] - panel[1], 24.),
            "the visible Baseline title starts at the panel's 24 dp insets: {title:?}"
        );
        let hour = size(tab, "time-input-modal-hour-input", 96., 72.).await?;
        let minute = size(tab, "time-input-modal-minute-input", 96., 72.).await?;
        let period = if dark {
            absent(tab, "time-input-modal-period").await?;
            Value::Null
        } else {
            checked(tab, "time-input-modal-am", true).await?;
            period_geometry(tab, INPUT, panel, (52., 72.)).await?
        };
        let tree = ax_tree(tab).await?;
        ax(&tree, "dialog", "選擇時間")?;
        let editor = ax(&tree, "textbox", "小時")?;
        let expected_hint = if dark { "0–23" } else { "1–12" };
        check!(
            editor["description"]["value"]
                .as_str()
                .is_some_and(|hint| hint.contains(expected_hint)),
            "the Baseline editor exposes the correct hour hint: {editor}"
        );
        let shell_color = token(tab, "surface_container_high").await?;
        let rest = token(tab, "surface_container_highest").await?;
        let active = token(tab, "primary_container").await?;
        let primary = token(tab, "primary").await?;
        let first = capture(env, tab, name).await?;
        expect_paint(&first, panel_fill(panel), shell_color, "Baseline shell")?;
        expect_paint(&first, field_fill(hour), active, "focused Hour fill")?;
        expect_paint(&first, field_fill(minute), rest, "resting Minute fill")?;
        expect_paint(&first, top_stroke(hour), primary, "focused Hour outline")?;
        if !dark {
            let [x, y, _, _] = bounds(tab, "time-input-modal-am-paint").await?;
            let tertiary = token(tab, "tertiary_container").await?;
            expect_paint(
                &first,
                [x + 5., y + 8., 4., 4.],
                tertiary,
                "selected AM fill",
            )?;
        }
        tab.press("Tab").await?;
        focused(tab, "time-input-modal-minute-input").await?;
        let second = capture(env, tab, &format!("{name}-focus-minute")).await?;
        expect_paint(&second, field_fill(hour), rest, "unfocused Hour fill")?;
        expect_paint(&second, field_fill(minute), active, "focused Minute fill")?;
        expect_paint(
            &second,
            top_stroke(minute),
            primary,
            "focused Minute outline",
        )?;
        tab.press("Escape").await?;
        INPUT_SCENE.closed(tab, "09:07", 0).await?;
        focused(tab, INPUT_SCENE.opener).await?;
        profiles.push(
            json!({"dark":dark,"panel":panel,"hour":hour,"minute":minute,"period":period,
            "colors":{"shell":shell_color,"rest":rest,"active":active,"outline":primary}}),
        );
    }
    Ok(json!(profiles))
}

async fn arrangements(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M156",
        "both Baseline Dial arrangements retain exact face and selector dimensions in both formats",
    );
    let mut profiles = Vec::new();
    for (width, height, clock, orientation) in [
        (390, 720, "TwelveHour", "Vertical"),
        (844, 390, "TwelveHour", "Horizontal"),
        (844, 390, "TwentyFourHour", "Horizontal"),
        (390, 720, "TwentyFourHour", "Vertical"),
    ] {
        if clock == "TwentyFourHour" && orientation == "Horizontal" {
            DIAL_SCENE.setting(tab, "format").await?;
        }
        if clock == "TwelveHour" && orientation == "Horizontal" {
            DIAL_SCENE.setting(tab, "theme").await?;
            until(tab, "s.dark === true").await?;
        }
        resize(tab, width, height).await?;
        DIAL_SCENE.open(tab).await?;
        DIAL_SCENE
            .ready(
                tab,
                &format!(
                    "d.clock === {} && d.orientation === {} && d.draft.mode === 'Dial' && d.draft.candidate === '09:07'",
                    js(clock),
                    js(orientation)
                ),
            )
            .await?;
        let viewport = [0., 0., f64::from(width), f64::from(height)];
        let panel = shell(tab, DIAL, viewport).await?;
        let clock_face = size(tab, FACE, 256., 256.).await?;
        size(tab, "time-dial-dial-center", 8., 8.).await?;
        let handle = size(tab, "time-dial-dial-handle", 48., 48.).await?;
        let track = node(tab, "time-dial-dial-track").await?;
        let points: [[f64; 2]; 4] =
            serde_json::from_value(track["fill_contour"]["points"].clone())?;
        let thickness = (points[3][0] - points[0][0]).hypot(points[3][1] - points[0][1]);
        check!(
            near(thickness, 2.),
            "the Baseline hand track is 2 dp: {points:?}"
        );
        let selector_width = if clock == "TwentyFourHour" && orientation == "Vertical" {
            114.
        } else {
            96.
        };
        let hour = size(tab, HOUR, selector_width, 80.).await?;
        let minute = size(tab, MINUTE, selector_width, 80.).await?;
        check!(
            [clock_face, hour, minute]
                .into_iter()
                .all(|rect| contains(panel, rect))
                && !overlap(clock_face, hour)
                && !overlap(clock_face, minute),
            "the full clock and both independent selectors fit the measured panel"
        );
        let period = if clock == "TwelveHour" {
            let paint = if orientation == "Vertical" {
                (52., 80.)
            } else {
                (216., 38.)
            };
            period_geometry(tab, DIAL, panel, paint).await?
        } else {
            absent(tab, "time-dial-period").await?;
            Value::Null
        };
        let shot = capture(
            env,
            tab,
            &format!(
                "dial-{}-{}h",
                orientation.to_ascii_lowercase(),
                if clock == "TwelveHour" { 12 } else { 24 }
            ),
        )
        .await?;
        let shell_color = token(tab, "surface_container_high").await?;
        let face_color = token(tab, "surface_container_highest").await?;
        expect_paint(&shot, panel_fill(panel), shell_color, "Baseline Dial shell")?;
        expect_paint(
            &shot,
            [clock_face[0] + 106., clock_face[1] + 101., 6., 4.],
            face_color,
            "Baseline clock face",
        )?;
        expect_paint(
            &shot,
            [handle[0] + 10., handle[1] + 10., 4., 3.],
            token(tab, "primary").await?,
            "Baseline clock handle",
        )?;
        if clock == "TwelveHour" && orientation == "Horizontal" {
            focus_projected(tab, "time-dial-am").await?;
            tab.press("ArrowRight").await?;
            focused(tab, "time-dial-pm").await?;
            DIAL_SCENE
                .ready(
                    tab,
                    "d.draft.period === 'Pm' && d.draft.candidate === '21:07' && d.commits === 0",
                )
                .await?;
            checked(tab, "time-dial-pm", true).await?;
            checked(tab, "time-dial-am", false).await?;
        }
        tab.press("Escape").await?;
        DIAL_SCENE.closed(tab, "09:07", 0).await?;
        focused(tab, DIAL_SCENE.opener).await?;
        profiles.push(
            json!({"clock":clock,"orientation":orientation,"panel":panel,
            "face":clock_face,"selectors":[hour,minute],"period":period}),
        );
    }
    Ok(json!(profiles))
}

async fn tick(tab: &Tab, value: f64, count: f64, radius: f64) -> Result<Position> {
    let face = point(tab, FACE).await?;
    let angle = value / count * std::f64::consts::TAU;
    Ok((face.x + radius * angle.sin(), face.y - radius * angle.cos()))
}

async fn mouse(tab: &Tab, kind: &str, (x, y): Position, held: bool) -> Result<()> {
    tab.cdp(
        "Input.dispatchMouseEvent",
        json!({"type":kind,"x":x,"y":y,"button":"left","buttons":u8::from(held),"clickCount":1}),
    )
    .await?;
    tab.settle().await
}

async fn tap(tab: &Tab, (x, y): Position) -> Result<()> {
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":x,"y":y,"id":1}]}),
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

async fn portrait_input(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M157",
        "a held pointer, both 24-hour rings and an exact minute share the Baseline clock",
    );
    DIAL_SCENE.setting(tab, "format").await?;
    resize(tab, 390, 720).await?;
    DIAL_SCENE.open(tab).await?;
    tab.eval(
        "(() => {
          window.baselineContacts = [];
          for (const kind of ['pointerdown', 'pointermove', 'pointerup']) {
            document.querySelector('canvas').addEventListener(kind, event => {
              if (event.isTrusted) window.baselineContacts.push({kind, type:event.pointerType});
            }, {capture:true,passive:true});
          }
        })()",
    )
    .await?;
    let outer = tick(tab, 0., 12., 100.).await?;
    let six = tick(tab, 6., 12., 100.).await?;
    mouse(tab, "mousePressed", outer, true).await?;
    mouse(tab, "mouseMoved", six, true).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.preview?.value === 6 && d.draft.stage === 'Hour' &&
             d.draft.candidate === '09:07' && d.commits === 0",
        )
        .await?;
    capture(env, tab, "dial-portrait-held").await?;
    mouse(tab, "mouseReleased", six, false).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.stage === 'Minute' && d.draft.candidate === '06:07'",
        )
        .await?;
    for (radius, hour) in [(100., "00"), (64., "12")] {
        activate(tab, HOUR).await?;
        DIAL_SCENE.ready(tab, "d.draft.stage === 'Hour'").await?;
        tap(tab, tick(tab, 0., 12., radius).await?).await?;
        DIAL_SCENE
            .ready(
                tab,
                &format!(
                    "d.draft.stage === 'Minute' && d.draft.raw_hour === {} && d.commits === 0",
                    js(hour)
                ),
            )
            .await?;
    }
    tap(tab, tick(tab, 59., 60., 100.).await?).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.candidate === '12:59' && d.saved === '09:07' && d.commits === 0",
        )
        .await?;
    let exact = checked(tab, "time-dial-dial-minute-59", true).await?;
    check_eq!(
        exact,
        "分鐘 59，共 60 個選項",
        "exact minute has a complete spoken name"
    );
    checked(tab, "time-dial-dial-minute-0", false).await?;
    let contacts = tab.eval("window.baselineContacts").await?;
    let records = contacts
        .as_array()
        .context("trusted Baseline pointer events")?;
    for (device, kind) in [
        ("mouse", "pointerdown"),
        ("mouse", "pointermove"),
        ("mouse", "pointerup"),
        ("touch", "pointerdown"),
        ("touch", "pointerup"),
    ] {
        check!(
            records
                .iter()
                .any(|event| event["type"] == device && event["kind"] == kind),
            "trusted {device} {kind} reached the real Baseline canvas"
        );
    }
    activate(tab, "time-dial-cancel").await?;
    DIAL_SCENE.closed(tab, "09:07", 0).await?;
    Ok(
        json!({"preview":6,"outer":"00","inner":"12","candidate":"12:59","exact_ax":exact,
        "trusted_contacts":contacts}),
    )
}

async fn switch_and_save(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M158",
        "Baseline Dial and Input share a draft, preedit survives a mode press, and Save commits once",
    );
    resize(tab, 960, 820).await?;
    DIAL_SCENE.open(tab).await?;
    let (x, y) = tick(tab, 4., 12., 100.).await?;
    tab.click(x, y).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.stage === 'Minute' && d.draft.candidate === '04:07'",
        )
        .await?;
    activate(tab, USE_INPUT).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Input' && d.draft.candidate === '04:07'",
        )
        .await?;
    focused(tab, "time-dial-hour-input").await?;
    size(tab, "time-dial-hour-input", 96., 72.).await?;
    tab.press("Tab").await?;
    let minute = "time-dial-minute-input";
    focused(tab, minute).await?;
    let tree = ax_tree(tab).await?;
    ax(&tree, "dialog", "選擇時間")?;
    check_eq!(
        tree.iter()
            .filter(|node| node["ignored"] == false && node["role"]["value"] == "dialog")
            .count(),
        1,
        "switching preserves one active dialog"
    );
    tab.shortcut("a").await?;
    tab.ime_compose("５９", 2, 2).await?;
    let composing = format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true",
        js(minute)
    );
    tab.wait_for(&composing).await?;
    click(tab, USE_DIAL).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Input' && d.draft.candidate === '04:07' && d.commits === 0",
        )
        .await?;
    tab.wait_for(&composing).await?;
    focused(tab, minute).await?;
    capture(env, tab, "shared-ime-preedit").await?;
    tab.press("Enter").await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Input' && d.saved === '09:07' && d.commits === 0",
        )
        .await?;
    if state(tab).await?["time_dial"]["draft"]["raw_minute"] == "07" {
        tab.insert_text("５９").await?;
    }
    DIAL_SCENE
        .ready(tab, "d.draft.candidate === '04:59' && d.commits === 0")
        .await?;
    tab.wait_for(&format!("!({composing})")).await?;
    focused(tab, minute).await?;
    tab.press("Enter").await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Input' && d.draft.raw_minute === '59' && d.commits === 0",
        )
        .await?;
    activate(tab, USE_DIAL).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Dial' && d.draft.candidate === '04:59'",
        )
        .await?;
    activate(tab, "time-dial-confirm").await?;
    DIAL_SCENE.closed(tab, "04:59", 1).await?;
    focused(tab, DIAL_SCENE.opener).await?;
    Ok(
        json!({"mode_press_preserves_preedit":true,"enter_commits":0,
        "saved":"04:59","commits":1,"one_dialog":true}),
    )
}

fn draft_snapshot(data: &Value) -> Value {
    json!({"raw_hour":data["raw_hour"],"raw_minute":data["raw_minute"],"period":data["period"],
        "candidate":data["candidate"],"hour_error":data["hour_error"],"minute_error":data["minute_error"]})
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M159",
        "the dark 320 dp RTL Baseline editor retains the 200 percent draft and full actions",
    );
    for setting in ["theme", "rtl", "large-text", "locale"] {
        DIAL_SCENE.setting(tab, setting).await?;
    }
    until(tab, "s.dark === true").await?;
    resize(tab, 1120, 880).await?;
    DIAL_SCENE.open(tab).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.rtl && d.text_scale === 2 && d.locale === 'ja' && d.draft.mode === 'Dial'",
        )
        .await?;
    let wide_hour = bounds(tab, HOUR).await?;
    let wide_minute = bounds(tab, MINUTE).await?;
    check!(
        wide_hour[0] > wide_minute[0],
        "dark Japanese RTL mirrors the Hour and Minute dial selectors"
    );
    let (x, y) = tick(tab, 4., 12., 100.).await?;
    tab.click(x, y).await?;
    DIAL_SCENE
        .ready(tab, "d.draft.candidate === '04:07'")
        .await?;
    let draft = draft_snapshot(&state(tab).await?["time_dial"]["draft"]);
    capture(env, tab, "dial-wide-dark-rtl-large-text").await?;
    resize(tab, 320, 240).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.orientation === null && !d.draft.fits && d.draft.mode === 'Input' &&
             d.draft.candidate === '04:07' && d.commits === 0",
        )
        .await?;
    absent(tab, FACE).await?;
    focused(tab, "time-dial-hour-input").await?;
    check_eq!(
        draft_snapshot(&state(tab).await?["time_dial"]["draft"]),
        draft,
        "the compact Baseline editor retains every draft field"
    );
    let panel = shell(tab, DIAL, [0., 0., 320., 240.]).await?;
    let actions = [
        bounds(tab, "time-dial-confirm").await?,
        bounds(tab, "time-dial-cancel").await?,
    ];
    check!(
        actions
            .iter()
            .all(|target| contains(panel, *target) && target[2] >= 47.5 && target[3] >= 47.5)
            && !overlap(actions[0], actions[1]),
        "the compact localized footer keeps two full targets: {actions:?}"
    );
    tab.press("Tab").await?;
    focused(tab, "time-dial-minute-input").await?;
    let hour = bounds(tab, "time-dial-hour-input").await?;
    let minute = bounds(tab, "time-dial-minute-input").await?;
    let body = bounds(tab, "time-dial-body").await?;
    check!(
        minute[1] > hour[1] && near(hour[0], minute[0]) && contains(body, minute),
        "the large localized editors stack and focus reveals the full Minute: {hour:?}, {minute:?}"
    );
    let tree = ax_tree(tab).await?;
    ax(&tree, "dialog", "時刻を入力")?;
    ax(&tree, "textbox", "分")?;
    capture(env, tab, "compact-dark-rtl-input").await?;
    resize(tab, 1120, 880).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Input' && d.draft.fits && d.draft.candidate === '04:07'",
        )
        .await?;
    activate(tab, USE_DIAL).await?;
    DIAL_SCENE
        .ready(
            tab,
            "d.draft.mode === 'Dial' && d.draft.candidate === '04:07' && d.commits === 0",
        )
        .await?;
    check_eq!(
        draft_snapshot(&state(tab).await?["time_dial"]["draft"]),
        draft,
        "explicit return restores the same draft"
    );
    let face = size(tab, FACE, 256., 256.).await?;
    activate(tab, "time-dial-cancel").await?;
    DIAL_SCENE.closed(tab, "09:07", 0).await?;
    Ok(json!({"compact_panel":panel,"actions":actions,"retained":draft,"reopened_face":face}))
}

async fn dismissal(tab: &Tab) -> Result<Value> {
    step(
        "M160",
        "Baseline Input discards three ways and commits only on explicit Save",
    );
    resize(tab, 960, 820).await?;
    for how in ["cancel", "escape", "scrim"] {
        INPUT_SCENE.open(tab).await?;
        tab.shortcut("a").await?;
        tab.insert_text("10").await?;
        INPUT_SCENE
            .ready(
                tab,
                "d.modal.candidate === '10:07' && d.modal_committed === '09:07' && d.modal_commits === 0",
            )
            .await?;
        match how {
            "cancel" => activate(tab, "time-input-modal-cancel").await?,
            "escape" => tab.press("Escape").await?,
            _ => {
                let panel = bounds(tab, INPUT).await?;
                let outside = bounds(tab, INPUT_SCENE.outside).await?;
                check!(
                    !contains(panel, center(outside)),
                    "the pointer targets a real background action outside the modal"
                );
                click(tab, INPUT_SCENE.outside).await?;
            }
        }
        INPUT_SCENE.closed(tab, "09:07", 0).await?;
        if how != "scrim" {
            focused(tab, INPUT_SCENE.opener).await?;
        }
    }
    INPUT_SCENE.open(tab).await?;
    tab.shortcut("a").await?;
    tab.insert_text("11").await?;
    tab.press("Enter").await?;
    focused(tab, INPUT_SCENE.first).await?;
    INPUT_SCENE
        .ready(
            tab,
            "d.modal.candidate === '11:07' && d.modal_committed === '09:07' && d.modal_commits === 0",
        )
        .await?;
    activate(tab, "time-input-modal-confirm").await?;
    INPUT_SCENE.closed(tab, "11:07", 1).await?;
    focused(tab, INPUT_SCENE.opener).await?;
    Ok(json!({"saved":"11:07","commits":1,"outside_actions":0,
        "discarded":["cancel","escape","scrim"]}))
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    INPUT_SCENE.reload(tab, url).await?;
    let input = standalone_input(env, tab).await?;
    DIAL_SCENE.reload(tab, url).await?;
    let layouts = arrangements(env, tab).await?;
    DIAL_SCENE.reload(tab, url).await?;
    let pointer = portrait_input(env, tab).await?;
    DIAL_SCENE.reload(tab, url).await?;
    let composition = switch_and_save(env, tab).await?;
    DIAL_SCENE.reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    INPUT_SCENE.reload(tab, url).await?;
    let dismissal = dismissal(tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Baseline time picker page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-time-baseline.json",
        &json!({"browser":chrome.version,"input":input,"layouts":layouts,"pointer":pointer,
            "composition":composition,"compact":compact,"dismissal":dismissal,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1444, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-time-baseline-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-time-baseline-failure.png"))
            .await;
    }
    result.context("Material 3 Baseline time pickers")
}
