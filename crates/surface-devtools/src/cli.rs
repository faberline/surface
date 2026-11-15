//! Argument parsing and the command layer.
use crate::connection::Connection;
use serde_json::Value;
use std::io::Write;

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

pub fn parse_args(_args: &[String]) -> Result<Parsed, UsageError> {
    unimplemented!("red")
}

/// Send one call. Result JSON on `out`; a protocol error on `err`.
/// Returns the exit code.
pub fn execute_call(
    _conn: &mut dyn Connection,
    _method: &str,
    _params: Value,
    _out: &mut dyn Write,
    _err: &mut dyn Write,
) -> i32 {
    unimplemented!("red")
}
