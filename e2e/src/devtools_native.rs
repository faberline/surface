//! `devtools-native`: the Surface devtools protocol over the native loopback
//! transport (issue #131, O7).
//!
//! Starts the native material gallery with `SURFACE_DEVTOOLS_SESSION`, reads
//! the session file and speaks raw newline-delimited JSON over TCP. The
//! Jet bridge runs next to it, with its own session
//! file. No real key or mouse input is sent.
//!
//! DN11 to DN14 drive the same app through the `surface-devtools` binary
//! (issue #132, O8): the CLI, MCP over stdio, error exit codes and a stale
//! session file.
//! DN15 to DN17 capture native PNGs over TCP, CLI and MCP (issue #155).
//! The separate `devtools-headless` case checks the same three transports
//! against the Gallery's windowless host.

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

use crate::check::step;
use crate::{check, check_eq, Env};

pub const STEPS: &[&str] = &[
    "DN1", "DN2", "DN3", "DN4", "DN5", "DN6", "DN7", "DN8", "DN9", "DN10", "DN11", "DN12", "DN13",
    "DN14", "DN15", "DN16", "DN17",
];

const NAME: &str = "devtools-native";
const HEADLESS_NAME: &str = "devtools-headless";

pub const HEADLESS_STEPS: &[&str] = &["DH1", "DH2", "DH3"];

pub async fn run(env: &Env, bin: &Path) -> Result<()> {
    let folder = temp_folder(NAME)?;
    let result = run_in(env, bin, &folder).await;
    let _ = std::fs::remove_dir_all(&folder);
    result
}

pub async fn run_headless(env: &Env, bin: &Path) -> Result<()> {
    let folder = temp_folder(HEADLESS_NAME)?;
    let result = run_headless_in(env, bin, &folder).await;
    let _ = std::fs::remove_dir_all(&folder);
    result
}

/// One persistent, authenticated connection.
struct Connection {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    token: String,
}

impl Connection {
    fn open(endpoint: &str, token: &str) -> Result<Self> {
        let stream = TcpStream::connect(endpoint).with_context(|| format!("connect {endpoint}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Self {
            stream,
            reader,
            token: token.into(),
        })
    }

    /// Send one raw line and read one reply line.
    fn line(&mut self, line: &str) -> Result<Value> {
        self.stream.write_all(line.as_bytes())?;
        self.stream.write_all(b"\n")?;
        let mut text = String::new();
        let read = self.reader.read_line(&mut text)?;
        if read == 0 {
            bail!("connection closed before a reply to {line}");
        }
        serde_json::from_str(&text).with_context(|| format!("reply line {text:?}"))
    }

    fn call(&mut self, id: u64, method: &str, params: Value) -> Result<Value> {
        let request =
            json!({"v": 1, "id": id, "method": method, "params": params, "token": self.token});
        let reply = self.line(&request.to_string())?;
        check_eq!(reply["id"].as_u64(), Some(id), "reply id");
        Ok(reply)
    }

    fn ok(&mut self, id: u64, method: &str, params: Value) -> Result<Value> {
        let reply = self.call(id, method, params)?;
        check!(reply["ok"] == true, "{method} failed: {reply}");
        Ok(reply["result"].clone())
    }

    /// Whether the peer closed the connection.
    fn closed(&mut self) -> Result<bool> {
        let mut rest = String::new();
        Ok(self.reader.read_line(&mut rest)? == 0)
    }
}

fn node<'a>(snapshot: &'a Value, id: &str) -> Result<&'a Value> {
    crate::ime::find_node(snapshot, id)
}

async fn wait_for_session(app: &mut Child, path: &Path) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = app.try_wait()? {
            bail!("native app exited with {status} before devtools readiness");
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(session) = serde_json::from_str::<Value>(&text) {
                return Ok(session);
            }
        }
        if Instant::now() > deadline {
            bail!("no session file at {} within 30s", path.display());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn run_headless_in(env: &Env, bin: &Path, folder: &Path) -> Result<()> {
    let session_path = folder.join("devtools-session.json");
    let tool = env.devtools_bin()?;
    let diagnostic = File::create(env.artifact("devtools-headless-stderr.log"))?;
    let mut app = Command::new(bin)
        .args(["--headless", "--width", "720", "--height", "640"])
        .current_dir(&env.root)
        .env("SURFACE_DEVTOOLS_SESSION", &session_path)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .stdout(Stdio::null())
        .stderr(Stdio::from(diagnostic))
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("starting {} --headless", bin.display()))?;
    let outcome = async {
        step(
            "DH1",
            "headless Gallery accepts an authenticated DevTools connection",
        );
        let session = wait_for_session(&mut app, &session_path).await?;
        check_eq!(
            session["pid"].as_u64(),
            app.id().map(u64::from),
            "session pid"
        );
        let endpoint = session["endpoint"].as_str().context("no endpoint")?;
        check!(
            endpoint.starts_with("127.0.0.1:"),
            "session is not on loopback"
        );
        let token = session["token"].as_str().context("no token")?;
        let mut client = Connection::open(endpoint, token)?;
        let caps = client.ok(1, "Devtools.capabilities", json!({}))?;
        check_eq!(caps["version"], json!(1), "capabilities version");
        let methods = caps["methods"]
            .as_array()
            .context("no capabilities methods")?;
        for method in ["DOM.snapshot", "Action.activate", "Page.screenshot"] {
            check!(
                methods.iter().any(|entry| entry == method),
                "capabilities lack {method}"
            );
        }

        step(
            "DH2",
            "an agent action renders a real 720 × 640 PNG without a window",
        );
        let activated = client.ok(2, "Action.activate", json!({"id": "section-1"}))?;
        check_eq!(activated["changed"], json!(true), "activate action section");
        let snapshot = client.ok(3, "DOM.snapshot", json!({}))?;
        node(&snapshot, "filled")?;
        let result = client.ok(4, "Page.screenshot", json!({}))?;
        let (shot, scale) = Screenshot::from_reply(&result)?;
        check_eq!((shot.width, shot.height), (720, 640), "headless PNG size");
        check_eq!(scale, 1.0, "headless screenshot scale");
        shot.verify_viewport(&snapshot, scale)?;
        let background = shot.pixel(1, 1)?;
        shot.verify_color(
            1,
            1,
            &node(&snapshot, "surface-page")?["style"]["background"],
        )?;
        check!(
            shot.pixels.chunks_exact(4).any(|pixel| pixel != background),
            "headless PNG contains only the background color"
        );
        std::fs::write(env.artifact("devtools-headless-screenshot.png"), &shot.png)?;

        step(
            "DH3",
            "stock CLI and MCP return the same headless PNG as raw TCP",
        );
        let result = client.ok(5, "Page.screenshot", json!({"max_side": 400}))?;
        let (scaled, _) = Screenshot::from_reply(&result)?;
        let session_arg = session_path.to_str().context("non-UTF-8 session path")?;
        let mut transcript = String::new();
        let cli_png = screenshot_cli(&tool, session_arg, folder, &mut transcript)?;
        let mcp = mcp_session(&tool, session_arg, &mut transcript)?;
        let mcp_png = screenshot_mcp(&mcp)?;
        check!(scaled.png == cli_png, "raw and CLI headless PNGs differ");
        check!(scaled.png == mcp_png, "raw and MCP headless PNGs differ");
        check!(
            !transcript.contains(token),
            "the client printed the session token"
        );
        Ok(json!({
            "width": shot.width,
            "height": shot.height,
            "scale": scale,
            "capabilities": methods,
            "action_rendered": true,
            "screenshot_transports_match": true,
        }))
    }
    .await;
    terminate(&mut app).await;
    let evidence: Value = outcome?;
    env.write_json("devtools-headless-result.json", &evidence)?;
    println!("  evidence: {evidence}");
    Ok(())
}

async fn run_in(env: &Env, bin: &Path, folder: &Path) -> Result<()> {
    let diagnostic = env.artifact(&format!("{NAME}-stderr.log"));
    File::create(&diagnostic)?;
    let devtools_file = folder.join("devtools-session.json");
    let bridge_file = folder.join("bridge-session.json");
    let mut app = Command::new(bin)
        .current_dir(&env.root)
        .env("SURFACE_DEVTOOLS_SESSION", &devtools_file)
        .env("SURFACE_BRIDGE_SESSION", &bridge_file)
        // The PNG comparison spans separate requests after a checkbox
        // activation. Hold the state static while comparing the transports.
        .env("SURFACE_REDUCED_MOTION", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::from(append(&diagnostic)?))
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("starting {}", bin.display()))?;
    let outcome = async {
        step(
            "DN1",
            "devtools session file appears within 30 s: app pid, 127.0.0.1, mode 0600",
        );
        let session = wait_for_session(&mut app, &devtools_file).await?;
        check_eq!(
            session["pid"].as_u64(),
            app.id().map(u64::from),
            "session pid"
        );
        check_eq!(
            session["protocol"].as_str(),
            Some("surface-devtools"),
            "protocol"
        );
        check_eq!(session["version"].as_u64(), Some(1), "session version");
        let endpoint = session["endpoint"]
            .as_str()
            .context("no endpoint")?
            .to_string();
        check!(
            endpoint.starts_with("127.0.0.1:"),
            "endpoint is not loopback: {endpoint}"
        );
        let token = session["token"].as_str().context("no token")?.to_string();
        check_eq!(token.len(), 64, "token length");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&devtools_file)?.permissions().mode() & 0o777;
            check_eq!(mode, 0o600, "session file mode");
        }

        step(
            "DN2",
            "jet bridge session file coexists with its own endpoint and token",
        );
        let bridge = wait_for_session(&mut app, &bridge_file).await?;
        check!(
            bridge["endpoint"] != session["endpoint"] && bridge["token"] != session["token"],
            "bridge and devtools share an endpoint or token: {bridge} / {session}"
        );

        step(
            "DN3",
            "wrong token and missing token → unauthorized, connection closed",
        );
        for (id, line) in [
            (
                91,
                json!({"v":1,"id":91,"method":"Devtools.capabilities","token":"wrong-token"}),
            ),
            (92, json!({"v":1,"id":92,"method":"Devtools.capabilities"})),
            (93, json!({"v":1,"id":93,"method":"Page.screenshot"})),
        ] {
            let mut bad = Connection::open(&endpoint, "")?;
            let reply = bad.line(&line.to_string())?;
            check!(
                reply["id"] == id
                    && reply["ok"] == false
                    && reply["error"]["code"] == "unauthorized",
                "bad token reply: {reply}"
            );
            check!(bad.closed()?, "connection stayed open after unauthorized");
        }

        let mut client = Connection::open(&endpoint, &token)?;
        step("DN4", "Devtools.capabilities over raw TCP");
        let caps = client.ok(1, "Devtools.capabilities", json!({}))?;
        check_eq!(caps["version"].as_u64(), Some(1), "capabilities version");
        let methods = caps["methods"].as_array().context("no methods")?;
        for method in ["DOM.snapshot", "Action.activate", "Page.screenshot"] {
            check!(
                methods.iter().any(|m| m == method),
                "capabilities lack {method}: {caps}"
            );
        }
        check_eq!(
            caps["limits"]["response_bytes"].as_u64(),
            Some(16 << 20),
            "response cap"
        );
        check_eq!(
            caps["limits"]["request_bytes"].as_u64(),
            Some(1 << 20),
            "request cap"
        );

        step("DN5", "DOM.snapshot on the same connection");
        let before = client.ok(2, "DOM.snapshot", json!({}))?;
        check_eq!(
            before["version"].as_u64(),
            Some(1),
            "surface.dom.v1 version"
        );
        check!(
            before["layout"].is_object(),
            "snapshot has no layout; the host draws before a query"
        );
        let check_before = node(&before, "check")?.clone();

        step(
            "DN6",
            "Action.activate check: mixed → checked → unchecked, seen in the next snapshots",
        );
        let result = client.ok(3, "Action.activate", json!({"id": "check"}))?;
        check_eq!(result["changed"], json!(true), "activate changed");
        let after = client.ok(4, "DOM.snapshot", json!({}))?;
        check_eq!(
            node(&after, "check")?["checked"],
            json!(true),
            "checked after activate"
        );
        check!(
            after["dom"] != before["dom"],
            "DOM did not change after activate (check was {check_before})"
        );
        // Checked → unchecked: a second activate shows a real toggle.
        client.ok(41, "Action.activate", json!({"id": "check"}))?;
        let again = client.ok(42, "DOM.snapshot", json!({}))?;
        check_eq!(
            node(&again, "check")?["checked"],
            json!(false),
            "checked after a second activate"
        );

        step("DN7", "protocol errors keep the connection usable");
        let reply = client.call(5, "Action.activate", json!({"id": "missing-node"}))?;
        check_eq!(
            reply["error"]["code"].as_str(),
            Some("missing_node"),
            "missing node code"
        );
        let reply = client.call(6, "No.such", json!({}))?;
        check_eq!(
            reply["error"]["code"].as_str(),
            Some("unknown_method"),
            "unknown method code"
        );
        client.ok(7, "Devtools.capabilities", json!({}))?;
        let screenshot = screenshot_protocol(&mut client, env)?;
        drop(client);

        let tool = env.devtools_bin()?;
        let stale_copy = folder.join("stale-session.json");
        std::fs::copy(&devtools_file, &stale_copy)?;
        let mut transcripts = String::new();

        step(
            "DN11",
            "the surface-devtools CLI: capabilities, snapshot, query, activate",
        );
        let session_arg = devtools_file.to_string_lossy().to_string();
        let run =
            |transcript: &mut String, args: &[&str]| cli(&tool, &session_arg, args, transcript);
        let (code, out, _) = run(&mut transcripts, &["capabilities"])?;
        check_eq!(code, 0, "cli capabilities exit");
        let caps_cli: Value = serde_json::from_str(&out).context("cli capabilities output")?;
        check!(
            caps_cli["methods"] == caps["methods"],
            "cli capabilities differ from raw TCP: {caps_cli}"
        );
        let (code, out, _) = run(&mut transcripts, &["dom", "snapshot"])?;
        check_eq!(code, 0, "cli snapshot exit");
        let cli_before: Value = serde_json::from_str(&out).context("cli snapshot output")?;
        check_eq!(cli_before["version"].as_u64(), Some(1), "cli dom version");
        check_eq!(
            node(&cli_before, "check")?["checked"],
            json!(false),
            "check before the CLI activate"
        );
        let (code, out, _) = run(&mut transcripts, &["dom", "query", "--id", "check"])?;
        check_eq!(code, 0, "cli query exit");
        let found: Value = serde_json::from_str(&out).context("cli query output")?;
        check_eq!(
            found["matches"].as_array().map(Vec::len),
            Some(1),
            "query matches"
        );
        let (code, out, _) = run(&mut transcripts, &["action", "activate", "check"])?;
        check_eq!(code, 0, "cli activate exit");
        let activated: Value = serde_json::from_str(&out).context("cli activate output")?;
        check_eq!(activated["changed"], json!(true), "cli activate changed");
        let (_, out, _) = run(&mut transcripts, &["dom", "snapshot"])?;
        let cli_after: Value = serde_json::from_str(&out)?;
        check_eq!(
            node(&cli_after, "check")?["checked"],
            json!(true),
            "check after the CLI activate"
        );
        // The session file can also come from the environment.
        let from_env = std::process::Command::new(&tool)
            .env("SURFACE_DEVTOOLS_SESSION", &devtools_file)
            .arg("capabilities")
            .output()?;
        check!(
            from_env.status.success(),
            "capabilities through SURFACE_DEVTOOLS_SESSION failed"
        );

        step(
            "DN12",
            "MCP over stdio: initialize, tools/list, DOM snapshot and screenshot",
        );
        let mcp = mcp_session(&tool, &session_arg, &mut transcripts)?;
        check_eq!(
            mcp[0]["result"]["serverInfo"]["name"].as_str(),
            Some("surface-devtools"),
            "mcp server name"
        );
        let tools = mcp[1]["result"]["tools"].as_array().context("no tools")?;
        for name in [
            "devtools_capabilities",
            "dom_snapshot",
            "dom_query",
            "action_activate",
            "page_screenshot",
        ] {
            check!(
                tools.iter().any(|t| t["name"] == name),
                "tools/list lacks {name}"
            );
        }
        check_eq!(
            tools.len(),
            caps["methods"].as_array().map(Vec::len).unwrap_or(0),
            "one tool per method"
        );
        check_eq!(
            mcp[2]["result"]["isError"],
            json!(false),
            "tools/call isError"
        );
        let text = mcp[2]["result"]["content"][0]["text"]
            .as_str()
            .context("no tool text")?;
        let snapshot: Value = serde_json::from_str(text).context("tool text is JSON")?;
        check_eq!(
            node(&snapshot, "check")?["checked"],
            json!(true),
            "check seen through MCP"
        );

        step(
            "DN13",
            "protocol error exits 1 with the code on stderr; a usage error exits 2",
        );
        let (code, out, errors) = run(&mut transcripts, &["action", "activate", "missing-node"])?;
        check_eq!(code, 1, "protocol error exit code");
        check!(out.is_empty(), "stdout on a protocol error: {out}");
        check!(errors.contains("missing_node"), "stderr: {errors}");
        let (code, _, _) = run(&mut transcripts, &["action", "fill", "only-an-id"])?;
        check_eq!(code, 2, "usage error exit code");

        let mut client = Connection::open(&endpoint, &token)?;
        let raw_result = client.ok(163, "Page.screenshot", json!({"max_side": 400}))?;
        let (raw, _) = Screenshot::from_reply(&raw_result)?;
        step(
            "DN16",
            "CLI screenshot outputs JSON or creates a private PNG once",
        );
        let cli_png = screenshot_cli(&tool, &session_arg, folder, &mut transcripts)?;
        step("DN17", "MCP page_screenshot returns a native PNG image");
        let mcp_png = screenshot_mcp(&mcp)?;
        std::fs::write(
            env.artifact("devtools-native-screenshot-scaled.png"),
            &raw.png,
        )?;
        std::fs::write(env.artifact("devtools-native-screenshot-cli.png"), &cli_png)?;
        std::fs::write(env.artifact("devtools-native-screenshot-mcp.png"), &mcp_png)?;
        check!(
            raw.png == cli_png,
            "raw and CLI PNGs differ for the same frame"
        );
        check!(
            raw.png == mcp_png,
            "raw and MCP PNGs differ for the same frame"
        );
        check!(
            !transcripts.contains(&token),
            "the client printed the session token"
        );

        step("DN8", "session token never appears in diagnostics");
        let log = std::fs::read_to_string(&diagnostic)?;
        check!(!log.contains(&token), "session token was logged");
        env.write_json(&format!("{NAME}-dom.json"), &after)?;
        Ok(json!({
            "endpoint_loopback": true,
            "capabilities": caps["methods"],
            "activate_toggled": true,
            "bridge_coexists": true,
            "unauthorized_closes": true,
            "screenshot": screenshot,
            "screenshot_transports_match": true,
        }))
    }
    .await;
    terminate(&mut app).await;
    let mut evidence: Value = outcome?;

    step(
        "DN14",
        "after the app exits, the CLI reports a stale session file and exits 3",
    );
    let stale_copy = folder.join("stale-session.json");
    let tool = env.devtools_bin()?;
    let mut ignored = String::new();
    let (code, out, errors) = cli(
        &tool,
        &stale_copy.to_string_lossy(),
        &["capabilities"],
        &mut ignored,
    )?;
    check_eq!(code, 3, "stale session exit code");
    check!(out.is_empty(), "stdout on a stale session: {out}");
    check!(
        errors.contains("not running")
            && errors.contains(&stale_copy.to_string_lossy().to_string()),
        "stale session message: {errors}"
    );
    evidence["cli_drove_the_app"] = json!(true);
    evidence["mcp_listed_and_called"] = json!(true);
    evidence["stale_session_exit_3"] = json!(true);

    step("DN9", "--smoke run exits 0 and removes its session file");
    let smoke_file = folder.join("smoke-session.json");
    let status = tokio::time::timeout(
        Duration::from_secs(30),
        Command::new(bin)
            .arg("--smoke")
            .current_dir(&env.root)
            .env("SURFACE_DEVTOOLS_SESSION", &smoke_file)
            .stdout(Stdio::null())
            .stderr(Stdio::from(append(&diagnostic)?))
            .kill_on_drop(true)
            .status(),
    )
    .await
    .map_err(|_| anyhow!("--smoke run did not exit within 30s"))??;
    check!(
        status.success() && !smoke_file.exists(),
        "native shutdown: {status}, session file left: {}",
        smoke_file.exists()
    );
    evidence["normal_shutdown_removes_session"] = json!(true);

    step(
        "DN10",
        "an existing session file is kept: the app fails and the file is unchanged",
    );
    let existing = folder.join("existing-session.json");
    std::fs::write(&existing, "keep me\n")?;
    let status = tokio::time::timeout(
        Duration::from_secs(30),
        Command::new(bin)
            .current_dir(&env.root)
            .env("SURFACE_DEVTOOLS_SESSION", &existing)
            .stdout(Stdio::null())
            .stderr(Stdio::from(append(&diagnostic)?))
            .kill_on_drop(true)
            .status(),
    )
    .await
    .map_err(|_| anyhow!("app with an existing session file did not exit within 30s"))??;
    check!(
        !status.success(),
        "app started over an existing session file: {status}"
    );
    check_eq!(
        std::fs::read_to_string(&existing)?,
        "keep me\n".to_string(),
        "existing session file content"
    );
    evidence["existing_file_kept"] = json!(true);
    env.write_json(&format!("{NAME}-result.json"), &evidence)?;
    println!("  evidence: {evidence}");
    Ok(())
}

/// Run the client with `--session <file> <args>`. Returns exit code, stdout
/// and stderr. Everything it printed is appended to `transcript`.
fn cli(
    tool: &Path,
    session: &str,
    args: &[&str],
    transcript: &mut String,
) -> Result<(i32, String, String)> {
    let output = std::process::Command::new(tool)
        .arg("--session")
        .arg(session)
        .args(args)
        .output()
        .with_context(|| format!("running {}", tool.display()))?;
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    let err = String::from_utf8_lossy(&output.stderr).into_owned();
    transcript.push_str(&out);
    transcript.push_str(&err);
    Ok((output.status.code().unwrap_or(-1), out, err))
}

/// Initialize, list tools and call two methods through `surface-devtools mcp`.
fn mcp_session(tool: &Path, session: &str, transcript: &mut String) -> Result<Vec<Value>> {
    let mut child = std::process::Command::new(tool)
        .args(["--session", session, "mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("starting {} mcp", tool.display()))?;
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"surface-e2e","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
            "name":"dom_snapshot","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
            "name":"page_screenshot","arguments":{"max_side":400}}}),
    ];
    {
        let mut stdin = child.stdin.take().context("no stdin")?;
        for request in requests {
            writeln!(stdin, "{request}")?;
        }
    }
    let output = child.wait_with_output()?;
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    transcript.push_str(&out);
    transcript.push_str(&String::from_utf8_lossy(&output.stderr));
    check!(output.status.success(), "mcp exited with {}", output.status);
    let responses: Vec<Value> = out
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .context("mcp output is one JSON value per line")?;
    check_eq!(
        responses.len(),
        4,
        "mcp responses (a notification gets none)"
    );
    Ok(responses)
}

struct Screenshot {
    png: Vec<u8>,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl Screenshot {
    fn decode(png: Vec<u8>) -> Result<Self> {
        let mut reader = png::Decoder::new(Cursor::new(&png)).read_info()?;
        let mut pixels = vec![0; reader.output_buffer_size().context("PNG is too large")?];
        let frame = reader.next_frame(&mut pixels)?;
        check_eq!(frame.color_type, png::ColorType::Rgba, "PNG color type");
        check_eq!(frame.bit_depth, png::BitDepth::Eight, "PNG bit depth");
        pixels.truncate(frame.buffer_size());
        Ok(Self {
            png,
            pixels,
            width: frame.width,
            height: frame.height,
        })
    }

    fn from_reply(result: &Value) -> Result<(Self, f64)> {
        let data = result["data_base64"]
            .as_str()
            .context("no screenshot data")?;
        let screenshot = Self::decode(STANDARD.decode(data)?)?;
        let scale = screenshot.verify_metadata(result)?;
        Ok((screenshot, scale))
    }

    fn verify_metadata(&self, metadata: &Value) -> Result<f64> {
        check_eq!(metadata["format"], json!("png"), "screenshot format");
        check_eq!(metadata["width"], json!(self.width), "decoded PNG width");
        check_eq!(metadata["height"], json!(self.height), "decoded PNG height");
        let scale = metadata["scale"].as_f64().context("no screenshot scale")?;
        check!(scale.is_finite() && scale > 0.0, "invalid scale {scale}");
        Ok(scale)
    }

    fn verify_viewport(&self, snapshot: &Value, scale: f64) -> Result<()> {
        let viewport = &snapshot["layout"]["viewport"];
        for (axis, pixels) in [("width", self.width), ("height", self.height)] {
            let logical = viewport[axis].as_f64().context("no viewport dimension")?;
            let expected = (logical * scale).round();
            check!(
                (f64::from(pixels) - expected).abs() <= 1.0,
                "PNG {axis} {pixels} does not match viewport {logical} at scale {scale}"
            );
        }
        Ok(())
    }

    fn pixel(&self, x: u32, y: u32) -> Result<&[u8]> {
        check!(
            x < self.width && y < self.height,
            "pixel {x},{y} is outside the PNG"
        );
        let offset = (y as usize * self.width as usize + x as usize) * 4;
        Ok(&self.pixels[offset..offset + 4])
    }

    fn verify_color(&self, x: u32, y: u32, color: &Value) -> Result<()> {
        let expected: [u8; 4] = serde_json::from_value(color.clone()).context("no DOM color")?;
        let actual = self.pixel(x, y)?;
        check!(
            actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1),
            "pixel {x},{y}: expected {expected:?}, got {actual:?}"
        );
        Ok(())
    }
}

fn screenshot_protocol(client: &mut Connection, env: &Env) -> Result<Value> {
    step(
        "DN15",
        "raw Page.screenshot: native viewport, PNG pixels, scale limit and fresh state",
    );
    // The action section includes a filled Material button with distinct R/B
    // bytes. Reading a solid interior pixel catches an incorrect BGRA swizzle.
    client.ok(150, "Action.activate", json!({"id": "section-1"}))?;
    let snapshot = client.ok(151, "DOM.snapshot", json!({}))?;
    let result = client.ok(152, "Page.screenshot", json!({}))?;
    let (light, scale) = Screenshot::from_reply(&result)?;
    light.verify_viewport(&snapshot, scale)?;
    let mut colors = HashSet::new();
    check!(
        light.pixels.chunks_exact(4).any(|pixel| {
            colors.insert(pixel);
            colors.len() >= 8
        }),
        "screenshot has fewer than eight colors"
    );
    let fill = node(&snapshot, "filled")?["first_child"]
        .as_u64()
        .context("filled button has no visual child")? as usize;
    let bounds = &snapshot["layout"]["nodes"][fill]["bounds"];
    let x = bounds["x"].as_f64().context("no button x")?
        + bounds["width"].as_f64().context("no button width")? * 0.2;
    let y = bounds["y"].as_f64().context("no button y")?
        + bounds["height"].as_f64().context("no button height")? * 0.5;
    let color = &snapshot["dom"]["nodes"][fill]["style"]["background"];
    check!(color[0] != color[2], "fill must distinguish red and blue");
    light.verify_color((x * scale) as u32, (y * scale) as u32, color)?;
    light.verify_color(
        1,
        1,
        &node(&snapshot, "surface-page")?["style"]["background"],
    )?;
    std::fs::write(env.artifact("devtools-native-screenshot.png"), &light.png)?;

    let result = client.ok(153, "Page.screenshot", json!({"scale": 1.0}))?;
    let (logical, logical_scale) = Screenshot::from_reply(&result)?;
    check_eq!(logical_scale, 1.0, "explicit screenshot scale");
    logical.verify_viewport(&snapshot, logical_scale)?;

    let result = client.ok(154, "Page.screenshot", json!({"max_side": 400}))?;
    let (small, small_scale) = Screenshot::from_reply(&result)?;
    check_eq!(small.width.max(small.height), 400, "max_side downscales");
    check!(small_scale < scale, "max_side did not reduce the scale");
    small.verify_viewport(&snapshot, small_scale)?;

    let wrong_version = json!({
        "v": 2, "id": 162, "method": "Page.screenshot", "token": client.token,
    });
    let reply = client.line(&wrong_version.to_string())?;
    check_eq!(
        reply["error"]["code"],
        json!("unsupported_version"),
        "screenshot still checks protocol version"
    );
    for (id, params) in [(155, json!({"max_side": 0})), (156, json!({"scale": -1.0}))] {
        let reply = client.call(id, "Page.screenshot", params)?;
        check_eq!(
            reply["error"]["code"],
            json!("invalid_params"),
            "bad screenshot params"
        );
    }
    client.ok(157, "Action.activate", json!({"id": "theme"}))?;
    // Capture first, so a DOM query cannot refresh stale pixels.
    let result = client.ok(158, "Page.screenshot", json!({}))?;
    let (dark, dark_scale) = Screenshot::from_reply(&result)?;
    let dark_snapshot = client.ok(159, "DOM.snapshot", json!({}))?;
    dark.verify_viewport(&dark_snapshot, dark_scale)?;
    dark.verify_color(
        1,
        1,
        &node(&dark_snapshot, "surface-page")?["style"]["background"],
    )?;
    check!(
        light.pixel(1, 1)? != dark.pixel(1, 1)?,
        "screenshot kept the old theme"
    );
    std::fs::write(
        env.artifact("devtools-native-screenshot-toggled.png"),
        &dark.png,
    )?;
    client.ok(160, "Action.activate", json!({"id": "theme"}))?;
    client.ok(161, "Action.activate", json!({"id": "section-0"}))?;
    Ok(json!({
        "format": "png",
        "width": light.width,
        "height": light.height,
        "scale": scale,
        "max_side": small.width.max(small.height),
        "colors_match": true,
        "reflects_theme_change": true,
    }))
}

fn screenshot_cli(
    tool: &Path,
    session: &str,
    folder: &Path,
    transcript: &mut String,
) -> Result<Vec<u8>> {
    let (code, out, _) = cli(
        tool,
        session,
        &["page", "screenshot", "--max-side", "400"],
        transcript,
    )?;
    check_eq!(code, 0, "CLI screenshot JSON exit code");
    let result = serde_json::from_str(&out).context("CLI screenshot JSON")?;
    let (stdout, _) = Screenshot::from_reply(&result)?;
    check_eq!(stdout.width.max(stdout.height), 400, "CLI JSON PNG size");

    let path = folder.join("cli-screenshot.png");
    let dest = path.to_str().context("non-UTF-8 test output path")?;
    let args = ["page", "screenshot", "--max-side", "400", "--out", dest];
    let (code, out, _) = cli(tool, session, &args, transcript)?;
    check_eq!(code, 0, "CLI screenshot file exit code");
    let metadata: Value = serde_json::from_str(&out).context("CLI PNG file metadata")?;
    check_eq!(metadata["path"], json!(dest), "CLI screenshot output path");
    let screenshot = Screenshot::decode(std::fs::read(&path)?)?;
    screenshot.verify_metadata(&metadata)?;
    check_eq!(
        screenshot.width.max(screenshot.height),
        400,
        "CLI file PNG size"
    );
    check!(
        metadata.get("data_base64").is_none(),
        "CLI file response includes image data"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
        check_eq!(mode, 0o600, "PNG file mode");
    }
    let args = ["page", "screenshot", "--max-side", "200", "--out", dest];
    let (code, out, _) = cli(tool, session, &args, transcript)?;
    check_eq!(code, 3, "CLI screenshot refuses an existing path");
    check!(
        out.is_empty(),
        "CLI screenshot reports success on an existing path"
    );
    check_eq!(
        std::fs::read(path)?,
        screenshot.png,
        "PNG kept after refusal"
    );
    check!(
        stdout.png == screenshot.png,
        "CLI JSON and file PNGs differ"
    );
    Ok(screenshot.png)
}

fn screenshot_mcp(mcp: &[Value]) -> Result<Vec<u8>> {
    let tool = mcp[1]["result"]["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "page_screenshot"))
        .context("no MCP screenshot tool")?;
    let properties = &tool["inputSchema"]["properties"];
    check_eq!(
        properties["scale"]["type"],
        json!("number"),
        "MCP scale type"
    );
    check_eq!(
        properties["max_side"]["type"],
        json!("integer"),
        "MCP max_side type"
    );
    let result = &mcp[3]["result"];
    check_eq!(result["isError"], json!(false), "MCP screenshot succeeded");
    let content = result["content"]
        .as_array()
        .context("no MCP screenshot content")?;
    let image = content
        .iter()
        .find(|block| block["type"] == "image")
        .context("no MCP image")?;
    check_eq!(image["mimeType"], json!("image/png"), "MCP image MIME type");
    let encoded = image["data"].as_str().context("no MCP image data")?;
    let screenshot = Screenshot::decode(STANDARD.decode(encoded)?)?;
    let metadata = content
        .iter()
        .find(|block| block["type"] == "text")
        .and_then(|block| block["text"].as_str())
        .context("no MCP image metadata")?;
    screenshot.verify_metadata(&serde_json::from_str(metadata)?)?;
    check_eq!(screenshot.width.max(screenshot.height), 400, "MCP PNG size");
    Ok(screenshot.png)
}

/// SIGTERM, then a hard kill after 10 s.
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

fn append(path: &Path) -> Result<File> {
    Ok(std::fs::OpenOptions::new().append(true).open(path)?)
}

fn temp_folder(name: &str) -> Result<std::path::PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("surface-{name}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
