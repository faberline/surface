//! Jet bridge compatibility adapter over the devtools executor.
//! Each jet command maps to one executor call. The JSON output is the same
//! as before the executor existed. Failures send the executor's jet reason.
use crate::devtools::{self, Call, Failure};
use crate::{Application, Runtime};
use jet_bridge_protocol::{Command, Key, Request, Response, VERSION};
use serde_json::{json, Value};

impl<A: Application> Runtime<A> {
    pub fn execute_bridge(&mut self, request: Request) -> Response {
        if request.version != VERSION {
            return Response::error(request.id, "unsupported_version", "use bridge version 1");
        }
        match self.bridge_command(request.command) {
            Ok(result) => Response::ok(request.id, result),
            Err(Failure {
                reason, message, ..
            }) => Response::error(request.id, reason, message),
        }
    }

    fn bridge_command(&mut self, command: Command) -> Result<Value, Failure> {
        let capabilities = matches!(command, Command::Capabilities);
        let result = self.run_devtools(call(command))?;
        if capabilities {
            return Ok(json!({
                "version":VERSION,"runtime":"surface","application":result["application"],
                "snapshot_schema":"surface.dom.v1","selection_offsets":"utf8_bytes",
                "methods":["capabilities","snapshot","query","activate","fill","set_value","key"],
                "role_names":"Surface Role enum names; for example TextInput"
            }));
        }
        Ok(result)
    }
}

fn call(command: Command) -> Call {
    match command {
        Command::Capabilities => Call::Capabilities,
        Command::Snapshot => Call::Snapshot,
        Command::Query { id, role, name } => Call::Query { id, role, name },
        Command::Activate { id } => Call::Activate { id },
        Command::Fill { id, value } => Call::Fill { id, value },
        Command::SetValue { id, value } => Call::SetValue { id, value },
        Command::Key {
            key,
            extend,
            by_word,
        } => Call::Key {
            key: self::key(key),
            extend,
            by_word,
        },
    }
}

fn key(key: Key) -> devtools::Key {
    match key {
        Key::Tab => devtools::Key::Tab,
        Key::Enter => devtools::Key::Enter,
        Key::Space => devtools::Key::Space,
        Key::Escape => devtools::Key::Escape,
        Key::Left => devtools::Key::Left,
        Key::Right => devtools::Key::Right,
        Key::Up => devtools::Key::Up,
        Key::Down => devtools::Key::Down,
        Key::Home => devtools::Key::Home,
        Key::End => devtools::Key::End,
        Key::PageUp => devtools::Key::PageUp,
        Key::PageDown => devtools::Key::PageDown,
        Key::Backspace => devtools::Key::Backspace,
        Key::Delete => devtools::Key::Delete,
        Key::SelectAll => devtools::Key::SelectAll,
        Key::Undo => devtools::Key::Undo,
        Key::Redo => devtools::Key::Redo,
    }
}
