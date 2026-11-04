//! Semantic operations use the same controlled state and input paths as the host.
use crate::{AppError, Application, EditKey, Event, Runtime};
use jet_bridge_protocol::{Command, Key, Request, Response, VERSION};
use serde_json::{json, Value};
use surface::{NodeId, Role};

struct Failure(&'static str, String);
impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        Self("action_failed", error.to_string())
    }
}
impl<A: Application> Runtime<A> {
    pub fn execute_bridge(&mut self, request: Request) -> Response {
        if request.version != VERSION {
            return Response::error(request.id, "unsupported_version", "use bridge version 1");
        }
        match self.bridge_command(request.command) {
            Ok(result) => Response::ok(request.id, result),
            Err(Failure(code, message)) => Response::error(request.id, code, message),
        }
    }

    fn bridge_target(&self, key: &str) -> Result<NodeId, Failure> {
        let id = self
            .dom
            .by_id(key)
            .ok_or_else(|| Failure("missing_node", format!("no current node with ID {key}")))?;
        let node = self.dom.get(id).map_err(AppError::from)?;
        if node.is_disabled() {
            return Err(Failure("disabled", format!("control {key} is disabled")));
        }
        if !self.in_scope(id) {
            return Err(Failure(
                "outside_modal",
                format!("control {key} is outside the active modal"),
            ));
        }
        Ok(id)
    }

    fn bridge_command(&mut self, command: Command) -> Result<Value, Failure> {
        let changed = match command {
            Command::Capabilities => {
                return Ok(json!({
                    "version":VERSION,"runtime":"surface","application":self.app.title(),
                    "snapshot_schema":"surface.dom.v1","selection_offsets":"utf8_bytes",
                    "methods":["capabilities","snapshot","query","activate","fill","set_value","key"],
                    "role_names":"Surface Role enum names; for example TextInput"
                }))
            }
            Command::Snapshot => {
                return serde_json::to_value(self.snapshot())
                    .map_err(|error| Failure("serialization_failed", error.to_string()))
            }
            Command::Query { id, role, name } => {
                let role: Option<Role> = role
                    .map(|role| serde_json::from_value(role.into()))
                    .transpose()
                    .map_err(|_| {
                        Failure(
                            "invalid_role",
                            "use a Surface Role enum name; for example TextInput".into(),
                        )
                    })?;
                let matches: Vec<_> = self.dom.iter().filter(|(_, node)| {
                    id.as_deref().is_none_or(|id| node.stable_id() == Some(id))
                        && role.is_none_or(|role| node.role() == role)
                        && name.as_deref().is_none_or(|name| node.name() == name)
                }).map(|(id, node)| json!({
                    "node":node,
                    "layout":self.computed.as_ref().and_then(|layout| layout.nodes.get(id.index())),
                    "in_scope":self.in_scope(id)
                })).collect();
                return Ok(json!({"matches":matches}));
            }
            Command::Activate { id } => {
                self.bridge_target(&id)?;
                self.activate(&id)?
            }
            Command::Fill { id, value } => {
                let target = self.bridge_target(&id)?;
                if self.dom.get(target).map_err(AppError::from)?.role() != Role::TextInput {
                    return Err(Failure("wrong_role", "fill requires a text input".into()));
                }
                if self.bindings[&target].text_options.read_only {
                    return Err(Failure("read_only", format!("field {id} is read-only")));
                }
                if self.editors.values().any(|editor| editor.is_composing()) {
                    return Err(Failure(
                        "composition_active",
                        "finish composition or send Escape before filling".into(),
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
            Command::SetValue { id, value } => {
                let target = self.bridge_target(&id)?;
                if self.dom.get(target).map_err(AppError::from)?.role() != Role::Slider {
                    return Err(Failure("wrong_role", "set_value requires a slider".into()));
                }
                if !value.is_finite() {
                    return Err(Failure(
                        "invalid_value",
                        "range value must be finite".into(),
                    ));
                }
                self.set_value(&id, value)?
            }
            Command::Key {
                key,
                extend,
                by_word,
            } => {
                if !matches!(key, Key::Escape)
                    && self.editors.values().any(|editor| editor.is_composing())
                {
                    return Err(Failure(
                        "composition_active",
                        "finish composition or send Escape before a semantic key".into(),
                    ));
                }
                let input = self.input_snapshot();
                if !matches!(key, Key::Tab | Key::Escape) && self.focused_node().is_none() {
                    return Err(Failure(
                        "no_focused_control",
                        "focus a control first".into(),
                    ));
                }
                if input.as_ref().is_some_and(|input| input.read_only)
                    && (matches!(
                        key,
                        Key::Space | Key::Backspace | Key::Delete | Key::Undo | Key::Redo
                    ) || (matches!(key, Key::Enter)
                        && input.as_ref().is_some_and(|input| input.multiline)))
                {
                    return Err(Failure(
                        "read_only",
                        "the focused field is read-only".into(),
                    ));
                }
                let event = match key {
                    Key::Tab => Event::FocusNext { reverse: extend },
                    Key::Escape => Event::Cancel,
                    Key::Enter if input.as_ref().is_some_and(|input| input.multiline) => {
                        Event::Text("\n".into())
                    }
                    Key::Space if input.is_some() => Event::Text(" ".into()),
                    Key::Enter | Key::Space => Event::Activate,
                    key => Event::Edit {
                        key: match key {
                            Key::Left => EditKey::Left,
                            Key::Right => EditKey::Right,
                            Key::Up => EditKey::Up,
                            Key::Down => EditKey::Down,
                            Key::Home => EditKey::Home,
                            Key::End => EditKey::End,
                            Key::PageUp => EditKey::PageUp,
                            Key::PageDown => EditKey::PageDown,
                            Key::Backspace => EditKey::Backspace,
                            Key::Delete => EditKey::Delete,
                            Key::SelectAll => EditKey::SelectAll,
                            Key::Undo => EditKey::Undo,
                            Key::Redo => EditKey::Redo,
                            _ => unreachable!("non-editing keys handled above"),
                        },
                        extend,
                        by_word,
                    },
                };
                self.dispatch(event)?
            }
        };
        Ok(json!({"changed":changed,"focused":self.focused}))
    }
}
