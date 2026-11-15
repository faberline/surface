//! `devtools-native`: the Surface devtools protocol over the native loopback
//! transport (issue #131, O7).
//!
//! Starts the native material gallery with `SURFACE_DEVTOOLS_SESSION`, reads
//! the session file and speaks raw newline-delimited JSON over TCP. There is
//! no client binary. The jet bridge runs next to it, with its own session
//! file. No real key or mouse input is sent.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

use crate::check::step;
use crate::{check, check_eq, Env};

pub const STEPS: &[&str] = &[
    "DN1", "DN2", "DN3", "DN4", "DN5", "DN6", "DN7", "DN8", "DN9", "DN10",
];

const NAME: &str = "devtools-native";

pub async fn run(env: &Env, bin: &Path) -> Result<()> {
    let folder = temp_folder()?;
    let result = run_in(env, bin, &folder).await;
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

async fn run_in(env: &Env, bin: &Path, folder: &Path) -> Result<()> {
    let diagnostic = env.artifact(&format!("{NAME}-stderr.log"));
    File::create(&diagnostic)?;
    let devtools_file = folder.join("devtools-session.json");
    let bridge_file = folder.join("bridge-session.json");
    let mut app = Command::new(bin)
        .current_dir(&env.root)
        .env("SURFACE_DEVTOOLS_SESSION", &devtools_file)
        .env("SURFACE_BRIDGE_SESSION", &bridge_file)
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
        for method in ["DOM.snapshot", "Action.activate"] {
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
        drop(client);

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
        }))
    }
    .await;
    terminate(&mut app).await;
    let mut evidence: Value = outcome?;

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

fn temp_folder() -> Result<std::path::PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("surface-{NAME}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
