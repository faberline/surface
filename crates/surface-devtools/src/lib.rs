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
use session::{Session, SessionError};
use std::io::{BufRead, Write};
use std::path::Path;

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
    args: &[String],
    env_session: Option<String>,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    connect: &mut dyn FnMut(&Session) -> Result<Box<dyn Connection>, ClientError>,
) -> i32 {
    let parsed = match cli::parse_args(args) {
        Ok(parsed) => parsed,
        Err(cli::UsageError(message)) => {
            let _ = writeln!(err, "surface-devtools: {message}\n\n{}", cli::USAGE);
            return EXIT_USAGE;
        }
    };
    let Some(path) = parsed.session.or(env_session).filter(|p| !p.is_empty()) else {
        let _ = writeln!(
            err,
            "surface-devtools: no session file: pass --session <file> or set \
             SURFACE_DEVTOOLS_SESSION"
        );
        return EXIT_USAGE;
    };
    let session = match Session::load(Path::new(&path)) {
        Ok(session) if !session::pid_alive(session.pid) => {
            let _ = writeln!(
                err,
                "surface-devtools: {}",
                SessionError::Stale(session.file)
            );
            return EXIT_CLIENT;
        }
        Ok(session) => session,
        Err(error) => {
            let _ = writeln!(err, "surface-devtools: {error}");
            return EXIT_CLIENT;
        }
    };
    let mut conn = match connect(&session) {
        Ok(conn) => conn,
        Err(error) => {
            let _ = writeln!(err, "surface-devtools: {error}");
            return EXIT_CLIENT;
        }
    };
    match parsed.command {
        cli::Command::Call { method, params } => {
            cli::execute_call(conn.as_mut(), &method, params, out, err)
        }
        cli::Command::Mcp => match mcp::McpServer::new().serve(conn.as_mut(), input, out) {
            Ok(()) => 0,
            Err(error) => {
                let _ = writeln!(err, "surface-devtools: mcp: {error}");
                EXIT_CLIENT
            }
        },
    }
}
