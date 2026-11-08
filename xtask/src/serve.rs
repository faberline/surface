//! A small local static server for built web packages.
//!
//! It sends the headers a production static host should send: the
//! `application/wasm` type, precompressed `.br`/`.gz` sidecars chosen by
//! `Accept-Encoding`, `immutable` caching for content-hashed names,
//! revalidation for everything else, and optional COOP/COEP. It is a
//! development tool. It binds to loopback by default and serves one
//! request per connection.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::dev::Reload;
use crate::manifest::Manifest;

pub struct ServeOptions {
    pub root: PathBuf,
    pub host: String,
    pub port: u16,
    pub isolate: bool,
}

/// Path of the dev reload event stream.
pub const EVENTS_PATH: &str = "/__surface/events";

pub const IMMUTABLE: &str = "public, max-age=31536000, immutable";
pub const REVALIDATE: &str = "no-cache";

/// Serves forever. `reload` is set by `web dev`.
pub fn serve(options: ServeOptions, reload: Option<Arc<Reload>>) -> Result<()> {
    let listener = bind(&options)?;
    let address = listener.local_addr()?;
    println!("serving {} at http://{address}/", options.root.display());
    if options.isolate {
        println!("  cross-origin isolation headers are on (COOP/COEP)");
    }
    accept(listener, Arc::new(options), reload);
    Ok(())
}

pub fn bind(options: &ServeOptions) -> Result<TcpListener> {
    TcpListener::bind((options.host.as_str(), options.port))
        .with_context(|| format!("binding {}:{}", options.host, options.port))
}

pub fn accept(listener: TcpListener, options: Arc<ServeOptions>, reload: Option<Arc<Reload>>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let options = Arc::clone(&options);
        let reload = reload.clone();
        std::thread::spawn(move || {
            let _ = handle(stream, &options, reload.as_deref());
        });
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
}

pub fn read_request(stream: &mut impl Read) -> Result<Request> {
    let mut reader = BufReader::new(stream.take(16 * 1024));
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().context("empty request")?.to_string();
    let target = parts.next().context("request has no target")?;
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();
    let mut headers = HashMap::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok(Request {
        method,
        path,
        headers,
    })
}

fn handle(mut stream: TcpStream, options: &ServeOptions, reload: Option<&Reload>) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let request = read_request(&mut stream)?;
    if let (Some(reload), EVENTS_PATH) = (reload, request.path.as_str()) {
        return reload.stream_events(stream);
    }
    let response = respond(&request, options, reload);
    response.write(&mut stream, request.method == "HEAD")
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
            body: body.into(),
        }
    }

    #[cfg(test)]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn set(&mut self, name: &str, value: impl Into<String>) {
        self.headers
            .retain(|(key, _)| !key.eq_ignore_ascii_case(name));
        self.headers.push((name.into(), value.into()));
    }

    fn write(&self, stream: &mut impl Write, head_only: bool) -> Result<()> {
        let reason = match self.status {
            200 => "OK",
            304 => "Not Modified",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            _ => "Error",
        };
        let mut head = format!("HTTP/1.1 {} {reason}\r\n", self.status);
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            self.body.len()
        ));
        stream.write_all(head.as_bytes())?;
        if !head_only && self.status != 304 {
            stream.write_all(&self.body)?;
        }
        stream.flush()?;
        Ok(())
    }
}

/// Builds the response for one request. Pure apart from reading `root`.
pub fn respond(request: &Request, options: &ServeOptions, reload: Option<&Reload>) -> Response {
    let mut response = respond_file(request, &options.root, reload);
    response.set("X-Content-Type-Options", "nosniff");
    if options.isolate {
        response.set("Cross-Origin-Opener-Policy", "same-origin");
        response.set("Cross-Origin-Embedder-Policy", "require-corp");
        response.set("Cross-Origin-Resource-Policy", "same-origin");
    }
    response
}

fn respond_file(request: &Request, root: &Path, reload: Option<&Reload>) -> Response {
    if !matches!(request.method.as_str(), "GET" | "HEAD") {
        let mut response = Response::new(405, "method not allowed\n");
        response.set("Allow", "GET, HEAD");
        return response;
    }
    let Some(relative) = safe_relative(&request.path) else {
        return Response::new(403, "forbidden\n");
    };
    let relative = if relative.is_empty() {
        "index.html".to_string()
    } else {
        relative
    };
    let file = root.join(&relative);
    let manifest = Manifest::read(root).ok();
    let entry = manifest
        .as_ref()
        .and_then(|manifest| manifest.file(&relative));
    // Sidecars and the manifest are served only through their sources.
    if !file.is_file() || relative.ends_with(".br") || relative.ends_with(".gz") {
        return match (reload, relative.as_str()) {
            (Some(reload), "index.html") => {
                let mut response = Response::new(200, reload.waiting_page());
                response.set("Content-Type", "text/html; charset=utf-8");
                response.set("Cache-Control", "no-store");
                response
            }
            _ => Response::new(404, "not found\n"),
        };
    }

    let mut response = Response::new(200, Vec::new());
    response.set("Content-Type", content_type(&relative));
    let immutable = entry.is_some_and(|entry| entry.immutable);
    response.set(
        "Cache-Control",
        match (reload.is_some(), immutable) {
            (_, true) => IMMUTABLE,
            (true, false) => "no-store",
            (false, false) => REVALIDATE,
        },
    );
    if let Some(entry) = entry {
        let etag = format!("\"{}\"", entry.sha256);
        let matches = request
            .headers
            .get("if-none-match")
            .is_some_and(|value| value.split(',').any(|tag| tag.trim() == etag));
        response.set("ETag", etag);
        if matches && reload.is_none() {
            response.status = 304;
            return response;
        }
    }

    // The dev page gets the reload client. It changes the body, so the
    // sidecars do not apply.
    if let (Some(reload), "index.html") = (reload, relative.as_str()) {
        let Ok(html) = std::fs::read_to_string(&file) else {
            return Response::new(404, "not found\n");
        };
        response.body = reload.inject(&html).into_bytes();
        response.headers.retain(|(key, _)| key != "ETag");
        return response;
    }

    let accept = request
        .headers
        .get("accept-encoding")
        .map(String::as_str)
        .unwrap_or("");
    let compressible = entry.is_some();
    for (encoding, suffix) in [("br", ".br"), ("gzip", ".gz")] {
        let sidecar = crate::web_build::with_suffix(&file, suffix);
        if compressible && accepts(accept, encoding) && sidecar.is_file() {
            if let Ok(body) = std::fs::read(&sidecar) {
                response.body = body;
                response.set("Content-Encoding", encoding);
                response.set("Vary", "Accept-Encoding");
                return response;
            }
        }
    }
    match std::fs::read(&file) {
        Ok(body) => {
            response.body = body;
            if compressible {
                response.set("Vary", "Accept-Encoding");
            }
            response
        }
        Err(_) => Response::new(404, "not found\n"),
    }
}

/// Decodes a URL path into a relative path with no `..`, root or prefix
/// parts and no hidden segments. `None` means the path is refused.
pub fn safe_relative(path: &str) -> Option<String> {
    let decoded = percent_decode(path)?;
    let trimmed = decoded.trim_start_matches('/');
    let mut parts = Vec::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str()?;
                if part.starts_with('.') || part.contains('\\') {
                    return None;
                }
                parts.push(part.to_string());
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(out).ok()?;
    (!decoded.contains('\0')).then_some(decoded)
}

/// True when `Accept-Encoding` lists `encoding` without `q=0`.
pub fn accepts(header: &str, encoding: &str) -> bool {
    header.split(',').any(|item| {
        let mut parts = item.split(';');
        let name = parts.next().unwrap_or("").trim();
        let refused = parts.any(|param| {
            param
                .trim()
                .strip_prefix("q=")
                .and_then(|q| q.trim().parse::<f32>().ok())
                .is_some_and(|q| q <= 0.0)
        });
        name.eq_ignore_ascii_case(encoding) && !refused
    })
}

pub fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{FileEntry, Tools, SCHEMA_VERSION};

    fn fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("xtask-serve-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let files = [
            ("index.html", "<html><script></script></html>", false),
            ("app-0011223344556677.js", "export default 1;", true),
            ("app_bg-8899aabbccddeeff.wasm", "\0asm", true),
        ];
        let mut entries = Vec::new();
        for (path, body, immutable) in files {
            std::fs::write(root.join(path), body).unwrap();
            entries.push(FileEntry {
                path: path.into(),
                sha256: crate::manifest::sha256_hex(body.as_bytes()),
                bytes: body.len() as u64,
                immutable,
                br_bytes: None,
                gzip_bytes: None,
            });
        }
        std::fs::write(root.join("app_bg-8899aabbccddeeff.wasm.br"), "BR").unwrap();
        std::fs::write(root.join("app_bg-8899aabbccddeeff.wasm.gz"), "GZ").unwrap();
        Manifest {
            schema_version: SCHEMA_VERSION,
            app: "app".into(),
            package: "app".into(),
            profile: "release".into(),
            cargo_profile: "wasm-release".into(),
            rustc_target: "wasm32-unknown-unknown".into(),
            features: vec![],
            tools: Tools {
                rustc: String::new(),
                wasm_bindgen: String::new(),
                wasm_opt: None,
                wasm_opt_args: vec![],
            },
            entry: "index.html".into(),
            js: "app-0011223344556677.js".into(),
            wasm: "app_bg-8899aabbccddeeff.wasm".into(),
            files: entries,
        }
        .write(&root)
        .unwrap();
        root
    }

    fn get(path: &str, headers: &[(&str, &str)]) -> Request {
        Request {
            method: "GET".into(),
            path: path.into(),
            headers: headers
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    fn options(root: &Path, isolate: bool) -> ServeOptions {
        ServeOptions {
            root: root.to_path_buf(),
            host: "127.0.0.1".into(),
            port: 0,
            isolate,
        }
    }

    #[test]
    fn wasm_gets_its_type_immutable_cache_and_the_best_sidecar() {
        let root = fixture("wasm");
        let options = options(&root, false);
        let path = "/app_bg-8899aabbccddeeff.wasm";

        let br = respond(
            &get(path, &[("accept-encoding", "gzip, br")]),
            &options,
            None,
        );
        assert_eq!(br.status, 200);
        assert_eq!(br.header("content-type"), Some("application/wasm"));
        assert_eq!(br.header("cache-control"), Some(IMMUTABLE));
        assert_eq!(br.header("content-encoding"), Some("br"));
        assert_eq!(br.header("vary"), Some("Accept-Encoding"));
        assert_eq!(br.body, b"BR");

        let gzip = respond(
            &get(path, &[("accept-encoding", "gzip, br;q=0")]),
            &options,
            None,
        );
        assert_eq!(gzip.header("content-encoding"), Some("gzip"));
        assert_eq!(gzip.body, b"GZ");

        let raw = respond(&get(path, &[]), &options, None);
        assert_eq!(raw.header("content-encoding"), None);
        assert_eq!(raw.body, b"\0asm");
        assert!(raw.header("cross-origin-opener-policy").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn html_revalidates_and_answers_matching_etags_with_304() {
        let root = fixture("html");
        let options = options(&root, true);
        let page = respond(&get("/", &[]), &options, None);
        assert_eq!(page.status, 200);
        assert_eq!(page.header("cache-control"), Some(REVALIDATE));
        assert_eq!(
            page.header("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            page.header("cross-origin-opener-policy"),
            Some("same-origin")
        );
        assert_eq!(
            page.header("cross-origin-embedder-policy"),
            Some("require-corp")
        );
        let etag = page.header("etag").unwrap().to_string();
        let cached = respond(
            &get("/index.html", &[("if-none-match", &etag)]),
            &options,
            None,
        );
        assert_eq!(cached.status, 304);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn traversal_hidden_files_sidecars_and_methods_are_refused() {
        let root = fixture("refuse");
        let options = options(&root, false);
        for path in [
            "/../Cargo.toml",
            "/%2e%2e/x",
            "/a/../../x",
            "/.git/config",
            "/%00",
        ] {
            assert_eq!(
                respond(&get(path, &[]), &options, None).status,
                403,
                "{path}"
            );
        }
        let sidecar = "/app_bg-8899aabbccddeeff.wasm.br";
        assert_eq!(respond(&get(sidecar, &[]), &options, None).status, 404);
        assert_eq!(
            respond(&get("/missing.js", &[]), &options, None).status,
            404
        );
        let mut post = get("/", &[]);
        post.method = "POST".into();
        assert_eq!(respond(&post, &options, None).status, 405);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn request_line_and_headers_parse() {
        let raw = b"GET /a.js?v=1 HTTP/1.1\r\nHost: x\r\nAccept-Encoding: br\r\n\r\n";
        let request = read_request(&mut &raw[..]).unwrap();
        assert_eq!(request.path, "/a.js");
        assert_eq!(request.headers["accept-encoding"], "br");
        assert!(accepts("gzip, deflate, br, zstd", "br"));
        assert!(!accepts("gzip;q=0", "gzip"));
        assert!(!accepts("brotli", "br"));
    }
}
