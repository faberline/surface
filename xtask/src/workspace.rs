//! Workspace paths.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::apps::App;
use crate::web_build::Profile;

pub struct Workspace {
    pub root: PathBuf,
    pub target: PathBuf,
}

impl Workspace {
    pub fn locate() -> Result<Self> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("xtask has no parent directory")?
            .to_path_buf();
        let target = match std::env::var_os("CARGO_TARGET_DIR") {
            Some(dir) => root.join(dir),
            None => root.join("target"),
        };
        Ok(Self { root, target })
    }

    /// Default output directory: `target/web/<profile>/<app>`.
    pub fn web_dir(&self, profile: Profile, app: &App) -> PathBuf {
        self.target.join("web").join(profile.name()).join(app.name)
    }

    pub fn cargo(&self) -> String {
        std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
    }
}
