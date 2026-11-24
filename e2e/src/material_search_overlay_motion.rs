//! Rendered Search spatial motion at a pinned host clock.

use super::*;

const CLOSING: &str = "search-overlay-demo-closing-shell";

async fn reduced(tab: &Tab, value: bool) -> Result<()> {
    let pref = if value { "reduce" } else { "no-preference" };
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[
            {"name":"prefers-reduced-motion","value":pref},
            {"name":"forced-colors","value":"none"}
        ]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {pref})"))
    ))
    .await?;
    tab.settle().await
}

async fn freeze(tab: &Tab) -> Result<()> {
    tab.eval(r#"(() => {
        const origin=performance.now();
        window.__searchMotionTime=0;
        Object.defineProperty(performance,"now",{configurable:true,value:()=>origin+window.__searchMotionTime});
        return true;
    })()"#).await?;
    tab.settle().await
}

async fn tick(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.__searchMotionTime={ms}")).await?;
    tab.settle().await
}

async fn motion(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["search_overlay"]["motion"].take())
}

async fn scene(tab: &Tab, url: &str, compact: bool, reduce: bool) -> Result<()> {
    reduced(tab, reduce).await?;
    route(tab, url).await?;
    if compact {
        activate(tab, "search-overlay-demo-style").await?;
        resize(tab, 320, 400).await?;
    } else {
        resize(tab, 960, 800).await?;
    }
    bounds(tab, ANCHOR).await?;
    freeze(tab).await
}

async fn no_shell(tab: &Tab) -> Result<()> {
    tab.wait_for(&format!(
        "window.gallery.control_bounds({}).length === 0",
        js(CLOSING)
    ))
    .await
}

async fn native_editor(tab: &Tab) -> Result<Value> {
    tab.eval(&format!(
        "(() => {{ const e={EDITOR}; return {{
        same:e===window.__searchMotionTextarea, hidden:e.hidden, rects:e.getClientRects().length,
        label:e.getAttribute('aria-label'), focused:document.activeElement===e
    }}; }})()"
    ))
    .await
}

pub(super) async fn run(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    let mut profiles = Vec::new();
    for compact in [false, true] {
        scene(tab, url, compact, false).await?;
        let region = if compact {
            [0., 0., 320., 400.]
        } else {
            [0., 0., 960., 800.]
        };
        let label = if compact {
            "fullscreen-divided"
        } else {
            "docked-contained"
        };
        let rest = bounds(tab, OPENER).await?;
        tab.eval(&format!("window.__searchMotionTextarea={EDITOR}; true"))
            .await?;
        let inactive_at_rest = native_editor(tab).await?;
        let rest_ax = ax_tree(tab).await?;
        let resting_img = paint(env, tab, &format!("motion-{label}-rest")).await?;
        let corner = [region[2] - 4., region[3] - 4.];
        let rest_px = resting_img.sample(corner[0], corner[1])?;
        activate(tab, OPENER).await?;
        until(tab,"s.search_overlay.open && s.search_overlay.motion.moving && s.search_overlay.motion.progress === 0").await?;
        edit_focus(tab).await?;
        let active_at_open = native_editor(tab).await?;
        check!(
            active_at_open["same"] == true
                && active_at_open["hidden"] == false
                && active_at_open["rects"] == 1
                && active_at_open["focused"] == true
                && active_at_open["label"] == "搜尋文件",
            "active overlay exposes its own label on the same focusable host"
        );
        let opening_ax = ax_tree(tab).await?;
        check_eq!(
            ax_property(ax(&opening_ax, "textbox", "搜尋文件")?, "focused"),
            Some(&json!(true)),
            "actual Chrome AX exposes the newly focused active host"
        );
        let initial = bounds(tab, DIALOG).await?;
        check_eq!(
            initial,
            rest,
            "normal entry begins at the measured resting anchor"
        );
        tick(tab, 64).await?;
        until(tab,"s.search_overlay.motion.moving && s.search_overlay.motion.progress > 0 && s.search_overlay.motion.progress < 1").await?;
        let entry = motion(tab).await?;
        let halfway = bounds(tab, DIALOG).await?;
        inside(region, halfway, "entry frame visible inside viewport")?;
        let entering = paint(env, tab, &format!("motion-{label}-entering")).await?;
        let entering_px = entering.sample(corner[0], corner[1])?;
        tick(tab, 1000).await?;
        until(
            tab,
            "s.search_overlay.motion.progress === 1 && !s.search_overlay.motion.moving",
        )
        .await?;
        edit_focus(tab).await?;
        let final_rect = bounds(tab, DIALOG).await?;
        let expanded = paint(env, tab, &format!("motion-{label}-expanded")).await?;
        let expanded_px = expanded.sample(corner[0], corner[1])?;
        check!(halfway[3] > initial[3]+4. && halfway[3] < final_rect[3]-4.,
               "real normal-motion geometry passes a strict intermediate height: {initial:?}/{halfway:?}/{final_rect:?}");
        if compact {
            check_eq!(
                final_rect,
                region,
                "fullscreen motion reaches exact viewport"
            );
            let x = final_rect[2] / 2.;
            let p = [x, 10.];
            check!(
                entering.sample(p[0], p[1])? != expanded.sample(p[0], p[1])?,
                "animated fullscreen panel truly changes the pixels at its top destination"
            );
        } else {
            check!(rest_px.iter().zip(entering_px).zip(expanded_px).all(|((a,b),c)| *a>b && b>c),
                   "mid-docked backdrop shade lies between actual endpoints: {rest_px:?}/{entering_px:?}/{expanded_px:?}");
        }

        tab.press("Escape").await?;
        until(
            tab,
            "!s.search_overlay.open && s.search_overlay.motion.moving",
        )
        .await?;
        focused(tab, OPENER).await?;
        check_eq!(
            bounds(tab, CLOSING).await?,
            final_rect,
            "closing begins at expanded pose"
        );
        let tree = ax_tree(tab).await?;
        check!(
            ax(&tree, "dialog", "搜尋工作區").is_err(),
            "exit paint has no live dialog or modal AX scope"
        );
        check_eq!(
            tab.eval(&format!(
                "{} === null && document.activeElement !== {EDITOR}",
                projected(INPUT)
            ))
            .await?,
            true,
            "the actual Search editor is no longer projected or focused while exit paint remains"
        );
        let retired_host = ax(&tree, "textbox", "搜尋文件").ok().cloned();
        if let Some(host) = &retired_host {
            check!(
                ax_property(host, "focused") != Some(&json!(true)),
                "the persistent host is not the focused exit paint"
            );
            let document = tab.cdp("DOM.getDocument", json!({})).await?;
            let by_selector = tab
                .cdp(
                    "DOM.querySelector",
                    json!({"nodeId":document["root"]["nodeId"],"selector":"#surface-text-input"}),
                )
                .await?;
            let described = tab
                .cdp("DOM.describeNode", json!({"nodeId":by_selector["nodeId"]}))
                .await?;
            check!(
                described["node"]["backendNodeId"].is_number(),
                "the retained textarea has a real browser backend identity"
            );
            check_eq!(host["backendDOMNodeId"],described["node"]["backendNodeId"],
                "the residual unfocused Chrome node is the host textarea, outside the removed Search component");
            env.write_json("search-motion-exit-stale-host.json",&json!({
                "profile":label,"ax":host,"native_host":described["node"],
                "search_state":state(tab).await?["search_overlay"],"closing_shell":bounds(tab,CLOSING).await?
            }))?;
        }
        check!(
            retired_host.is_none(),
            "closed Search must not leave a stale named textbox in Chrome AX"
        );
        let inactive_at_exit = native_editor(tab).await?;
        for idle in [&inactive_at_rest, &inactive_at_exit] {
            check!(idle["same"]==true && idle["hidden"]==true && idle["rects"]==0 && idle["focused"]==false,
                "the same native textarea has no box or focus when its editor is unavailable: {idle}");
        }
        check!(
            rest_ax
                .iter()
                .all(|node| node["ignored"] == true || node["role"]["value"] != "textbox"),
            "the Search route exposes no textbox in Chrome AX before opening"
        );
        tick(tab, 1064).await?;
        let leaving = bounds(tab, CLOSING).await?;
        check!(
            leaving[3] > rest[3] + 4. && leaving[3] < final_rect[3] - 4.,
            "closing shell interpolates in place"
        );
        let exit = motion(tab).await?;
        let exiting = paint(env, tab, &format!("motion-{label}-exiting")).await?;
        let exiting_px = exiting.sample(corner[0], corner[1])?;
        if compact {
            check!(
                exiting.sample(region[2] / 2., 10.)? != expanded.sample(region[2] / 2., 10.)?,
                "normal fullscreen exit leaves the top destination before the shell unmounts"
            );
        } else {
            check!(rest_px.iter().zip(exiting_px).zip(expanded_px).all(|((a,b),c)| *a>b && b>c),
                "actual mid-exit docked scrim lies between rest and expanded: {rest_px:?}/{exiting_px:?}/{expanded_px:?}");
        }
        tick(tab, 2000).await?;
        until(
            tab,
            "s.search_overlay.motion.progress === 0 && !s.search_overlay.motion.moving",
        )
        .await?;
        no_shell(tab).await?;
        focused(tab, OPENER).await?;
        activate(tab, OPENER).await?;
        until(
            tab,
            "s.search_overlay.open && s.search_overlay.motion.moving",
        )
        .await?;
        tick(tab, 3000).await?;
        until(
            tab,
            "s.search_overlay.motion.progress === 1 && !s.search_overlay.motion.moving",
        )
        .await?;
        edit_focus(tab).await?;
        let reopened = native_editor(tab).await?;
        check_eq!(
            reopened,
            active_at_open,
            "reopening reuses the same unhidden DOM host and current Search label"
        );
        let reopened_ax = ax_tree(tab).await?;
        check_eq!(
            ax_property(ax(&reopened_ax, "textbox", "搜尋文件")?, "focused"),
            Some(&json!(true)),
            "the same native editor returns to the Chrome AX tree focused after reopening"
        );

        scene(tab, url, compact, true).await?;
        activate(tab, OPENER).await?;
        until(tab,"s.search_overlay.open && s.search_overlay.motion.progress === 1 && !s.search_overlay.motion.moving").await?;
        edit_focus(tab).await?;
        check_eq!(
            bounds(tab, DIALOG).await?,
            final_rect,
            "Reduce Motion goes directly to exact endpoint without ticking the clock"
        );
        let reduced_img = paint(env, tab, &format!("motion-{label}-reduced")).await?;
        check_eq!(
            reduced_img.sample(corner[0], corner[1])?,
            expanded_px,
            "same exact endpoint sample with Reduce Motion"
        );
        tab.press("Escape").await?;
        until(tab,"!s.search_overlay.open && s.search_overlay.motion.progress === 0 && !s.search_overlay.motion.moving").await?;
        no_shell(tab).await?;
        focused(tab, OPENER).await?;
        profiles.push(json!({"presentation":label,"initial":initial,"halfway":halfway,"final":final_rect,
            "entry":entry,"exit":exit,"leaving":leaving,"corner":{"rest":rest_px,"entering":entering_px,"expanded":expanded_px,"exiting":exiting_px},
            "exit_dimensions":exiting.dimensions(),"reduced_exact_endpoints":true,
            "editor_lifecycle":{"rest":inactive_at_rest,"open":active_at_open,"exit":inactive_at_exit,"reopened":reopened},
            "unfocused_persistent_web_textarea_ax":retired_host}));
    }
    reduced(tab, false).await?;
    Ok(json!({"manual_performance_time":true,"profiles":profiles}))
}
