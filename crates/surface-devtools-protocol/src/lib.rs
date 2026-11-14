//! Wire types of the Surface devtools protocol. Data and serde only.
//! See `docs/devtools-protocol.md`.
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod input;
pub use input::{EditKey, ImeEvent, InputEvent};

/// The one integer major version of the protocol.
pub const VERSION: u32 = 1;
pub const REQUEST_CAP_BYTES: u64 = 1 << 20;
pub const RESPONSE_CAP_BYTES: u64 = 16 << 20;
pub const DEFAULT_RING_SIZE: u64 = 4096;
pub const SCHEMA_DOM: &str = "surface.dom.v1";
pub const SCHEMA_TRACE: &str = "surface.trace.v1";
pub const SCHEMA_PAINT: &str = "surface.paint.v1";
/// Methods this version of the crate knows. Later issues extend the list.
pub const METHODS: &[&str] = &[
    "Devtools.capabilities",
    "Trace.start",
    "Trace.stop",
    "Trace.read",
    "Page.screenshot",
    "Input.dispatch",
    "Wait.for",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    UnsupportedVersion,
    UnknownMethod,
    InvalidParams,
    Unauthorized,
    NotReady,
    MissingNode,
    Disabled,
    OutsideModal,
    TooLarge,
    Timeout,
    ActionFailed,
    HostTimeout,
    HostClosed,
    RenderFailed,
}
impl ErrorCode {
    pub const ALL: [ErrorCode; 14] = [
        Self::UnsupportedVersion,
        Self::UnknownMethod,
        Self::InvalidParams,
        Self::Unauthorized,
        Self::NotReady,
        Self::MissingNode,
        Self::Disabled,
        Self::OutsideModal,
        Self::TooLarge,
        Self::Timeout,
        Self::ActionFailed,
        Self::HostTimeout,
        Self::HostClosed,
        Self::RenderFailed,
    ];
    /// The wire string. It matches the serde form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported_version",
            Self::UnknownMethod => "unknown_method",
            Self::InvalidParams => "invalid_params",
            Self::Unauthorized => "unauthorized",
            Self::NotReady => "not_ready",
            Self::MissingNode => "missing_node",
            Self::Disabled => "disabled",
            Self::OutsideModal => "outside_modal",
            Self::TooLarge => "too_large",
            Self::Timeout => "timeout",
            Self::ActionFailed => "action_failed",
            Self::HostTimeout => "host_timeout",
            Self::HostClosed => "host_closed",
            Self::RenderFailed => "render_failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub v: u32,
    pub id: u64,
    pub method: String,
    #[serde(default = "empty_params")]
    pub params: Value,
}
fn empty_params() -> Value {
    Value::Object(Default::default())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub id: u64,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}
impl Reply {
    pub fn ok(id: u64, result: Value) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }
    pub fn error(id: u64, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(ErrorBody {
                code,
                message: message.into(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub event: String,
    pub seq: u64,
    pub params: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Limits {
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub ring_size: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    pub version: u32,
    pub domains: Vec<String>,
    pub methods: Vec<String>,
    pub limits: Limits,
    pub schemas: Vec<String>,
}
impl Capabilities {
    /// What this version of the crate describes. A host may narrow it.
    pub fn current() -> Self {
        let mut domains: Vec<String> = Vec::new();
        for method in METHODS {
            let domain = method.split('.').next().unwrap_or(method);
            if !domains.iter().any(|d| d == domain) {
                domains.push(domain.to_string());
            }
        }
        Self {
            version: VERSION,
            domains,
            methods: METHODS.iter().map(|m| m.to_string()).collect(),
            limits: Limits {
                request_bytes: REQUEST_CAP_BYTES,
                response_bytes: RESPONSE_CAP_BYTES,
                ring_size: DEFAULT_RING_SIZE,
            },
            schemas: vec![SCHEMA_DOM.into(), SCHEMA_TRACE.into(), SCHEMA_PAINT.into()],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceKind {
    Input,
    Message,
    Update,
    Rebuild,
    Layout,
    Paint,
    Present,
    Error,
}

/// One line of `surface.trace.v1`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceRecord {
    pub seq: u64,
    pub frame: u64,
    pub kind: TraceKind,
    pub t_ms: f64,
    pub dur_ms: f64,
    pub data: Value,
}

/// Parse one request line. Errors come back as the reply to send.
///
/// Order: JSON and envelope shape (`invalid_params`), then `v`
/// (`unsupported_version`), then the method (`unknown_method`). Unknown
/// fields in the envelope and in `params` are tolerated. A missing `params`
/// is `{}`. When the id cannot be read, the error reply has id 0.
pub fn parse_request(line: &str) -> Result<Request, Reply> {
    let invalid = |id: u64, message: &str| Reply::error(id, ErrorCode::InvalidParams, message);
    let value: Value =
        serde_json::from_str(line).map_err(|e| invalid(0, &format!("invalid JSON: {e}")))?;
    let id = value.get("id").and_then(Value::as_u64);
    let Some(id) = id else {
        return Err(invalid(0, "request needs an integer id"));
    };
    match value.get("v").and_then(Value::as_u64) {
        Some(v) if v == u64::from(VERSION) => {}
        Some(v) => {
            return Err(Reply::error(
                id,
                ErrorCode::UnsupportedVersion,
                format!("protocol version {v} is not supported; this is version {VERSION}"),
            ))
        }
        None => return Err(invalid(id, "request needs an integer v")),
    }
    let request: Request =
        serde_json::from_value(value).map_err(|e| invalid(id, &e.to_string()))?;
    if !METHODS.contains(&request.method.as_str()) {
        return Err(Reply::error(
            id,
            ErrorCode::UnknownMethod,
            format!("unknown method {}", request.method),
        ));
    }
    Ok(request)
}
