//! The connection interface and its native TCP implementation.
use crate::session::Session;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use surface_devtools_protocol::{Reply, Request, RESPONSE_CAP_BYTES};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The host waits at most 5 s for its event loop; allow for that.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum ClientError {
    /// The app is not running: dead pid or refused connection.
    AppNotRunning(String),
    /// A request may have reached the app; it must not be sent again.
    Io(String),
    Protocol(String),
    /// The session file is unusable.
    Session(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AppNotRunning(m) | Self::Io(m) | Self::Protocol(m) | Self::Session(m) => {
                f.write_str(m)
            }
        }
    }
}

/// One authenticated request/reply channel to an app. A call sends its
/// request once. It is never retried.
pub trait Connection {
    fn call(&mut self, request: &Request) -> Result<Reply, ClientError>;
}

/// Newline-delimited JSON over loopback TCP.
pub struct NativeTcp {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    token: String,
}

impl NativeTcp {
    /// The token is sent only to a loopback address.
    pub fn connect(session: &Session) -> Result<NativeTcp, ClientError> {
        let file = session.file.display();
        let address: SocketAddr = session.endpoint.parse().map_err(|_| {
            ClientError::Session(format!(
                "session file {file}: endpoint {:?} is not an IP address and port",
                session.endpoint
            ))
        })?;
        if !address.ip().is_loopback() {
            return Err(ClientError::Session(format!(
                "session file {file}: endpoint {address} is not a loopback address; \
                 the token is not sent there"
            )));
        }
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(|error| {
            if error.kind() == std::io::ErrorKind::ConnectionRefused {
                ClientError::AppNotRunning(format!(
                    "the app is not running: {address} refused the connection \
                     (session file {file})"
                ))
            } else {
                ClientError::Io(format!("connect {address} (session file {file}): {error}"))
            }
        })?;
        let io = |error: std::io::Error| ClientError::Io(error.to_string());
        stream.set_read_timeout(Some(REPLY_TIMEOUT)).map_err(io)?;
        stream.set_write_timeout(Some(REPLY_TIMEOUT)).map_err(io)?;
        let reader = BufReader::new(stream.try_clone().map_err(io)?);
        Ok(NativeTcp {
            stream,
            reader,
            token: session.token.clone(),
        })
    }
}

impl Connection for NativeTcp {
    fn call(&mut self, request: &Request) -> Result<Reply, ClientError> {
        let mut line = request_line(request, &self.token);
        line.push('\n');
        self.stream
            .write_all(line.as_bytes())
            .and_then(|()| self.stream.flush())
            .map_err(|e| ClientError::Io(format!("send: {e}")))?;
        let mut text = String::new();
        let read = (&mut self.reader)
            .take(RESPONSE_CAP_BYTES + 1)
            .read_line(&mut text)
            .map_err(|e| ClientError::Io(format!("read reply: {e}")))?;
        if read == 0 {
            return Err(ClientError::Io(
                "the connection closed before a reply; the request may have been executed, \
                 so it was not sent again"
                    .into(),
            ));
        }
        if !text.ends_with('\n') {
            return Err(ClientError::Protocol(
                "the reply is longer than the response cap or was cut off".into(),
            ));
        }
        let reply: Reply = serde_json::from_str(&text)
            .map_err(|e| ClientError::Protocol(format!("reply line: {e}")))?;
        // Id 0 is the id of a reply whose request id could not be read.
        if reply.id != request.id && reply.id != 0 {
            return Err(ClientError::Protocol(format!(
                "reply id {} does not match request id {}",
                reply.id, request.id
            )));
        }
        Ok(reply)
    }
}

/// The wire line of a request: the envelope plus the token. No newline.
pub fn request_line(request: &Request, token: &str) -> String {
    serde_json::json!({
        "v": request.v,
        "id": request.id,
        "method": request.method,
        "params": request.params,
        "token": token,
    })
    .to_string()
}
