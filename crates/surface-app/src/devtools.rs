//! Protocol-typed devtools executor. See `docs/devtools-protocol.md`.
//!
//! Methods use the same controlled state and input paths as the host.
//! The jet bridge (`bridge.rs`) is a thin adapter over [`Call`].
//!
//! The protocol has a closed set of error codes. A failure also keeps a
//! finer `reason`: the jet bridge code. The adapter sends the reason. The
//! executor sends the protocol code and puts the reason in front of the
//! message when it differs, for example `read_only: field x is read-only`.
//!
//! | reason | protocol code |
//! | --- | --- |
//! | `missing_node`, `disabled`, `outside_modal`, `action_failed` | same |
//! | `invalid_role`, `wrong_role`, `invalid_value` | `invalid_params` |
//! | `composition_active`, `no_focused_control` | `not_ready` |
//! | `read_only`, `serialization_failed` | `action_failed` |
use crate::{AppError, Application, EditKey, Event, KeyPress, Modifiers, Runtime};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use surface::{NodeId, Role};
use surface_devtools_protocol::{
    Capabilities, ErrorCode, Limits, Reply, Request, DEFAULT_RING_SIZE, REQUEST_CAP_BYTES,
    RESPONSE_CAP_BYTES, SCHEMA_DOM, VERSION,
};

/// The methods this executor runs.
pub(crate) const METHODS: [&str; 7] = [
    "Devtools.capabilities",
    "DOM.snapshot",
    "DOM.query",
    "Action.activate",
    "Action.fill",
    "Action.setValue",
    "Action.key",
];

pub(crate) struct Failure {
    pub(crate) code: ErrorCode,
    /// The jet bridge code. It equals `code.as_str()` for protocol codes.
    pub(crate) reason: &'static str,
    pub(crate) message: String,
}
impl Failure {
    fn new(code: ErrorCode, reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            reason,
            message: message.into(),
        }
    }
    fn protocol(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(code, code.as_str(), message)
    }
    fn wire_message(&self) -> String {
        if self.reason == self.code.as_str() {
            self.message.clone()
        } else {
            format!("{}: {}", self.reason, self.message)
        }
    }
}
impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        Self::protocol(ErrorCode::ActionFailed, error.to_string())
    }
}

/// A semantic key. The wire names are snake case, as in the jet bridge.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Key {
    Tab,
    Enter,
    Space,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    SelectAll,
    Undo,
    Redo,
}

/// One parsed executor call.
pub(crate) enum Call {
    Capabilities,
    Snapshot,
    Query {
        id: Option<String>,
        role: Option<String>,
        name: Option<String>,
    },
    Activate {
        id: String,
    },
    Fill {
        id: String,
        value: String,
    },
    SetValue {
        id: String,
        value: f64,
    },
    Key {
        key: Key,
        extend: bool,
        by_word: bool,
    },
}

#[derive(Deserialize)]
struct QueryParams {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    name: Option<String>,
}
#[derive(Deserialize)]
struct IdParams {
    id: String,
}
#[derive(Deserialize)]
struct FillParams {
    id: String,
    value: String,
}
#[derive(Deserialize)]
struct SetValueParams {
    id: String,
    value: f64,
}
#[derive(Deserialize)]
struct KeyParams {
    key: Key,
    #[serde(default)]
    extend: bool,
    #[serde(default)]
    by_word: bool,
}

fn params<T: DeserializeOwned>(method: &str, params: Value) -> Result<T, Failure> {
    serde_json::from_value(params)
        .map_err(|error| Failure::protocol(ErrorCode::InvalidParams, format!("{method}: {error}")))
}

impl Call {
    /// Unknown fields in `params` are tolerated.
    fn parse(method: &str, value: Value) -> Result<Self, Failure> {
        Ok(match method {
            "Devtools.capabilities" => Self::Capabilities,
            "DOM.snapshot" => Self::Snapshot,
            "DOM.query" => {
                let QueryParams { id, role, name } = params(method, value)?;
                Self::Query { id, role, name }
            }
            "Action.activate" => {
                let IdParams { id } = params(method, value)?;
                Self::Activate { id }
            }
            "Action.fill" => {
                let FillParams { id, value } = params(method, value)?;
                Self::Fill { id, value }
            }
            "Action.setValue" => {
                let SetValueParams { id, value } = params(method, value)?;
                Self::SetValue { id, value }
            }
            "Action.key" => {
                let KeyParams {
                    key,
                    extend,
                    by_word,
                } = params(method, value)?;
                Self::Key {
                    key,
                    extend,
                    by_word,
                }
            }
            method => {
                return Err(Failure::protocol(
                    ErrorCode::UnknownMethod,
                    format!("unknown method {method}"),
                ))
            }
        })
    }
}

impl<A: Application> Runtime<A> {
    /// Advertise methods that the host handles before calling the executor.
    pub fn set_host_methods(&mut self, methods: &'static [&'static str]) {
        self.host_methods = methods;
    }

    /// Run one devtools request. A rejected request changes no state.
    pub fn execute_devtools(&mut self, request: Request) -> Reply {
        if request.v != VERSION {
            return Reply::error(
                request.id,
                ErrorCode::UnsupportedVersion,
                format!(
                    "protocol version {} is not supported; this is version {VERSION}",
                    request.v
                ),
            );
        }
        match Call::parse(&request.method, request.params).and_then(|call| self.run_devtools(call))
        {
            Ok(result) => Reply::ok(request.id, result),
            Err(failure) => Reply::error(request.id, failure.code, failure.wire_message()),
        }
    }

    fn devtools_capabilities(&self) -> Result<Value, Failure> {
        let methods = METHODS.into_iter().chain(self.host_methods.iter().copied());
        let mut domains: Vec<String> = Vec::new();
        for method in methods.clone() {
            let domain = method.split('.').next().unwrap_or(method);
            if !domains.iter().any(|known| known == domain) {
                domains.push(domain.into());
            }
        }
        let capabilities = Capabilities {
            version: VERSION,
            domains,
            methods: methods.map(str::to_owned).collect(),
            limits: Limits {
                request_bytes: REQUEST_CAP_BYTES,
                response_bytes: RESPONSE_CAP_BYTES,
                ring_size: DEFAULT_RING_SIZE,
            },
            schemas: vec![SCHEMA_DOM.into()],
        };
        let mut result = serialize(&capabilities)?;
        if let Value::Object(map) = &mut result {
            map.insert("application".into(), self.app.title().into());
        }
        Ok(result)
    }

    /// A target for an action: present, enabled and inside the active modal.
    fn devtools_target(&self, key: &str) -> Result<NodeId, Failure> {
        let id = self.dom.by_id(key).ok_or_else(|| {
            Failure::protocol(
                ErrorCode::MissingNode,
                format!("no current node with ID {key}"),
            )
        })?;
        let node = self.dom.get(id).map_err(AppError::from)?;
        if node.is_disabled() {
            return Err(Failure::protocol(
                ErrorCode::Disabled,
                format!("control {key} is disabled"),
            ));
        }
        if !self.in_scope(id) {
            return Err(Failure::protocol(
                ErrorCode::OutsideModal,
                format!("control {key} is outside the active modal"),
            ));
        }
        Ok(id)
    }

    fn composing(&self) -> bool {
        self.editors.values().any(|editor| editor.is_composing())
    }

    /// Run one parsed call. Validation happens before any state changes.
    pub(crate) fn run_devtools(&mut self, call: Call) -> Result<Value, Failure> {
        let changed = match call {
            Call::Capabilities => return self.devtools_capabilities(),
            Call::Snapshot => return serialize(&self.snapshot()),
            Call::Query { id, role, name } => {
                let role: Option<Role> = role
                    .map(|role| serde_json::from_value(role.into()))
                    .transpose()
                    .map_err(|_| {
                        Failure::new(
                            ErrorCode::InvalidParams,
                            "invalid_role",
                            "use a Surface Role enum name; for example TextInput",
                        )
                    })?;
                let matches: Vec<_> = self
                    .dom
                    .iter()
                    .filter(|(_, node)| {
                        id.as_deref().is_none_or(|id| node.stable_id() == Some(id))
                            && role.is_none_or(|role| node.role() == role)
                            && name.as_deref().is_none_or(|name| node.name() == name)
                    })
                    .map(|(id, node)| {
                        json!({
                            "node":node,
                            "layout":self.computed.as_ref().and_then(|layout| layout.nodes.get(id.index())),
                            "in_scope":self.in_scope(id)
                        })
                    })
                    .collect();
                return Ok(json!({"matches":matches}));
            }
            Call::Activate { id } => {
                self.devtools_target(&id)?;
                self.activate(&id)?
            }
            Call::Fill { id, value } => {
                let target = self.devtools_target(&id)?;
                if self.dom.get(target).map_err(AppError::from)?.role() != Role::TextInput {
                    return Err(Failure::new(
                        ErrorCode::InvalidParams,
                        "wrong_role",
                        "fill requires a text input",
                    ));
                }
                if self.bindings[&target].text_options.read_only {
                    return Err(Failure::new(
                        ErrorCode::ActionFailed,
                        "read_only",
                        format!("field {id} is read-only"),
                    ));
                }
                if self.composing() {
                    return Err(Failure::new(
                        ErrorCode::NotReady,
                        "composition_active",
                        "finish composition or send Escape before filling",
                    ));
                }
                // Validate before changing focus. Do not activate an optional press handler.
                self.focused = Some(id);
                let end = value.len();
                self.dispatch(Event::InputValue {
                    value,
                    anchor: end,
                    cursor: end,
                })?
            }
            Call::SetValue { id, value } => {
                let target = self.devtools_target(&id)?;
                if self.dom.get(target).map_err(AppError::from)?.role() != Role::Slider {
                    return Err(Failure::new(
                        ErrorCode::InvalidParams,
                        "wrong_role",
                        "set_value requires a slider",
                    ));
                }
                if !value.is_finite() {
                    return Err(Failure::new(
                        ErrorCode::InvalidParams,
                        "invalid_value",
                        "range value must be finite",
                    ));
                }
                self.set_value(&id, value)?
            }
            Call::Key {
                key,
                extend,
                by_word,
            } => {
                if !matches!(key, Key::Escape) && self.composing() {
                    return Err(Failure::new(
                        ErrorCode::NotReady,
                        "composition_active",
                        "finish composition or send Escape before a semantic key",
                    ));
                }
                let input = self.input_snapshot();
                if !matches!(key, Key::Tab | Key::Escape) && self.focused_node().is_none() {
                    return Err(Failure::new(
                        ErrorCode::NotReady,
                        "no_focused_control",
                        "focus a control first",
                    ));
                }
                let multiline = input.as_ref().is_some_and(|input| input.multiline);
                if input.as_ref().is_some_and(|input| input.read_only)
                    && (matches!(
                        key,
                        Key::Space | Key::Backspace | Key::Delete | Key::Undo | Key::Redo
                    ) || (matches!(key, Key::Enter) && multiline))
                {
                    return Err(Failure::new(
                        ErrorCode::ActionFailed,
                        "read_only",
                        "the focused field is read-only",
                    ));
                }
                if matches!(key, Key::Enter) && !multiline {
                    let press = KeyPress::new(
                        crate::Key::Enter,
                        Modifiers {
                            shift: extend,
                            ..Modifiers::default()
                        },
                    );
                    let uses_key_navigation = self
                        .focused_node()
                        .and_then(|id| self.bindings.get(&id))
                        .is_some_and(|binding| binding.key_focus_navigation.is_some());
                    if (input.is_some() || uses_key_navigation) && self.key(&press)? {
                        return Ok(json!({"changed":true,"focused":self.focused}));
                    }
                }
                let event = match key {
                    Key::Tab => Event::FocusNext { reverse: extend },
                    Key::Escape => Event::Cancel,
                    Key::Enter if multiline => Event::Text("\n".into()),
                    Key::Space if input.is_some() => Event::Text(" ".into()),
                    Key::Enter | Key::Space => Event::Activate,
                    Key::Left => edit(EditKey::Left, extend, by_word),
                    Key::Right => edit(EditKey::Right, extend, by_word),
                    Key::Up => edit(EditKey::Up, extend, by_word),
                    Key::Down => edit(EditKey::Down, extend, by_word),
                    Key::Home => edit(EditKey::Home, extend, by_word),
                    Key::End => edit(EditKey::End, extend, by_word),
                    Key::PageUp => edit(EditKey::PageUp, extend, by_word),
                    Key::PageDown => edit(EditKey::PageDown, extend, by_word),
                    Key::Backspace => edit(EditKey::Backspace, extend, by_word),
                    Key::Delete => edit(EditKey::Delete, extend, by_word),
                    Key::SelectAll => edit(EditKey::SelectAll, extend, by_word),
                    Key::Undo => edit(EditKey::Undo, extend, by_word),
                    Key::Redo => edit(EditKey::Redo, extend, by_word),
                };
                self.dispatch(event)?
            }
        };
        Ok(json!({"changed":changed,"focused":self.focused}))
    }
}

fn edit(key: EditKey, extend: bool, by_word: bool) -> Event {
    Event::Edit {
        key,
        extend,
        by_word,
    }
}

fn serialize<T: serde::Serialize>(value: &T) -> Result<Value, Failure> {
    serde_json::to_value(value).map_err(|error| {
        Failure::new(
            ErrorCode::ActionFailed,
            "serialization_failed",
            error.to_string(),
        )
    })
}
