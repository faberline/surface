//! Material 3 inline list expansion on the real Chrome WebGPU host.

use anyhow::{Context, Result};
use serde::Serialize;
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

const LIST: &str = "list-expand-items";
const SCROLL: &str = "list-expand-scroll";
const BEFORE: &str = "list-expand-demo-before";
const AFTER: &str = "list-expand-demo-after";
const PREVIOUS: &str = "list-expand-before";
const NEXT: &str = "list-expand-after";
const HEADER: &str = "list-expand-header";
const BRANCH: &str = "list-expand-header-branch";
const BODY: &str = "list-expand-header-expansion-content";
const INDICATOR: &str = "list-expand-header-indicator";
const CHILD_A: &str = "list-expand-child-a";
const CHILD_B: &str = "list-expand-child-b";
const DISABLED: &str = "list-expand-child-disabled";
const HEADER_NAME: &str = "旅行行程";
const CHILD_A_NAME: &str = "步道地圖與無障礙路線";
const CHILD_B_NAME: &str = "下載地圖";
const DISABLED_NAME: &str = "暫停行程";

type Rect = [f64; 4];

async fn data(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["list_expand"].take())
}

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => {condition})(s.list_expand)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 1744).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 1700).await?;
    activate(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    activate(tab, "list-expand-demo-show").await?;
    ready(
        tab,
        "d.visible && d.variant === 'segmented' && !d.expanded && !d.rtl &&
         d.text_scale === 1 && d.toggles === 0 && d.activations === 0 && d.last === null",
    )
    .await?;
    resize(tab, 960, 820).await
}

async fn preset(tab: &Tab, choice: &str) -> Result<()> {
    activate(tab, &format!("list-expand-demo-{choice}")).await
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-list-expansion-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

async fn follow(tab: &Tab, key: &str, expected: &str) -> Result<()> {
    tab.press(key).await?;
    focused(tab, expected).await
}

async fn enter_header(tab: &Tab) -> Result<()> {
    focus_projected(tab, BEFORE).await?;
    follow(tab, "Tab", PREVIOUS).await?;
    follow(tab, "ArrowDown", HEADER).await
}

async fn command(tab: &Tab, count: u8, id: &str) -> Result<()> {
    ready(
        tab,
        &format!("d.activations === {count} && d.last === {}", js(id)),
    )
    .await
}

async fn disclosure(tab: &Tab, expanded: bool) -> Result<Value> {
    let header = projected(HEADER);
    let branch = projected(BRANCH);
    let indicator = projected(INDICATOR);
    let list = projected(LIST);
    let attributes = tab.eval(&format!(
        "(() => {{
          const h={header}, b={branch}, i={indicator}, l={list};
          return {{
            name:h?.getAttribute('aria-label'), role:h?.getAttribute('role'),
            expanded:h?.getAttribute('aria-expanded'), action:h?.getAttribute('data-surface-action'),
            named_tab:h?.getAttribute('tabindex'), list:l?.getAttribute('role'),
            branch_role:b?.getAttribute('role'), branch_action:b?.getAttribute('data-surface-action'),
            indicator_action:i?.getAttribute('data-surface-action') ?? null,
            indicator_tab:i?.getAttribute('tabindex') ?? null,
            inside_branch:!!b?.contains(h), branch_in_list:!!l?.contains(b),
          }};
        }})()"
    )).await?;
    let expected = expanded.to_string();
    check!(
        attributes["name"] == HEADER_NAME
            && attributes["role"] == "button"
            && attributes["expanded"].as_str() == Some(expected.as_str())
            && attributes["action"] == "press"
            && attributes["named_tab"] == "-1"
            && attributes["list"] == "list"
            && attributes["branch_role"] == "listitem"
            && attributes["branch_action"].is_null()
            && attributes["indicator_action"].is_null()
            && attributes["indicator_tab"].is_null()
            && attributes["inside_branch"] == true
            && attributes["branch_in_list"] == true,
        "the named disclosure is the only header control: {attributes}"
    );
    let tree = ax_tree(tab).await?;
    ax(&tree, "list", "旅行清單")?;
    let header = ax(&tree, "button", HEADER_NAME)?;
    let ax_expanded = ax_property(header, "expanded");
    check!(
        ax_expanded == Some(&json!(expanded))
            || ax_expanded.and_then(Value::as_str) == Some(expected.as_str()),
        "Chrome reports expanded={expanded} for the disclosure button: {header}"
    );
    Ok(json!({"dom":attributes,"chrome_expanded":ax_expanded}))
}

async fn child(tab: &Tab, id: &str, name: &str, disabled: bool) -> Result<Value> {
    let proxy = projected(id);
    let attributes = tab
        .eval(&format!(
            "(() => {{
          const node={proxy};
          return node && {{
            role:node.getAttribute('role'), name:node.getAttribute('aria-label'),
            action:node.getAttribute('data-surface-action'), tab:node.getAttribute('tabindex'),
            disabled:node.getAttribute('aria-disabled'),
            inside_branch:{}?.contains(node),
            nested_in_button:!!node.parentElement?.closest('[role=button]'),
          }};
        }})()",
            projected(BRANCH),
        ))
        .await?;
    check!(
        attributes["name"] == name
            && attributes["role"] == "button"
            && attributes["action"].as_str() == (!disabled).then_some("press")
            && attributes["tab"].as_str() == (!disabled).then_some("-1")
            && attributes["disabled"].as_str() == disabled.then_some("true")
            && attributes["inside_branch"] == true
            && attributes["nested_in_button"] == false,
        "{id} has its own semantic action and disabled state: {attributes}"
    );
    let tree = ax_tree(tab).await?;
    let child = ax(&tree, "button", name)?;
    if disabled {
        check_eq!(
            ax_property(child, "disabled"),
            Some(&json!(true)),
            "Chrome exposes the disabled child"
        );
    }
    Ok(attributes)
}

async fn suppressed(tab: &Tab) -> Result<()> {
    let tree = ax_tree(tab).await?;
    for (id, name) in [
        (CHILD_A, CHILD_A_NAME),
        (DISABLED, DISABLED_NAME),
        (CHILD_B, CHILD_B_NAME),
    ] {
        check_eq!(
            tab.eval(&format!("{} === null", projected(id))).await?,
            json!(true),
            format!("{id} has no projected action while the body is suppressed")
        );
        check!(
            ax(&tree, "button", name).is_err(),
            "Chrome hides the suppressed child {name}"
        );
    }
    Ok(())
}

async fn mounted(tab: &Tab, id: &str, exists: bool) -> Result<()> {
    check_eq!(
        tab.eval(&format!(
            "window.gallery.control_bounds({}).length === 4",
            js(id)
        ))
        .await?,
        json!(exists),
        format!("paint geometry for {id} is mounted={exists}")
    );
    Ok(())
}

#[derive(Debug, Serialize)]
struct Pose {
    header: Rect,
    branch: Rect,
    next: Rect,
}

impl Pose {
    async fn read(tab: &Tab) -> Result<Self> {
        Ok(Self {
            header: bounds(tab, HEADER).await?,
            branch: bounds(tab, BRANCH).await?,
            next: bounds(tab, NEXT).await?,
        })
    }

    fn extent(&self) -> f64 {
        self.branch[3] - self.header[3]
    }

    fn starts_like(&self, other: &Self) -> bool {
        near(self.extent(), other.extent())
            && near(
                self.next[1] - self.branch[1],
                other.next[1] - other.branch[1],
            )
    }

    fn follows(&self, closed: &Self) -> Result<()> {
        let next_motion = (self.next[1] - self.branch[1]) - (closed.next[1] - closed.branch[1]);
        let height_motion = self.branch[3] - closed.branch[3];
        check!(
            near(next_motion, height_motion) && near(closed.header[3], self.header[3]),
            "the following row moves with the inline branch; the header height stays fixed: {self:?} from {closed:?}"
        );
        Ok(())
    }
}

async fn freeze(tab: &Tab) -> Result<()> {
    tab.eval(
        "(() => {
            const start = performance.now();
            window.listExpansionClock = 0;
            Object.defineProperty(performance, 'now', {
                configurable: true, value: () => start + window.listExpansionClock,
            });
        })()",
    )
    .await?;
    tab.settle().await
}

async fn tick(tab: &Tab, milliseconds: u16) -> Result<()> {
    tab.eval(&format!("window.listExpansionClock = {milliseconds}"))
        .await?;
    tab.settle().await
}

async fn reduced_motion(tab: &Tab, reduced: bool) -> Result<()> {
    let value = if reduced { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[{"name":"prefers-reduced-motion","value":value}]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {value})"))
    ))
    .await?;
    tab.settle().await
}

async fn partial_body_point(tab: &Tab, pose: &Pose) -> Result<(f64, f64)> {
    let canvas = tab.canvas_box().await?;
    let child = bounds(tab, CHILD_A).await?;
    let clip = bounds(tab, SCROLL).await?;
    let clipped_bottom = (pose.branch[1] + pose.branch[3]).min(clip[1] + clip[3]);
    let child_y = child[1].max(clip[1]);
    let exposed = clipped_bottom.min(child[1] + child[3]) - child_y;
    check!(
        exposed > 8.,
        "a real part of the first child is painted inside the animated clip: {exposed}"
    );
    Ok((
        canvas[0] + child[0] + 0.5 * child[2],
        canvas[1] + child_y + 0.5 * exposed,
    ))
}

async fn closed_scope(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M178",
        "a closed named disclosure stays in ordinary list flow and exposes no child controls",
    );
    let closed = disclosure(tab, false).await?;
    suppressed(tab).await?;
    mounted(tab, BODY, false).await?;
    let initial = Pose::read(tab).await?;
    let indicator = bounds(tab, INDICATOR).await?;
    check!(
        near(initial.extent(), 0.),
        "the closed branch occupies just its header height: {initial:?}"
    );
    check!(
        contains(initial.header, indicator)
            && near(indicator[2], 32.)
            && near(indicator[3], 32.)
            && indicator[0] > initial.header[0] + 0.5 * initial.header[2],
        "the 32 dp decorative indicator stays at the trailing edge in LTR: {indicator:?}"
    );
    focus_projected(tab, BEFORE).await?;
    follow(tab, "Tab", PREVIOUS).await?;
    tab.press("Enter").await?;
    command(tab, 1, PREVIOUS).await?;
    follow(tab, "ArrowDown", HEADER).await?;
    follow(tab, "ArrowDown", NEXT).await?;
    tab.press("Space").await?;
    command(tab, 2, NEXT).await?;
    follow(tab, "Tab", AFTER).await?;
    follow(tab, "Shift+Tab", PREVIOUS).await?;
    follow(tab, "ArrowDown", HEADER).await?;
    ready(tab, "!d.expanded && d.toggles === 0").await?;
    capture(env, tab, "collapsed-light").await?;
    Ok(
        json!({"disclosure":closed,"pose":initial,"indicator":indicator,"ordinary_actions":[PREVIOUS,NEXT]}),
    )
}

async fn moving(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M179",
        "the branch and following row move together and reverse without a jump",
    );
    reduced_motion(tab, false).await?;
    enter_header(tab).await?;
    let initial = Pose::read(tab).await?;
    freeze(tab).await?;
    tab.press("Space").await?;
    ready(tab, "d.expanded && d.toggles === 1 && d.activations === 0").await?;
    focused(tab, HEADER).await?;
    let opening_disclosure = disclosure(tab, true).await?;
    suppressed(tab).await?;
    let first = Pose::read(tab).await?;
    check!(
        first.starts_like(&initial),
        "opening starts at the collapsed pose: {first:?}"
    );
    tick(tab, 48).await?;
    let opening = Pose::read(tab).await?;
    let natural = bounds(tab, BODY).await?;
    check!(
        opening.extent() > 8. && opening.extent() < natural[3] - 8.,
        "there is a distinct partial inline body: {opening:?}, natural {natural:?}"
    );
    opening.follows(&initial)?;
    suppressed(tab).await?;
    capture(env, tab, "opening-midpoint-light").await?;
    let (x, y) = partial_body_point(tab, &opening).await?;
    tab.click(x, y).await?;
    ready(tab, "d.expanded && d.toggles === 1 && d.activations === 0").await?;
    tab.mouse_move(x, y).await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("browser device pixel ratio")?;
    // Winit reports this pixel delta in physical units. The layout uses dp.
    tab.wheel(0., 24. * scale).await?;
    tab.wait_for(&format!(
        "window.gallery.control_bounds({})[1] < {}",
        js(HEADER),
        opening.header[1] - 1.,
    ))
    .await?;
    let scrolled = Pose::read(tab).await?;
    check!(
        near(scrolled.header[1], opening.header[1] - 24.)
            && near(scrolled.next[1], opening.next[1] - 24.)
            && scrolled.starts_like(&opening),
        "a wheel over the partial paint moves the whole list by 24 dp: {opening:?} → {scrolled:?}"
    );
    focus_projected(tab, HEADER).await?;
    follow(tab, "ArrowDown", NEXT).await?;
    follow(tab, "ArrowUp", HEADER).await?;

    tab.press("Enter").await?;
    ready(tab, "!d.expanded && d.toggles === 2 && d.activations === 0").await?;
    let reverse = Pose::read(tab).await?;
    check!(
        reverse.starts_like(&opening),
        "a close during opening preserves the current display pose: {opening:?} → {reverse:?}"
    );
    suppressed(tab).await?;
    tick(tab, 112).await?;
    let closing = Pose::read(tab).await?;
    check!(
        closing.extent() > 8. && closing.extent() < opening.extent() - 1.,
        "the reversed branch is shrinking toward closed: {closing:?}"
    );
    closing.follows(&initial)?;

    tab.press("Space").await?;
    ready(tab, "d.expanded && d.toggles === 3 && d.activations === 0").await?;
    let reopened = Pose::read(tab).await?;
    check!(reopened.starts_like(&closing), "reopening a partially closed branch preserves the display pose: {closing:?} → {reopened:?}");
    suppressed(tab).await?;
    tick(tab, 160).await?;
    let reopen_midpoint = Pose::read(tab).await?;
    check!(
        reopen_midpoint.extent() > reopened.extent() + 1.
            && reopen_midpoint.extent() < natural[3] - 8.,
        "the second opening advances smoothly: {reopen_midpoint:?}"
    );
    reopen_midpoint.follows(&initial)?;
    tick(tab, 1160).await?;
    let settled = Pose::read(tab).await?;
    check!(
        near(settled.extent(), natural[3]),
        "the settled body reaches its intrinsic height: {settled:?}, {natural:?}"
    );
    settled.follows(&initial)?;
    focused(tab, HEADER).await?;
    follow(tab, "ArrowDown", CHILD_A).await?;
    let enabled = child(tab, CHILD_A, CHILD_A_NAME, false).await?;
    let disabled = child(tab, DISABLED, DISABLED_NAME, true).await?;
    let initial_b = bounds(tab, CHILD_B).await?;
    let initial_clip = bounds(tab, SCROLL).await?;
    check!(
        !contains(initial_clip, initial_b),
        "the final child starts outside part of the named scroller"
    );
    follow(tab, "ArrowDown", CHILD_B).await?;
    let revealed = bounds(tab, CHILD_B).await?;
    let clip = bounds(tab, SCROLL).await?;
    check!(revealed[1] < initial_b[1] && contains(clip, revealed), "the named scroller reveals the complete second child while skipping the disabled child: {revealed:?}, {clip:?}");
    child(tab, CHILD_B, CHILD_B_NAME, false).await?;
    capture(env, tab, "open-light").await?;
    follow(tab, "ArrowDown", NEXT).await?;
    follow(tab, "ArrowUp", CHILD_B).await?;
    ready(tab, "d.expanded && d.toggles === 3 && d.activations === 0").await?;
    Ok(
        json!({"initial":initial,"first":first,"opening":opening,"reverse":reverse,"closing":closing,
        "reopened":reopened,"reopen_midpoint":reopen_midpoint,"settled":settled,"natural_body":natural,
        "sample_times_ms":{"opening":48,"closing":112,"reopen_midpoint":160,"settled":1160},
        "wheel_over_partial":scrolled,
        "expanded_disclosure":opening_disclosure,"enabled_child":enabled,"disabled_child":disabled,
        "child_b_initial":initial_b,"child_b_revealed":revealed,"clip":clip}),
    )
}

async fn external_close(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M180",
        "external collapse restores the header and suppresses all child actions before paint ends",
    );
    reduced_motion(tab, false).await?;
    enter_header(tab).await?;
    let initial = Pose::read(tab).await?;
    freeze(tab).await?;
    tab.press("Enter").await?;
    ready(tab, "d.expanded && d.toggles === 1").await?;
    tick(tab, 1000).await?;
    follow(tab, "ArrowDown", CHILD_A).await?;
    tab.press("Enter").await?;
    command(tab, 1, CHILD_A).await?;
    click(tab, DISABLED).await?;
    command(tab, 1, CHILD_A).await?;
    focus_projected(tab, CHILD_A).await?;
    follow(tab, "ArrowDown", CHILD_B).await?;
    let open = Pose::read(tab).await?;
    tab.eval(
        "(() => {
            document.addEventListener('keydown', event => {
                if (event.key === 'Escape') window.listExpansionEscape = {
                    id: event.target?.dataset?.surfaceId, trusted: event.isTrusted,
                };
            }, {capture:true, passive:true});
        })()",
    )
    .await?;
    tab.press("Escape").await?;
    ready(tab, "!d.expanded && d.toggles === 1 && d.activations === 1").await?;
    focused(tab, HEADER).await?;
    let closed = disclosure(tab, false).await?;
    let close_start = Pose::read(tab).await?;
    suppressed(tab).await?;
    mounted(tab, CHILD_A, true).await?;
    let trusted = tab.eval("window.listExpansionEscape").await?;
    check_eq!(
        trusted,
        json!({"id":CHILD_B,"trusted":true}),
        "trusted Escape reached the actual focused child"
    );
    check!(
        close_start.starts_like(&open),
        "close keeps the measured body at its initial rendered height: {close_start:?}"
    );
    tick(tab, 1048).await?;
    let midpoint = Pose::read(tab).await?;
    check!(
        midpoint.extent() > 8. && midpoint.extent() < open.extent() - 8.,
        "remaining child paint shrinks continuously during close: {midpoint:?}"
    );
    midpoint.follows(&initial)?;
    suppressed(tab).await?;
    capture(env, tab, "closing-midpoint-light").await?;
    let (x, y) = partial_body_point(tab, &midpoint).await?;
    tab.click(x, y).await?;
    command(tab, 1, CHILD_A).await?;
    tick(tab, 2048).await?;
    let end = Pose::read(tab).await?;
    check!(
        end.starts_like(&initial),
        "the closed branch restores the original following row pose: {end:?}"
    );
    mounted(tab, BODY, false).await?;
    mounted(tab, CHILD_B, false).await?;
    suppressed(tab).await?;
    Ok(
        json!({"initial":initial,"open":open,"close_start":close_start,"midpoint":midpoint,"end":end,
        "sample_times_ms":{"close_start":1000,"midpoint":1048,"end":2048},
        "closed_disclosure":closed,"trusted_escape":trusted,"restored_focus":HEADER,"activated_child":CHILD_A,"activations":1}),
    )
}

async fn tap(tab: &Tab, id: &str) -> Result<()> {
    let p = point(tab, id).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"id":1,"x":p.x,"y":p.y}]}),
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

async fn style_endpoints(tab: &Tab) -> Result<Value> {
    let mut profiles = Vec::new();
    for style in ["baseline", "standard", "segmented"] {
        preset(tab, style).await?;
        ready(tab, &format!("d.variant === {} && !d.expanded", js(style))).await?;
        enter_header(tab).await?;
        let closed = Pose::read(tab).await?;
        tab.press("Enter").await?;
        ready(tab, "d.expanded").await?;
        let open = Pose::read(tab).await?;
        let body = bounds(tab, BODY).await?;
        check!(
            body[3] > 100. && near(open.extent(), body[3]),
            "{style} discloses the full inline body under Reduce Motion: {open:?}, {body:?}"
        );
        open.follows(&closed)?;
        tab.press("Space").await?;
        ready(tab, "!d.expanded").await?;
        let end = Pose::read(tab).await?;
        check!(
            end.starts_like(&closed),
            "{style} closes to the original inline geometry: {end:?}"
        );
        mounted(tab, BODY, false).await?;
        profiles.push(json!({"style":style,"closed":closed,"open":open,"body":body}));
    }
    preset(tab, "before").await?;
    ready(tab, "!d.expanded && d.toggles === 0 && d.activations === 0").await?;
    Ok(json!(profiles))
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M181",
        "Reduce Motion snaps the endpoints and 320 dp RTL wraps text without hiding controls",
    );
    reduced_motion(tab, true).await?;
    let styles = style_endpoints(tab).await?;
    for setting in ["theme", "rtl", "large-text"] {
        preset(tab, setting).await?;
    }
    until(tab, "s.dark === true").await?;
    ready(tab, "d.rtl && d.text_scale === 2 && !d.expanded").await?;
    resize(tab, 320, 900).await?;
    enter_header(tab).await?;
    let closed = Pose::read(tab).await?;
    let support = bounds(tab, "list-expand-header-supporting").await?;
    let indicator = bounds(tab, INDICATOR).await?;
    check!(
        contains([0., 0., 320., 900.], closed.header) && contains(closed.header, support)
            && contains(closed.header, indicator) && support[3] > 48.
            && indicator[0] + indicator[2] <= support[0] + 0.75,
        "RTL mirrors the indicator without covering the complete wrapping supporting text: {support:?}, {indicator:?}, {closed:?}"
    );
    freeze(tab).await?;
    tab.press("Space").await?;
    ready(tab, "d.expanded && d.toggles === 1").await?;
    let open = Pose::read(tab).await?;
    let intrinsic = bounds(tab, BODY).await?;
    check!(
        open.extent() > 100. && near(open.extent(), intrinsic[3]),
        "the body reaches its full height on the frozen clock: {open:?}, {intrinsic:?}"
    );
    open.follows(&closed)?;
    disclosure(tab, true).await?;
    focused(tab, HEADER).await?;
    follow(tab, "ArrowDown", CHILD_A).await?;
    child(tab, CHILD_A, CHILD_A_NAME, false).await?;
    let shown_a = bounds(tab, CHILD_A).await?;
    let text = bounds(tab, "list-expand-child-a-headline").await?;
    let clip = bounds(tab, SCROLL).await?;
    check!(contains(clip, shown_a) && contains(shown_a, text) && shown_a[2] >= 48. && shown_a[3] >= 48. && text[3] > 48., "the fully revealed child has a complete multiline label and target: {shown_a:?}, {text:?}, {clip:?}");
    let image = capture(env, tab, "compact-dark-rtl-open").await?;
    let [red, green, blue, alpha]: [u8; 4] =
        serde_json::from_value(data(tab).await?["colors"]["on_surface"].clone())
            .context("text color")?;
    let ink = image
        .colors(text)?
        .into_iter()
        .filter(|(sample, _)| {
            sample
                .iter()
                .zip([red, green, blue])
                .all(|(a, b)| a.abs_diff(b) <= 5)
        })
        .map(|(_, pixels)| pixels)
        .sum::<usize>();
    check!(
        alpha == 255 && ink >= 16,
        "the large RTL child label paints Material OnSurface ink: {ink} pixels"
    );
    tap(tab, CHILD_A).await?;
    command(tab, 1, CHILD_A).await?;
    focus_projected(tab, CHILD_A).await?;
    tab.press("Escape").await?;
    ready(tab, "!d.expanded && d.toggles === 1 && d.activations === 1").await?;
    focused(tab, HEADER).await?;
    let snapped = Pose::read(tab).await?;
    check!(
        snapped.starts_like(&closed) && contains(bounds(tab, SCROLL).await?, snapped.header),
        "external close snaps without time passing and reveals the restored header: {snapped:?}"
    );
    check_eq!(
        tab.eval("window.listExpansionClock").await?,
        json!(0),
        "Reduce Motion endpoints do not need the animation clock"
    );
    mounted(tab, BODY, false).await?;
    suppressed(tab).await?;
    follow(tab, "ArrowDown", NEXT).await?;
    command(tab, 1, CHILD_A).await?;
    Ok(
        json!({"viewport":[320,900],"rtl":true,"text_scale":2,"elapsed_ms":0,"style_endpoints":styles,
        "closed":closed,"open":open,"snapped":snapped,"indicator":indicator,"header_support":support,
        "child_target":shown_a,"child_headline":text,"child_ink_pixels":ink,"clip":clip,"touch_activated":CHILD_A}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let scope = closed_scope(env, tab).await?;
    reload(tab, url).await?;
    let motion = moving(env, tab).await?;
    reload(tab, url).await?;
    let external = external_close(env, tab).await?;
    reload(tab, url).await?;
    let rtl = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "Material list expansion browser errors: {errors:?}"
    );
    env.write_json("browser-material-list-expansion.json", &json!({
        "browser":chrome.version,"scope":scope,"motion":motion,"external":external,"compact":rtl,"errors":errors,
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 1744, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json("material-list-expansion-failure.json", &json!({
            "error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors(),
        }));
        let _ = tab
            .screenshot(&env.artifact("material-list-expansion-failure.png"))
            .await;
    }
    result.context("Material 3 list expansion")
}
