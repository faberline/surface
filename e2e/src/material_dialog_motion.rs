//! Actual Basic Dialog pixels under a controlled browser animation clock.

use super::*;

const BASIC_OPEN: &str = "dialog-demo-basic-open";
const BASIC_CONFIRM: &str = "dialog-demo-basic-confirm";
const DISCARD_CONTINUE: &str = "dialog-demo-discard-cancel";
pub(super) const CLOCK: &str = "__surfaceDialogMotionTestMs";

type Pixel = [u8; 3];

pub(super) async fn preference(tab: &Tab, reduced: bool) -> Result<()> {
    let value = if reduced { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[
            {"name":"prefers-reduced-motion","value":value},
            {"name":"forced-colors","value":"none"}
        ]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {value})"))
    ))
    .await
}

async fn scene(tab: &Tab, url: &str, width: u32, height: u32, reduced: bool) -> Result<()> {
    preference(tab, reduced).await?;
    route(tab, url, width, height).await?;
    activate(tab, "dialog-demo-theme").await?;
    until(tab, "s.dark").await?;
    freeze(tab).await
}

pub(super) async fn freeze(tab: &Tab) -> Result<()> {
    tab.eval(&format!(
        r#"(() => {{
            const origin=performance.now();
            window.{CLOCK}=0;
            Object.defineProperty(performance,"now",{{configurable:true,value:()=>origin+window.{CLOCK}}});
            return true;
        }})()"#
    )).await?;
    tab.settle().await
}

pub(super) async fn tick(tab: &Tab, now: u32) -> Result<()> {
    tab.eval(&format!("window.{CLOCK}={now}")).await?;
    tab.settle().await
}

async fn alpha(tab: &Tab, name: &str) -> Result<f64> {
    state(tab).await?["dialog_demo"]["opacity"][name]
        .as_f64()
        .context("real Dialog opacity state")
}

async fn absent(tab: &Tab, id: &str) -> Result<()> {
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(id)
    ))
    .await
}

pub(super) async fn visible_ax(tab: &Tab, panel: &str, label: &str, action: &str) -> Result<()> {
    check_eq!(
        tab.eval(&format!("{} !== null", projected(panel))).await?,
        true,
        "active dialog has a real DOM projection"
    );
    let tree = ax_tree(tab).await?;
    check_eq!(
        ax_property(ax(&tree, "dialog", label)?, "modal"),
        Some(&json!(true)),
        "Chrome exposes the active dialog as modal"
    );
    ax(&tree, "button", action)?;
    Ok(())
}

pub(super) async fn retired_ax(
    tab: &Tab,
    id: &str,
    label: &str,
    control_id: &str,
    control_name: &str,
) -> Result<()> {
    let tree = ax_tree(tab).await?;
    check!(
        ax(&tree, "dialog", label).is_err(),
        "retained visual dialog is absent in Chrome AX"
    );
    check!(
        ax(&tree, "button", control_name).is_err(),
        "retired action is absent in Chrome AX"
    );
    for id in [id, control_id] {
        check_eq!(
            tab.eval(&format!("{} === null", projected(id))).await?,
            true,
            "retained {id} has no projected DOM input target"
        );
    }
    Ok(())
}

fn spring_40_ms() -> f64 {
    1. - 2.6 * (-1.6_f64).exp()
}

fn lerp_channel(start: u8, end: u8, actual: u8) -> bool {
    let low = start.min(end);
    let high = start.max(end);
    actual >= low && actual <= high
}

fn mid_pixels(rest: [Pixel; 2], mid: [Pixel; 2], full: [Pixel; 2]) -> Result<()> {
    let [rest_scrim, _] = rest;
    let [mid_scrim, mid_panel] = mid;
    let [full_scrim, full_panel] = full;
    check!(
        rest_scrim.iter().zip(mid_scrim).zip(full_scrim).all(|((rest, mid), full)| *rest > mid && mid > full),
        "the actual intermediate scrim shade lies strictly between endpoints: {rest_scrim:?}/{mid_scrim:?}/{full_scrim:?}"
    );
    check!(
        mid_scrim.iter().zip(full_panel).zip(mid_panel).all(|((behind, opaque), mid)| lerp_channel(*behind, opaque, mid))
            && mid_panel != mid_scrim && mid_panel != full_panel,
        "the visible panel is composited between the same-frame backdrop and its opaque endpoint: {mid_scrim:?}/{mid_panel:?}/{full_panel:?}"
    );
    Ok(())
}

fn sample(frame: &Raster, points: [[f64; 2]; 2]) -> Result<[Pixel; 2]> {
    let [[a, b], [c, d]] = points;
    Ok([frame.sample(a, b)?, frame.sample(c, d)?])
}

async fn basic_motion(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    scene(tab, url, 960, 720, false).await?;
    let rest = raster(env, tab, "motion-basic-rest").await?;
    open(tab, BASIC, "basic").await?;
    focused(tab, BASIC_CONFIRM).await?;
    ready(tab, "d.opacity.basic === 0").await?;
    visible_ax(tab, BASIC, "行程通知", "我已閱讀").await?;
    background_blocked(tab).await?;
    let panel = bounds(tab, BASIC).await?;
    let points = [[940., 690.], [panel[0] + 14., panel[1] + panel[3] / 2.]];
    let rest_pixels = sample(&rest, points)?;
    check_eq!(
        rest_pixels[0],
        rest_pixels[1],
        "both samples start over the same blank surface"
    );
    let first = raster(env, tab, "motion-basic-enter-zero").await?;
    check_eq!(
        sample(&first, points)?,
        rest_pixels,
        "zero-opacity entry has no visible panel or scrim flash"
    );

    tick(tab, 40).await?;
    let entering_alpha = alpha(tab, "basic").await?;
    check!(
        (entering_alpha - spring_40_ms()).abs() < 0.002,
        "40 ms entry opacity matches the critical step: {entering_alpha}"
    );
    let entering = sample(&raster(env, tab, "motion-basic-entering").await?, points)?;
    tick(tab, 450).await?;
    ready(tab, "d.open === 'basic' && d.opacity.basic === 1").await?;
    focused(tab, BASIC_CONFIRM).await?;
    check_eq!(
        bounds(tab, BASIC).await?,
        panel,
        "Basic changes opacity without moving the panel"
    );
    let full = sample(&raster(env, tab, "motion-basic-open").await?, points)?;
    mid_pixels(rest_pixels, entering, full)?;

    tab.press("Escape").await?;
    ready(tab, "d.open === null && d.opacity.basic === 1").await?;
    focused(tab, BASIC_OPEN).await?;
    retired_ax(tab, BASIC, "行程通知", BASIC_CONFIRM, "我已閱讀").await?;
    ax(&ax_tree(tab).await?, "button", "基本對話框")?;
    check_eq!(
        bounds(tab, BASIC).await?,
        panel,
        "first exit frame keeps the real paint panel at the same bounds"
    );
    let first_exit = sample(&raster(env, tab, "motion-basic-exit-start").await?, points)?;
    check_eq!(
        first_exit,
        full,
        "first exit frame still paints the fully visible dialog surface and scrim"
    );

    tick(tab, 490).await?;
    let exit_alpha = alpha(tab, "basic").await?;
    check!(
        (exit_alpha + spring_40_ms() - 1.).abs() < 0.002,
        "40 ms exit opacity matches the reverse critical step: {exit_alpha}"
    );
    let exiting = sample(&raster(env, tab, "motion-basic-exiting").await?, points)?;
    mid_pixels(rest_pixels, exiting, full)?;

    let opener = point(tab, BASIC_OPEN).await?;
    let [cx, cy, ..] = opener.canvas;
    check!(
        !contains(panel, [opener.x - cx, opener.y - cy, 1., 1.]),
        "actual opener hit lies covered by the retiring scrim, outside the panel: {panel:?}"
    );
    tab.click(opener.x, opener.y).await?;
    ready(tab, "d.open === 'basic'").await?;
    pointer_focus(tab, BASIC_CONFIRM).await?;
    visible_ax(tab, BASIC, "行程通知", "我已閱讀").await?;
    check!(
        (alpha(tab, "basic").await? - exit_alpha).abs() < 0.0001,
        "trusted pointer passes through and reverses from the currently visible opacity"
    );
    let reversed = sample(&raster(env, tab, "motion-basic-reversed").await?, points)?;
    check_eq!(
        reversed,
        exiting,
        "opening in the same frame keeps the exact intermediate surface pixels"
    );
    tick(tab, 590).await?;
    check!(
        alpha(tab, "basic").await? > exit_alpha,
        "the reversed panel becomes more opaque"
    );

    tick(tab, 1000).await?;
    ready(tab, "d.opacity.basic === 1").await?;
    tab.press("Escape").await?;
    ready(tab, "d.open === null").await?;
    tick(tab, 1450).await?;
    ready(tab, "d.opacity.basic === 0").await?;
    absent(tab, BASIC).await?;
    absent(tab, "dialog-demo-basic-layer").await?;
    pointer_focus(tab, BASIC_OPEN).await?;
    let gone = sample(&raster(env, tab, "motion-basic-closed").await?, points)?;
    check_eq!(
        gone,
        rest_pixels,
        "fully retired Basic leaves exact resting pixels"
    );
    Ok(
        json!({"panel":panel,"sample_points":points,"entry_40ms":entering_alpha,"exit_40ms":exit_alpha,
        "pixels":{"rest":rest_pixels,"enter_zero":rest_pixels,"entering":entering,"opaque":full,
                  "exit_start":first_exit,"exiting":exiting,"reversed":reversed,"closed":gone},
        "enter_scope":BASIC_CONFIRM,"exit_scope":BASIC_OPEN,"exit_ax_absent":true,
        "trusted_pass_through_reopened":true}),
    )
}

async fn nested(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    scene(tab, url, 320, 400, false).await?;
    open(tab, EDIT, "edit").await?;
    ready(tab, "d.presentation === 'fullscreen'").await?;
    fill(tab, CHANGED).await?;
    editor_focus(tab).await?;
    let corner = [312., 390.];
    let outer = raster(env, tab, "motion-nested-edit")
        .await?
        .sample(corner[0], corner[1])?;

    tab.press("Escape").await?;
    ready(tab, "d.open === 'discard' && d.opacity.discard === 0").await?;
    focused(tab, DISCARD_CONTINUE).await?;
    visible_ax(tab, DISCARD, "捨棄變更？", "繼續編輯").await?;
    check!(
        ax(&ax_tree(tab).await?, "dialog", "編輯行程").is_err(),
        "the entering nested scope owns AX even at opacity zero"
    );
    let bounds_before = bounds(tab, DISCARD).await?;
    tick(tab, 40).await?;
    let entering_alpha = alpha(tab, "discard").await?;
    check!(
        (entering_alpha - spring_40_ms()).abs() < 0.002,
        "nested uses the same 40 ms opacity"
    );
    let entering = raster(env, tab, "motion-nested-entering")
        .await?
        .sample(corner[0], corner[1])?;
    tick(tab, 450).await?;
    ready(tab, "d.open === 'discard' && d.opacity.discard === 1").await?;
    let covered = raster(env, tab, "motion-nested-open")
        .await?
        .sample(corner[0], corner[1])?;
    check!(outer.iter().zip(entering).zip(covered).all(|((a, b), c)| *a > b && b > c),
        "the real nested scrim composites above the still-painted full-screen editor: {outer:?}/{entering:?}/{covered:?}");

    tab.press("Enter").await?;
    ready(
        tab,
        "d.open === 'edit' && d.opacity.discard === 1 && d.saves === 0 && d.draft === '秋日散步'",
    )
    .await?;
    editor_focus(tab).await?;
    visible_ax(tab, EDIT, "編輯行程", "儲存").await?;
    retired_ax(tab, DISCARD, "捨棄變更？", DISCARD_CONTINUE, "繼續編輯").await?;
    check_eq!(
        bounds(tab, DISCARD).await?,
        bounds_before,
        "retired nested panel keeps its physical order and bounds"
    );
    let first_exit = raster(env, tab, "motion-nested-exit-start")
        .await?
        .sample(corner[0], corner[1])?;
    check_eq!(
        first_exit,
        covered,
        "lower focused editor is still visually covered by the real retiring nested scrim"
    );
    tick(tab, 490).await?;
    let exit_alpha = alpha(tab, "discard").await?;
    let exiting = raster(env, tab, "motion-nested-exiting")
        .await?
        .sample(corner[0], corner[1])?;
    check!(outer.iter().zip(exiting).zip(covered).all(|((a,b),c)| *a>b && b>c),
        "exit still paints above the active editor at an intermediate opacity: {outer:?}/{exiting:?}/{covered:?}");

    tab.insert_text("！").await?;
    ready(
        tab,
        "d.open === 'edit' && d.draft === '秋日散步！' && d.saves === 0",
    )
    .await?;
    check_eq!(
        bounds(tab, DISCARD).await?,
        bounds_before,
        "trusted typing reaches the lower editor while nested paint remains"
    );
    tick(tab, 950).await?;
    ready(tab, "d.opacity.discard === 0").await?;
    absent(tab, DISCARD).await?;
    editor_focus(tab).await?;
    let gone = raster(env, tab, "motion-nested-closed")
        .await?
        .sample(corner[0], corner[1])?;
    check_eq!(
        gone,
        outer,
        "removing the retired nested layer fully restores the lower page shade"
    );
    Ok(
        json!({"sample_point":corner,"retained_panel":bounds_before,"entry_40ms":entering_alpha,"exit_40ms":exit_alpha,
             "scrim":{"initial":outer,"entering":entering,"covered":covered,"exit_start":first_exit,"exiting":exiting,"closed":gone},
             "upper_ax_absent_during_exit":true,"live_lower_editor":EDITOR,"draft_after_trusted_input":"秋日散步！"}),
    )
}

async fn reduced(env: &Env, tab: &Tab, url: &str, expected: &Value) -> Result<Value> {
    scene(tab, url, 960, 720, true).await?;
    check_eq!(
        tab.eval(&format!("window.{CLOCK}")).await?,
        0,
        "no manual frame advances in reduce-motion profile"
    );
    open(tab, BASIC, "basic").await?;
    ready(tab, "d.opacity.basic === 1").await?;
    focused(tab, BASIC_CONFIRM).await?;
    let panel = bounds(tab, BASIC).await?;
    let motion_panel: Rect = serde_json::from_value(expected["panel"].clone())?;
    check_eq!(
        panel,
        motion_panel,
        "Reduced Motion uses the same panel geometry"
    );
    let points = serde_json::from_value(expected["sample_points"].clone())?;
    let now = sample(
        &raster(env, tab, "motion-basic-reduced-open").await?,
        points,
    )?;
    check_eq!(
        json!(now),
        expected["pixels"]["opaque"],
        "Reduced Motion snaps to identical settled paint without advancing the clock"
    );
    tab.press("Escape").await?;
    dismiss(tab, BASIC_OPEN).await?;
    ready(tab, "d.opacity.basic === 0").await?;
    absent(tab, BASIC).await?;
    absent(tab, "dialog-demo-basic-layer").await?;
    retired_ax(tab, BASIC, "行程通知", BASIC_CONFIRM, "我已閱讀").await?;
    check_eq!(
        tab.eval(&format!("window.{CLOCK}")).await?,
        0,
        "Reduced Motion removes the entire retired layer with no clock advance"
    );
    let gone = sample(
        &raster(env, tab, "motion-basic-reduced-closed").await?,
        points,
    )?;
    check_eq!(
        json!(gone),
        expected["pixels"]["rest"],
        "Reduced Motion exit restores the exact resting paint"
    );
    preference(tab, false).await?;
    Ok(json!({"clock_ms":0,"open":now,"closed":gone,"exact_endpoints":true}))
}

pub(super) async fn run(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let basic = basic_motion(env, tab, url)
        .await
        .context("Basic fade/reversal")?;
    let nested = nested(env, tab, url)
        .await
        .context("nested Discard paint and scope")?;
    let reduced = reduced(env, tab, url, &basic)
        .await
        .context("Reduced Motion")?;
    Ok(json!({"manual_performance_time":true,"basic":basic,"nested":nested,"reduced":reduced}))
}
