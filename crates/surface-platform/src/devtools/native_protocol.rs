//! The Surface devtools protocol over loopback TCP. Native only.
//! See `docs/devtools-protocol.md`, section "Native transport".
use crate::{HostError, UserEvent};
use std::{
    io::Read,
    path::PathBuf,
    sync::{atomic::AtomicBool, mpsc},
    time::Duration,
};
use surface_devtools_protocol::{Reply, Request};
use winit::event_loop::EventLoopProxy;

/// The session file path. Separate from the jet bridge's `SURFACE_BRIDGE_SESSION`.
pub(crate) const ENV: &str = "SURFACE_DEVTOOLS_SESSION";

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
    /// One request line, without its newline.
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

/// Splits a byte stream into newline-delimited request lines.
pub(crate) struct FrameReader {
    #[allow(dead_code)]
    buffer: Vec<u8>,
    #[allow(dead_code)]
    cap: usize,
}
impl FrameReader {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            buffer: Vec::new(),
            cap,
        }
    }
    pub(crate) fn next(
        &mut self,
        _stream: &mut impl Read,
        _stop: &AtomicBool,
        _idle: Duration,
    ) -> Frame {
        Frame::Failed
    }
}

/// One reply line with its newline. A reply over `cap` bytes becomes
/// `too_large` with the same id.
pub(crate) fn encode(_reply: &Reply, _cap: usize) -> Vec<u8> {
    Vec::new()
}

pub(crate) struct Server;
impl Server {
    pub(crate) fn install(
        _proxy: EventLoopProxy<UserEvent>,
        _application: &str,
    ) -> Result<Option<Self>, HostError> {
        Ok(None)
    }
    pub(crate) fn start<D>(
        _path: PathBuf,
        _application: &str,
        _dispatch: D,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        D: Fn(Request) -> Reply + Clone + Send + 'static,
    {
        Err("native devtools transport is not implemented".into())
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
