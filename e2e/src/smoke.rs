//! Showcase pointer, keyboard, theme, resize and scroll journey.
//! Port of `scripts/browser-smoke.mjs` (S1–S16 in issue #62).

use anyhow::Result;
use serde_json::json;

use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

pub const STEPS: &[&str] = &[
    "S1", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "S11", "S12", "S13", "S14", "S15",
    "S16", "S17", "S18", "S19",
];

/// Click the center of a semantic control, as the old `click(id)`.
pub async fn click(tab: &Tab, app: &str, id: &str) -> Result<()> {
    tab.wait_for(&format!(
        "window.{app}.control_bounds({}).length === 4",
        js(id)
    ))
    .await?;
    let r = tab
        .eval_f64s(&format!("window.{app}.control_bounds({})", js(id)))
        .await?;
    check_eq!(r.len(), 4, format!("semantic control {id}"));
    let c = tab.canvas_box().await?;
    tab.click(c[0] + r[0] + r[2] / 2.0, c[1] + r[1] + r[3] / 2.0)
        .await
}

pub async fn ready(tab: &Tab, global: &str) -> Result<()> {
    let status = "document.querySelector('#status')?.textContent ?? ''";
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    crate::check::count();
    loop {
        let text = tab.eval_str(status).await?;
        if text.starts_with("Ready") && tab.eval(&format!("!!window.{global}")).await? == true {
            return Ok(());
        }
        if text.starts_with("Surface error:") || text.starts_with("Error:") {
            anyhow::bail!("app failed to start: {text}");
        }
        if std::time::Instant::now() > deadline {
            anyhow::bail!("#status did not reach Ready within 45s: {text:?}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

pub async fn run(env: &Env, chrome: &Chrome) -> Result<()> {
    let (_server, url) = env.serve("showcase")?;
    let tab = chrome.page(1000, 900, 2.0).await?;
    tab.goto(&url).await?;
    let inspect = "window.showcase.inspect()";

    step("S1", "#status reaches Ready within 45 s");
    ready(&tab, "showcase").await?;

    step("S2", "navigator.gpu exists");
    check!(
        tab.eval("!!navigator.gpu").await? == true,
        "navigator.gpu is missing"
    );

    step("S3", "root width matches canvas; buffer is CSS width × DPR");
    tab.wait_for(
        "(() => { const canvas = document.querySelector('canvas');
          const root = window.showcase.control_bounds('surface-page');
          return root.length === 4 && Math.abs(root[2] - canvas.clientWidth) <= 1
            && canvas.width === Math.round(canvas.clientWidth * devicePixelRatio); })()",
    )
    .await?;

    step("S4", "click increment → count=1");
    click(&tab, "showcase", "increment").await?;
    tab.wait_for(&format!("{inspect}.startsWith('count=1;')"))
        .await?;

    step("S5", "Enter → count=2");
    tab.press("Enter").await?;
    tab.wait_for(&format!("{inspect}.startsWith('count=2;')"))
        .await?;

    step("S6", "Tab → reset, Tab → name");
    tab.press("Tab").await?;
    tab.wait_for(&format!("{inspect}.endsWith('focused=reset')"))
        .await?;
    tab.press("Tab").await?;
    tab.wait_for(&format!("{inspect}.endsWith('focused=name')"))
        .await?;

    step("S7", "type Chris, Backspace → Chri");
    tab.type_text("Chris").await?;
    tab.wait_for(&format!("{inspect}.includes('name=Chris;')"))
        .await?;
    tab.press("Backspace").await?;
    tab.wait_for(&format!("{inspect}.includes('name=Chri;')"))
        .await?;

    step("S8", "click dark-mode → dark=true");
    click(&tab, "showcase", "dark-mode").await?;
    tab.wait_for(&format!("{inspect}.includes('dark=true;')"))
        .await?;

    step("S9", "click disabled, two frames → still count=2");
    click(&tab, "showcase", "disabled").await?;
    tab.settle().await?;
    let state = tab.eval_str(inspect).await?;
    check!(
        state.starts_with("count=2;"),
        "disabled control changed state: {state}"
    );

    step("S10", "screenshot browser-showcase-wide.png");
    tab.screenshot(&env.artifact("browser-showcase-wide.png"))
        .await?;

    step("S11", "resize to 480×600 → root width 480");
    tab.viewport(480, 600).await?;
    tab.wait_for("window.showcase.control_bounds('surface-page')[2] === 480")
        .await?;

    step("S12", "click increment → count=3");
    click(&tab, "showcase", "increment").await?;
    tab.wait_for(&format!("{inspect}.startsWith('count=3;')"))
        .await?;

    step("S13", "wheel 350 at (200,450) → title moves up");
    tab.wait_for("window.showcase.control_bounds('title').length === 4")
        .await?;
    let before = tab
        .eval_f64s("window.showcase.control_bounds('title')")
        .await?[1];
    tab.mouse_move(200.0, 450.0).await?;
    tab.wheel(0.0, 350.0).await?;
    tab.wait_for(&format!(
        "window.showcase.control_bounds('title')[1] < {before}"
    ))
    .await?;
    tab.settle().await?;

    step("S14", "no page errors and no console errors");
    let errors = tab.errors();
    check!(errors.is_empty(), "page errors:\n{}", errors.join("\n"));

    step("S15", "#status still starts with Ready");
    let status = tab
        .eval_str("document.querySelector('#status').textContent")
        .await?;
    check!(status.starts_with("Ready"), "status changed: {status}");

    step("S16", "screenshot and evidence JSON");
    tab.screenshot(&env.artifact("browser-showcase.png"))
        .await?;
    let adapter = tab
        .eval(
            "(async () => { const adapter = await navigator.gpu.requestAdapter();
              return adapter ? { vendor: adapter.info.vendor, architecture: adapter.info.architecture,
                device: adapter.info.device, description: adapter.info.description,
                isFallbackAdapter: adapter.info.isFallbackAdapter } : null; })()",
        )
        .await?;
    check!(!adapter.is_null(), "no WebGPU adapter");
    let evidence = json!({
        "browser": chrome.version,
        "driver": "jet-bridge CDP",
        "state": tab.eval_str(inspect).await?,
        "errors": errors,
        "adapter": adapter,
        "viewport": tab.viewport_size().await?,
    });
    env.write_json("browser-smoke.json", &evidence)?;
    println!("  evidence: {evidence}");

    // Out-of-band write (issue #102, migration step 12): a JS timer writes
    // state and wakes the host. No input event is involved.
    step("S17", "navigator.gpu asserted before the timer write");
    check!(
        tab.eval("!!navigator.gpu").await? == true,
        "navigator.gpu is missing"
    );
    let before_shot = env.artifact("browser-wake-before.png");
    tab.screenshot(&before_shot).await?;
    let before_dom = tab.eval_str("window.showcase.export_dom()").await?;
    check!(
        !before_dom.contains("Hello, Timer."),
        "greeting already changed"
    );

    step(
        "S18",
        "setTimeout write wakes the host: name and DOM change",
    );
    tab.eval("setTimeout(() => window.showcase.write_from_timer('Timer'), 50); true")
        .await?;
    tab.wait_for(&format!("{inspect}.includes('name=Timer;')"))
        .await?;
    tab.wait_for("window.showcase.export_dom().includes('Hello, Timer.')")
        .await?;
    // A later wake must also arrive: the host cleared the pending flag.
    tab.eval("setTimeout(() => window.showcase.write_from_timer('Timer2'), 50); true")
        .await?;
    tab.wait_for(&format!("{inspect}.includes('name=Timer2;')"))
        .await?;
    tab.wait_for("window.showcase.export_dom().includes('Hello, Timer2.')")
        .await?;

    step(
        "S19",
        "the canvas repainted: screenshot differs, no page errors",
    );
    tab.settle().await?;
    let after_shot = env.artifact("browser-wake-after.png");
    tab.screenshot(&after_shot).await?;
    check!(
        std::fs::read(&before_shot)? != std::fs::read(&after_shot)?,
        "the canvas did not change after the timer write"
    );
    let errors = tab.errors();
    check!(errors.is_empty(), "page errors:\n{}", errors.join("\n"));
    Ok(())
}
