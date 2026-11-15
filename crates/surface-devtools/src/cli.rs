//! Argument parsing and the command layer.
use crate::connection::Connection;
use crate::{EXIT_CLIENT, EXIT_PROTOCOL};
use serde_json::{json, Map, Value};
use std::io::Write;
use surface_devtools_protocol::{Request, VERSION};

pub const USAGE: &str = "\
usage: surface-devtools [--session <file>] <command>

The session file comes from --session or SURFACE_DEVTOOLS_SESSION.

commands:
  capabilities
  dom snapshot
  dom query [--id <id>] [--role <role>] [--name <name>]
  action activate <id>
  action fill <id> <text>
  action set-value <id> <number>
  action key <key> [--extend] [--by-word]
  mcp                       serve MCP over stdio

Output is JSON on stdout. Exit codes: 0 ok, 1 protocol error (code on
stderr), 2 usage, 3 session, app or connection failure.";

#[derive(Debug, PartialEq)]
pub enum Command {
    /// One protocol call.
    Call { method: String, params: Value },
    /// Serve MCP over stdio.
    Mcp,
}

#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub session: Option<String>,
    pub command: Command,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UsageError(pub String);

fn usage<T>(message: impl Into<String>) -> Result<T, UsageError> {
    Err(UsageError(message.into()))
}

/// Flags that take a value, and flags that do not.
const VALUE_FLAGS: [&str; 4] = ["--session", "--id", "--role", "--name"];
const SWITCHES: [&str; 2] = ["--extend", "--by-word"];

pub fn parse_args(args: &[String]) -> Result<Parsed, UsageError> {
    let mut positional: Vec<&str> = Vec::new();
    let mut values: Vec<(&str, &str)> = Vec::new();
    let mut switches: Vec<&str> = Vec::new();
    let mut iter = args.iter().map(String::as_str);
    let mut literal = false;
    while let Some(arg) = iter.next() {
        if literal || !arg.starts_with("--") {
            positional.push(arg);
        } else if arg == "--" {
            literal = true;
        } else if let Some(flag) = VALUE_FLAGS.iter().find(|flag| **flag == arg) {
            match iter.next() {
                Some(value) => values.push((flag, value)),
                None => return usage(format!("{arg} needs a value")),
            }
        } else if let Some(flag) = SWITCHES.iter().find(|flag| **flag == arg) {
            switches.push(flag);
        } else {
            return usage(format!("unknown option {arg}"));
        }
    }
    let session = take_value(&mut values, "--session")?;
    let command = match positional.as_slice() {
        ["mcp"] => Command::Mcp,
        ["capabilities"] | ["devtools", "capabilities"] => call("Devtools.capabilities", json!({})),
        ["dom", "snapshot"] => call("DOM.snapshot", json!({})),
        ["dom", "query"] => {
            let mut params = Map::new();
            for key in ["id", "role", "name"] {
                if let Some(value) = take_value(&mut values, &format!("--{key}"))? {
                    params.insert(key.into(), value.into());
                }
            }
            call("DOM.query", Value::Object(params))
        }
        ["action", "activate", id] => call("Action.activate", json!({"id": id})),
        ["action", "fill", id, value] => call("Action.fill", json!({"id": id, "value": value})),
        ["action", "set-value", id, value] => {
            let number = value
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .ok_or_else(|| UsageError(format!("{value:?} is not a finite number")))?;
            call("Action.setValue", json!({"id": id, "value": number}))
        }
        ["action", "key", key] => {
            let extend = take_switch(&mut switches, "--extend");
            let by_word = take_switch(&mut switches, "--by-word");
            call(
                "Action.key",
                json!({"key": key, "extend": extend, "by_word": by_word}),
            )
        }
        [] => return usage("no command"),
        ["dom"] | ["action"] => return usage("missing operation"),
        ["dom", op, ..] | ["action", op, ..] => {
            return usage(format!("unknown or incomplete operation {op:?}"))
        }
        [other, ..] => return usage(format!("unknown command {other:?}")),
    };
    if let Some((flag, _)) = values.first() {
        return usage(format!("{flag} does not apply to this command"));
    }
    if let Some(flag) = switches.first() {
        return usage(format!("{flag} does not apply to this command"));
    }
    Ok(Parsed { session, command })
}

fn call(method: &str, params: Value) -> Command {
    Command::Call {
        method: method.into(),
        params,
    }
}

/// Remove one flag and return its value. A repeated flag is an error.
fn take_value(values: &mut Vec<(&str, &str)>, flag: &str) -> Result<Option<String>, UsageError> {
    let mut found = None;
    let mut count = 0;
    values.retain(|(name, value)| {
        if *name == flag {
            count += 1;
            found = Some(value.to_string());
            false
        } else {
            true
        }
    });
    if count > 1 {
        return usage(format!("{flag} given more than once"));
    }
    Ok(found)
}

fn take_switch(switches: &mut Vec<&str>, flag: &str) -> bool {
    let before = switches.len();
    switches.retain(|name| *name != flag);
    switches.len() != before
}

/// Send one call. The result JSON goes to `out`, a protocol error to `err`.
/// Returns the exit code. The request is sent once.
pub fn execute_call(
    conn: &mut dyn Connection,
    method: &str,
    params: Value,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let request = Request {
        v: VERSION,
        id: 1,
        method: method.into(),
        params,
    };
    match conn.call(&request) {
        Ok(reply) if reply.ok => {
            let result = reply.result.unwrap_or(Value::Null);
            let _ = writeln!(out, "{result}");
            0
        }
        Ok(reply) => {
            let (code, message) = reply
                .error
                .map(|error| (error.code.as_str(), error.message))
                .unwrap_or(("unknown", String::new()));
            let _ = writeln!(err, "error: {code}: {message}");
            EXIT_PROTOCOL
        }
        Err(error) => {
            let _ = writeln!(err, "surface-devtools: {error}");
            EXIT_CLIENT
        }
    }
}
