//! `cargo xtask web dev`: build, serve, watch, rebuild and reload.
//!
//! Builds use the dev profile. A rebuild writes to a staging directory, so
//! a failed rebuild keeps serving the last good build. The page holds an
//! `EventSource` on [`EVENTS_PATH`]; it reloads after a good rebuild and
//! shows the error after a failed one.

use anyhow::Result;
use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::apps::App;
use crate::serve::{self, ServeOptions, EVENTS_PATH};
use crate::web_build::{self, BuildOptions, Profile};
use crate::workspace::Workspace;

const POLL: Duration = Duration::from_millis(400);
const SETTLE: Duration = Duration::from_millis(250);
const KEEPALIVE: Duration = Duration::from_secs(15);

/// Build state shared by the watcher and the event streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildState {
    /// Increases after each successful build.
    pub generation: u64,
    /// The last failure, cleared by the next success.
    pub error: Option<String>,
    /// Increases after each build attempt.
    pub attempt: u64,
}

pub struct Reload {
    state: Mutex<BuildState>,
    changed: Condvar,
}

impl Default for Reload {
    fn default() -> Self {
        Self {
            state: Mutex::new(BuildState {
                generation: 0,
                error: None,
                attempt: 0,
            }),
            changed: Condvar::new(),
        }
    }
}

impl Reload {
    pub fn succeeded(&self) {
        let mut state = self.state.lock().unwrap();
        state.generation += 1;
        state.attempt += 1;
        state.error = None;
        self.changed.notify_all();
    }

    pub fn failed(&self, error: String) {
        let mut state = self.state.lock().unwrap();
        state.attempt += 1;
        state.error = Some(error);
        self.changed.notify_all();
    }

    pub fn snapshot(&self) -> BuildState {
        self.state.lock().unwrap().clone()
    }

    /// Waits until the attempt counter passes `seen`, or the timeout ends.
    pub fn wait_after(&self, seen: u64, timeout: Duration) -> BuildState {
        let state = self.state.lock().unwrap();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| state.attempt <= seen)
            .unwrap();
        state.clone()
    }

    /// Server-sent events: `build` carries the generation, `error` carries
    /// the failure text. Runs until the browser disconnects.
    pub fn stream_events(&self, mut stream: TcpStream) -> Result<()> {
        stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
              Cache-Control: no-store\r\nConnection: keep-alive\r\n\r\n",
        )?;
        let mut state = self.snapshot();
        stream.write_all(event(&state).as_bytes())?;
        loop {
            let next = self.wait_after(state.attempt, KEEPALIVE);
            if next.attempt == state.attempt {
                stream.write_all(b": keepalive\n\n")?;
            } else {
                stream.write_all(event(&next).as_bytes())?;
                state = next;
            }
            stream.flush()?;
        }
    }

    /// Adds the reload client to a built page.
    pub fn inject(&self, html: &str) -> String {
        let client = client_script();
        match html.rfind("</html>") {
            Some(at) => format!("{}{client}\n{}", &html[..at], &html[at..]),
            None => format!("{html}\n{client}\n"),
        }
    }

    /// Shown while no build has succeeded yet.
    pub fn waiting_page(&self) -> String {
        let state = self.snapshot();
        let message = match &state.error {
            Some(_) => {
                "The first build failed. Fix the error; this page reloads after a good build."
            }
            None => "Building… this page reloads when the first build is ready.",
        };
        format!(
            "<!doctype html>\n<meta charset=\"utf-8\">\n<title>Surface dev</title>\n\
             <body style=\"font:14px system-ui;margin:24px\"><p>{message}</p>\n{}\n</body>\n",
            client_script()
        )
    }
}

/// One SSE message for `state`.
pub fn event(state: &BuildState) -> String {
    match &state.error {
        None => format!("event: build\ndata: {}\n\n", state.generation),
        Some(error) => {
            let mut text = String::from("event: error\n");
            for line in error.lines().take(40) {
                text.push_str("data: ");
                text.push_str(line);
                text.push('\n');
            }
            text.push('\n');
            text
        }
    }
}

fn client_script() -> String {
    format!(
        r#"<script>
(() => {{
  let seen = null, banner = null;
  const events = new EventSource('{EVENTS_PATH}');
  events.addEventListener('build', (e) => {{
    if (seen !== null && e.data !== seen) location.reload();
    seen = e.data;
    banner?.remove(); banner = null;
  }});
  events.addEventListener('error', (e) => {{
    if (!e.data) return;
    console.error('surface dev: rebuild failed; serving the last good build\n' + e.data);
    banner ??= document.body.appendChild(document.createElement('pre'));
    banner.style.cssText = 'position:fixed;left:0;right:0;bottom:0;max-height:40vh;overflow:auto;margin:0;padding:12px;background:#601410;color:#fff;font:12px ui-monospace,monospace;z-index:2147483647';
    banner.textContent = 'Rebuild failed. Serving the last good build.\n\n' + e.data;
  }});
}})();
</script>"#
    )
}

pub fn run(
    ws: &Workspace,
    app: &'static App,
    features: Vec<String>,
    options: ServeOptions,
) -> Result<()> {
    let reload = Arc::new(Reload::default());
    let listener = serve::bind(&options)?;
    println!(
        "dev server for {} at http://{}/ (Ctrl-C to stop)",
        app.name,
        listener.local_addr()?
    );
    let out = options.root.clone();
    {
        let reload = Arc::clone(&reload);
        let options = Arc::new(options);
        std::thread::spawn(move || serve::accept(listener, options, Some(reload)));
    }

    let build = |reload: &Reload| {
        let started = Instant::now();
        let options = BuildOptions {
            app,
            profile: Profile::Dev,
            features: features.clone(),
            out: out.clone(),
            compress: false,
        };
        match web_build::build(ws, &options) {
            Ok(report) => {
                report.print();
                println!(
                    "  ready in {:.1}s; reloading the page",
                    started.elapsed().as_secs_f32()
                );
                reload.succeeded();
            }
            Err(error) => {
                let text = format!("{error:#}");
                eprintln!("error: {text}");
                eprintln!("  keeping the last good build; waiting for changes");
                reload.failed(text);
            }
        }
    };

    let roots = watch_roots(&ws.root, app);
    let mut seen = snapshot(&roots);
    build(&reload);
    loop {
        std::thread::sleep(POLL);
        let now = snapshot(&roots);
        if now == seen {
            continue;
        }
        // Wait for editors and formatters to finish writing.
        let mut settled = now;
        loop {
            std::thread::sleep(SETTLE);
            let again = snapshot(&roots);
            if again == settled {
                break;
            }
            settled = again;
        }
        for path in changed(&seen, &settled).iter().take(5) {
            println!("changed: {}", path.display());
        }
        build(&reload);
        // Ignore changes the build itself made under the watched roots.
        seen = snapshot(&roots);
    }
}

/// Inputs that can change the app: the workspace manifests and every
/// workspace crate and the app's own directory.
pub fn watch_roots(root: &Path, app: &App) -> Vec<PathBuf> {
    vec![
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
        root.join("crates"),
        root.join(app.dir),
    ]
}

type Snapshot = BTreeMap<PathBuf, (SystemTime, u64)>;

pub fn snapshot(roots: &[PathBuf]) -> Snapshot {
    fn walk(path: &Path, out: &mut Snapshot) {
        let Ok(meta) = std::fs::metadata(path) else {
            return;
        };
        if meta.is_dir() {
            let Ok(entries) = std::fs::read_dir(path) else {
                return;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') || name == "target" || name == "pkg" {
                    continue;
                }
                walk(&entry.path(), out);
            }
        } else if meta.is_file() {
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            out.insert(path.to_path_buf(), (modified, meta.len()));
        }
    }
    let mut out = Snapshot::new();
    for root in roots {
        walk(root, &mut out);
    }
    out
}

fn changed(before: &Snapshot, after: &Snapshot) -> Vec<PathBuf> {
    let mut paths: Vec<_> = after
        .iter()
        .filter(|(path, stamp)| before.get(*path) != Some(stamp))
        .map(|(path, _)| path.clone())
        .collect();
    paths.extend(
        before
            .keys()
            .filter(|path| !after.contains_key(*path))
            .cloned(),
    );
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_carry_generation_or_multiline_error() {
        let reload = Reload::default();
        reload.succeeded();
        assert_eq!(event(&reload.snapshot()), "event: build\ndata: 1\n\n");
        reload.failed("line one\nline two".into());
        let state = reload.snapshot();
        assert_eq!(
            state.generation, 1,
            "a failure keeps the last good generation"
        );
        assert_eq!(
            event(&state),
            "event: error\ndata: line one\ndata: line two\n\n"
        );
        reload.succeeded();
        let state = reload.snapshot();
        assert_eq!((state.generation, state.attempt, state.error), (2, 3, None));
    }

    #[test]
    fn waiters_wake_on_the_next_attempt_only() {
        let reload = Arc::new(Reload::default());
        let early = reload.wait_after(0, Duration::from_millis(10));
        assert_eq!(early.attempt, 0, "timeout without a build");
        let waiter = {
            let reload = Arc::clone(&reload);
            std::thread::spawn(move || reload.wait_after(0, Duration::from_secs(10)))
        };
        reload.failed("boom".into());
        let state = waiter.join().unwrap();
        assert_eq!(state.attempt, 1);
        assert_eq!(state.error.as_deref(), Some("boom"));
    }

    #[test]
    fn reload_client_goes_before_the_closing_html_tag() {
        let reload = Reload::default();
        let html = reload.inject("<html><body>x</body></html>\n");
        assert!(html.contains(EVENTS_PATH));
        assert!(html.find("EventSource").unwrap() < html.find("</html>").unwrap());
        assert!(reload.inject("<p>no end tag").contains("EventSource"));
        assert!(reload.waiting_page().contains("EventSource"));
    }

    #[test]
    fn snapshot_sees_edits_and_skips_target_and_hidden_dirs() {
        let root = std::env::temp_dir().join(format!("xtask-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["src", "target", ".git", "web/pkg"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("src/lib.rs"), "a").unwrap();
        std::fs::write(root.join("target/out"), "a").unwrap();
        std::fs::write(root.join(".git/HEAD"), "a").unwrap();
        std::fs::write(root.join("web/pkg/x.js"), "a").unwrap();
        let roots = vec![root.clone()];
        let before = snapshot(&roots);
        assert_eq!(
            before.keys().collect::<Vec<_>>(),
            [&root.join("src/lib.rs")]
        );

        std::fs::write(root.join("target/out"), "bb").unwrap();
        assert_eq!(snapshot(&roots), before, "ignored dirs do not trigger");
        std::fs::write(root.join("src/lib.rs"), "bb").unwrap();
        std::fs::write(root.join("src/new.rs"), "c").unwrap();
        let after = snapshot(&roots);
        let mut paths = changed(&before, &after);
        paths.sort();
        assert_eq!(paths, [root.join("src/lib.rs"), root.join("src/new.rs")]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
