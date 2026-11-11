//! Chrome driver on top of `jet_bridge::browser`.
//!
//! jet-bridge launches Chrome, opens pages and sends CDP commands. This
//! module adds what the old Playwright scripts used: viewport and device
//! scale, focus emulation, mouse and key input, IME composition, polling
//! waits and page error collection.

use anyhow::{bail, Context, Result};
use jet_bridge::browser::{Browser, BrowserLauncher, LaunchOptions, Page};
use serde_json::{json, Value};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::keys::{self, Chord};

/// Default wait, as Playwright's `waitForFunction`.
pub const WAIT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(16);

type ErrorLog = Arc<Mutex<Vec<(Option<String>, String)>>>;

pub struct Chrome {
    browser: Option<Browser>,
    errors: ErrorLog,
    drain: tokio::task::JoinHandle<()>,
    pub version: String,
    pub executable: PathBuf,
}

impl Chrome {
    pub async fn launch(executable: Option<PathBuf>, args: Vec<String>) -> Result<Self> {
        let executable = match executable {
            Some(path) => path,
            None => BrowserLauncher::find_system_chrome()
                .context("no Chrome found; pass --chrome or set SURFACE_CHROME")?,
        };
        let options = LaunchOptions {
            executable: Some(executable.clone()),
            headless: true,
            args,
            ..LaunchOptions::default()
        };
        let mut browser = Browser::launch(options)
            .await
            .with_context(|| format!("launching {}", executable.display()))?;
        let errors: ErrorLog = Arc::default();
        // The CDP reader blocks when this channel is full, so always drain it.
        let mut events = browser
            .take_event_receiver()
            .context("CDP event stream already taken")?;
        let sink = errors.clone();
        let drain = tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let Some(message) = page_error(&event.method, &event.params) {
                    sink.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push((event.session_id, message));
                }
            }
        });
        let version = browser
            .root_session()
            .send("Browser.getVersion", json!({}))
            .await?["product"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        Ok(Self {
            browser: Some(browser),
            errors,
            drain,
            version,
            executable,
        })
    }

    /// Open a page with a CSS viewport and device scale, as Playwright's
    /// `newPage({viewport, deviceScaleFactor})`.
    pub async fn page(&self, width: u32, height: u32, scale: f64) -> Result<Tab> {
        let browser = self.browser.as_ref().context("browser closed")?;
        let page = browser.new_page().await?;
        let session_id = page
            .session()
            .session_id()
            .context("page has no CDP session id")?
            .to_string();
        let tab = Tab {
            page,
            session_id,
            errors: self.errors.clone(),
            scale,
            mouse: Cell::new((0.0, 0.0)),
            modifiers: Cell::new(0),
        };
        tab.cdp("Runtime.enable", json!({})).await?;
        tab.cdp("Log.enable", json!({})).await?;
        tab.cdp(
            "Emulation.setFocusEmulationEnabled",
            json!({"enabled": true}),
        )
        .await?;
        tab.viewport(width, height).await?;
        Ok(tab)
    }

    pub async fn close(mut self) {
        if let Some(browser) = self.browser.take() {
            let _ = browser.close().await;
        }
        self.drain.abort();
    }
}

/// Text of a page exception, `console.error` call or error log entry.
pub fn page_error(method: &str, params: &Value) -> Option<String> {
    match method {
        "Runtime.exceptionThrown" => {
            let details = &params["exceptionDetails"];
            let text = details["exception"]["description"]
                .as_str()
                .or_else(|| details["text"].as_str())
                .unwrap_or("uncaught exception");
            Some(format!("pageerror: {text}"))
        }
        "Runtime.consoleAPICalled" if params["type"] == "error" => {
            let args = params["args"].as_array().cloned().unwrap_or_default();
            let text = args
                .iter()
                .map(|arg| match &arg["value"] {
                    Value::String(s) => s.clone(),
                    Value::Null => arg["description"].as_str().unwrap_or("").to_string(),
                    other => other.to_string(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            Some(format!("console.error: {text}"))
        }
        "Log.entryAdded" if params["entry"]["level"] == "error" => Some(format!(
            "log error: {} {}",
            params["entry"]["text"].as_str().unwrap_or(""),
            params["entry"]["url"].as_str().unwrap_or("")
        )),
        _ => None,
    }
}

pub struct Tab {
    page: Page,
    session_id: String,
    errors: ErrorLog,
    scale: f64,
    mouse: Cell<(f64, f64)>,
    modifiers: Cell<i64>,
}

impl Tab {
    pub async fn cdp(&self, method: &str, params: Value) -> Result<Value> {
        self.page
            .session()
            .send(method, params)
            .await
            .with_context(|| format!("CDP {method}"))
    }

    pub async fn goto(&self, url: &str) -> Result<()> {
        self.page.goto(url).await
    }

    pub async fn viewport(&self, width: u32, height: u32) -> Result<()> {
        self.cdp(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": self.scale,
                "mobile": false,
                "screenWidth": width,
                "screenHeight": height,
            }),
        )
        .await?;
        Ok(())
    }

    /// Errors seen on this page so far.
    pub fn errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|(session, _)| session.as_deref() == Some(self.session_id.as_str()))
            .map(|(_, text)| text.clone())
            .collect()
    }

    pub async fn eval(&self, expression: &str) -> Result<Value> {
        self.page
            .evaluate(expression)
            .await
            .with_context(|| format!("evaluating `{}`", short(expression)))
    }

    pub async fn eval_str(&self, expression: &str) -> Result<String> {
        match self.eval(expression).await? {
            Value::String(s) => Ok(s),
            other => bail!("`{}` returned {other}, not a string", short(expression)),
        }
    }

    /// `JSON.parse` of a string the page returns.
    pub async fn eval_json(&self, expression: &str) -> Result<Value> {
        let text = self.eval_str(expression).await?;
        serde_json::from_str(&text).with_context(|| format!("parsing `{}`", short(expression)))
    }

    pub async fn eval_f64s(&self, expression: &str) -> Result<Vec<f64>> {
        let value = self.eval(&format!("Array.from({expression})")).await?;
        value
            .as_array()
            .context("not an array")?
            .iter()
            .map(|v| v.as_f64().context("not a number"))
            .collect()
    }

    /// Poll until a JS expression is truthy. Counts as one check.
    pub async fn wait_for(&self, expression: &str) -> Result<()> {
        self.wait_for_within(expression, WAIT).await
    }

    pub async fn wait_for_within(&self, expression: &str, timeout: Duration) -> Result<()> {
        crate::check::count();
        let probe = format!("!!({expression})");
        let deadline = Instant::now() + timeout;
        loop {
            if self.eval(&probe).await? == Value::Bool(true) {
                return Ok(());
            }
            if Instant::now() > deadline {
                bail!(
                    "timed out after {}s waiting for `{expression}`",
                    timeout.as_secs()
                );
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// Two animation frames, as the old `settle()`.
    pub async fn settle(&self) -> Result<()> {
        self.eval("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
            .await?;
        Ok(())
    }

    /// The canvas rectangle in CSS pixels: x, y, width, height.
    pub async fn canvas_box(&self) -> Result<[f64; 4]> {
        let b = self
            .eval_f64s("(() => { const r = document.querySelector('canvas').getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; })()")
            .await?;
        Ok([b[0], b[1], b[2], b[3]])
    }

    pub async fn screenshot(&self, path: &Path) -> Result<()> {
        let png = self.page.screenshot().await?;
        std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))
    }

    async fn mouse_event(
        &self,
        kind: &str,
        x: f64,
        y: f64,
        button: &str,
        buttons: i64,
    ) -> Result<()> {
        let mut params = json!({
            "type": kind,
            "x": x,
            "y": y,
            "button": button,
            "buttons": buttons,
            "modifiers": self.modifiers.get(),
        });
        if kind != "mouseMoved" {
            params["clickCount"] = json!(1);
        }
        self.cdp("Input.dispatchMouseEvent", params).await?;
        Ok(())
    }

    pub async fn mouse_move(&self, x: f64, y: f64) -> Result<()> {
        self.mouse.set((x, y));
        self.mouse_event("mouseMoved", x, y, "none", 0).await
    }

    pub async fn mouse_down(&self) -> Result<()> {
        let (x, y) = self.mouse.get();
        self.mouse_event("mousePressed", x, y, "left", 1).await
    }

    pub async fn mouse_up(&self) -> Result<()> {
        let (x, y) = self.mouse.get();
        self.mouse_event("mouseReleased", x, y, "left", 0).await
    }

    pub async fn click(&self, x: f64, y: f64) -> Result<()> {
        self.mouse_move(x, y).await?;
        self.mouse_down().await?;
        self.mouse_up().await
    }

    pub async fn wheel(&self, dx: f64, dy: f64) -> Result<()> {
        let (x, y) = self.mouse.get();
        self.cdp(
            "Input.dispatchMouseEvent",
            json!({"type": "mouseWheel", "x": x, "y": y, "deltaX": dx, "deltaY": dy, "modifiers": self.modifiers.get()}),
        )
        .await?;
        Ok(())
    }

    async fn key_down(&self, key: &keys::Key) -> Result<()> {
        let bit = keys::modifier_bit(&key.key);
        self.modifiers.set(self.modifiers.get() | bit);
        let modifiers = self.modifiers.get();
        // With a modifier other than Shift held, no text is produced.
        let text = if modifiers & !keys::SHIFT != 0 {
            ""
        } else {
            key.text.as_str()
        };
        let commands = keys::mac_commands(&key.code, modifiers);
        self.cdp(
            "Input.dispatchKeyEvent",
            json!({
                "type": if text.is_empty() { "rawKeyDown" } else { "keyDown" },
                "modifiers": modifiers,
                "windowsVirtualKeyCode": key.key_code,
                "code": key.code,
                "key": key.key,
                "text": text,
                "unmodifiedText": text,
                "autoRepeat": false,
                "location": key.location,
                "isKeypad": false,
                "commands": commands,
            }),
        )
        .await?;
        Ok(())
    }

    async fn key_up(&self, key: &keys::Key) -> Result<()> {
        let bit = keys::modifier_bit(&key.key);
        self.modifiers.set(self.modifiers.get() & !bit);
        self.cdp(
            "Input.dispatchKeyEvent",
            json!({
                "type": "keyUp",
                "modifiers": self.modifiers.get(),
                "key": key.key,
                "windowsVirtualKeyCode": key.key_code,
                "code": key.code,
                "location": key.location,
            }),
        )
        .await?;
        Ok(())
    }

    /// Press a key or chord such as `Tab`, `Shift+Tab` or `Meta+A`.
    pub async fn press(&self, combo: &str) -> Result<()> {
        let chord = Chord::parse(combo)?;
        for modifier in &chord.modifiers {
            self.key_down(modifier).await?;
        }
        self.key_down(&chord.key).await?;
        self.key_up(&chord.key).await?;
        for modifier in chord.modifiers.iter().rev() {
            self.key_up(modifier).await?;
        }
        Ok(())
    }

    /// Press the platform's select-all, undo or redo shortcut.
    pub async fn shortcut(&self, rest: &str) -> Result<()> {
        self.press(&format!("{}+{rest}", keys::primary())).await
    }

    /// Type ASCII text one key at a time, as Playwright's `keyboard.type`.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        for c in text.chars() {
            self.press(&c.to_string()).await?;
        }
        Ok(())
    }

    pub async fn ime_compose(&self, text: &str, start: usize, end: usize) -> Result<()> {
        self.cdp(
            "Input.imeSetComposition",
            json!({"text": text, "selectionStart": start, "selectionEnd": end}),
        )
        .await?;
        Ok(())
    }

    pub async fn insert_text(&self, text: &str) -> Result<()> {
        self.cdp("Input.insertText", json!({ "text": text }))
            .await?;
        Ok(())
    }

    pub async fn viewport_size(&self) -> Result<Value> {
        self.eval("({width: innerWidth, height: innerHeight})")
            .await
    }
}

/// Encode a value as a JS literal for use inside an expression.
pub fn js(value: impl serde::Serialize) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "null".into())
}

fn short(expression: &str) -> String {
    let flat: String = expression.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 120 {
        format!("{}…", flat.chars().take(120).collect::<String>())
    } else {
        flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_page_errors_like_playwright() {
        let thrown = json!({"exceptionDetails": {"text": "Uncaught", "exception": {"description": "Error: boom"}}});
        assert_eq!(
            page_error("Runtime.exceptionThrown", &thrown).unwrap(),
            "pageerror: Error: boom"
        );
        let console = json!({"type": "error", "args": [{"type": "string", "value": "bad"}, {"type": "number", "value": 2}]});
        assert_eq!(
            page_error("Runtime.consoleAPICalled", &console).unwrap(),
            "console.error: bad 2"
        );
        let log = json!({"type": "log", "args": []});
        assert!(page_error("Runtime.consoleAPICalled", &log).is_none());
        let entry = json!({"entry": {"level": "error", "text": "Failed to load resource", "url": "http://x/a.js"}});
        assert!(page_error("Log.entryAdded", &entry)
            .unwrap()
            .contains("Failed to load resource"));
        let warning = json!({"entry": {"level": "warning", "text": "slow"}});
        assert!(page_error("Log.entryAdded", &warning).is_none());
    }

    #[test]
    fn encodes_js_literals() {
        assert_eq!(js("a'b\"c"), "\"a'b\\\"c\"");
        assert_eq!(js(3), "3");
    }
}
