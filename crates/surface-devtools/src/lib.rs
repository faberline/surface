//! CLI and MCP client for the Surface devtools protocol.
//! See `docs/devtools-protocol.md`, section "Client".
//!
//! The command layer talks to a [`connection::Connection`]. Only the native
//! loopback TCP implementation exists; a web transport can be added without
//! changing the commands.
pub mod cli;
pub mod connection;
pub mod mcp;
pub mod session;

use connection::{ClientError, Connection};
use session::Session;
use std::io::{BufRead, Write};

/// Exit code of a protocol error reply.
pub const EXIT_PROTOCOL: i32 = 1;
/// Exit code of a usage error.
pub const EXIT_USAGE: i32 = 2;
/// Exit code of a session, app or connection failure.
pub const EXIT_CLIENT: i32 = 3;

/// Run the program. Returns the process exit code.
pub fn run(
    args: Vec<String>,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let env = std::env::var("SURFACE_DEVTOOLS_SESSION").ok();
    run_with(&args, env, input, out, err, &mut |session| {
        Ok(Box::new(connection::NativeTcp::connect(session)?) as Box<dyn Connection>)
    })
}

/// [`run`] with the session environment value and the connector given.
pub fn run_with(
    _args: &[String],
    _env_session: Option<String>,
    _input: &mut dyn BufRead,
    _out: &mut dyn Write,
    _err: &mut dyn Write,
    _connect: &mut dyn FnMut(&Session) -> Result<Box<dyn Connection>, ClientError>,
) -> i32 {
    unimplemented!("red")
}
