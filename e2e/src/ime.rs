//! Showcase IME journey through CDP composition events.
//! Port of `scripts/browser-ime.mjs` (I1–I19 in issue #62).

use anyhow::Result;
use serde_json::json;

use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub const STEPS: &[&str] = &[
    "I1", "I2", "I3", "I4", "I5", "I6", "I7", "I8", "I9", "I10", "I11", "I12", "I13", "I14", "I15",
    "I16", "I17", "I18", "I19",
];

const STATE: &str = "window.showcase.editing_state()";

async fn wait_value(tab: &Tab, value: &str) -> Result<()> {
    tab.wait_for(&format!("{STATE}[0] === {}", js(value))).await
}

async fn composing(tab: &Tab, on: bool) -> Result<()> {
    tab.wait_for(&format!("{STATE}[4] === '{on}'")).await
}

async fn compose(tab: &Tab, text: &str) -> Result<()> {
    let n = text.encode_utf16().count();
    tab.ime_compose(text, n, n).await
}

async fn click_at(tab: &Tab, canvas: [f64; 4], bounds: &[f64]) -> Result<()> {
    tab.click(canvas[0] + bounds[0] + 24.0, canvas[1] + bounds[1] + 24.0)
        .await
}

pub async fn run(env: &Env, chrome: &Chrome) -> Result<()> {
    let (_server, url) = env.serve("showcase")?;
    let tab = chrome.page(1000, 900, 2.0).await?;
    tab.goto(&url).await?;

    step("I1", "Ready; click name → textarea focused");
    crate::smoke::ready(&tab, "showcase").await?;
    let bounds = tab
        .eval_f64s("window.showcase.control_bounds('name')")
        .await?;
    check_eq!(bounds.len(), 4, "name bounds");
    let canvas = tab.canvas_box().await?;
    click_at(&tab, canvas, &bounds).await?;
    tab.wait_for("document.activeElement?.id === 'surface-text-input'")
        .await?;

    step("I2", "record composition and input events");
    tab.eval(
        "(() => { window.imeEvents = [];
          for (const type of ['compositionstart','compositionupdate','compositionend','beforeinput','input'])
            document.querySelector('#surface-text-input').addEventListener(type, e =>
              window.imeEvents.push({type, data: e.data, inputType: e.inputType, isComposing: e.isComposing}));
          return true; })()",
    )
    .await?;

    step("I3", "compose ㄓㄨㄥ → preedit, composing, empty value");
    compose(&tab, "ㄓㄨㄥ").await?;
    tab.wait_for(&format!("{STATE}[1] === 'ㄓㄨㄥ' && {STATE}[4] === 'true'"))
        .await?;
    check_eq!(
        tab.eval_str(&format!("{STATE}[0]")).await?,
        "",
        "value during preedit"
    );

    step("I4", "compose 中文 → preedit; screenshot");
    compose(&tab, "中文").await?;
    tab.wait_for(&format!("{STATE}[1] === '中文'")).await?;
    tab.screenshot(&env.artifact("browser-ime-preedit.png"))
        .await?;

    step("I5", "DOM export during preedit");
    let export = tab.eval_json("window.showcase.export_dom()").await?;
    check_eq!(export["version"], json!(1), "export version");
    let name = &export["text_inputs"]["name"];
    check_eq!(name["value"], json!(""), "preedit value");
    check_eq!(
        name["display_value"],
        json!("中文"),
        "preedit display value"
    );
    check_eq!(name["composing"], json!(true), "preedit composing");
    let node = find_node(&export, "name")?;
    check_eq!(node["text"], json!(""), "name node text during preedit");
    check!(
        export["layout"]["nodes"].as_array().map(Vec::len)
            == export["dom"]["nodes"].as_array().map(Vec::len),
        "layout count differs from node count"
    );

    step("I6", "textarea position matches input_caret()");
    let candidate = tab
        .eval(
            "(() => { const area = document.querySelector('#surface-text-input').getBoundingClientRect();
              const canvas = document.querySelector('canvas').getBoundingClientRect();
              const caret = Array.from(window.showcase.input_caret());
              return {left: area.left - canvas.left, top: area.top - canvas.top, caret}; })()",
        )
        .await?;
    let caret = candidate["caret"].as_array().cloned().unwrap_or_default();
    check_eq!(caret.len(), 4, "input_caret length");
    let num = |v: &serde_json::Value| v.as_f64().unwrap_or(f64::NAN);
    check!(
        (num(&candidate["left"]) - num(&caret[0])).abs() <= 1.0,
        "textarea left {} vs caret {}",
        candidate["left"],
        caret[0]
    );
    check!(
        (num(&candidate["top"]) - num(&caret[1])).abs() <= 1.0,
        "textarea top {} vs caret {}",
        candidate["top"],
        caret[1]
    );

    step("I7", "commit 中文");
    tab.insert_text("中文").await?;
    wait_value(&tab, "中文").await?;
    composing(&tab, false).await?;

    step("I8", "setSelectionRange(0,1) → bytes 0..3");
    tab.eval("document.querySelector('#surface-text-input').setSelectionRange(0, 1)")
        .await?;
    tab.wait_for(&format!("{STATE}[2] === '0' && {STATE}[3] === '3'"))
        .await?;

    step("I9", "compose and commit 台 → 台文");
    compose(&tab, "台").await?;
    tab.insert_text("台").await?;
    wait_value(&tab, "台文").await?;

    step("I10", "compose ㄅㄆ then cancel → value kept");
    compose(&tab, "ㄅㄆ").await?;
    composing(&tab, true).await?;
    tab.ime_compose("", 0, 0).await?;
    composing(&tab, false).await?;
    wait_value(&tab, "台文").await?;

    step(
        "I11",
        "select all, commit text with ZWJ emoji and combining mark",
    );
    let mixed = "中👨‍👩‍👧‍👦e\u{301}";
    tab.shortcut("A").await?;
    tab.insert_text(mixed).await?;
    wait_value(&tab, mixed).await?;

    step("I12", "Backspace deletes one grapheme at a time");
    tab.press("Backspace").await?;
    wait_value(&tab, "中👨‍👩‍👧‍👦").await?;
    tab.press("Backspace").await?;
    wait_value(&tab, "中").await?;

    step("I13", "undo and redo");
    tab.shortcut("Z").await?;
    wait_value(&tab, "中👨‍👩‍👧‍👦").await?;
    tab.shortcut("Shift+Z").await?;
    wait_value(&tab, "中").await?;

    let mixed_script = "繁體中文、简体中文";
    step("I14", "select all, commit mixed scripts");
    tab.shortcut("A").await?;
    tab.insert_text(mixed_script).await?;
    wait_value(&tab, mixed_script).await?;

    step(
        "I15",
        "compose ㄅ then click increment → composition cancelled",
    );
    compose(&tab, "ㄅ").await?;
    composing(&tab, true).await?;
    tab.wait_for("window.showcase.control_bounds('increment').length === 4")
        .await?;
    let increment = tab
        .eval_f64s("window.showcase.control_bounds('increment')")
        .await?;
    click_at(&tab, canvas, &increment).await?;
    composing(&tab, false).await?;
    check_eq!(
        tab.eval_str(&format!("{STATE}[0]")).await?,
        mixed_script,
        "value after focus loss"
    );

    step("I16", "click name again, select all, commit; screenshot");
    tab.wait_for("window.showcase.control_bounds('name').length === 4")
        .await?;
    click_at(&tab, canvas, &bounds).await?;
    tab.wait_for("document.activeElement?.id === 'surface-text-input'")
        .await?;
    tab.shortcut("A").await?;
    tab.insert_text(mixed_script).await?;
    wait_value(&tab, mixed_script).await?;
    tab.screenshot(&env.artifact("browser-ime-committed.png"))
        .await?;

    step("I17", "final DOM export");
    let export = tab.eval_json("window.showcase.export_dom()").await?;
    check_eq!(
        find_node(&export, "name")?["text"],
        json!(mixed_script),
        "name node text"
    );
    check_eq!(
        export["text_inputs"]["name"]["value"],
        json!(mixed_script),
        "value"
    );
    check_eq!(
        export["text_inputs"]["name"]["composing"],
        json!(false),
        "composing"
    );
    check_eq!(export["interaction"]["focused"], json!("name"), "focused");
    env.write_json("dom-export.json", &export)?;

    step("I18", "events include composition and input");
    let events = tab.eval("window.imeEvents").await?;
    let list = events.as_array().cloned().unwrap_or_default();
    for kind in [
        "compositionstart",
        "compositionupdate",
        "compositionend",
        "input",
    ] {
        check!(
            list.iter().any(|e| e["type"] == kind),
            "no {kind} event recorded"
        );
    }

    step("I19", "no page errors; evidence JSON");
    let errors = tab.errors();
    check!(errors.is_empty(), "page errors:\n{}", errors.join("\n"));
    let evidence = json!({
        "browser": chrome.version,
        "driver": "jet-bridge CDP Input.imeSetComposition / Input.insertText",
        "committed": tab.eval_str(&format!("{STATE}[0]")).await?,
        "candidate": candidate,
        "events": events,
        "errors": errors,
    });
    env.write_json("browser-ime.json", &evidence)?;
    println!(
        "  evidence: committed={} events={}",
        evidence["committed"],
        list.len()
    );
    Ok(())
}

pub fn find_node<'a>(export: &'a serde_json::Value, id: &str) -> Result<&'a serde_json::Value> {
    export["dom"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["stable_id"] == id))
        .ok_or_else(|| anyhow::anyhow!("no node with stable_id {id}"))
}
