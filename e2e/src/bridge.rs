//! The real jet-bridge CLI and MCP server against the native and WASM
//! material gallery. Port of `scripts/bridge-e2e.py` (B1–B19, BN1–BN5 and
//! BW1–BW5 in issue #62).
//!
//! This journey drives the jet-bridge binary on purpose: it tests the CLI
//! and MCP contract, not only the browser.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::check::step;
use crate::{check, check_eq, Env};

pub const NATIVE_STEPS: &[&str] = &[
    "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12", "B13", "B14", "B15",
    "B16", "B17", "B18", "BN1", "BN2", "BN3", "BN4", "BN5",
];
pub const NATIVE_OVERFLOW_STEPS: &[&str] = &[
    "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12", "B13", "B14", "B15",
    "B16", "B17", "B18", "B19", "BN1", "BN2", "BN3", "BN4", "BN5",
];
pub const WEB_STEPS: &[&str] = &[
    "B1", "B2", "B3", "B4", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12", "B13", "B14", "B15",
    "B16", "B17", "B18", "B19", "BW1", "BW2", "BW3", "BW4", "BW5",
];

const TOOLS: [&str; 7] = [
    "ui_capabilities",
    "ui_snapshot",
    "ui_query",
    "ui_activate",
    "ui_fill",
    "ui_set_value",
    "ui_key",
];

/// The jet-bridge binary, its global arguments and the diagnostic log.
#[derive(Clone)]
struct Bridge {
    bin: PathBuf,
    prefix: Vec<String>,
    diagnostic: PathBuf,
    cwd: PathBuf,
    chrome_path: Option<PathBuf>,
}

impl Bridge {
    fn stderr(&self) -> Result<Stdio> {
        Ok(Stdio::from(append(&self.diagnostic)?))
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.bin);
        command.args(&self.prefix).current_dir(&self.cwd);
        if let Some(chrome) = &self.chrome_path {
            command.env("CHROME_PATH", chrome);
        }
        command.kill_on_drop(true);
        command
    }

    /// Run one CLI command and parse its JSON output.
    async fn cli(&self, args: &[&str]) -> Result<Value> {
        let mut command = self.command();
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(self.stderr()?);
        let output = tokio::time::timeout(Duration::from_secs(40), command.output())
            .await
            .map_err(|_| anyhow!("jet-bridge {args:?} timed out after 40s"))??;
        if !output.status.success() {
            bail!(
                "jet-bridge {args:?} failed with {}; see {}",
                output.status,
                self.diagnostic.display()
            );
        }
        let text = String::from_utf8(output.stdout)?;
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).with_context(|| format!("jet-bridge {args:?} printed {text:?}"))
    }
}

fn append(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))
}

/// MCP client over the server's stdio.
struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Lines<AsyncBufReader<ChildStdout>>,
    sequence: u64,
}

impl Mcp {
    async fn start(bridge: &Bridge, web: bool) -> Result<Self> {
        let mut command = bridge.command();
        command.arg("mcp");
        if web {
            command.arg("--ui");
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(bridge.stderr()?);
        let mut child = command.spawn().context("starting jet-bridge mcp")?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().context("no MCP stdout")?;
        let mut mcp = Self {
            child,
            stdin,
            lines: AsyncBufReader::new(stdout).lines(),
            sequence: 0,
        };
        step(
            "B1",
            "MCP initialize: protocol 2025-06-18, server jet-bridge-ui",
        );
        let hello = mcp
            .request(
                "initialize",
                json!({"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "surface-e2e", "version": "1"}}),
            )
            .await?;
        check_eq!(
            hello["protocolVersion"],
            json!("2025-06-18"),
            "MCP protocol"
        );
        check_eq!(
            hello["serverInfo"]["name"],
            json!("jet-bridge-ui"),
            "MCP server"
        );
        mcp.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .await?;
        step("B2", "tools/list = the seven ui_* tools");
        let tools = mcp.request("tools/list", json!({})).await?;
        let mut names: Vec<String> = tools["tools"]
            .as_array()
            .context("no tools")?
            .iter()
            .filter_map(|t| t["name"].as_str().map(String::from))
            .collect();
        names.sort();
        let mut expected: Vec<String> = TOOLS.iter().map(|s| s.to_string()).collect();
        expected.sort();
        check_eq!(names, expected, "MCP tools");
        Ok(mcp)
    }

    async fn send(&mut self, message: &Value) -> Result<()> {
        let stdin = self.stdin.as_mut().context("MCP stdin closed")?;
        stdin.write_all(format!("{message}\n").as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.sequence += 1;
        let id = self.sequence;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await?;
        let line = tokio::time::timeout(Duration::from_secs(20), self.lines.next_line())
            .await
            .map_err(|_| anyhow!("MCP response to {method} timed out"))??
            .context("MCP server closed its output")?;
        let response: Value = serde_json::from_str(&line)?;
        check!(
            response["id"] == id && response.get("error").is_none(),
            "bad MCP response to {method}: {response}"
        );
        Ok(response["result"].clone())
    }

    async fn call(&mut self, name: &str, args: Value) -> Result<Value> {
        self.request("tools/call", json!({"name": name, "arguments": args}))
            .await
    }

    async fn tool(&mut self, name: &str, args: Value) -> Result<Value> {
        let result = self.call(name, args).await?;
        check!(result["isError"] != true, "{name} failed: {result}");
        let text = result["content"][0]["text"]
            .as_str()
            .context("no tool text")?;
        Ok(serde_json::from_str(text)?)
    }

    async fn tool_error(&mut self, name: &str, args: Value, error: &str) -> Result<()> {
        let result = self.call(name, args).await?;
        let text = result["content"][0]["text"].as_str().unwrap_or("");
        check!(
            result["isError"] == true && text.contains(error),
            "{name} should fail with {error}: {result}"
        );
        Ok(())
    }

    async fn close(mut self) -> Result<()> {
        drop(self.stdin.take());
        let status = match tokio::time::timeout(Duration::from_secs(10), self.child.wait()).await {
            Ok(status) => status?,
            Err(_) => {
                self.child.kill().await.ok();
                bail!("MCP server did not exit within 10s");
            }
        };
        check!(status.success(), "MCP server exited with {status}");
        Ok(())
    }
}

fn node<'a>(snapshot: &'a Value, id: &str) -> Result<&'a Value> {
    crate::ime::find_node(snapshot, id)
}

/// Whether a node's bounds lie inside its clip, as the old `fully_visible`.
pub fn fully_visible(snapshot: &Value, id: &str) -> Result<bool> {
    let index = snapshot["dom"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().position(|n| n["stable_id"] == id))
        .with_context(|| format!("no node {id}"))?;
    let geometry = &snapshot["layout"]["nodes"][index];
    let f = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
    let (b, c) = (&geometry["bounds"], &geometry["clip"]);
    Ok(f(&b["x"]) >= f(&c["x"]) - 0.5
        && f(&b["y"]) >= f(&c["y"]) - 0.5
        && f(&b["x"]) + f(&b["width"]) <= f(&c["x"]) + f(&c["width"]) + 0.5
        && f(&b["y"]) + f(&b["height"]) <= f(&c["y"]) + f(&c["height"]) + 0.5)
}

async fn journey(
    env: &Env,
    bridge: &Bridge,
    client: &mut Mcp,
    mode: &str,
    require_overflow: bool,
) -> Result<Value> {
    step(
        "B3",
        "ui_capabilities: runtime surface, schema surface.dom.v1",
    );
    let caps = client.tool("ui_capabilities", json!({})).await?;
    check!(
        caps["runtime"] == "surface" && caps["snapshot_schema"] == "surface.dom.v1",
        "capabilities: {caps}"
    );

    step("B4", "CLI snapshot: version, nodes, layout, root width");
    let first = bridge.cli(&["ui", "snapshot"]).await?;
    let nodes = first["dom"]["nodes"].as_array().map(Vec::len).unwrap_or(0);
    check!(
        first["version"] == 1 && nodes > 10,
        "snapshot version {} with {nodes} nodes",
        first["version"]
    );
    check_eq!(
        first["layout"]["nodes"].as_array().map(Vec::len),
        Some(nodes),
        "missing current geometry"
    );
    check!(
        first["layout"]["nodes"][0]["bounds"]["width"].as_f64() > Some(0.0),
        "root width is zero"
    );
    let initially_clipped = !fully_visible(&first, "volume")?;
    if require_overflow {
        step("B19", "the range starts clipped");
        check!(
            initially_clipped,
            "{mode} fixture did not exercise scrolling"
        );
    }

    step(
        "B5",
        "Tab reaches volume without a cycle; range then visible",
    );
    let mut visited: Vec<String> = Vec::new();
    let mut reached = false;
    for _ in 0..20 {
        bridge.cli(&["ui", "key", "tab"]).await?;
        let frame = client.tool("ui_snapshot", json!({})).await?;
        let focused = frame["interaction"]["focused"]
            .as_str()
            .unwrap_or("")
            .to_string();
        check!(
            !visited.contains(&focused),
            "Tab cycled before reaching volume: {visited:?}"
        );
        visited.push(focused.clone());
        if focused == "volume" {
            check!(
                fully_visible(&frame, "volume")?,
                "focused range is still clipped"
            );
            reached = true;
            break;
        }
    }
    check!(reached, "Tab did not reach the range control: {visited:?}");

    step("B6", "ui_set_value volume 67 → 65");
    client
        .tool("ui_set_value", json!({"id": "volume", "value": 67}))
        .await?;
    let snap = bridge.cli(&["ui", "snapshot"]).await?;
    // Python compared numbers, so 65 and 65.0 are equal.
    check_eq!(
        node(&snap, "volume")?["range"]["value"].as_f64(),
        Some(65.0),
        "snapped range"
    );

    step("B7", "CLI activate check → checked");
    bridge.cli(&["ui", "activate", "check"]).await?;
    let snap = client.tool("ui_snapshot", json!({})).await?;
    check_eq!(node(&snap, "check")?["checked"], json!(true), "checked");

    step("B8", "removed node → missing_node");
    client
        .tool("ui_activate", json!({"id": "section-1"}))
        .await?;
    client
        .tool_error(
            "ui_set_value",
            json!({"id": "volume", "value": 25}),
            "missing_node",
        )
        .await?;

    step("B9", "activate filled; disabled action rejected");
    client.tool("ui_activate", json!({"id": "filled"})).await?;
    let snap = bridge.cli(&["ui", "snapshot"]).await?;
    check_eq!(
        node(&snap, "click-count")?["text"],
        json!("已執行 1 次"),
        "click count"
    );
    client
        .tool_error("ui_activate", json!({"id": "disabled-action"}), "disabled")
        .await?;
    let snap = client.tool("ui_snapshot", json!({})).await?;
    check_eq!(
        node(&snap, "click-count")?["text"],
        json!("已執行 1 次"),
        "click count after disabled"
    );

    step(
        "B10",
        "open dialog; query TextInput 名稱 → dialog-name only",
    );
    bridge.cli(&["ui", "activate", "section-2"]).await?;
    client
        .tool("ui_activate", json!({"id": "open-dialog"}))
        .await?;
    let result = client
        .tool("ui_query", json!({"role": "TextInput", "name": "名稱"}))
        .await?;
    let matches = result["matches"].as_array().cloned().unwrap_or_default();
    check!(
        matches.len() == 1 && matches[0]["node"]["stable_id"] == "dialog-name",
        "query result: {result}"
    );

    let text = "中文橋接🦀";
    step("B11", "CLI fill Chinese text; UTF-8 cursor");
    bridge.cli(&["ui", "fill", "dialog-name", text]).await?;
    let editing = client.tool("ui_snapshot", json!({})).await?;
    let input = &editing["text_inputs"]["dialog-name"];
    check_eq!(input["value"], json!(text), "filled value");
    check_eq!(
        input["selection_cursor"],
        json!(text.len()),
        "cursor in UTF-8 bytes"
    );

    step("B12", "background control → outside_modal");
    client
        .tool_error("ui_activate", json!({"id": "open-menu"}), "outside_modal")
        .await?;

    step("B13", "Tab loop over MCP and CLI: cancel → save → name");
    client.tool("ui_key", json!({"key": "tab"})).await?;
    let snap = bridge.cli(&["ui", "snapshot"]).await?;
    check_eq!(
        snap["interaction"]["focused"],
        json!("dialog-cancel"),
        "focus"
    );
    bridge.cli(&["ui", "key", "tab"]).await?;
    let snap = client.tool("ui_snapshot", json!({})).await?;
    check_eq!(
        snap["interaction"]["focused"],
        json!("dialog-save"),
        "focus"
    );
    client.tool("ui_key", json!({"key": "tab"})).await?;
    let snap = bridge.cli(&["ui", "snapshot"]).await?;
    check_eq!(
        snap["interaction"]["focused"],
        json!("dialog-name"),
        "focus"
    );

    step("B14", "save → saved name; focus open-dialog");
    client
        .tool("ui_activate", json!({"id": "dialog-save"}))
        .await?;
    let saved = bridge.cli(&["ui", "snapshot"]).await?;
    check_eq!(
        node(&saved, "saved-name")?["text"],
        json!(format!("名稱：{text}")),
        "saved name"
    );
    check_eq!(
        saved["interaction"]["focused"],
        json!("open-dialog"),
        "focus restored"
    );

    step("B15", "fill removed dialog-name → missing_node");
    client
        .tool_error(
            "ui_fill",
            json!({"id": "dialog-name", "value": "stale"}),
            "missing_node",
        )
        .await?;

    step("B16", "reopen, fill draft, Escape → saved name kept");
    bridge.cli(&["ui", "activate", "open-dialog"]).await?;
    client
        .tool(
            "ui_fill",
            json!({"id": "dialog-name", "value": "取消的草稿"}),
        )
        .await?;
    bridge.cli(&["ui", "key", "escape"]).await?;
    let last = client.tool("ui_snapshot", json!({})).await?;
    check_eq!(
        node(&last, "saved-name")?["text"],
        json!(format!("名稱：{text}")),
        "saved name after Escape"
    );
    check_eq!(
        last["interaction"]["focused"],
        json!("open-dialog"),
        "focus after Escape"
    );

    step("B17", "unknown tool → invalid application tool arguments");
    client
        .tool_error(
            "ui_not_a_tool",
            json!({}),
            "invalid application tool arguments",
        )
        .await?;
    env.write_json(&format!("bridge-{mode}-dom.json"), &last)?;

    Ok(json!({
        "mode": mode, "tools": 7, "saved_name": text, "focus_restored": true,
        "controlled_range": 65, "stale_node_rejected": true, "disabled_rejected": true,
        "modal_scope_enforced": true, "cli_mcp_share_state": true, "matching_layout": true,
        "tab_reaches_scrolled_controls": true, "tab_order": visited,
        "initial_range_clipped": initially_clipped,
    }))
}

fn temp_folder(mode: &str) -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "surface-bridge-{mode}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub struct NativeOptions<'a> {
    pub name: &'a str,
    pub bin: &'a Path,
    pub require_overflow: bool,
}

pub async fn native(env: &Env, options: NativeOptions<'_>) -> Result<()> {
    let folder = temp_folder(options.name)?;
    let result = native_in(env, &options, &folder).await;
    let _ = std::fs::remove_dir_all(&folder);
    result
}

async fn native_in(env: &Env, options: &NativeOptions<'_>, folder: &Path) -> Result<()> {
    let diagnostic = env.artifact(&format!("{}-stderr.log", options.name));
    File::create(&diagnostic)?;
    let session_file = folder.join("app-session.json");
    let mut app = Command::new(options.bin)
        .current_dir(&env.root)
        .env("SURFACE_BRIDGE_SESSION", &session_file)
        .stdout(Stdio::null())
        .stderr(Stdio::from(append(&diagnostic)?))
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("starting {}", options.bin.display()))?;
    let bridge = Bridge {
        bin: env.jet_bridge()?,
        prefix: vec!["--app-session".into(), session_file.display().to_string()],
        diagnostic: diagnostic.clone(),
        cwd: env.root.clone(),
        chrome_path: None,
    };
    let outcome = async {
        step("BN1", "session file appears within 30 s with the app pid");
        let deadline = Instant::now() + Duration::from_secs(30);
        let session = loop {
            if let Some(status) = app.try_wait()? {
                bail!("native app exited with {status} before bridge readiness");
            }
            if let Ok(text) = std::fs::read_to_string(&session_file) {
                if let Ok(session) = serde_json::from_str::<Value>(&text) {
                    break session;
                }
            }
            if Instant::now() > deadline {
                bail!("no session file within 30s");
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        check_eq!(
            session["pid"].as_u64(),
            app.id().map(u64::from),
            "session pid"
        );

        step("BN2", "session file mode is 0600");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&session_file)?.permissions().mode() & 0o777;
            check_eq!(mode, 0o600, "session file mode");
        }

        step("BN3", "wrong token over TCP → unauthorized (id 91)");
        let endpoint = session["endpoint"].as_str().context("no endpoint")?;
        let mut stream = std::net::TcpStream::connect(endpoint)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let request = json!({"token": "wrong-token", "request": {
            "version": 1, "id": 91, "command": {"method": "activate", "id": "section-1"}}});
        stream.write_all(format!("{request}\n").as_bytes())?;
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line)?;
        let response: Value =
            serde_json::from_str(&line).with_context(|| format!("TCP response {line:?}"))?;
        check!(
            response["id"] == 91 && response["code"] == "unauthorized",
            "wrong token response: {response}"
        );
        drop(stream);

        let mut client = Mcp::start(&bridge, false).await?;
        let mut evidence = journey(
            env,
            &bridge,
            &mut client,
            options.name,
            options.require_overflow,
        )
        .await?;
        step("B18", "MCP server exits with code 0");
        client.close().await?;

        step("BN4", "session token never appears in diagnostics");
        let token = session["token"].as_str().context("no token")?;
        let log = std::fs::read_to_string(&diagnostic)?;
        check!(
            !token.is_empty() && !log.contains(token),
            "session token was logged"
        );
        evidence["private_descriptor"] = json!(true);
        Ok(evidence)
    }
    .await;
    terminate(&mut app).await;
    let mut evidence = outcome?;

    step("BN5", "--smoke run exits 0 and removes its session file");
    let smoke_file = folder.join("smoke-session.json");
    let status = tokio::time::timeout(
        Duration::from_secs(30),
        Command::new(options.bin)
            .arg("--smoke")
            .current_dir(&env.root)
            .env("SURFACE_BRIDGE_SESSION", &smoke_file)
            .stdout(Stdio::null())
            .stderr(Stdio::from(append(&diagnostic)?))
            .kill_on_drop(true)
            .status(),
    )
    .await
    .map_err(|_| anyhow!("--smoke run did not exit within 30s"))??;
    check!(
        status.success() && !smoke_file.exists(),
        "native shutdown: {status}, descriptor left: {}",
        smoke_file.exists()
    );
    evidence["normal_shutdown_removes_descriptor"] = json!(true);
    env.write_json(&format!("{}-result.json", options.name), &evidence)?;
    println!("  evidence: {evidence}");
    Ok(())
}

/// SIGTERM, as Python's `terminate()`, then a hard kill after 10 s.
async fn terminate(child: &mut Child) {
    if let Some(pid) = child.id() {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
    if tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .is_err()
    {
        let _ = child.kill().await;
    }
}

fn alive(pid: u64) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A `CHROME_PATH` target that adds browser arguments. `bb launch` takes no
/// browser arguments (see the jet-bridge gap issue), so the wrapper adds
/// them and `exec`s Chrome, which keeps the reported pid Chrome's own.
pub fn chrome_wrapper(folder: &Path, chrome: &Path, args: &[String]) -> Result<PathBuf> {
    if args.is_empty() {
        return Ok(chrome.to_path_buf());
    }
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let mut script = String::from("#!/bin/sh\nexec ");
    script.push_str(&quote(&chrome.display().to_string()));
    for arg in args {
        script.push(' ');
        script.push_str(&quote(arg));
    }
    script.push_str(" \"$@\"\n");
    let path = folder.join("chrome");
    std::fs::write(&path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(path)
}

pub struct WebOptions<'a> {
    pub chrome: &'a Path,
    pub chrome_args: &'a [String],
    pub require_overflow: bool,
}

pub async fn web(env: &Env, options: WebOptions<'_>) -> Result<()> {
    let folder = temp_folder("web")?;
    let result = web_in(env, &options, &folder).await;
    let _ = std::fs::remove_dir_all(&folder);
    result
}

async fn web_in(env: &Env, options: &WebOptions<'_>, folder: &Path) -> Result<()> {
    let diagnostic = env.artifact("bridge-web-stderr.log");
    File::create(&diagnostic)?;
    let server = crate::server::StaticServer::start(&env.bridge_web_dir())?;
    let root = folder.join("root");
    std::fs::create_dir_all(&root)?;
    let bridge = Bridge {
        bin: env.jet_bridge()?,
        prefix: vec!["--root".into(), root.display().to_string()],
        diagnostic,
        cwd: env.root.clone(),
        chrome_path: Some(chrome_wrapper(folder, options.chrome, options.chrome_args)?),
    };
    let mut owned: Option<u64> = None;
    let outcome = async {
        step("BW1", "bb launch; #status reaches Ready within 40 s");
        let launched = bridge.cli(&["bb", "launch", &server.url()]).await?;
        owned = launched["pid"].as_u64();
        check!(owned.is_some(), "bb launch returned no pid: {launched}");
        let deadline = Instant::now() + Duration::from_secs(40);
        let mut status = String::new();
        while Instant::now() < deadline {
            status = bridge
                .cli(&[
                    "bb",
                    "eval",
                    "document.querySelector('#status').textContent",
                ])
                .await?
                .as_str()
                .unwrap_or("")
                .to_string();
            if status.starts_with("Ready") {
                break;
            }
            if status.starts_with("Surface error:") || status.starts_with("Error:") {
                bail!("{status}");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        check!(status.starts_with("Ready"), "status: {status:?}");

        step("BW2", "window.__jet_bridge.version = 1");
        check_eq!(
            bridge
                .cli(&["bb", "eval", "window.__jet_bridge.version"])
                .await?,
            json!(1),
            "bridge version"
        );

        step("BW3", "version 999 → unsupported_version (id 92)");
        let unsupported = bridge
            .cli(&[
                "bb",
                "eval",
                "window.__jet_bridge.request({version:999,id:92,command:{method:\"snapshot\"}})",
            ])
            .await?;
        check!(
            unsupported["id"] == 92 && unsupported["code"] == "unsupported_version",
            "unsupported version response: {unsupported}"
        );

        let mut client = Mcp::start(&bridge, true).await?;
        let mut evidence =
            journey(env, &bridge, &mut client, "web", options.require_overflow).await?;

        step("BW4", "bb screenshot writes a PNG");
        let png = env.artifact("bridge-web.png");
        let _ = std::fs::remove_file(&png);
        bridge
            .cli(&["bb", "screenshot", "--out", &png.display().to_string()])
            .await?;
        let bytes = std::fs::read(&png).context("bb screenshot wrote no file")?;
        check!(
            bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "screenshot is not a PNG"
        );
        evidence["webgpu_ready"] = json!(true);

        step("B18", "MCP server exits with code 0");
        client.close().await?;

        step("BW5", "bb shutdown → owned browser exits within 10 s");
        bridge.cli(&["bb", "shutdown"]).await?;
        let pid = owned.context("no owned pid")?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while alive(pid) && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        check!(!alive(pid), "shutdown ACK did not prove owned browser exit");
        owned = None;
        evidence["owned_browser_exited"] = json!(true);
        Ok::<Value, anyhow::Error>(evidence)
    }
    .await;
    if let Some(pid) = owned {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
    drop(server);
    let evidence = outcome?;
    env.write_json("bridge-web-result.json", &evidence)?;
    println!("  evidence: {evidence}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(bounds: (f64, f64, f64, f64), clip: (f64, f64, f64, f64)) -> Value {
        json!({
            "dom": {"nodes": [{"stable_id": "root"}, {"stable_id": "volume"}]},
            "layout": {"nodes": [{}, {
                "bounds": {"x": bounds.0, "y": bounds.1, "width": bounds.2, "height": bounds.3},
                "clip": {"x": clip.0, "y": clip.1, "width": clip.2, "height": clip.3},
            }]},
        })
    }

    #[test]
    fn visibility_matches_the_python_rule() {
        let clip = (0.0, 0.0, 400.0, 300.0);
        assert!(fully_visible(&snapshot((10.0, 10.0, 100.0, 20.0), clip), "volume").unwrap());
        assert!(fully_visible(&snapshot((-0.4, 0.0, 400.4, 300.0), clip), "volume").unwrap());
        assert!(!fully_visible(&snapshot((10.0, 290.0, 100.0, 20.0), clip), "volume").unwrap());
        assert!(fully_visible(&snapshot((0.0, 0.0, 1.0, 1.0), clip), "missing").is_err());
    }

    #[test]
    fn wrapper_quotes_arguments() {
        let dir = std::env::temp_dir().join(format!("surface-e2e-wrapper-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let chrome = Path::new("/opt/My Chrome/chrome");
        assert_eq!(chrome_wrapper(&dir, chrome, &[]).unwrap(), chrome);
        let path = chrome_wrapper(&dir, chrome, &["--flag=it's".to_string()]).unwrap();
        let script = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            script,
            "#!/bin/sh\nexec '/opt/My Chrome/chrome' '--flag=it'\\''s' \"$@\"\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
