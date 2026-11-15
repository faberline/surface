//! The connection interface and its native TCP implementation.
use crate::session::Session;
use surface_devtools_protocol::{Reply, Request};

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

/// One authenticated request/reply channel to an app.
pub trait Connection {
    fn call(&mut self, request: &Request) -> Result<Reply, ClientError>;
}

/// Newline-delimited JSON over loopback TCP.
pub struct NativeTcp;

impl NativeTcp {
    pub fn connect(_session: &Session) -> Result<NativeTcp, ClientError> {
        unimplemented!("red")
    }
}

impl Connection for NativeTcp {
    fn call(&mut self, _request: &Request) -> Result<Reply, ClientError> {
        unimplemented!("red")
    }
}

/// The wire line of a request: the envelope plus the token.
pub fn request_line(_request: &Request, _token: &str) -> String {
    unimplemented!("red")
}
