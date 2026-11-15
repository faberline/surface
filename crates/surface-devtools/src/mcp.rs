//! MCP over stdio: `initialize`, `tools/list`, `tools/call`.
//!
//! Hand-written JSON-RPC 2.0, one message per line. One tool per protocol
//! method. The tool list and the descriptions come from
//! `Devtools.capabilities` of the connected app.
use crate::connection::Connection;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use surface_devtools_protocol::{Request, VERSION};

const SUPPORTED: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-06-18"];
const LATEST: &str = "2025-06-18";

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// An MCP server over one connection. It caches `Devtools.capabilities`.
#[derive(Default)]
pub struct McpServer {
    capabilities: Option<Value>,
    next_id: u64,
}

impl McpServer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle one JSON-RPC line. Returns the response, or `None` for a
    /// notification.
    pub fn handle_line(&mut self, conn: &mut dyn Connection, line: &str) -> Option<Value> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(error) => {
                return Some(failure(
                    Value::Null,
                    PARSE_ERROR,
                    &format!("parse error: {error}"),
                ))
            }
        };
        let Some(object) = message.as_object() else {
            return Some(failure(
                Value::Null,
                INVALID_REQUEST,
                "expected a JSON object",
            ));
        };
        let id = object.get("id").cloned();
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return id.map(|id| failure(id, INVALID_REQUEST, "no method"));
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = id else {
            // A notification gets no response, known or not.
            return None;
        };
        Some(match method {
            "initialize" => {
                let asked = params["protocolVersion"].as_str().unwrap_or("");
                let version = SUPPORTED.iter().find(|v| **v == asked).unwrap_or(&LATEST);
                success(
                    id,
                    json!({
                        "protocolVersion": version,
                        "capabilities": {"tools": {"listChanged": false}},
                        "serverInfo": {
                            "name": "surface-devtools",
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    }),
                )
            }
            "ping" => success(id, json!({})),
            "tools/list" => match self.capabilities(conn) {
                Ok(capabilities) => success(id, json!({"tools": tools(&capabilities)})),
                Err(message) => failure(id, INTERNAL_ERROR, &message),
            },
            "tools/call" => self.call_tool(conn, id, &params),
            other => failure(id, METHOD_NOT_FOUND, &format!("method not found: {other}")),
        })
    }

    /// Serve until the input ends. One response line per request line.
    pub fn serve(
        &mut self,
        conn: &mut dyn Connection,
        input: &mut dyn BufRead,
        out: &mut dyn Write,
    ) -> std::io::Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            if input.read_line(&mut line)? == 0 {
                return Ok(());
            }
            let text = line.trim();
            if text.is_empty() {
                continue;
            }
            if let Some(response) = self.handle_line(conn, text) {
                writeln!(out, "{response}")?;
                out.flush()?;
            }
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Request {
        self.next_id += 1;
        Request {
            v: VERSION,
            id: self.next_id,
            method: method.into(),
            params,
        }
    }

    /// `Devtools.capabilities`, fetched once.
    fn capabilities(&mut self, conn: &mut dyn Connection) -> Result<Value, String> {
        if let Some(cached) = &self.capabilities {
            return Ok(cached.clone());
        }
        let request = self.request("Devtools.capabilities", json!({}));
        let reply = conn.call(&request).map_err(|e| e.to_string())?;
        let result = match (reply.ok, reply.result, reply.error) {
            (true, Some(result), _) => result,
            (_, _, Some(error)) => {
                return Err(format!(
                    "Devtools.capabilities: {}: {}",
                    error.code.as_str(),
                    error.message
                ))
            }
            _ => return Err("Devtools.capabilities: no result".into()),
        };
        self.capabilities = Some(result.clone());
        Ok(result)
    }

    fn call_tool(&mut self, conn: &mut dyn Connection, id: Value, params: &Value) -> Value {
        let Some(name) = params["name"].as_str() else {
            return failure(id, INVALID_PARAMS, "tools/call needs a tool name");
        };
        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => json!({}),
            Some(arguments) if arguments.is_object() => arguments.clone(),
            Some(_) => return failure(id, INVALID_PARAMS, "arguments must be an object"),
        };
        let capabilities = match self.capabilities(conn) {
            Ok(capabilities) => capabilities,
            Err(message) => return failure(id, INTERNAL_ERROR, &message),
        };
        // Only methods the app lists are forwarded.
        let method = methods(&capabilities)
            .into_iter()
            .find(|method| tool_name(method) == name);
        let Some(method) = method else {
            return failure(id, INVALID_PARAMS, &format!("unknown tool {name:?}"));
        };
        let request = self.request(&method, arguments);
        // The request is sent once. A failure is reported, not retried.
        match conn.call(&request) {
            Ok(reply) if reply.ok => success(
                id,
                tool_result(false, &reply.result.unwrap_or(Value::Null).to_string()),
            ),
            Ok(reply) => {
                let (code, message) = reply
                    .error
                    .map(|error| (error.code.as_str(), error.message))
                    .unwrap_or(("unknown", String::new()));
                success(id, tool_result(true, &format!("error: {code}: {message}")))
            }
            Err(error) => success(id, tool_result(true, &format!("error: {error}"))),
        }
    }
}

fn tool_result(is_error: bool, text: &str) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn failure(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn methods(capabilities: &Value) -> Vec<String> {
    capabilities["methods"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|m| m.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The tool name of a protocol method, e.g. `Action.setValue` ->
/// `action_set_value`.
pub fn tool_name(method: &str) -> String {
    let mut name = String::new();
    let mut previous_lower = false;
    for ch in method.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_lower {
                name.push('_');
            }
            name.push(ch.to_ascii_lowercase());
            previous_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if !name.ends_with('_') {
                name.push('_');
            }
            previous_lower = false;
        }
    }
    name
}

fn tools(capabilities: &Value) -> Vec<Value> {
    let application = capabilities["application"]
        .as_str()
        .unwrap_or("a Surface app");
    let version = &capabilities["version"];
    let schemas = capabilities["schemas"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let limits = &capabilities["limits"];
    methods(capabilities)
        .into_iter()
        .map(|method| {
            let domain = method.split('.').next().unwrap_or(&method);
            let (summary, schema) = known(&method);
            json!({
                "name": tool_name(&method),
                "description": format!(
                    "{summary} Surface devtools method {method} ({domain} domain, protocol \
                     version {version}) on {application}. Schemas: {schemas}. Request limit \
                     {} bytes, response limit {} bytes.",
                    limits["request_bytes"], limits["response_bytes"]
                ),
                "inputSchema": schema,
            })
        })
        .collect()
}

/// A summary and an input schema for the methods the client knows. Any
/// other method listed by the app gets a generic object schema.
fn known(method: &str) -> (&'static str, Value) {
    let string = json!({"type": "string"});
    match method {
        "Devtools.capabilities" => (
            "Describe the protocol: methods, limits and schemas.",
            json!({"type": "object", "properties": {}}),
        ),
        "DOM.snapshot" => (
            "Return the surface.dom.v1 snapshot with layout.",
            json!({"type": "object", "properties": {}}),
        ),
        "DOM.query" => (
            "Find nodes by stable id, role and accessible name.",
            json!({"type": "object", "properties": {
                "id": string, "role": string, "name": string}}),
        ),
        "Action.activate" => (
            "Activate a control, as a click or Enter would.",
            json!({"type": "object", "properties": {"id": string}, "required": ["id"]}),
        ),
        "Action.fill" => (
            "Replace the text of a text input.",
            json!({"type": "object",
                "properties": {"id": string, "value": string}, "required": ["id", "value"]}),
        ),
        "Action.setValue" => (
            "Set the numeric value of a slider or similar control.",
            json!({"type": "object",
                "properties": {"id": string, "value": {"type": "number"}},
                "required": ["id", "value"]}),
        ),
        "Action.key" => (
            "Send a semantic key to the focused control.",
            json!({"type": "object",
                "properties": {
                    "key": {"type": "string", "enum": [
                        "tab", "enter", "space", "escape", "left", "right", "up", "down",
                        "home", "end", "page_up", "page_down", "backspace", "delete",
                        "select_all", "undo", "redo"]},
                    "extend": {"type": "boolean"},
                    "by_word": {"type": "boolean"}},
                "required": ["key"]}),
        ),
        _ => (
            "Call a method the client has no schema for.",
            json!({"type": "object", "additionalProperties": true}),
        ),
    }
}
