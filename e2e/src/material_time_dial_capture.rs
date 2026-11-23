//! Lost pointer capture on the Gallery's real browser canvas.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{capture, closed, mouse, open, ready, resize, runtime_focus, state, tick, HOUR};
use crate::browser::Tab;
use crate::check::step;
use crate::{check, check_eq, Env};

const TRACE: &str = r#"
    (() => {
        const canvas = document.getElementById('surface');
        window.timeDialCaptureTrace = [];
        for (const kind of ['pointerdown', 'gotpointercapture', 'pointermove',
                            'lostpointercapture', 'pointerup', 'pointercancel']) {
            document.addEventListener(kind, event => {
                if (event.target !== canvas || event.pointerType !== 'mouse') return;
                window.timeDialCaptureTrace.push({
                    kind, id: event.pointerId, trusted: event.isTrusted,
                    buttons: event.buttons, captured: canvas.hasPointerCapture(event.pointerId),
                });
            }, {capture: true, passive: true});
        }
    })()
"#;

async fn capture_owner(tab: &Tab) -> Result<i64> {
    let id = tab
        .eval("window.timeDialCaptureTrace.find(event => event.kind === 'pointerdown' && event.trusted)?.id")
        .await?
        .as_i64()
        .context("no trusted mouse down reached the dial canvas")?;
    tab.wait_for(&format!(
        "document.getElementById('surface').hasPointerCapture({id}) &&
         window.timeDialCaptureTrace.some(event => event.kind === 'gotpointercapture' &&
             event.id === {id} && event.trusted)"
    ))
    .await?;
    Ok(id)
}

async fn trusted_order(tab: &Tab, id: i64, order: &[&str]) -> Result<Value> {
    let trace = tab.eval("window.timeDialCaptureTrace").await?;
    let mut events = trace
        .as_array()
        .context("expected pointer event trace")?
        .iter();
    check!(
        order.iter().all(|kind| events.any(|event| {
            event["kind"].as_str() == Some(*kind)
                && event["id"].as_i64() == Some(id)
                && event["trusted"] == true
        })),
        "expected trusted pointer events {order:?} for ID {id}: {trace}",
    );
    Ok(trace)
}

pub(super) async fn run(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M142",
        "unexpected browser capture loss restores the draft and a fresh drag still works",
    );
    resize(tab, 390, 720).await?;
    open(tab).await?;
    ready(
        tab,
        "d.draft.candidate === '09:07' && d.saved === '09:07' && d.commits === 0",
    )
    .await?;
    let before = state(tab).await?["time_dial"].clone();
    tab.eval(TRACE).await?;

    let start = tick(tab, 3., 12., 100.).await?;
    let preview = tick(tab, 6., 12., 100.).await?;
    let different_release = tick(tab, 8., 12., 100.).await?;
    mouse(tab, "mousePressed", start, true).await?;
    mouse(tab, "mouseMoved", preview, true).await?;
    ready(tab, "d.draft.preview?.value === 6").await?;
    let owner = capture_owner(tab).await?;
    let held = state(tab).await?["time_dial"].clone();
    let mut expected_held = before.clone();
    expected_held["draft"]["preview"] = json!({"field":"Hour","value":6});
    check_eq!(held, expected_held, "held drag only changes the preview");
    capture(env, tab, "capture-held").await?;

    tab.eval(&format!(
        "document.getElementById('surface').dispatchEvent(new PointerEvent('lostpointercapture',
           {{pointerId:{owner}, pointerType:'mouse', buttons:1, bubbles:true}}))"
    ))
    .await?;
    tab.settle().await?;
    check_eq!(
        state(tab).await?["time_dial"],
        held,
        "a synthetic loss for the same pointer does not cancel the preview"
    );
    check_eq!(
        tab.eval(&format!(
            "document.getElementById('surface').hasPointerCapture({owner}) &&
             window.timeDialCaptureTrace.some(event => event.kind === 'lostpointercapture' &&
                 event.id === {owner} && !event.trusted)"
        ))
        .await?,
        json!(true),
        "the synthetic event reached the canvas while real capture stayed active"
    );

    tab.eval(&format!(
        "document.getElementById('surface').releasePointerCapture({owner})"
    ))
    .await?;
    // Chrome delivers the pending loss with the next input. Send up immediately
    // so the loss tests the release path even when the event has no buttons held.
    mouse(tab, "mouseReleased", different_release, false).await?;
    let loss = trusted_order(
        tab,
        owner,
        &[
            "pointerdown",
            "gotpointercapture",
            "pointermove",
            "lostpointercapture",
            "pointerup",
        ],
    )
    .await?;
    let restored = state(tab).await?["time_dial"].clone();
    check_eq!(restored, before, "capture loss restores the entire draft");
    runtime_focus(tab, HOUR).await?;
    capture(env, tab, "capture-restored").await?;

    tab.eval("window.timeDialCaptureTrace.length = 0").await?;
    let fresh_start = tick(tab, 10., 12., 100.).await?;
    let fresh_end = tick(tab, 4., 12., 100.).await?;
    mouse(tab, "mousePressed", fresh_start, true).await?;
    mouse(tab, "mouseMoved", fresh_end, true).await?;
    ready(
        tab,
        "d.draft.stage === 'Hour' && d.draft.preview?.value === 4",
    )
    .await?;
    let fresh_owner = capture_owner(tab).await?;
    mouse(tab, "mouseReleased", fresh_end, false).await?;
    ready(
        tab,
        "d.draft.preview === null && d.draft.stage === 'Minute' &&
         d.draft.candidate === '04:07' && d.saved === '09:07' && d.commits === 0",
    )
    .await?;
    let normal = trusted_order(
        tab,
        fresh_owner,
        &[
            "pointerdown",
            "gotpointercapture",
            "pointermove",
            "pointerup",
            "lostpointercapture",
        ],
    )
    .await?;
    let rearmed = state(tab).await?["time_dial"].clone();
    tab.press("Escape").await?;
    closed(tab, "09:07", 0).await?;
    Ok(json!({
        "explicit_loss_events":loss, "normal_release_events":normal,
        "before":before, "held":held, "restored":restored, "rearmed":rearmed,
    }))
}
