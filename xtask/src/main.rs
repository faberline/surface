//! Repository tasks for Surface.
//!
//! `cargo xtask web build <app>` builds a browser package. `web dev`
//! rebuilds on change and reloads the page. `web serve` serves a built
//! package with the headers a static host should send.

mod apps;
mod dev;
mod manifest;
mod serve;
mod tools;
mod web_build;
mod workspace;

use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::apps::App;
use crate::serve::ServeOptions;
use crate::web_build::{BuildOptions, Profile};
use crate::workspace::Workspace;

const USAGE: &str = "\
Usage:
  cargo xtask web build <app> [--release | --dev] [--features <list>] [--out <dir>] [--no-compress]
  cargo xtask web dev <app> [--features <list>] [--host <addr>] [--port <n>] [--isolate]
  cargo xtask web serve <app> [--release | --dev] [--dir <dir>] [--host <addr>] [--port <n>] [--isolate]
  cargo xtask web apps

Apps: showcase, material-gallery.

build   Release by default (--release says so explicitly): the wasm-release profile, wasm-opt, and
        .br/.gz sidecars. --dev uses the dev profile without wasm-opt.
        Output: target/web/<release|dev>/<app>/.
dev     Builds with --dev, serves it, rebuilds on source changes and
        reloads the page. A failed rebuild keeps the last good build.
serve   Serves a built package. Hashed files are cached as immutable.
        --isolate adds COOP/COEP headers for cross-origin isolation.
";

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<()> {
    let mut args = args.into_iter();
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("web"), Some(command)) => web(command, args.collect()),
        (None | Some("help" | "--help" | "-h"), _) | (Some("web"), None) => {
            print!("{USAGE}");
            Ok(())
        }
        (Some(other), _) => bail!("unknown task `{other}`\n\n{USAGE}"),
    }
}

fn web(command: &str, args: Vec<String>) -> Result<()> {
    let ws = Workspace::locate()?;
    if command == "apps" {
        for app in apps::ALL {
            println!("{:<18} {}", app.name, app.package);
        }
        return Ok(());
    }
    if !matches!(command, "build" | "dev" | "serve") {
        bail!("unknown web command `{command}`\n\n{USAGE}");
    }
    let flags = Flags::parse(args)?;
    let app = flags
        .app
        .context("missing <app>; run `cargo xtask web apps`")?;
    match command {
        "build" => {
            flags.reject(&["--host", "--port", "--isolate", "--dir"])?;
            let profile = flags.profile();
            let out = flags
                .out
                .clone()
                .unwrap_or_else(|| ws.web_dir(profile, app));
            let options = BuildOptions {
                app,
                profile,
                features: flags.features.clone(),
                out,
                compress: profile == Profile::Release && !flags.no_compress,
            };
            let report = web_build::build(&ws, &options)?;
            report.print();
            Ok(())
        }
        "dev" => {
            flags.reject(&["--dev", "--release", "--out", "--dir", "--no-compress"])?;
            dev::run(
                &ws,
                app,
                flags.features.clone(),
                flags.serve_options(ws.web_dir(Profile::Dev, app))?,
            )
        }
        _ => {
            flags.reject(&["--features", "--out", "--no-compress"])?;
            let root = flags
                .dir
                .clone()
                .unwrap_or_else(|| ws.web_dir(flags.profile(), app));
            if !root.join("index.html").is_file() {
                bail!(
                    "{} has no index.html; run `cargo xtask web build {}{}` first",
                    root.display(),
                    app.name,
                    if flags.dev { " --dev" } else { "" }
                );
            }
            serve::serve(flags.serve_options(root)?, None)
        }
    }
}

/// Parsed command-line flags. Each command rejects the ones it does not use.
#[derive(Default)]
struct Flags {
    app: Option<&'static App>,
    dev: bool,
    release: bool,
    features: Vec<String>,
    out: Option<PathBuf>,
    dir: Option<PathBuf>,
    host: Option<String>,
    port: Option<u16>,
    isolate: bool,
    no_compress: bool,
    seen: Vec<String>,
}

impl Flags {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut flags = Flags::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let (name, inline) = match arg.split_once('=') {
                Some((name, value)) if name.starts_with("--") => {
                    (name.to_string(), Some(value.to_string()))
                }
                _ => (arg.clone(), None),
            };
            let mut value = |name: &str| -> Result<String> {
                inline
                    .clone()
                    .or_else(|| args.next())
                    .with_context(|| format!("{name} needs a value"))
            };
            match name.as_str() {
                "--dev" => flags.dev = true,
                "--release" => flags.release = true,
                "--isolate" => flags.isolate = true,
                "--no-compress" => flags.no_compress = true,
                "--features" | "-F" => flags.features.extend(
                    value(&name)?
                        .split([',', ' '])
                        .filter(|feature| !feature.is_empty())
                        .map(str::to_string),
                ),
                "--out" => flags.out = Some(PathBuf::from(value(&name)?)),
                "--dir" => flags.dir = Some(PathBuf::from(value(&name)?)),
                "--host" => flags.host = Some(value(&name)?),
                "--port" => {
                    let raw = value(&name)?;
                    flags.port = Some(raw.parse().with_context(|| format!("bad port `{raw}`"))?);
                }
                other if other.starts_with('-') => bail!("unknown flag `{other}`\n\n{USAGE}"),
                other => {
                    if flags.app.is_some() {
                        bail!("unexpected argument `{other}`");
                    }
                    flags.app = Some(apps::find(other)?);
                    continue;
                }
            }
            flags.seen.push(name);
        }
        if flags.dev && flags.release {
            bail!("--dev and --release cannot be used together");
        }
        Ok(flags)
    }

    fn reject(&self, unsupported: &[&str]) -> Result<()> {
        match self
            .seen
            .iter()
            .find(|flag| unsupported.contains(&flag.as_str()))
        {
            Some(flag) => bail!("`{flag}` does not apply to this command"),
            None => Ok(()),
        }
    }

    fn profile(&self) -> Profile {
        if self.dev {
            Profile::Dev
        } else {
            Profile::Release
        }
    }

    fn serve_options(&self, root: PathBuf) -> Result<ServeOptions> {
        Ok(ServeOptions {
            root,
            host: self.host.clone().unwrap_or_else(|| "127.0.0.1".into()),
            port: self.port.unwrap_or(8080),
            isolate: self.isolate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Flags> {
        Flags::parse(args.iter().map(|arg| arg.to_string()).collect())
    }

    #[test]
    fn flags_accept_both_value_forms_and_split_features() {
        let flags = parse(&[
            "material-gallery",
            "--features=devtools",
            "--port",
            "0",
            "-F",
            "a,b",
        ])
        .unwrap();
        assert_eq!(flags.app.unwrap().package, "surface-material-gallery");
        assert_eq!(flags.features, ["devtools", "a", "b"]);
        assert_eq!(flags.port, Some(0));
        assert_eq!(flags.profile(), Profile::Release);
    }

    #[test]
    fn flags_reject_unknown_apps_flags_and_misplaced_flags() {
        assert!(parse(&["nope"]).is_err());
        assert!(parse(&["showcase", "--fast"]).is_err());
        assert!(parse(&["showcase", "--port"]).is_err());
        let flags = parse(&["showcase", "--isolate"]).unwrap();
        assert!(flags.reject(&["--isolate"]).is_err());
        assert!(flags.reject(&["--dev"]).is_ok());
        assert!(parse(&["showcase", "--dev", "--release"]).is_err());
        let release = parse(&["showcase", "--release"]).unwrap();
        assert_eq!(release.profile(), Profile::Release);
        assert_eq!(
            parse(&["showcase", "--dev"]).unwrap().profile(),
            Profile::Dev
        );
    }
}
