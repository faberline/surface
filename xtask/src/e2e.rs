//! `cargo xtask web e2e`: build every input of the end-to-end tests, then
//! run `surface-e2e`.

use anyhow::{bail, Result};
use std::process::Command;

use crate::apps;
use crate::web_build::{self, BuildOptions, Profile};
use crate::workspace::Workspace;

pub fn run(ws: &Workspace, mut args: Vec<String>) -> Result<()> {
    let build = match args.iter().position(|a| a == "--no-build") {
        Some(index) => {
            args.remove(index);
            false
        }
        None => true,
    };
    let help = args.iter().any(|a| a == "--help" || a == "-h");
    if build && !help {
        let gallery = apps::find("material-gallery")?;
        let packages = [
            (apps::find("showcase")?, Vec::new(), None),
            (gallery, Vec::new(), None),
            (
                gallery,
                vec!["devtools".to_string()],
                Some(ws.target.join("web/devtools/material-gallery")),
            ),
        ];
        for (app, features, out) in packages {
            let options = BuildOptions {
                app,
                profile: Profile::Release,
                features,
                out: out.unwrap_or_else(|| ws.web_dir(Profile::Release, app)),
                compress: false,
            };
            web_build::build(ws, &options)?.print();
        }
        cargo(
            ws,
            &[
                "build",
                "-p",
                "surface-material-gallery",
                "--features",
                "devtools",
                "--bins",
                "--examples",
            ],
        )?;
        cargo(ws, &["build", "-p", "jet-bridge", "--bin", "jet-bridge"])?;
    }
    let mut command = vec!["run", "-q", "-p", "surface-e2e", "--"];
    command.extend(args.iter().map(String::as_str));
    cargo(ws, &command)
}

fn cargo(ws: &Workspace, args: &[&str]) -> Result<()> {
    println!("+ cargo {}", args.join(" "));
    let status = Command::new(ws.cargo())
        .args(args)
        .current_dir(&ws.root)
        .status()?;
    if !status.success() {
        bail!("cargo {} failed with {status}", args.join(" "));
    }
    Ok(())
}
