//! Checkable vertical menus through trusted Chrome input and Canvas pixels.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::material_time_input::{
    activate, ax, ax_property, ax_tree, bounds, contains, focus_projected, focused, near,
    projected, resize, Raster,
};
use super::{state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub(super) const STEPS: [&str; 3] = ["M373", "M374", "M375"];

const SHOW: &str = "menu-demo-choice-show";
const BACK: &str = "menu-demo-back";
const OPENER: &str = "menu-demo-opener";
const OUTSIDE: &str = "menu-demo-list";
const PANEL: &str = "menu-demo-panel";
const VIEWPORT: &str = "menu-demo-panel-viewport";
const CONTENT: &str = "menu-demo-panel-content";
const THUMB: &str = "menu-demo-panel-scroll-thumb";
const RECENT: &str = "menu-demo-choice-recent";
const NAME: &str = "menu-demo-choice-name";
const OWNER: &str = "menu-demo-choice-owner";
const COMFORTABLE: &str = "menu-demo-choice-comfortable";
const COMPACT: &str = "menu-demo-choice-compact";
const KIND: &str = "menu-demo-choice-kind";
const SIZE: &str = "menu-demo-choice-size";
const LOCKED: &str = "menu-demo-choice-locked";
const GROUPS: [(&str, &str); 3] = [
    ("menu-demo-choice-sort", "排序方式"),
    ("menu-demo-choice-density", "顯示密度"),
    ("menu-demo-choice-fields", "在清單中顯示"),
];

struct ChoiceRow {
    id: &'static str,
    name: &'static str,
    group: usize,
    disabled: bool,
}

const ROWS: [ChoiceRow; 8] = [
    ChoiceRow {
        id: RECENT,
        name: "最近修改",
        group: 0,
        disabled: false,
    },
    ChoiceRow {
        id: NAME,
        name: "名稱",
        group: 0,
        disabled: false,
    },
    ChoiceRow {
        id: OWNER,
        name: "擁有者",
        group: 0,
        disabled: true,
    },
    ChoiceRow {
        id: COMFORTABLE,
        name: "舒適",
        group: 1,
        disabled: false,
    },
    ChoiceRow {
        id: COMPACT,
        name: "緊湊",
        group: 1,
        disabled: false,
    },
    ChoiceRow {
        id: KIND,
        name: "檔案類型",
        group: 2,
        disabled: false,
    },
    ChoiceRow {
        id: SIZE,
        name: "檔案大小與可用空間的詳細資料",
        group: 2,
        disabled: false,
    },
    ChoiceRow {
        id: LOCKED,
        name: "鎖定狀態",
        group: 2,
        disabled: true,
    },
];

#[derive(Clone, Copy)]
struct Selection {
    sort: &'static str,
    density: &'static str,
    checked: &'static [&'static str],
    changes: u64,
}

const INITIAL: Selection = Selection {
    sort: RECENT,
    density: COMFORTABLE,
    checked: &[KIND, LOCKED],
    changes: 0,
};
const SORTED: Selection = Selection {
    sort: NAME,
    changes: 1,
    ..INITIAL
};
const DENSE: Selection = Selection {
    density: COMPACT,
    changes: 2,
    ..SORTED
};
const SIZE_ON: Selection = Selection {
    checked: &[KIND, SIZE, LOCKED],
    changes: 3,
    ..DENSE
};
const KIND_OFF: Selection = Selection {
    checked: &[SIZE, LOCKED],
    changes: 4,
    ..SIZE_ON
};
const KIND_BACK: Selection = Selection {
    changes: 5,
    ..SIZE_ON
};
const SIZE_OFF: Selection = Selection {
    checked: &[KIND, LOCKED],
    changes: 6,
    ..KIND_BACK
};

impl Selection {
    fn includes(self, row: &ChoiceRow) -> bool {
        match row.group {
            0 => self.sort == row.id,
            1 => self.density == row.id,
            _ => self.checked.contains(&row.id),
        }
    }
}

type Rect = [f64; 4];
type Rgb = [u8; 3];

async fn route(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2400).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, SHOW).await?;
    until(tab, "s.menu_demo.visible && s.menu_demo.choice !== null").await?;
    focused(tab, BACK).await?;
    resize(tab, 960, 1120).await
}

async fn open(tab: &Tab) -> Result<()> {
    activate(tab, OPENER).await?;
    until(tab, "s.menu_demo.open").await?;
    focused(tab, RECENT).await?;
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-expanded')",
            projected(OPENER)
        ))
        .await?,
        "true",
        "the opener reports expansion"
    );
    Ok(())
}

async fn closed(tab: &Tab, restore: &str) -> Result<()> {
    until(tab, "!s.menu_demo.open").await?;
    focused(tab, restore).await?;
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-expanded')",
            projected(OPENER)
        ))
        .await?,
        "false",
        "the opener reports collapse"
    );
    check_eq!(
        tab.eval(&format!("{} === null", projected(PANEL))).await?,
        json!(true),
        "the closed menu leaves the accessibility tree"
    );
    Ok(())
}

async fn expect(tab: &Tab, expected: Selection, open: bool) -> Result<Value> {
    let keys = json!(expected.checked);
    until(tab, &format!(
        "(() => {{ const m = s.menu_demo, c = m.choice, expected = {keys};
             return m.open === {open} && c?.sort === {} && c.density === {} && c.changes === {}
                 && c.checked.length === expected.length && expected.every(id => c.checked.includes(id)); }})()",
        js(expected.sort), js(expected.density), expected.changes,
    )).await?;
    tab.settle().await?;
    let inspected = state(tab).await?;
    let menu = &inspected["menu_demo"];
    let choice = &menu["choice"];
    let mut checks: Vec<String> = serde_json::from_value(choice["checked"].clone())?;
    checks.sort_unstable();
    let mut expected_checks = expected.checked.to_vec();
    expected_checks.sort_unstable();
    let mut receipt = json!({"open":menu["open"],"sort":choice["sort"],"density":choice["density"],
                            "checked":checks,"changes":choice["changes"]});
    check_eq!(
        receipt,
        json!({"open":open,"sort":expected.sort,"density":expected.density,
                             "checked":expected_checks,"changes":expected.changes}),
        "the model committed only the requested choice"
    );
    receipt["focus"] = inspected["focused"].clone();
    Ok(receipt)
}

fn ax_owns(tree: &[Value], parent: &Value, child: &Value) -> bool {
    let mut ancestor_id = child["parentId"].as_str();
    while let Some(id) = ancestor_id {
        if parent["nodeId"] == id {
            return true;
        }
        ancestor_id = tree
            .iter()
            .find(|node| node["nodeId"] == id)
            .and_then(|node| node["parentId"].as_str());
    }
    false
}

async fn semantics(tab: &Tab, expected: Selection) -> Result<Vec<Value>> {
    let tree = ax_tree(tab).await?;
    let menu_name = tab
        .eval_str(&format!("{}?.getAttribute('aria-label')", projected(PANEL)))
        .await?;
    check!(
        !menu_name.trim().is_empty(),
        "the choice menu has an accessible name"
    );
    let menu = ax(&tree, "menu", &menu_name)?;
    let mut rows = Vec::new();
    for row in &ROWS {
        let (group_id, group_name) = GROUPS[row.group];
        let role = if row.group == 2 {
            "menuitemcheckbox"
        } else {
            "menuitemradio"
        };
        let group = ax(&tree, "group", group_name)?;
        let child = ax(&tree, role, row.name)?;
        check!(
            ax_owns(&tree, menu, group) && ax_owns(&tree, group, child),
            "Chrome AX keeps {} inside its named group",
            row.name
        );
        let checked = expected.includes(row).to_string();
        check_eq!(
            ax_property(child, "checked"),
            Some(&json!(checked)),
            "Chrome AX exposes the current checked value"
        );
        check_eq!(
            ax_property(child, "disabled") == Some(&json!(true)),
            row.disabled,
            "Chrome AX exposes the authored enabled or disabled state"
        );
        let dom = tab
            .eval(&format!(
                "(() => {{ const e = {}, group = {};
                 return [e?.getAttribute('role'), e?.getAttribute('aria-checked'),
                     e?.getAttribute('aria-selected'), e?.getAttribute('aria-disabled') === 'true',
                     Boolean(group?.contains(e)),
                     e?.querySelector('[tabindex], button, input, [aria-checked]') === null,
                     {} === null]; }})()",
                projected(row.id),
                projected(group_id),
                projected(&format!("{}-leading", row.id)),
            ))
            .await?;
        check_eq!(
            dom,
            json!([role, checked, null, row.disabled, true, true, true]),
            "one projected choice action with a passive vector mark"
        );
        rows.push(
            json!({"id":row.id,"name":row.name,"role":role,"group":group_id,
                         "group_name":group_name,"checked":checked,"disabled":row.disabled,"dom":dom}),
        );
    }
    Ok(rows)
}

fn rgb(value: &Value) -> Result<Rgb> {
    let [r, g, b, alpha]: [u8; 4] =
        serde_json::from_value(value.clone()).context("the fixture exposes its RGBA token")?;
    check_eq!(alpha, 255, "the enabled choice paint token is opaque");
    Ok([r, g, b])
}

fn luminance(color: Rgb) -> f64 {
    color
        .into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(channel, weight)| {
            let s = f64::from(channel) / 255.;
            weight
                * if s <= 0.04045 {
                    s / 12.92
                } else {
                    ((s + 0.055) / 1.055).powf(2.4)
                }
        })
        .sum()
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

async fn radii(tab: &Tab, id: &str) -> Result<Rect> {
    let corners = tab.eval(&format!(
        "(() => {{ const s = JSON.parse(window.gallery.export_dom()).dom.nodes
              .find(n => n.stable_id === {})?.style;
              return ['top_left','top_right','bottom_right','bottom_left'].map(k => s?.corners?.[k] ?? s?.radius); }})()",
        js(format!("{id}-face")),
    )).await?;
    serde_json::from_value(corners).context("read all four actual menu face corners")
}

async fn paint(
    env: &Env,
    tab: &Tab,
    tag: &str,
    id: &str,
    unchecked: Option<&str>,
    rtl: bool,
) -> Result<Value> {
    let [left, top, width, _] = tab.canvas_box().await?;
    tab.mouse_move(left + width - 2., top + 2.).await?;
    tab.settle().await?;
    let owner = bounds(tab, id).await?;
    let face = bounds(tab, &format!("{id}-face")).await?;
    let mark = bounds(tab, &format!("{id}-leading")).await?;
    let label = bounds(tab, &format!("{id}-label")).await?;
    let corners = radii(tab, id).await?;
    check_eq!(corners, [12.; 4], "selection rounds every corner to 12 dp");
    check!(
        owner[2] >= 48. && owner[3] >= 48. && face[3] >= 44. && contains(owner, face),
        "the choice face fits inside its minimum 48 dp target"
    );
    check!(
        near(mark[2], 20.) && near(mark[3], 20.) && contains(face, mark) && contains(face, label),
        "a 20 dp vector and the complete label fit on the selected face"
    );
    let gap = if rtl {
        mark[0] - label[0] - label[2]
    } else {
        label[0] - mark[0] - mark[2]
    };
    check!(
        near(gap, 12.),
        "paint uses a 12 dp gap and a logical leading mark: {gap}"
    );
    let menu = state(tab).await?["menu_demo"].clone();
    let selected = rgb(&menu["choice"]["palette"][0])?;
    let ink = rgb(&menu["choice"]["palette"][1])?;
    let group = rgb(&menu["palette"][0])?;
    let path = env.artifact(&format!("material-choice-menu-{tag}.png"));
    tab.screenshot(&path).await?;
    let raster = Raster::read(tab, path).await?;
    let [x, y, w, _] = face;
    let fill = raster.sample(x + w / 2., y + 4.)?;
    // The compact scrollbar can cover the logical trailing corner.
    let leading_x = |inset| if rtl { x + w - inset } else { x + inset };
    let corner = raster.sample(leading_x(1.), y + 1.)?;
    let inner = raster.sample(leading_x(6.), y + 3.)?;
    check_eq!(
        fill,
        selected,
        "the real selected face paints its palette token"
    );
    check_eq!(
        corner,
        group,
        "the selected corner leaves the actual group paint visible"
    );
    check_eq!(
        inner,
        selected,
        "the inside of the rounded corner is painted"
    );
    let mark_pixels = raster.colors(mark)?.get(&ink).copied().unwrap_or(0);
    check!(
        mark_pixels > 8 && contrast(fill, ink) >= 3.,
        "the selected vector is visibly distinct from the face"
    );
    let unchecked = if let Some(id) = unchecked {
        let face = bounds(tab, &format!("{id}-face")).await?;
        let color = raster.sample(face[0] + face[2] / 2., face[1] + 4.)?;
        let corners = radii(tab, id).await?;
        check_eq!(
            color,
            group,
            "an unselected face keeps the real group paint"
        );
        check!(
            corners != [12.; 4],
            "unchecked choice keeps the positional corner shape"
        );
        Some(json!({"id":id,"face":face,"color":color,"corners":corners}))
    } else {
        None
    };
    Ok(
        json!({"id":id,"target":owner,"face":face,"mark":mark,"label":label,"mark_gap":gap,
              "corners":corners,"fill":fill,"corner_rgb":corner,"inner_rgb":inner,"ink":ink,
              "group_rgb":group,"mark_pixels":mark_pixels,"unchecked":unchecked,
              "tokens":{"selected_face":selected,"selected_ink":ink,"group":group},
              "face_contrast":contrast(fill,group),"mark_contrast":contrast(fill,ink)}),
    )
}

async fn held_space(tab: &Tab, expected: Selection) -> Result<Value> {
    let key = crate::keys::key("Space")?;
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
        expect(tab, expected, true).await?;
    }
    tab.cdp(
        "Input.dispatchKeyEvent",
        json!({
            "type":"keyUp","key":key.key,"code":key.code,"windowsVirtualKeyCode":key.key_code,
            "modifiers":0,"location":key.location,
        }),
    )
    .await?;
    expect(tab, expected, true).await
}

async fn standard(env: &Env, tab: &Tab) -> Result<Value> {
    open(tab).await?;
    let initial = expect(tab, INITIAL, true).await?;
    let initial_roles = semantics(tab, INITIAL).await?;
    let original_name_corners = radii(tab, NAME).await?;
    let reselect = held_space(tab, INITIAL).await?;
    focused(tab, RECENT).await?;
    tab.press("ArrowDown").await?;
    focused(tab, NAME).await?;
    tab.press("ArrowDown").await?;
    focused(tab, OWNER).await?;
    tab.press("Enter").await?;
    focused(tab, OWNER).await?;
    let disabled = expect(tab, INITIAL, true).await?;
    let (x, y) = point(tab, OWNER).await?;
    tab.click(x, y).await?;
    let disabled_mouse = expect(tab, INITIAL, true).await?;
    check_eq!(
        disabled_mouse["focus"],
        OWNER,
        "the disabled click preserves row focus"
    );
    focus_projected(tab, OWNER).await?;
    tab.press("ArrowUp").await?;
    focused(tab, NAME).await?;
    tab.press("Enter").await?;
    closed(tab, OPENER).await?;
    let accepted = expect(tab, SORTED, false).await?;
    open(tab).await?;
    let reopened = expect(tab, SORTED, true).await?;
    let selected_roles = semantics(tab, SORTED).await?;
    let pixels = paint(env, tab, "standard", NAME, Some(COMPACT), false).await?;
    check!(
        original_name_corners != [12.; 4],
        "the formerly unselected row had a different shape"
    );
    check_eq!(
        pixels["fill"],
        json!([255, 217, 227]),
        "the light Standard selection uses TertiaryContainer"
    );
    tab.press("Escape").await?;
    closed(tab, OPENER).await?;
    expect(tab, SORTED, false).await?;
    Ok(
        json!({"initial":initial,"initial_roles":initial_roles,"reselect":reselect,"disabled":disabled,
              "disabled_mouse":disabled_mouse,
              "accepted":accepted,"reopened":reopened,"selected_roles":selected_roles,
              "initial_name_corners":original_name_corners,"paint":pixels}),
    )
}

async fn point(tab: &Tab, id: &str) -> Result<(f64, f64)> {
    let [x, y, w, h] = bounds(tab, id).await?;
    let [left, top, _, _] = tab.canvas_box().await?;
    Ok((left + x + w / 2., top + y + h / 2.))
}

async fn touch(tab: &Tab, id: &str, terminal: &str) -> Result<()> {
    let (x, y) = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":x,"y":y,"id":1}]}),
    )
    .await?;
    tab.settle().await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":terminal,"touchPoints":[]}),
    )
    .await?;
    tab.settle().await
}

async fn vibrant(env: &Env, tab: &Tab) -> Result<Value> {
    activate(tab, "menu-demo-vibrant").await?;
    until(tab, "s.menu_demo.vibrant").await?;
    open(tab).await?;
    for id in [NAME, OWNER, COMFORTABLE, COMPACT] {
        tab.press("ArrowDown").await?;
        focused(tab, id).await?;
    }
    tab.press("Enter").await?;
    closed(tab, OPENER).await?;
    let density = expect(tab, DENSE, false).await?;
    open(tab).await?;
    tab.press("End").await?;
    focused(tab, LOCKED).await?;
    tab.press("Space").await?;
    focused(tab, LOCKED).await?;
    let disabled = expect(tab, DENSE, true).await?;
    tab.press("ArrowUp").await?;
    focused(tab, SIZE).await?;
    let keyboard = held_space(tab, SIZE_ON).await?;
    focused(tab, SIZE).await?;

    let (x, y) = point(tab, &format!("{KIND}-leading")).await?;
    tab.click(x, y).await?;
    let mouse = expect(tab, KIND_OFF, true).await?;
    check_eq!(
        mouse["focus"],
        KIND,
        "the mouse action stays on the original row"
    );
    check_eq!(
        tab.eval("document.activeElement === document.querySelector('canvas')")
            .await?,
        json!(true),
        "the browser keeps real Canvas focus after the mouse action"
    );
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":1}),
    )
    .await?;
    tab.eval(
        "(() => { window.choiceMenuTouches = [];
        for (const type of ['pointerdown','pointercancel','pointerup']) {
            document.querySelector('canvas').addEventListener(type, e => {
                if (e.pointerType === 'touch') window.choiceMenuTouches.push([type,e.isTrusted]);
            }, true);
        } })()",
    )
    .await?;
    touch(tab, KIND, "touchCancel").await?;
    let cancelled = expect(tab, KIND_OFF, true).await?;
    touch(tab, KIND, "touchEnd").await?;
    let tapped = expect(tab, KIND_BACK, true).await?;
    let touch_events = tab.eval("window.choiceMenuTouches").await?;
    check_eq!(
        touch_events,
        json!([
            ["pointerdown", true],
            ["pointercancel", true],
            ["pointerdown", true],
            ["pointerup", true]
        ]),
        "the Canvas received a trusted cancelled touch and a trusted tap"
    );
    tab.cdp(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":false}),
    )
    .await?;

    focus_projected(tab, KIND).await?;
    let roles = semantics(tab, KIND_BACK).await?;
    let pixels = paint(env, tab, "vibrant", SIZE, Some(COMFORTABLE), false).await?;
    check!(
        pixels["fill"] != json!([255, 217, 227])
            && pixels["face_contrast"].as_f64().is_some_and(|v| v >= 3.),
        "light Vibrant distinguishes its selected face from TertiaryContainer"
    );
    tab.press("Tab").await?;
    closed(tab, OUTSIDE).await?;
    let tabbed = expect(tab, KIND_BACK, false).await?;
    Ok(
        json!({"density":density,"disabled":disabled,"keyboard":keyboard,"mouse":mouse,
              "cancelled":cancelled,"tapped":tapped,"touch_events":touch_events,
              "roles":roles,"paint":pixels,"tabbed":tabbed}),
    )
}

async fn scroll(tab: &Tab) -> Result<f64> {
    tab.eval(&format!(
        "JSON.parse(window.gallery.export_dom()).dom.nodes
        .find(n => n.stable_id === {})?.style.scroll_y",
        js(VIEWPORT)
    ))
    .await?
    .as_f64()
    .context("read the measured menu scroller offset")
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    for id in [
        "menu-demo-theme",
        "menu-demo-rtl",
        "menu-demo-large-text",
        "menu-demo-bottom",
    ] {
        activate(tab, id).await?;
    }
    until(tab, "s.dark && s.menu_demo.vibrant && s.menu_demo.rtl && s.menu_demo.text_scale === 2 && s.menu_demo.bottom_anchor").await?;
    open(tab).await?;
    resize(tab, 320, 480).await?;
    focused(tab, RECENT).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({THUMB:?})?.length === 4"
    ))
    .await?;
    let panel = bounds(tab, PANEL).await?;
    let viewport = bounds(tab, VIEWPORT).await?;
    let content = bounds(tab, CONTENT).await?;
    let anchor = bounds(tab, OPENER).await?;
    let top = bounds(tab, THUMB).await?;
    let before = scroll(tab).await?;
    check!(
        contains([8., 8., 304., 464.], panel)
            && contains(panel, viewport)
            && near(panel[1] + panel[3] + 4., anchor[1])
            && near(panel[0] + panel[2], anchor[0] + anchor[2]),
        "the compact RTL popup aligns to its opener and flips above it"
    );
    tab.press("End").await?;
    focused(tab, LOCKED).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({THUMB:?})?.[1] > {}",
        top[1]
    ))
    .await?;
    let bottom = bounds(tab, THUMB).await?;
    let after = scroll(tab).await?;
    let last = bounds(tab, LOCKED).await?;
    let long = bounds(tab, SIZE).await?;
    let label = bounds(tab, &format!("{SIZE}-label")).await?;
    check!(
        after > before && contains(viewport, last) && contains(viewport, long),
        "End reveals the full last choice and its preceding wrapped option"
    );
    check!(
        long[3] > 48. && label[3] > 40. && contains(long, label),
        "the long label uses more than one line at 200 percent"
    );
    check!(
        contains(panel, top)
            && contains(panel, bottom)
            && top[0] < panel[0] + panel[2] / 2.
            && near(top[0], bottom[0])
            && near(top[2], bottom[2])
            && bottom[1] > top[1],
        "the scroll thumb moves inside the left logical trailing edge"
    );
    check_eq!(
        bounds(tab, OPENER).await?,
        anchor,
        "the footer opener stays fixed during menu scrolling"
    );
    let pixels = paint(env, tab, "compact-dark-rtl", SIZE, None, true).await?;
    check!(
        pixels["face_contrast"].as_f64().is_some_and(|v| v >= 3.),
        "dark Vibrant faces retain their contrast"
    );
    tab.press("ArrowUp").await?;
    focused(tab, SIZE).await?;
    let before_toggle = bounds(tab, SIZE).await?;
    let label_before = bounds(tab, &format!("{SIZE}-label")).await?;
    let full_name = ROWS[6].name;
    let tree = ax_tree(tab).await?;
    check_eq!(
        ax_property(ax(&tree, "menuitemcheckbox", full_name)?, "checked"),
        Some(&json!("true")),
        "the full wrapped name is exposed with its selected state"
    );
    tab.press("Enter").await?;
    focused(tab, SIZE).await?;
    let toggled = expect(tab, SIZE_OFF, true).await?;
    check_eq!(
        tab.eval(&format!(
            "{}?.getAttribute('aria-checked')",
            projected(SIZE)
        ))
        .await?,
        "false",
        "the same compact choice publishes its unchecked state"
    );
    let tree = ax_tree(tab).await?;
    check_eq!(
        ax_property(ax(&tree, "menuitemcheckbox", full_name)?, "checked"),
        Some(&json!("false")),
        "the accessibility tree keeps the full name after toggling"
    );
    check_eq!(
        bounds(tab, SIZE).await?,
        before_toggle,
        "toggling keeps the wrapped row bounds stable"
    );
    check_eq!(
        bounds(tab, &format!("{SIZE}-label")).await?,
        label_before,
        "toggling keeps the wrapped label bounds stable"
    );
    tab.press("Escape").await?;
    closed(tab, OPENER).await?;
    let dismissed = expect(tab, SIZE_OFF, false).await?;
    Ok(
        json!({"panel":panel,"viewport":viewport,"content":content,"anchor":anchor,
              "thumb_top":top,"thumb_bottom":bottom,"scroll":[before,after],"last":last,
              "full_name":full_name,"long":long,"label":label,"paint":pixels,
              "toggled":toggled,"dismissed":dismissed}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2444, 2.).await?;
    let mut report = json!({"browser":chrome.version});
    let result: Result<()> = async {
        step(STEPS[0], "Standard checkable menu exposes named radio and checkbox groups and selects once");
        route(&tab, url).await.context("M373 route")?;
        report["adapter"] = tab.eval("(async () => { const a = await navigator.gpu?.requestAdapter();
           if (!a) return null; const {isFallbackAdapter,vendor,architecture} = a.info;
           return {isFallbackAdapter,vendor,architecture}; })()").await?;
        check!(report["adapter"]["isFallbackAdapter"].is_boolean(), "WebGPU exposes actual adapter metadata");
        report["standard"] = standard(env, &tab).await.context("M373")?;
        step(STEPS[1], "Vibrant menu toggles independent choices by key, mouse and real touch and stays open");
        report["vibrant"] = vibrant(env, &tab).await.context("M374")?;
        step(STEPS[2], "short dark RTL checkable menu reveals a wrapped selected row and keeps state on Escape");
        report["compact"] = compact(env, &tab).await.context("M375")?;
        check!(tab.errors().is_empty(), "no page errors: {:?}", tab.errors());
        Ok(())
    }.await;
    report["errors"] = json!(tab.errors());
    if let Err(error) = &result {
        report["failure"] = json!(format!("{error:#}"));
        report["state_at_failure"] = state(&tab).await.unwrap_or(Value::Null);
        let _ = tab
            .screenshot(&env.artifact("material-choice-menu-failure.png"))
            .await;
    }
    env.write_json("browser-material-choice-menu.json", &report)?;
    result
}
