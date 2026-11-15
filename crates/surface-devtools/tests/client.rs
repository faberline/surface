//! Unit tests of the surface-devtools client (issue #132, O8).
//!
//! Row numbers refer to the Acceptance table of the issue.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Cursor, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use surface_devtools::cli::{parse_args, Command};
use surface_devtools::connection::{request_line, ClientError, Connection, NativeTcp};
use surface_devtools::mcp::{tool_name, McpServer};
use surface_devtools::session::{pid_alive, Session, SessionError};
use surface_devtools::{run_with, EXIT_CLIENT, EXIT_PROTOCOL, EXIT_USAGE};
use surface_devtools_protocol::{ErrorCode, Reply, Request};

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("surface-devtools-test-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn session_text(endpoint: &str, pid: u32) -> String {
    json!({"protocol":"surface-devtools","version":1,"endpoint":endpoint,
        "token":"t".repeat(64),"pid":pid,"application":"Test App"})
    .to_string()
}

fn write_session(dir: &std::path::Path, endpoint: &str, pid: u32) -> PathBuf {
    let file = dir.join("session.json");
    std::fs::write(&file, session_text(endpoint, pid)).unwrap();
    file
}

/// A pid that is certainly not running: a child that has been reaped.
fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

/// A loopback endpoint that refuses connections.
fn closed_endpoint() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    addr.to_string()
}

/// Records requests and answers from a script.
struct Fake {
    requests: Vec<Request>,
    replies: Vec<Reply>,
}
impl Fake {
    fn new(replies: Vec<Reply>) -> Self {
        Self {
            requests: Vec::new(),
            replies,
        }
    }
}
impl Connection for Fake {
    fn call(&mut self, request: &Request) -> Result<Reply, ClientError> {
        self.requests.push(request.clone());
        let mut reply = self.replies.remove(0);
        reply.id = request.id;
        Ok(reply)
    }
}

fn capabilities_reply() -> Reply {
    Reply::ok(
        0,
        json!({"version":1,"domains":["Devtools","DOM","Action"],
            "methods":["Devtools.capabilities","DOM.snapshot","DOM.query",
                "Action.activate","Action.fill","Action.setValue","Action.key","Future.thing"],
            "limits":{"request_bytes":1048576,"response_bytes":16777216,"ring_size":4096},
            "schemas":["surface.dom.v1"],"application":"Test App"}),
    )
}

// ---- argument parsing ------------------------------------------------

fn call_of(args: &[&str]) -> (String, Value) {
    match parse_args(&strings(args)).unwrap().command {
        Command::Call { method, params } => (method, params),
        other => panic!("not a call: {other:?}"),
    }
}

#[test]
fn args_map_commands_to_methods() {
    assert_eq!(
        call_of(&["capabilities"]),
        ("Devtools.capabilities".into(), json!({}))
    );
    assert_eq!(
        call_of(&["devtools", "capabilities"]),
        ("Devtools.capabilities".into(), json!({}))
    );
    assert_eq!(
        call_of(&["dom", "snapshot"]),
        ("DOM.snapshot".into(), json!({}))
    );
    assert_eq!(
        call_of(&["dom", "query", "--role", "Button", "--name", "Save", "--id", "x"]),
        (
            "DOM.query".into(),
            json!({"role":"Button","name":"Save","id":"x"})
        )
    );
    assert_eq!(
        call_of(&["action", "activate", "check"]),
        ("Action.activate".into(), json!({"id":"check"}))
    );
    assert_eq!(
        call_of(&["action", "fill", "field", "hello world"]),
        ("Action.fill".into(), json!({"id":"field","value":"hello world"}))
    );
    assert_eq!(
        call_of(&["action", "set-value", "volume", "0.5"]),
        ("Action.setValue".into(), json!({"id":"volume","value":0.5}))
    );
    assert_eq!(
        call_of(&["action", "key", "tab"]),
        (
            "Action.key".into(),
            json!({"key":"tab","extend":false,"by_word":false})
        )
    );
    assert_eq!(
        call_of(&["action", "key", "left", "--extend", "--by-word"]),
        (
            "Action.key".into(),
            json!({"key":"left","extend":true,"by_word":true})
        )
    );
}

#[test]
fn args_session_option_and_mcp() {
    let parsed = parse_args(&strings(&["--session", "/tmp/s.json", "dom", "snapshot"])).unwrap();
    assert_eq!(parsed.session.as_deref(), Some("/tmp/s.json"));
    let parsed = parse_args(&strings(&["dom", "snapshot", "--session", "/tmp/s.json"])).unwrap();
    assert_eq!(parsed.session.as_deref(), Some("/tmp/s.json"));
    let parsed = parse_args(&strings(&["mcp"])).unwrap();
    assert_eq!(parsed.command, Command::Mcp);
    assert_eq!(parsed.session, None);
}

#[test]
fn args_reject_bad_input() {
    for bad in [
        vec![],
        vec!["nope"],
        vec!["dom"],
        vec!["dom", "nope"],
        vec!["action", "activate"],
        vec!["action", "fill", "only-id"],
        vec!["action", "set-value", "id", "not-a-number"],
        vec!["action", "set-value", "id", "NaN"],
        vec!["action", "key"],
        vec!["action", "activate", "a", "extra"],
        vec!["dom", "query", "--role"],
        vec!["dom", "snapshot", "--session"],
        vec!["dom", "snapshot", "--bogus"],
    ] {
        assert!(parse_args(&strings(&bad)).is_err(), "accepted {bad:?}");
    }
}

// ---- session file ----------------------------------------------------

#[test]
fn session_loads_the_native_file() {
    let dir = temp_dir("load");
    let file = write_session(&dir, "127.0.0.1:4000", std::process::id());
    let session = Session::load(&file).unwrap();
    assert_eq!(session.endpoint, "127.0.0.1:4000");
    assert_eq!(session.token, "t".repeat(64));
    assert_eq!(session.pid, std::process::id());
    assert_eq!(session.application, "Test App");
    assert_eq!(session.file, file);
}

#[test]
fn session_rejects_missing_and_invalid_files() {
    let dir = temp_dir("invalid");
    assert!(matches!(
        Session::load(&dir.join("none.json")),
        Err(SessionError::Missing(_))
    ));
    for text in [
        "not json",
        "{}",
        &session_text("127.0.0.1:1", 1).replace("surface-devtools", "other"),
        &session_text("127.0.0.1:1", 1).replace("\"version\":1", "\"version\":2"),
        &json!({"protocol":"surface-devtools","version":1,"endpoint":"127.0.0.1:1","pid":1,
            "application":"x"})
        .to_string(),
    ] {
        let file = dir.join("bad.json");
        std::fs::write(&file, text).unwrap();
        assert!(
            matches!(Session::load(&file), Err(SessionError::Invalid(..))),
            "accepted {text}"
        );
    }
}

#[test]
fn pid_checks() {
    assert!(pid_alive(std::process::id()));
    assert!(!pid_alive(dead_pid()));
    // A pid that would be a negative number for `kill` must not be probed.
    assert!(!pid_alive(0));
    assert!(!pid_alive(u32::MAX));
}

// ---- wire ------------------------------------------------------------

#[test]
fn request_line_carries_token_and_envelope() {
    let request = Request {
        v: 1,
        id: 7,
        method: "DOM.query".into(),
        params: json!({"role":"Button"}),
    };
    let line = request_line(&request, "secret");
    assert!(!line.contains('\n'));
    let value: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        value,
        json!({"v":1,"id":7,"method":"DOM.query","params":{"role":"Button"},"token":"secret"})
    );
}

/// Accepts one connection, reads request lines, answers `ok` with the id.
fn echo_server() -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut seen = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let value: Value = serde_json::from_str(&line).unwrap();
            let reply = json!({"id":value["id"],"ok":true,"result":{"echo":value["method"]}});
            writeln!(writer, "{reply}").unwrap();
            seen.push(line.trim_end().to_string());
        }
        seen
    });
    (endpoint, handle)
}

fn session_for(endpoint: &str, file: PathBuf) -> Session {
    Session {
        endpoint: endpoint.into(),
        token: "secret-token".into(),
        pid: std::process::id(),
        application: "Test App".into(),
        file,
    }
}

#[test]
fn native_tcp_sends_the_token_and_reads_replies() {
    let (endpoint, server) = echo_server();
    let session = session_for(&endpoint, PathBuf::from("/tmp/x.json"));
    let mut conn = NativeTcp::connect(&session).unwrap();
    for id in [1, 2] {
        let reply = conn
            .call(&Request {
                v: 1,
                id,
                method: "DOM.snapshot".into(),
                params: json!({}),
            })
            .unwrap();
        assert_eq!(reply.id, id);
        assert!(reply.ok);
    }
    drop(conn);
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 2);
    for line in seen {
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap()["token"], "secret-token");
    }
}

#[test]
fn native_tcp_refuses_a_non_loopback_endpoint() {
    // Negative control: the token goes only to the loopback endpoint.
    for endpoint in ["10.0.0.1:9000", "example.com:80", "localhost:9000", "[::2]:80", "nonsense"] {
        let session = session_for(endpoint, PathBuf::from("/tmp/x.json"));
        assert!(
            matches!(NativeTcp::connect(&session), Err(ClientError::Session(_))),
            "connected to {endpoint}"
        );
    }
}

// ---- row 3: stale session -------------------------------------------

fn run_cli(args: &[&str], env: Option<String>) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with(
        &strings(args),
        env,
        &mut Cursor::new(Vec::new()),
        &mut out,
        &mut err,
        &mut |session| Ok(Box::new(NativeTcp::connect(session)?) as Box<dyn Connection>),
    );
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn stale_session_with_a_dead_pid_names_the_file() {
    let dir = temp_dir("stale-pid");
    let file = write_session(&dir, &closed_endpoint(), dead_pid());
    let (code, out, err) = run_cli(&["--session", file.to_str().unwrap(), "capabilities"], None);
    assert_eq!(code, EXIT_CLIENT);
    assert_eq!(out, "");
    assert!(err.contains(file.to_str().unwrap()), "stderr: {err}");
    assert!(err.contains("not running"), "stderr: {err}");
}

#[test]
fn stale_session_with_a_refused_connection_names_the_file() {
    let dir = temp_dir("stale-refused");
    // The pid is alive (this test), the endpoint is closed.
    let file = write_session(&dir, &closed_endpoint(), std::process::id());
    let (code, out, err) = run_cli(&["dom", "snapshot"], Some(file.to_str().unwrap().into()));
    assert_eq!(code, EXIT_CLIENT);
    assert_eq!(out, "");
    assert!(err.contains(file.to_str().unwrap()), "stderr: {err}");
    assert!(err.contains("not running"), "stderr: {err}");
}

#[test]
fn a_dead_pid_is_not_connected_to() {
    // The dead-pid check runs before any connection, so a live listener at
    // the endpoint sees no connection.
    let dir = temp_dir("stale-no-connect");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let file = write_session(&dir, &endpoint, dead_pid());
    let (code, _, _) = run_cli(&["--session", file.to_str().unwrap(), "capabilities"], None);
    assert_eq!(code, EXIT_CLIENT);
    std::thread::sleep(Duration::from_millis(100));
    assert!(listener.accept().is_err(), "the client connected anyway");
}

#[test]
fn missing_session_is_a_usage_error() {
    let (code, _, err) = run_cli(&["dom", "snapshot"], None);
    assert_eq!(code, EXIT_USAGE);
    assert!(err.contains("SURFACE_DEVTOOLS_SESSION"), "stderr: {err}");
    let (code, _, err) = run_cli(&["--session", "/nonexistent/s.json", "dom", "snapshot"], None);
    assert_eq!(code, EXIT_CLIENT);
    assert!(err.contains("/nonexistent/s.json"), "stderr: {err}");
}

#[test]
fn a_submitted_action_is_not_retried() {
    // The server reads the request and closes without a reply. The client
    // fails and does not open a second connection.
    let dir = temp_dir("no-retry");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let file = write_session(&dir, &endpoint, std::process::id());
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut extra = 0;
        while Instant::now() < deadline {
            if listener.accept().is_ok() {
                extra += 1;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        (line, extra)
    });
    let (code, out, err) = run_cli(
        &["--session", file.to_str().unwrap(), "action", "activate", "check"],
        None,
    );
    assert_eq!(code, EXIT_CLIENT);
    assert_eq!(out, "");
    assert!(!err.is_empty());
    let (line, extra) = server.join().unwrap();
    assert!(line.contains("Action.activate"), "server saw {line:?}");
    assert_eq!(extra, 0, "the client retried the action");
}

// ---- exit codes and output ------------------------------------------

fn run_fake(args: &[&str], replies: Vec<Reply>) -> (i32, String, String, Vec<Request>) {
    let dir = temp_dir("fake");
    let file = write_session(&dir, "127.0.0.1:1", std::process::id());
    let mut out = Vec::new();
    let mut err = Vec::new();
    let captured = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = captured.clone();
    let code = run_with(
        &strings(args),
        Some(file.to_str().unwrap().into()),
        &mut Cursor::new(Vec::new()),
        &mut out,
        &mut err,
        &mut |_| {
            struct Recording(Fake, std::rc::Rc<std::cell::RefCell<Vec<Request>>>);
            impl Connection for Recording {
                fn call(&mut self, request: &Request) -> Result<Reply, ClientError> {
                    self.1.borrow_mut().push(request.clone());
                    self.0.call(request)
                }
            }
            Ok(Box::new(Recording(Fake::new(replies.clone()), sink.clone())) as Box<dyn Connection>)
        },
    );
    let requests = captured.borrow().clone();
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        requests,
    )
}

#[test]
fn ok_reply_prints_the_result_json_and_exits_zero() {
    let (code, out, err, requests) = run_fake(
        &["action", "activate", "check"],
        vec![Reply::ok(0, json!({"changed":true}))],
    );
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), json!({"changed":true}));
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "Action.activate");
    assert_eq!(requests[0].params, json!({"id":"check"}));
    assert_eq!(requests[0].v, 1);
}

#[test]
fn protocol_error_exits_non_zero_and_prints_the_code_on_stderr() {
    // Negative control: a CLI that exits 0 here must fail this test.
    let (code, out, err, _) = run_fake(
        &["action", "activate", "nope"],
        vec![Reply::error(0, ErrorCode::MissingNode, "no current node")],
    );
    assert_eq!(code, EXIT_PROTOCOL);
    assert_ne!(code, 0);
    assert_eq!(out, "");
    assert!(err.contains("missing_node"), "stderr: {err}");
}

#[test]
fn usage_error_exits_two() {
    let (code, _, err) = run_cli(&["action", "fill", "only-id"], None);
    assert_eq!(code, EXIT_USAGE);
    assert!(!err.is_empty());
}

// ---- row 2: MCP framing ----------------------------------------------

fn rpc(server: &mut McpServer, conn: &mut dyn Connection, request: Value) -> Value {
    server
        .handle_line(conn, &request.to_string())
        .expect("a response")
}

#[test]
fn mcp_initialize() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![]);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"t","version":"1"}}}),
    );
    assert_eq!(reply["jsonrpc"], "2.0");
    assert_eq!(reply["id"], 1);
    assert!(reply["result"]["protocolVersion"].is_string());
    assert_eq!(reply["result"]["serverInfo"]["name"], "surface-devtools");
    assert!(reply["result"]["capabilities"]["tools"].is_object());
    assert!(fake.requests.is_empty(), "initialize must not touch the app");
}

#[test]
fn mcp_notifications_get_no_response() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![]);
    let line = json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string();
    assert!(server.handle_line(&mut fake, &line).is_none());
}

#[test]
fn mcp_tools_list_has_one_tool_per_method_with_generated_descriptions() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![capabilities_reply()]);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    );
    assert_eq!(fake.requests.len(), 1);
    assert_eq!(fake.requests[0].method, "Devtools.capabilities");
    let tools = reply["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "devtools_capabilities",
            "dom_snapshot",
            "dom_query",
            "action_activate",
            "action_fill",
            "action_set_value",
            "action_key",
            "future_thing"
        ]
    );
    for tool in tools {
        let description = tool["description"].as_str().unwrap();
        assert!(!description.is_empty());
        assert_eq!(tool["inputSchema"]["type"], "object");
    }
    let activate = tools.iter().find(|t| t["name"] == "action_activate").unwrap();
    assert!(activate["description"].as_str().unwrap().contains("Action.activate"));
    assert_eq!(activate["inputSchema"]["required"], json!(["id"]));
    let future = tools.iter().find(|t| t["name"] == "future_thing").unwrap();
    assert!(future["description"].as_str().unwrap().contains("Future.thing"));
    // Capabilities are cached: a second list does not ask the app again.
    rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
    );
    assert_eq!(fake.requests.len(), 1);
}

#[test]
fn mcp_tools_call_forwards_one_request() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![
        capabilities_reply(),
        Reply::ok(0, json!({"changed":true})),
    ]);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":"abc","method":"tools/call","params":{
            "name":"action_set_value","arguments":{"id":"volume","value":0.25}}}),
    );
    assert_eq!(reply["id"], "abc");
    assert_eq!(reply["result"]["isError"], false);
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(serde_json::from_str::<Value>(text).unwrap(), json!({"changed":true}));
    let call = fake.requests.last().unwrap();
    assert_eq!(call.method, "Action.setValue");
    assert_eq!(call.params, json!({"id":"volume","value":0.25}));
}

#[test]
fn mcp_protocol_error_is_a_tool_error() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![
        capabilities_reply(),
        Reply::error(0, ErrorCode::MissingNode, "no current node"),
    ]);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
            "name":"action_activate","arguments":{"id":"nope"}}}),
    );
    assert_eq!(reply["result"]["isError"], true);
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("missing_node"), "text: {text}");
}

#[test]
fn mcp_unknown_tool_is_not_sent_to_the_app() {
    // Negative control: only methods in Devtools.capabilities are forwarded.
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![capabilities_reply()]);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
            "name":"trace_read","arguments":{}}}),
    );
    assert_eq!(reply["error"]["code"], -32602);
    assert_eq!(fake.requests.len(), 1, "only the capabilities request");
}

#[test]
fn mcp_json_rpc_errors() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![]);
    let reply = server.handle_line(&mut fake, "{not json").unwrap();
    assert_eq!(reply["error"]["code"], -32700);
    assert_eq!(reply["id"], Value::Null);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":9,"method":"no/such"}),
    );
    assert_eq!(reply["error"]["code"], -32601);
    assert_eq!(reply["id"], 9);
    let reply = rpc(
        &mut server,
        &mut fake,
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{}}),
    );
    assert_eq!(reply["error"]["code"], -32602);
}

#[test]
fn mcp_serve_frames_one_line_per_message() {
    let mut server = McpServer::new();
    let mut fake = Fake::new(vec![capabilities_reply()]);
    let input = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}).to_string(),
        String::new(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
    ]
    .join("\n");
    let mut out = Vec::new();
    server
        .serve(&mut fake, &mut Cursor::new(input.into_bytes()), &mut out)
        .unwrap();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "output: {text}");
    assert_eq!(serde_json::from_str::<Value>(lines[0]).unwrap()["id"], 1);
    assert_eq!(serde_json::from_str::<Value>(lines[1]).unwrap()["id"], 2);
    assert!(text.ends_with('\n'));
}

#[test]
fn tool_names_are_snake_case() {
    assert_eq!(tool_name("Devtools.capabilities"), "devtools_capabilities");
    assert_eq!(tool_name("DOM.snapshot"), "dom_snapshot");
    assert_eq!(tool_name("Action.setValue"), "action_set_value");
    assert_eq!(tool_name("Page.screenshot"), "page_screenshot");
}
