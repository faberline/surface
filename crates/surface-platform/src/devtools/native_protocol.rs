//! The Surface devtools protocol over loopback TCP. Native only.
//! See `docs/devtools-protocol.md`, section "Native transport".
//!
//! One listener on `127.0.0.1:0`, a 32-byte random token and a session file
//! created once with mode 0600. A connection carries many requests, one
//! newline-delimited JSON line each. Every request line carries the token; a
//! wrong or missing token gets `unauthorized` and the connection closes.
use crate::{HostError, UserEvent};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use surface_devtools_protocol::{
    ErrorCode, Reply, Request, REQUEST_CAP_BYTES, RESPONSE_CAP_BYTES, VERSION,
};
use winit::event_loop::EventLoopProxy;

/// The session file path. Separate from the jet bridge's `SURFACE_BRIDGE_SESSION`.
pub(crate) const ENV: &str = "SURFACE_DEVTOOLS_SESSION";
/// The `protocol` field of the session file.
const PROTOCOL: &str = "surface-devtools";
/// How long a connection thread waits for the event loop.
const HOST_TIMEOUT: Duration = Duration::from_secs(5);
/// A connection with no complete request line for this long is closed.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// How long a write of one reply may take.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
/// Read poll interval, so a connection sees `stop` soon.
const POLL: Duration = Duration::from_millis(200);
/// Open connections at one time. More are refused.
const MAX_CONNECTIONS: usize = 8;

/// The reply channel of one `UserEvent::Devtools`.
pub(crate) struct DevtoolsReply(mpsc::SyncSender<Reply>);
impl DevtoolsReply {
    pub(crate) fn send(self, reply: Reply) {
        let _ = self.0.send(reply);
    }
}

/// What one read of the frame reader returns.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    /// One request line, without its newline (and without a trailing `\r`).
    Line(Vec<u8>),
    /// The peer closed the connection.
    Closed,
    /// The line is longer than the request cap.
    TooLarge,
    /// No complete line within the idle timeout.
    Idle,
    /// The server is stopping.
    Stopped,
    /// A read failed.
    Failed,
}

/// Splits a byte stream into newline-delimited request lines. Bytes after a
/// newline stay in the buffer for the next call.
pub(crate) struct FrameReader {
    buffer: Vec<u8>,
    cap: usize,
}
impl FrameReader {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            buffer: Vec::new(),
            cap,
        }
    }
    /// The stream should have a short read timeout, so `stop` is seen.
    pub(crate) fn next(
        &mut self,
        stream: &mut impl Read,
        stop: &AtomicBool,
        idle: Duration,
    ) -> Frame {
        let deadline = Instant::now() + idle;
        let mut chunk = [0u8; 64 * 1024];
        loop {
            if stop.load(Ordering::Acquire) {
                return Frame::Stopped;
            }
            if let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
                if end > self.cap {
                    return Frame::TooLarge;
                }
                let mut line: Vec<u8> = self.buffer.drain(..=end).collect();
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Frame::Line(line);
            }
            if self.buffer.len() > self.cap {
                return Frame::TooLarge;
            }
            if Instant::now() >= deadline {
                return Frame::Idle;
            }
            match stream.read(&mut chunk) {
                Ok(0) => return Frame::Closed,
                Ok(count) => self.buffer.extend_from_slice(&chunk[..count]),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => return Frame::Failed,
            }
        }
    }
}

/// One reply line with its newline. A reply over `cap` bytes becomes
/// `too_large` with the same id.
pub(crate) fn encode(reply: &Reply, cap: usize) -> Vec<u8> {
    let mut bytes = match serde_json::to_vec(reply) {
        Ok(bytes) if bytes.len() <= cap => bytes,
        Ok(bytes) => serde_json::to_vec(&Reply::error(
            reply.id,
            ErrorCode::TooLarge,
            format!(
                "the reply has {} bytes; the response cap is {cap} bytes",
                bytes.len()
            ),
        ))
        .unwrap_or_default(),
        Err(error) => serde_json::to_vec(&Reply::error(
            reply.id,
            ErrorCode::ActionFailed,
            format!("serialization_failed: {error}"),
        ))
        .unwrap_or_default(),
    };
    bytes.push(b'\n');
    bytes
}

/// Compare without an early exit on the first differing byte.
fn same_token(given: &str, token: &str) -> bool {
    let (given, token) = (given.as_bytes(), token.as_bytes());
    given.len() == token.len()
        && given
            .iter()
            .zip(token)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

/// What to do after one request line.
enum Next {
    Reply(Reply),
    /// Send the reply, then close the connection.
    Close(Reply),
}

/// Check one request line and run it. The token is checked before anything
/// is dispatched.
fn handle_line(line: &[u8], token: &str, dispatch: &impl Fn(Request) -> Reply) -> Next {
    let value: Value = match serde_json::from_slice(line) {
        Ok(value) => value,
        Err(error) => {
            return Next::Reply(Reply::error(
                0,
                ErrorCode::InvalidParams,
                format!("invalid JSON: {error}"),
            ))
        }
    };
    let id = value.get("id").and_then(Value::as_u64);
    let given = value.get("token").and_then(Value::as_str).unwrap_or("");
    if !same_token(given, token) {
        return Next::Close(Reply::error(
            id.unwrap_or(0),
            ErrorCode::Unauthorized,
            "invalid devtools session token",
        ));
    }
    let Some(id) = id else {
        return Next::Reply(Reply::error(
            0,
            ErrorCode::InvalidParams,
            "request needs an integer id",
        ));
    };
    match serde_json::from_value::<Request>(value) {
        Ok(request) => Next::Reply(dispatch(request)),
        Err(error) => Next::Reply(Reply::error(
            id,
            ErrorCode::InvalidParams,
            error.to_string(),
        )),
    }
}

fn write_reply(stream: &mut TcpStream, reply: &Reply) -> bool {
    let bytes = encode(reply, RESPONSE_CAP_BYTES as usize);
    stream
        .write_all(&bytes)
        .and_then(|_| stream.flush())
        .is_ok()
}

/// Serve one connection until it closes, idles, fails or the server stops.
fn serve(
    mut stream: TcpStream,
    token: &str,
    dispatch: &impl Fn(Request) -> Reply,
    stop: &AtomicBool,
) {
    if stream.set_nonblocking(false).is_err()
        || stream.set_read_timeout(Some(POLL)).is_err()
        || stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err()
    {
        return;
    }
    let mut reader = FrameReader::new(REQUEST_CAP_BYTES as usize);
    loop {
        match reader.next(&mut stream, stop, IDLE_TIMEOUT) {
            Frame::Line(line) if line.iter().all(u8::is_ascii_whitespace) => {}
            Frame::Line(line) => match handle_line(&line, token, dispatch) {
                Next::Reply(reply) => {
                    if !write_reply(&mut stream, &reply) {
                        return;
                    }
                }
                Next::Close(reply) => {
                    write_reply(&mut stream, &reply);
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return;
                }
            },
            Frame::TooLarge => {
                let reply = Reply::error(
                    0,
                    ErrorCode::TooLarge,
                    format!("a request line is over the cap of {REQUEST_CAP_BYTES} bytes"),
                );
                write_reply(&mut stream, &reply);
                // The line cannot be resynced. Read the rest for a moment, so
                // the peer gets the reply and not a reset, then close.
                let _ = stream.shutdown(std::net::Shutdown::Write);
                drain(&mut stream, stop);
                return;
            }
            Frame::Closed | Frame::Idle | Frame::Stopped | Frame::Failed => return,
        }
    }
}

fn drain(stream: &mut TcpStream, stop: &AtomicBool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut chunk = [0u8; 64 * 1024];
    while Instant::now() < deadline && !stop.load(Ordering::Acquire) {
        match stream.read(&mut chunk) {
            Ok(0) => return,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return,
        }
    }
}

/// Counts one open connection while alive.
struct Slot(Arc<AtomicUsize>);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(crate) struct Server {
    path: PathBuf,
    token: String,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Server {
    /// Start when `SURFACE_DEVTOOLS_SESSION` is set. Requests go to the event
    /// loop as `UserEvent::Devtools`.
    pub(crate) fn install(
        proxy: EventLoopProxy<UserEvent>,
        application: &str,
    ) -> Result<Option<Self>, HostError> {
        let Some(path) = std::env::var_os(ENV).map(PathBuf::from) else {
            return Ok(None);
        };
        let dispatch = move |request: Request| {
            let id = request.id;
            let (sender, receiver) = mpsc::sync_channel(1);
            let event = UserEvent::Devtools {
                request,
                reply: DevtoolsReply(sender),
            };
            if proxy.send_event(event).is_err() {
                return Reply::error(
                    id,
                    ErrorCode::HostClosed,
                    "the application event loop closed",
                );
            }
            receiver.recv_timeout(HOST_TIMEOUT).unwrap_or_else(|_| {
                Reply::error(
                    id,
                    ErrorCode::HostTimeout,
                    "the application did not reply; a submitted action may already have run",
                )
            })
        };
        Self::start(path, application, dispatch)
            .map(Some)
            .map_err(|error| HostError(format!("native devtools: {error}")))
    }

    /// Bind, write the session file once, then serve on worker threads.
    pub(crate) fn start<D>(
        path: PathBuf,
        application: &str,
        dispatch: D,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        D: Fn(Request) -> Reply + Clone + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|error| format!("session token: {error}"))?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let session = json!({
            "protocol": PROTOCOL,
            "version": VERSION,
            "endpoint": listener.local_addr()?.to_string(),
            "token": token,
            "pid": std::process::id(),
            "application": application,
        });
        let mut options = OpenOptions::new();
        // Never overwrite: an existing file is an error.
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path)?;
        let stop = Arc::new(AtomicBool::new(false));
        // From here on, `Drop` removes the file we created.
        let mut server = Self {
            path,
            token: token.clone(),
            stop: stop.clone(),
            worker: None,
        };
        serde_json::to_writer(&mut file, &session)?;
        file.flush()?;
        drop(file);
        let open = Arc::new(AtomicUsize::new(0));
        server.worker = Some(
            std::thread::Builder::new()
                .name("surface-devtools".into())
                .spawn(move || {
                    while !stop.load(Ordering::Acquire) {
                        match listener.accept() {
                            Ok((mut stream, _)) => {
                                if open.fetch_add(1, Ordering::AcqRel) >= MAX_CONNECTIONS {
                                    open.fetch_sub(1, Ordering::AcqRel);
                                    let _ = stream.set_nonblocking(false);
                                    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
                                    write_reply(
                                        &mut stream,
                                        &Reply::error(
                                            0,
                                            ErrorCode::NotReady,
                                            "too many devtools connections",
                                        ),
                                    );
                                    continue;
                                }
                                let slot = Slot(open.clone());
                                let (token, dispatch, stop) =
                                    (token.clone(), dispatch.clone(), stop.clone());
                                let spawned = std::thread::Builder::new()
                                    .name("surface-devtools-conn".into())
                                    .spawn(move || {
                                        let _slot = slot;
                                        serve(stream, &token, &dispatch, &stop);
                                    });
                                // On a spawn error the closure, and its slot, drop here.
                                drop(spawned);
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::sleep(Duration::from_millis(20))
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                            Err(_) => break,
                        }
                    }
                })?,
        );
        Ok(server)
    }
}
impl Drop for Server {
    /// Stop accepting, then remove the session file if it is still ours.
    /// Connection threads see `stop` within one poll interval; they are not
    /// joined, so a request waiting on a closed event loop cannot hold up exit.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let owned = std::fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .is_some_and(|session| {
                session["token"].as_str() == Some(self.token.as_str())
                    && session["pid"].as_u64() == Some(u64::from(std::process::id()))
            });
        if owned {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::io::{BufRead, BufReader, Cursor, Write};
    use std::net::TcpStream;
    use surface_devtools_protocol::{ErrorCode, REQUEST_CAP_BYTES, RESPONSE_CAP_BYTES};

    const IDLE: Duration = Duration::from_secs(1);

    fn frames(input: &[u8], cap: usize) -> Vec<Frame> {
        let mut reader = FrameReader::new(cap);
        let mut stream = Cursor::new(input.to_vec());
        let stop = AtomicBool::new(false);
        let mut out = Vec::new();
        loop {
            let frame = reader.next(&mut stream, &stop, IDLE);
            let done = !matches!(frame, Frame::Line(_));
            out.push(frame);
            if done {
                return out;
            }
        }
    }

    #[test]
    fn frame_reader_splits_lines_and_keeps_the_rest() {
        assert_eq!(
            frames(b"{\"a\":1}\n{\"b\":2}\r\n\nrest", 64),
            vec![
                Frame::Line(b"{\"a\":1}".to_vec()),
                Frame::Line(b"{\"b\":2}".to_vec()),
                Frame::Line(Vec::new()),
                Frame::Closed,
            ]
        );
    }

    #[test]
    fn frame_reader_rejects_a_line_over_the_request_cap() {
        assert_eq!(
            frames(b"12345678\n", 8),
            vec![Frame::Line(b"12345678".to_vec()), Frame::Closed]
        );
        assert_eq!(frames(b"123456789\n", 8), vec![Frame::TooLarge]);
        // No newline at all: the reader stops at the cap, not at the end.
        let cap = REQUEST_CAP_BYTES as usize;
        let long = vec![b'x'; cap + 1];
        assert_eq!(frames(&long, cap), vec![Frame::TooLarge]);
    }

    #[test]
    fn frame_reader_stops_when_asked() {
        let mut reader = FrameReader::new(64);
        let stop = AtomicBool::new(true);
        assert_eq!(
            reader.next(&mut Cursor::new(b"x".to_vec()), &stop, IDLE),
            Frame::Stopped
        );
    }

    #[test]
    fn encode_keeps_a_reply_at_the_cap_and_replaces_one_over_it() {
        let small = Reply::ok(7, json!({"a": 1}));
        let line = encode(&small, 1024);
        assert_eq!(line.last(), Some(&b'\n'));
        let back: Reply = serde_json::from_slice(&line[..line.len() - 1]).unwrap();
        assert_eq!(back, small);
        let exact = line.len() - 1;
        assert_eq!(encode(&small, exact), line);

        let line = encode(&small, exact - 1);
        let back: Reply = serde_json::from_slice(&line[..line.len() - 1]).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.error.unwrap().code, ErrorCode::TooLarge);
    }

    /// A temporary folder for one session file.
    fn folder(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "surface-devtools-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Echoes the method. `Big` returns a result over the response cap.
    fn fake(request: Request) -> Reply {
        if request.method == "Big" {
            let size = RESPONSE_CAP_BYTES as usize + 1;
            return Reply::ok(request.id, Value::String("x".repeat(size)));
        }
        Reply::ok(request.id, json!({"method": request.method}))
    }

    fn session(path: &std::path::Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    fn connect(session: &Value) -> TcpStream {
        let stream = TcpStream::connect(session["endpoint"].as_str().unwrap()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
    }

    fn call(stream: &mut TcpStream, reader: &mut BufReader<TcpStream>, line: &str) -> Value {
        stream.write_all(line.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        let mut text = String::new();
        reader.read_line(&mut text).unwrap();
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("reply {text:?}: {e}"))
    }

    #[test]
    fn server_writes_a_private_loopback_session_file_and_removes_it() {
        let dir = folder("session");
        let path = dir.join("session.json");
        let server = Server::start(path.clone(), "Test app", fake).unwrap();
        let session = session(&path);
        assert_eq!(session["protocol"], "surface-devtools");
        assert_eq!(session["version"], 1);
        assert_eq!(session["application"], "Test app");
        assert_eq!(session["pid"], std::process::id());
        assert!(session["endpoint"]
            .as_str()
            .unwrap()
            .starts_with("127.0.0.1:"));
        assert_eq!(session["token"].as_str().unwrap().len(), 64);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        drop(server);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn server_never_overwrites_an_existing_session_file() {
        let dir = folder("existing");
        let path = dir.join("session.json");
        std::fs::write(&path, "keep me").unwrap();
        assert!(Server::start(path.clone(), "Test app", fake).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn server_rejects_a_wrong_or_missing_token_and_closes() {
        let dir = folder("token");
        let path = dir.join("session.json");
        let _server = Server::start(path.clone(), "Test app", fake).unwrap();
        let session = session(&path);
        for line in [
            r#"{"v":1,"id":5,"method":"DOM.snapshot","token":"wrong"}"#,
            r#"{"v":1,"id":5,"method":"DOM.snapshot"}"#,
        ] {
            let mut stream = connect(&session);
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let reply = call(&mut stream, &mut reader, line);
            assert_eq!(reply["id"], 5);
            assert_eq!(reply["ok"], false);
            assert_eq!(reply["error"]["code"], "unauthorized");
            let mut rest = String::new();
            assert_eq!(reader.read_line(&mut rest).unwrap(), 0, "closed");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn server_answers_several_requests_on_one_connection() {
        let dir = folder("connection");
        let path = dir.join("session.json");
        let _server = Server::start(path.clone(), "Test app", fake).unwrap();
        let session = session(&path);
        let token = session["token"].as_str().unwrap().to_string();
        let mut stream = connect(&session);
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        for (id, method) in [(1, "Devtools.capabilities"), (2, "DOM.snapshot")] {
            let line = json!({"v":1,"id":id,"method":method,"token":token}).to_string();
            let reply = call(&mut stream, &mut reader, &line);
            assert_eq!(reply, json!({"id":id,"ok":true,"result":{"method":method}}));
        }
        // Not JSON, then no id: invalid_params, the connection stays open.
        let reply = call(&mut stream, &mut reader, "not json");
        assert_eq!(reply["error"]["code"], "invalid_params");
        let reply = call(
            &mut stream,
            &mut reader,
            &json!({"v":1,"method":"DOM.snapshot","token":token}).to_string(),
        );
        assert_eq!(reply["id"], 0);
        assert_eq!(reply["error"]["code"], "invalid_params");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn server_turns_an_over_cap_reply_into_too_large_and_keeps_the_connection() {
        let dir = folder("response-cap");
        let path = dir.join("session.json");
        let _server = Server::start(path.clone(), "Test app", fake).unwrap();
        let session = session(&path);
        let token = session["token"].as_str().unwrap().to_string();
        let mut stream = connect(&session);
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let line = json!({"v":1,"id":3,"method":"Big","token":token}).to_string();
        let reply = call(&mut stream, &mut reader, &line);
        assert_eq!(reply["id"], 3);
        assert_eq!(reply["error"]["code"], "too_large");
        let line = json!({"v":1,"id":4,"method":"DOM.snapshot","token":token}).to_string();
        let reply = call(&mut stream, &mut reader, &line);
        assert_eq!(reply["id"], 4);
        assert_eq!(reply["ok"], true);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn server_rejects_a_request_over_the_request_cap() {
        let dir = folder("request-cap");
        let path = dir.join("session.json");
        let _server = Server::start(path.clone(), "Test app", fake).unwrap();
        let session = session(&path);
        let mut stream = connect(&session);
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let padding = "x".repeat(REQUEST_CAP_BYTES as usize);
        let line = json!({"v":1,"id":9,"method":"DOM.snapshot","pad":padding}).to_string();
        let reply = call(&mut stream, &mut reader, &line);
        assert_eq!(reply["error"]["code"], "too_large");
        let _ = std::fs::remove_dir_all(dir);
    }
}
