//! External tools: the wasm-bindgen CLI, wasm-opt and rustc.
//!
//! `SURFACE_WASM_BINDGEN` and `SURFACE_WASM_OPT` override the binary paths.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn wasm_bindgen_bin() -> String {
    std::env::var("SURFACE_WASM_BINDGEN").unwrap_or_else(|_| "wasm-bindgen".into())
}

pub fn wasm_opt_bin() -> String {
    std::env::var("SURFACE_WASM_OPT").unwrap_or_else(|_| "wasm-opt".into())
}

/// Checks that the wasm-bindgen CLI matches the `wasm-bindgen` crate in
/// `Cargo.lock`. The JS glue and the WASM schema only match when both
/// versions are equal, so a mismatch is an error, not a warning.
pub fn check_wasm_bindgen(lockfile: &Path) -> Result<String> {
    let lock = std::fs::read_to_string(lockfile)
        .with_context(|| format!("reading {}", lockfile.display()))?;
    let locked = locked_version(&lock, "wasm-bindgen")?;
    let install = format!("cargo install wasm-bindgen-cli --version {locked} --locked --force");
    let bin = wasm_bindgen_bin();
    let output = match Command::new(&bin).arg("--version").output() {
        Ok(output) => output,
        Err(error) => bail!("cannot run `{bin}` ({error}). Install it with: {install}"),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let found = match parse_wasm_bindgen_version(&text) {
        Some(found) if output.status.success() => found,
        _ => bail!("cannot read the version from `{bin} --version`: {text:?}"),
    };
    if found != locked {
        bail!(
            "wasm-bindgen CLI is {found} but Cargo.lock pins wasm-bindgen {locked}. \
             They must match. Install the locked CLI with: {install}"
        );
    }
    Ok(found)
}

/// Returns the wasm-opt version, or an error with an install hint.
pub fn wasm_opt_version() -> Result<String> {
    let bin = wasm_opt_bin();
    let hint = "Install binaryen (`brew install binaryen`, or a release from \
                https://github.com/WebAssembly/binaryen/releases) or set SURFACE_WASM_OPT. \
                Use --dev for a build without wasm-opt.";
    let output = match Command::new(&bin).arg("--version").output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => bail!("`{bin} --version` failed with {}. {hint}", output.status),
        Err(error) => {
            bail!("release builds need wasm-opt, but `{bin}` cannot run ({error}). {hint}")
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    parse_wasm_opt_version(&text)
        .with_context(|| format!("cannot read the version from `{bin} --version`: {text:?}"))
}

pub fn rustc_version() -> Result<String> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let output = Command::new(&rustc)
        .arg("--version")
        .output()
        .with_context(|| format!("running `{rustc} --version`"))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Checks that the `rustc` on this PATH has the standard library for
/// `target`. A PATH that finds a non-rustup Rust first (for example
/// Homebrew) often lacks it, and cargo's own error is easy to misread.
pub fn check_rust_target(target: &str) -> Result<()> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let output = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .with_context(|| format!("running `{rustc} --print sysroot`"))?;
    let sysroot = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !Path::new(&sysroot)
        .join("lib/rustlib")
        .join(target)
        .is_dir()
    {
        bail!(
            "`{rustc}` (sysroot {sysroot}) has no {target} standard library. \
             Run `rustup target add {target}`, and make sure rustup's \
             ~/.cargo/bin comes before other Rust installs in PATH."
        );
    }
    Ok(())
}

/// Finds the single version of `name` in a `Cargo.lock`.
pub fn locked_version(lock: &str, name: &str) -> Result<String> {
    let mut versions = Vec::new();
    for package in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            package.lines().find_map(|line| {
                let value = line.trim().strip_prefix(key)?.trim_start();
                let value = value.strip_prefix('=')?.trim();
                Some(value.trim_matches('"').to_string())
            })
        };
        if field("name").as_deref() == Some(name) {
            versions.extend(field("version"));
        }
    }
    match versions.as_slice() {
        [version] => Ok(version.clone()),
        [] => bail!("Cargo.lock has no `{name}` package"),
        _ => bail!(
            "Cargo.lock has several `{name}` versions ({}); a single CLI cannot match them",
            versions.join(", ")
        ),
    }
}

/// `wasm-bindgen 0.2.129` -> `0.2.129`.
fn parse_wasm_bindgen_version(text: &str) -> Option<String> {
    let version = text.trim().strip_prefix("wasm-bindgen ")?;
    let version = version.split_whitespace().next()?;
    version
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.')
        .then(|| version.to_string())
}

/// `wasm-opt version 133 (version_133)` -> `133`.
fn parse_wasm_opt_version(text: &str) -> Option<String> {
    let version = text.trim().strip_prefix("wasm-opt version ")?;
    let version = version.split_whitespace().next()?;
    version
        .chars()
        .next()?
        .is_ascii_digit()
        .then(|| version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"
version = 4

[[package]]
name = "wasm-bindgen"
version = "0.2.129"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "wasm-bindgen-shared"
version = "0.2.130"

[[package]]
name = "dup"
version = "1.0.0"

[[package]]
name = "dup"
version = "2.0.0"
"#;

    #[test]
    fn lockfile_version_matches_exact_package_name_only() {
        assert_eq!(locked_version(LOCK, "wasm-bindgen").unwrap(), "0.2.129");
        assert_eq!(
            locked_version(LOCK, "wasm-bindgen-shared").unwrap(),
            "0.2.130"
        );
        assert!(locked_version(LOCK, "missing").is_err());
        assert!(locked_version(LOCK, "dup").is_err());
    }

    #[test]
    fn workspace_lockfile_pins_one_wasm_bindgen() {
        let lock = include_str!("../../Cargo.lock");
        let version = locked_version(lock, "wasm-bindgen").unwrap();
        assert!(version.starts_with("0.2."), "{version}");
    }

    #[test]
    fn tool_version_output_parses() {
        assert_eq!(
            parse_wasm_bindgen_version("wasm-bindgen 0.2.129\n").as_deref(),
            Some("0.2.129")
        );
        assert_eq!(
            parse_wasm_bindgen_version("wasm-bindgen 0.2.129 (abc123)").as_deref(),
            Some("0.2.129")
        );
        assert_eq!(parse_wasm_bindgen_version("wasm-pack 0.13.1"), None);
        assert_eq!(
            parse_wasm_opt_version("wasm-opt version 133 (version_133)\n").as_deref(),
            Some("133")
        );
        assert_eq!(
            parse_wasm_opt_version("wasm-opt version 116").as_deref(),
            Some("116")
        );
        assert_eq!(parse_wasm_opt_version("wasm-opt"), None);
    }
}
