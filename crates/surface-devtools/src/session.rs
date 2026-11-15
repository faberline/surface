//! The session file written by a native app.
//! Format: `docs/devtools-protocol.md`, section "Native transport".
use serde_json::Value;
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
            Self::Stale(p) => write!(
                f,
                "the app is not running: its process in session file {} is gone (stale file)",
                p.display()
            ),
        }
    }
}

impl Session {
    pub fn load(path: &Path) -> Result<Session, SessionError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(path, &text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(SessionError::Missing(path.to_path_buf()))
            }
            Err(error) => Err(SessionError::Invalid(
                path.to_path_buf(),
                format!("cannot read: {error}"),
            )),
        }
    }

    pub fn parse(file: &Path, text: &str) -> Result<Session, SessionError> {
        let invalid = |message: &str| SessionError::Invalid(file.to_path_buf(), message.into());
        let value: Value =
            serde_json::from_str(text).map_err(|e| invalid(&format!("not JSON: {e}")))?;
        if value["protocol"] != "surface-devtools" {
            return Err(invalid("protocol is not \"surface-devtools\""));
        }
        if value["version"] != 1 {
            return Err(invalid("unsupported session version"));
        }
        let text_field = |name: &str| {
            value[name]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| invalid(&format!("no {name}")))
        };
        let pid = value["pid"]
            .as_u64()
            .and_then(|pid| u32::try_from(pid).ok())
            .ok_or_else(|| invalid("no pid"))?;
        Ok(Session {
            endpoint: text_field("endpoint")?,
            token: text_field("token")?,
            pid,
            application: value["application"].as_str().unwrap_or("").to_string(),
            file: file.to_path_buf(),
        })
    }
}

/// Whether a process with this pid exists. A pid that is zero or does not
/// fit a positive `i32` is never probed: `kill` would address a process
/// group.
pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }
    #[cfg(not(unix))]
    {
        true
    }
}
