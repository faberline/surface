//! Compare WebGPU underline pixels on both sides of a preedit selection.

use std::ops::Range;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    bounds, contrast, editor_focus, forced, matches_fill, media, popup, popup_state, select,
    CanvasPng, Clip, Lane, Palette, References, Rgb, Seams, ALL, EDITOR, FACE, MIDDLE, PREFIX, RAW,
    SUFFIX, TEXTAREA, TEXT_END,
};
use crate::browser::{js, Chrome, Tab};
use crate::{check, check_eq, Env};

const SCALE: f64 = 2.;
const BACKING: &str = "1680";

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<CanvasPng> {
    CanvasPng::capture_at_scale(env, tab, &format!("preedit-underline-{name}"), SCALE).await
}

async fn ordinary(env: &Env, tab: &Tab, name: &str, range: [usize; 2]) -> Result<CanvasPng> {
    select(tab, range).await?;
    capture(env, tab, name).await
}

fn editing_expression() -> String {
    format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]",
        js(EDITOR)
    )
}

async fn begin_preedit(tab: &Tab, [start, end]: [usize; 2]) -> Result<Value> {
    editor_focus(tab).await?;
    tab.shortcut("a").await?;
    tab.insert_text(BACKING).await?;
    popup_state(
        tab,
        "d.open && d.quantity.raw === '1680' && d.quantity.value === 1200 &&
         d.quantity.commits === 0 && d.unit.id === 'ml' && d.unit.commits === 0",
    )
    .await?;
    tab.shortcut("a").await?;
    tab.wait_for(&format!(
        "{TEXTAREA}.selectionStart === 0 && {TEXTAREA}.selectionEnd === {TEXT_END}"
    ))
    .await?;

    // Input.imeSetComposition uses UTF-16 offsets. One ASCII digit per
    // position keeps the offsets identical across the browser and Runtime.
    // Replace different draft content so each fresh composition emits input.
    tab.ime_compose(RAW, start, end).await?;
    let editor = editing_expression();
    tab.wait_for(&format!(
        "(() => {{ const e = {editor}; const t = {TEXTAREA};
          return e?.composing === true && e.value === {} && e.display_value === {}
             && e.preedit_range?.start === 0 && e.preedit_range?.end === {TEXT_END}
             && t?.value === {} && t.selectionStart === {start} && t.selectionEnd === {end};
        }})()",
        js(BACKING),
        js(RAW),
        js(RAW),
    ))
    .await
    .context("Chrome did not establish the requested synthetic preedit selection")?;
    editor_focus(tab).await?;
    let snapshot = preedit_snapshot(tab).await?;
    // The editor export retains the backing buffer's replacement range.
    // The browser's textarea separately exposes its preedit selection.
    let runtime = &snapshot["runtime"];
    let mut endpoints: [usize; 2] = serde_json::from_value(json!([
        runtime["selection_anchor"],
        runtime["selection_cursor"]
    ]))?;
    endpoints.sort_unstable();
    check_eq!(
        endpoints,
        ALL,
        "the backing replacement still spans all four digits"
    );
    Ok(snapshot)
}

async fn preedit_snapshot(tab: &Tab) -> Result<Value> {
    editor_focus(tab).await?;
    let editor = editing_expression();
    tab.eval(&format!(
        "(() => {{ const t = {TEXTAREA};
             return {{runtime:{editor},textarea_selection:[t.selectionStart,t.selectionEnd]}};
        }})()"
    ))
    .await
}

async fn cancel_preedit(tab: &Tab) -> Result<()> {
    let composing = format!("({})?.composing === true", editing_expression());
    tab.press("Escape").await?;
    if tab.eval(&composing).await? == true {
        tab.ime_compose("", 0, 0).await?;
    }
    tab.wait_for(&format!("!({composing})")).await?;
    editor_focus(tab).await?;
    popup_state(
        tab,
        "d.open && d.quantity.raw === '1680' && d.quantity.value === 1200 &&
         d.quantity.commits === 0 && d.unit.id === 'ml' && d.unit.commits === 0",
    )
    .await
}

fn spans(seams: &Seams) -> Result<[Range<usize>; 3]> {
    const INSET: usize = 3;
    let parts = [
        seams.full.start..seams.middle.start,
        seams.middle.clone(),
        seams.middle.end..seams.full.end,
    ];
    check!(
        parts.iter().all(|range| range.len() >= 2 * INSET + 12),
        "the independently measured preedit spans have physical room for the three ink probes"
    );
    Ok(parts.map(|range| range.start + INSET..range.end - INSET))
}

fn stroke_row(
    clip: &Clip,
    spans: &[Range<usize>; 3],
    plain: &CanvasPng,
    composing: &CanvasPng,
    palette: Palette,
) -> Result<(usize, [usize; 3])> {
    for y in clip.y.clone().rev() {
        let mut counts = [0; 3];
        for (columns, count) in spans.iter().zip(&mut counts) {
            for x in columns.clone() {
                if plain.pixel(x, y)? == palette.canvas
                    && composing.pixel(x, y)? == palette.ordinary_ink
                {
                    *count += 1;
                }
            }
        }
        // The references display identical glyphs and a collapsed final
        // caret. Dense foreground added below the run across all three
        // regions identifies the actual underline. No authored Y guess.
        if spans
            .iter()
            .zip(counts)
            .all(|(range, count)| count >= 12 && count * 5 >= range.len() * 4)
        {
            return Ok((y, counts));
        }
    }
    anyhow::bail!("no resolvable 1 dp WebGPU underline row in all three collapsed-preedit regions")
}

fn count_ink(image: &CanvasPng, y: usize, span: Range<usize>, pigment: Rgb) -> Result<usize> {
    let mut count = 0;
    for x in span {
        count += usize::from(image.pixel(x, y)? == pigment);
    }
    Ok(count)
}

async fn journey(env: &Env, tab: &Tab, url: &str) -> Result<Value> {
    popup(tab, url).await?;
    tab.shortcut("a").await?;
    tab.insert_text(RAW).await?;
    popup_state(
        tab,
        "d.open && d.quantity.raw === '1688' && d.quantity.commits === 0",
    )
    .await?;
    let (palette, css) = forced(tab, Lane::Browser).await?;
    check!(
        palette.ordinary_ink != palette.selected_ink,
        "Chrome system keywords must yield distinguishable ink for this real-color proof"
    );

    let face = bounds(tab, FACE).await?;
    // Small physical inset; all samples come from the actual editor bounds.
    let [x, y, w, h] = face;
    let edge = |v: f64| (v * SCALE).round() as usize;
    let clip = Clip {
        x: edge(x + 7.)..edge(x + w - 7.),
        y: edge(y + 7.)..edge(y + h - 7.),
    };
    let plain_clear = ordinary(env, tab, "plain-clear", [TEXT_END, TEXT_END]).await?;
    let plain_all = ordinary(env, tab, "plain-all", ALL).await?;
    let plain_prefix = ordinary(env, tab, "plain-prefix", PREFIX).await?;
    let plain_suffix = ordinary(env, tab, "plain-suffix", SUFFIX).await?;
    let plain_partial = ordinary(env, tab, "plain-partial", MIDDLE).await?;
    let refs = References {
        ordinary: &plain_clear,
        all: &plain_all,
        prefix: &plain_prefix,
        suffix: &plain_suffix,
    };
    let seams = refs.seams(&clip, palette)?;
    let spans = spans(&seams)?;

    let clear = begin_preedit(tab, [TEXT_END, TEXT_END]).await?;
    let preedit_clear = capture(env, tab, "preedit-clear").await?;
    cancel_preedit(tab).await?;
    media(tab, false, false).await?;
    contrast(tab, "normal").await?;
    let partial = begin_preedit(tab, MIDDLE).await?;
    let normal_before = capture(env, tab, "preedit-normal-before").await?;
    let (_, forced_css) = forced(tab, Lane::Browser).await?;
    check_eq!(
        forced_css["pairs"],
        css["pairs"],
        "the same forced palette is restored"
    );
    check_eq!(
        preedit_snapshot(tab).await?,
        partial,
        "live Forced preserves preedit and selection"
    );
    let preedit_partial = capture(env, tab, "preedit-partial").await?;
    media(tab, false, false).await?;
    contrast(tab, "normal").await?;
    check_eq!(
        preedit_snapshot(tab).await?,
        partial,
        "back in Normal, preedit and selection are unchanged"
    );
    let normal_after = capture(env, tab, "preedit-normal-restored").await?;
    let mut normal_pixel_mismatches = 0;
    for y in clip.y.clone() {
        for x in clip.x.clone() {
            normal_pixel_mismatches +=
                usize::from(normal_before.pixel(x, y)? != normal_after.pixel(x, y)?);
        }
    }
    check_eq!(
        bounds(tab, FACE).await?,
        face,
        "preedit keeps measured editor bounds"
    );

    let (underline_y, clear_ink) =
        stroke_row(&clip, &spans, &plain_clear, &preedit_clear, palette)?;
    // The DOM selection alone cannot prove the bridge gave Runtime a
    // noncollapsed display selection. Require its observed fill, from the
    // independent ordinary partial reference, well away from the underline.
    check!(
        seams.axis_row.abs_diff(underline_y) >= 3,
        "the independent highlight row lies away from the actual underline"
    );
    let mut highlighted_columns = 0;
    for x in spans[1].clone() {
        let expected = plain_partial.pixel(x, seams.axis_row)?;
        let selected = preedit_partial.pixel(x, seams.axis_row)?;
        if matches_fill(expected, seams.expected_fill) && selected == expected {
            highlighted_columns += 1;
        }
    }
    check!(
        highlighted_columns >= 12 && highlighted_columns * 2 >= spans[1].len(),
        "real preedit selection paint matches the independent expected Highlight band"
    );

    let ordinary_ink = [
        count_ink(
            &preedit_partial,
            underline_y,
            spans[0].clone(),
            palette.ordinary_ink,
        )?,
        count_ink(
            &preedit_partial,
            underline_y,
            spans[1].clone(),
            palette.ordinary_ink,
        )?,
        count_ink(
            &preedit_partial,
            underline_y,
            spans[2].clone(),
            palette.ordinary_ink,
        )?,
    ];
    let selected_ink = count_ink(
        &preedit_partial,
        underline_y,
        spans[1].clone(),
        palette.selected_ink,
    )?;
    let report = json!({"source":"CDP synthetic composition; Chrome-resolved CSS colors",
        "device_pixel_ratio":SCALE,"physical_size":preedit_partial.dimensions(),"face":face,
        "expected_ink":{"canvas_text":palette.ordinary_ink,"highlight_text":palette.selected_ink},"system_colors":css,
        "stroke_row":underline_y,"selection_witness_row":seams.axis_row,
        "physical_spans":spans.map(|r| [r.start,r.end]),"clear_outline_pixels":clear_ink,
        "ordinary_ink_partial":ordinary_ink,"selected_ink_middle":selected_ink,
        "highlighted_columns":highlighted_columns,"normal_pixel_mismatches":normal_pixel_mismatches,
        "preedit_clear":clear,"preedit_partial":partial});
    // Preserve the numeric red receipt before evaluating the regression.
    env.write_json(
        "material-web-contrast-preedit-underline-measurements.json",
        &report,
    )?;
    check!(
        ordinary_ink[0] >= clear_ink[0]
            && ordinary_ink[2] >= clear_ink[2]
            && ordinary_ink[1] == 0
            && selected_ink >= clear_ink[1],
        "the same underline uses CanvasText at both flanks and HighlightText in the selected middle:
         ordinary {ordinary_ink:?}, selected {selected_ink}, clear {clear_ink:?}"
    );
    check_eq!(
        normal_pixel_mismatches,
        0,
        "Normal preedit and selection pixels restore exactly"
    );
    cancel_preedit(tab).await?;
    editor_focus(tab).await?;
    check!(
        tab.errors().is_empty(),
        "preedit underline page errors: {:?}",
        tab.errors()
    );
    Ok(report)
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<Value> {
    let tab = chrome.page(960, 864, SCALE).await?;
    tab.cdp("Page.enable", json!({})).await?;
    journey(env, &tab, url)
        .await
        .context("forced-color synthetic preedit underline")
}
