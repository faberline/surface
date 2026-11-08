//! `surface-web.json`: what a web build produced and how.
//!
//! Static hosts and `cargo xtask web serve` read it. Every file in the
//! output directory except the manifest itself and the compressed
//! sidecars is listed with its sha256 and size.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const FILE_NAME: &str = "surface-web.json";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub schema_version: u32,
    pub app: String,
    pub package: String,
    /// `release` or `dev`.
    pub profile: String,
    pub cargo_profile: String,
    pub rustc_target: String,
    pub features: Vec<String>,
    pub tools: Tools,
    /// HTML shell, relative to the output directory.
    pub entry: String,
    /// Hashed JS glue module.
    pub js: String,
    /// Hashed WASM binary.
    pub wasm: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Tools {
    pub rustc: String,
    pub wasm_bindgen: String,
    /// `None` for dev builds, which skip wasm-opt.
    pub wasm_opt: Option<String>,
    pub wasm_opt_args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FileEntry {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    /// True when the name carries a content hash, so hosts may cache it
    /// forever.
    pub immutable: bool,
    /// Size of the `.br` sidecar, when written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub br_bytes: Option<u64>,
    /// Size of the `.gz` sidecar, when written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gzip_bytes: Option<u64>,
}

impl Manifest {
    pub fn read(dir: &Path) -> Result<Self> {
        let path = dir.join(FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn write(&self, dir: &Path) -> Result<()> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        std::fs::write(dir.join(FILE_NAME), text).context("writing the build manifest")
    }

    pub fn file(&self, path: &str) -> Option<&FileEntry> {
        self.files.iter().find(|file| file.path == path)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `name.ext` + hash -> `name-<first 16 hex>.ext`.
pub fn hashed_name(stem: &str, extension: &str, sha256: &str) -> String {
    format!("{stem}-{}.{extension}", &sha256[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_and_hashed_names_are_stable() {
        let hash = sha256_hex(b"abc");
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hashed_name("app_bg", "wasm", &hash),
            "app_bg-ba7816bf8f01cfea.wasm"
        );
    }

    #[test]
    fn manifest_round_trips_and_omits_missing_sidecars() {
        let manifest = Manifest {
            schema_version: SCHEMA_VERSION,
            app: "showcase".into(),
            package: "surface-showcase".into(),
            profile: "dev".into(),
            cargo_profile: "dev".into(),
            rustc_target: "wasm32-unknown-unknown".into(),
            features: vec![],
            tools: Tools {
                rustc: "rustc 1.96.1".into(),
                wasm_bindgen: "0.2.129".into(),
                wasm_opt: None,
                wasm_opt_args: vec![],
            },
            entry: "index.html".into(),
            js: "a-0011223344556677.js".into(),
            wasm: "a_bg-0011223344556677.wasm".into(),
            files: vec![FileEntry {
                path: "index.html".into(),
                sha256: sha256_hex(b""),
                bytes: 0,
                immutable: false,
                br_bytes: None,
                gzip_bytes: Some(20),
            }],
        };
        let text = serde_json::to_string(&manifest).unwrap();
        assert!(!text.contains("br_bytes"));
        assert!(text.contains("\"gzip_bytes\":20"));
        let parsed: Manifest = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed, manifest);
        assert!(parsed.file("index.html").is_some());
        assert!(parsed.file("missing").is_none());
    }
}
