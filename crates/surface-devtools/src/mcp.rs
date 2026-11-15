//! MCP over stdio: `initialize`, `tools/list`, `tools/call`.
use crate::connection::Connection;
use serde_json::Value;
use std::io::{BufRead, Write};

/// An MCP server over one connection. It caches `Devtools.capabilities`.
#[derive(Default)]
pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        Self
    }

    /// Handle one JSON-RPC line. Returns the response, or `None` for a
    /// notification.
    pub fn handle_line(&mut self, _conn: &mut dyn Connection, _line: &str) -> Option<Value> {
        unimplemented!("red")
    }

    /// Serve until the input ends. One response line per request line.
    pub fn serve(
        &mut self,
        _conn: &mut dyn Connection,
        _input: &mut dyn BufRead,
        _out: &mut dyn Write,
    ) -> std::io::Result<()> {
        unimplemented!("red")
    }
}

/// The tool name of a protocol method, e.g. `Action.setValue` ->
/// `action_set_value`.
pub fn tool_name(_method: &str) -> String {
    unimplemented!("red")
}
