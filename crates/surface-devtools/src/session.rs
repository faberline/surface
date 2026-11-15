//! The session file written by a native app.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub endpoint: String,
    pub token: String,
    pub pid: u32,
    pub application: String,
    pub file: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionError {
    Missing(PathBuf),
    Invalid(PathBuf, String),
    /// The pid in the file is not running.
    Stale(PathBuf),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(p) => write!(f, "session file {} does not exist", p.display()),
            Self::Invalid(p, m) => write!(f, "session file {}: {m}", p.display()),
            Self::Stale(p) => write!(f, "session file {} is stale", p.display()),
        }
    }
}

impl Session {
    pub fn load(_path: &Path) -> Result<Session, SessionError> {
        unimplemented!("red")
    }
    pub fn parse(_file: &Path, _text: &str) -> Result<Session, SessionError> {
        unimplemented!("red")
    }
}

/// Whether a process with this pid exists.
pub fn pid_alive(_pid: u32) -> bool {
    unimplemented!("red")
}
