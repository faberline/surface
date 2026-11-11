//! Surface browser and bridge end-to-end tests.
//!
//! Drives Chrome through jet-bridge's CDP client, and the real jet-bridge
//! CLI and MCP server for the bridge journeys. Replaces the Playwright and
//! Python scripts listed in issue #62. Normally run through
//! `cargo xtask web e2e`, which builds every input first.
//!
//! Every case prints its step IDs. The run fails when a case fails, when a
//! case skips a step, or when no case ran. There is no skip path.

mod bridge;
mod browser;
mod check;
mod ime;
mod keys;
mod material;
mod server;
mod smoke;

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;

const USAGE: &str = "\
usage: surface-e2e [options] [suite...]

suites: smoke ime material bridge-native bridge-native-compact bridge-web
        (default: all of them, in that order)

options:
  --chrome <path>       Chrome or Chromium (default: SURFACE_CHROME, then a system install)
  --chrome-arg <arg>    extra browser argument; repeatable (also SURFACE_CHROME_ARGS, space separated)
  --web-root <dir>      built web apps (default: target/web/release)
  --url <url>           test an app already served at <url> instead (one browser suite at a time)
  --bridge-web <dir>    devtools material gallery (default: target/web/devtools/material-gallery)
  --jet-bridge <path>   jet-bridge binary (default: target/debug/jet-bridge)
  --native-bin <path>   native gallery (default: target/debug/surface-material-gallery)
  --compact-bin <path>  compact native gallery (default: target/debug/examples/compact)
  --artifacts <dir>     screenshots and evidence (default: artifacts)
";

const SUITES: [&str; 6] = [
    "smoke",
    "ime",
    "material",
    "bridge-native",
    "bridge-native-compact",
    "bridge-web",
];

/// Paths shared by every suite.
pub struct Env {
    pub root: PathBuf,
    web_root: PathBuf,
    url: Option<String>,
    bridge_web: PathBuf,
    jet_bridge: PathBuf,
    artifacts: PathBuf,
}

impl Env {
    /// Serve a built app, or use `--url`. Keep the server alive while the
    /// page is open.
    pub fn serve(&self, app: &str) -> Result<(Option<server::StaticServer>, String)> {
        if let Some(url) = &self.url {
            return Ok((None, url.clone()));
        }
        let server = server::StaticServer::start(&self.web_root.join(app))?;
        let url = server.url();
        Ok((Some(server), url))
    }

    pub fn bridge_web_dir(&self) -> PathBuf {
        self.bridge_web.clone()
    }

    pub fn jet_bridge(&self) -> Result<PathBuf> {
        if !self.jet_bridge.is_file() {
            bail!(
                "no jet-bridge binary at {}; run `cargo xtask web e2e` or pass --jet-bridge",
                self.jet_bridge.display()
            );
        }
        Ok(self.jet_bridge.clone())
    }

    pub fn artifact(&self, name: &str) -> PathBuf {
        self.artifacts.join(name)
    }

    pub fn write_json(&self, name: &str, value: &Value) -> Result<()> {
        let mut text = serde_json::to_string_pretty(value)?;
        text.push('\n');
        std::fs::write(self.artifact(name), text).with_context(|| format!("writing {name}"))
    }
}

struct Options {
    suites: Vec<String>,
    chrome: Option<PathBuf>,
    chrome_args: Vec<String>,
    native_bin: PathBuf,
    compact_bin: PathBuf,
}

fn parse(root: &Path) -> Result<(Env, Options)> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let mut env = Env {
        root: root.to_path_buf(),
        web_root: target.join("web/release"),
        url: None,
        bridge_web: target.join("web/devtools/material-gallery"),
        jet_bridge: target.join("debug/jet-bridge"),
        artifacts: root.join("artifacts"),
    };
    let mut options = Options {
        suites: Vec::new(),
        chrome: std::env::var_os("SURFACE_CHROME").map(PathBuf::from),
        chrome_args: std::env::var("SURFACE_CHROME_ARGS")
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default(),
        native_bin: target.join("debug/surface-material-gallery"),
        compact_bin: target.join("debug/examples/compact"),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .with_context(|| format!("{arg} needs a value\n\n{USAGE}"))
        };
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--chrome" => options.chrome = Some(value()?.into()),
            "--chrome-arg" => options.chrome_args.push(value()?),
            "--web-root" => env.web_root = value()?.into(),
            "--url" => env.url = Some(value()?),
            "--bridge-web" => env.bridge_web = value()?.into(),
            "--jet-bridge" => env.jet_bridge = value()?.into(),
            "--native-bin" => options.native_bin = value()?.into(),
            "--compact-bin" => options.compact_bin = value()?.into(),
            "--artifacts" => env.artifacts = value()?.into(),
            "all" => options.suites.extend(SUITES.iter().map(|s| s.to_string())),
            suite if SUITES.contains(&suite) => options.suites.push(suite.to_string()),
            other => bail!("unknown argument {other}\n\n{USAGE}"),
        }
    }
    if options.suites.is_empty() {
        options.suites = SUITES.iter().map(|s| s.to_string()).collect();
    }
    std::fs::create_dir_all(&env.artifacts)?;
    Ok((env, options))
}

fn workspace_root() -> PathBuf {
    // The crate lives at <root>/e2e; prefer the caller's directory when it is
    // a checkout, so a moved binary still finds its inputs.
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("Cargo.toml").is_file() && cwd.join("e2e").is_dir() {
        return cwd;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or(cwd)
}

fn expected_steps(suite: &str) -> &'static [&'static str] {
    match suite {
        "smoke" => smoke::STEPS,
        "ime" => ime::STEPS,
        "material" => material::STEPS,
        "bridge-native" => bridge::NATIVE_STEPS,
        "bridge-native-compact" => bridge::NATIVE_OVERFLOW_STEPS,
        _ => bridge::WEB_STEPS,
    }
}

async fn run_suite(
    suite: &str,
    env: &Env,
    options: &Options,
    chrome: &mut Option<browser::Chrome>,
) -> Result<()> {
    if matches!(suite, "smoke" | "ime" | "material") && chrome.is_none() {
        let launched =
            browser::Chrome::launch(options.chrome.clone(), options.chrome_args.clone()).await?;
        println!(
            "browser: {} ({})",
            launched.version,
            launched.executable.display()
        );
        *chrome = Some(launched);
    }
    match suite {
        "smoke" => smoke::run(env, chrome.as_ref().context("no browser")?).await,
        "ime" => ime::run(env, chrome.as_ref().context("no browser")?).await,
        "material" => material::run(env, chrome.as_ref().context("no browser")?).await,
        "bridge-native" => {
            bridge::native(
                env,
                bridge::NativeOptions {
                    name: "bridge-native",
                    bin: &options.native_bin,
                    require_overflow: false,
                },
            )
            .await
        }
        "bridge-native-compact" => {
            bridge::native(
                env,
                bridge::NativeOptions {
                    name: "bridge-native-compact",
                    bin: &options.compact_bin,
                    require_overflow: true,
                },
            )
            .await
        }
        _ => {
            let executable = match &options.chrome {
                Some(path) => path.clone(),
                None => jet_bridge::browser::BrowserLauncher::find_system_chrome()
                    .context("no Chrome found; pass --chrome or set SURFACE_CHROME")?,
            };
            bridge::web(
                env,
                bridge::WebOptions {
                    chrome: &executable,
                    chrome_args: &options.chrome_args,
                    require_overflow: true,
                },
            )
            .await
        }
    }
}

#[tokio::main]
async fn main() {
    let code = match real_main().await {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            eprintln!("error: {error:#}");
            2
        }
    };
    std::process::exit(code);
}

async fn real_main() -> Result<bool> {
    let (env, options) = parse(&workspace_root())?;
    let mut chrome = None;
    let mut passed = 0usize;
    let mut failed = Vec::new();
    for suite in &options.suites {
        println!("case {suite} ...");
        let started = Instant::now();
        let before = check::total();
        check::take_steps();
        let result = run_suite(suite, &env, &options, &mut chrome).await;
        let reached = check::take_steps();
        let missing = check::missing(expected_steps(suite), &reached);
        let checks = check::total() - before;
        let seconds = started.elapsed().as_secs_f64();
        match (result, missing.is_empty()) {
            (Ok(()), true) if checks > 0 => {
                println!(
                    "case {suite} ... ok ({} steps, {checks} checks, {seconds:.1}s)",
                    reached.len()
                );
                passed += 1;
            }
            (Ok(()), true) => {
                println!("case {suite} ... FAILED: no checks ran");
                failed.push(suite.clone());
            }
            (Ok(()), false) => {
                println!("case {suite} ... FAILED: steps not reached: {missing:?}");
                failed.push(suite.clone());
            }
            (Err(error), _) => {
                println!("case {suite} ... FAILED after {seconds:.1}s: {error:#}");
                if let Some(last) = reached.last() {
                    println!("  last step: {last}");
                }
                failed.push(suite.clone());
            }
        }
    }
    if let Some(chrome) = chrome {
        chrome.close().await;
    }
    println!(
        "\ne2e result: {passed} passed; {} failed; {} checks; artifacts in {}",
        failed.len(),
        check::total(),
        env.artifacts.display()
    );
    if passed == 0 {
        println!("no case passed");
        return Ok(false);
    }
    if !failed.is_empty() {
        println!("failed: {}", failed.join(", "));
    }
    Ok(failed.is_empty())
}
