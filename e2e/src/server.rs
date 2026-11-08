//! A small static file server owned by one test run.
//!
//! Each journey starts its own server on a free port and stops it when the
//! journey ends, so an interrupted run leaves no daemon behind.

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct StaticServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl StaticServer {
    pub fn start(root: &Path) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("web root {} does not exist", root.display()))?;
        if !root.join("index.html").is_file() {
            anyhow::bail!("{} has no index.html; build it first", root.display());
        }
        let listener = TcpListener::bind("127.0.0.1:0").context("binding the test server")?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    let root = root.clone();
                    std::thread::spawn(move || {
                        let _ = serve(stream, &root);
                    });
                }
            }
        });
        Ok(Self {
            addr,
            stop,
            thread: Some(thread),
        })
    }

    pub fn url(&self) -> String {
        format!("http://{}/", self.addr)
    }
}

impl Drop for StaticServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so it sees the flag.
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(stream: TcpStream, root: &Path) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    // Drain headers.
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header == "\r\n" || header == "\n" {
            break;
        }
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let mut out = stream;
    if method != "GET" && method != "HEAD" {
        return respond(&mut out, 405, "text/plain", b"", false);
    }
    let Some(file) = resolve(root, target) else {
        return respond(&mut out, 403, "text/plain", b"", false);
    };
    match std::fs::read(&file) {
        Ok(data) => respond(&mut out, 200, content_type(&file), &data, method == "HEAD"),
        Err(_) => respond(&mut out, 404, "text/plain", b"", false),
    }
}

fn respond(
    out: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    write!(
        out,
        "HTTP/1.1 {status} {reason}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n",
        body.len()
    )?;
    if !head {
        out.write_all(body)?;
    }
    out.flush()
}

/// Map a request target to a file under `root`. Returns `None` for any
/// path that would leave the root.
pub fn resolve(root: &Path, target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let decoded = percent_decode(path)?;
    let relative = if decoded == "/" {
        "index.html"
    } else {
        decoded.trim_start_matches('/')
    };
    let mut file = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => file.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(file)
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("css") => "text/css",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_index_and_files() {
        let root = Path::new("/srv/app");
        assert_eq!(resolve(root, "/").unwrap(), root.join("index.html"));
        assert_eq!(
            resolve(root, "/app-1.js?v=1").unwrap(),
            root.join("app-1.js")
        );
        assert_eq!(resolve(root, "/a%20b.wasm").unwrap(), root.join("a b.wasm"));
    }

    #[test]
    fn rejects_paths_that_leave_the_root() {
        let root = Path::new("/srv/app");
        assert!(resolve(root, "/../etc/passwd").is_none());
        assert!(resolve(root, "/%2e%2e/etc/passwd").is_none());
        assert!(resolve(root, "/a/%2E%2E/%2E%2E/x").is_none());
        assert!(resolve(root, "/bad%zz").is_none());
    }

    #[test]
    fn serves_files_over_http() {
        let dir = std::env::temp_dir().join(format!("surface-e2e-server-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<p>ok</p>").unwrap();
        let server = StaticServer::start(&dir).unwrap();
        let get = |path: &str| {
            let mut stream = TcpStream::connect(server.addr).unwrap();
            write!(stream, "GET {path} HTTP/1.1\r\nhost: x\r\n\r\n").unwrap();
            let mut body = String::new();
            std::io::Read::read_to_string(&mut stream, &mut body).unwrap();
            body
        };
        let ok = get("/");
        assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
        assert!(ok.contains("content-type: text/html"));
        assert!(ok.ends_with("<p>ok</p>"));
        assert!(get("/missing.js").starts_with("HTTP/1.1 404"));
        assert!(get("/../secret").starts_with("HTTP/1.1 403"));
        drop(server);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
