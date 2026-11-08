//! The browser apps this repository builds.

use anyhow::{bail, Result};

/// One browser app: a workspace `cdylib` package and its HTML template.
#[derive(Debug, PartialEq, Eq)]
pub struct App {
    /// Short name used on the command line and in output paths.
    pub name: &'static str,
    /// Cargo package name.
    pub package: &'static str,
    /// Package directory, relative to the workspace root. The HTML
    /// template is `<dir>/web/index.html`.
    pub dir: &'static str,
}

impl App {
    /// File stem of the `.wasm` that cargo writes and of wasm-bindgen output.
    pub fn lib_stem(&self) -> String {
        self.package.replace('-', "_")
    }
}

pub const ALL: &[App] = &[
    App {
        name: "showcase",
        package: "surface-showcase",
        dir: "examples/showcase",
    },
    App {
        name: "material-gallery",
        package: "surface-material-gallery",
        dir: "examples/material-gallery",
    },
];

pub fn find(name: &str) -> Result<&'static App> {
    match ALL
        .iter()
        .find(|app| app.name == name || app.package == name)
    {
        Some(app) => Ok(app),
        None => {
            let known: Vec<_> = ALL.iter().map(|app| app.name).collect();
            bail!("unknown app `{name}`; known apps: {}", known.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apps_resolve_by_name_or_package() {
        assert_eq!(find("showcase").unwrap().lib_stem(), "surface_showcase");
        assert_eq!(
            find("surface-material-gallery").unwrap().name,
            "material-gallery"
        );
        assert!(find("gallery").is_err());
    }

    #[test]
    fn every_app_has_a_template_and_package() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for app in ALL {
            let dir = root.join(app.dir);
            assert!(dir.join("web/index.html").is_file(), "{}", app.name);
            let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
            assert!(
                manifest.contains(&format!("name = \"{}\"", app.package)),
                "{}",
                app.name
            );
        }
    }
}
