//! `cargo xtask provenance`: check the hashes in docs/provenance.json
//! against the files it pins. Port of the old `scripts/check-provenance.py`.
//!
//! Only two sets of files are hashed: the source files imported from Core,
//! listed in "original_source_sha256", and the pinned Noto CJK files. Other
//! files are not tracked here. Each imported source file must still hash to
//! its Core value, unless "modified_since_import" lists it with a reason. A
//! listed file must differ from Core, so the list cannot go stale. The
//! pinned fonts must match their hash and size. The checker tests itself on
//! a temporary tree first.

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

use crate::workspace::Workspace;

const MANIFEST: &str = "docs/provenance.json";
const FONT_DIR: &str = "crates/surface-wgpu/assets";

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sha256(path: &Path) -> Result<String> {
    Ok(sha256_bytes(&std::fs::read(path)?))
}

/// The non-empty map at `keys`, or None when it is missing or empty.
fn required_map<'a>(manifest: &'a Value, keys: &[&str]) -> Option<&'a Map<String, Value>> {
    let mut value = manifest;
    for key in keys {
        value = value.as_object()?.get(*key)?;
    }
    value.as_object().filter(|map| !map.is_empty())
}

pub fn problems(root: &Path, manifest: &Value) -> Result<Vec<String>> {
    let mut found = Vec::new();
    let empty = Map::new();
    let core = required_map(manifest, &["original_source_sha256"]).unwrap_or_else(|| {
        found.push("original_source_sha256: missing or empty".into());
        &empty
    });
    let fonts = required_map(manifest, &["noto_cjk", "files"]).unwrap_or_else(|| {
        found.push("noto_cjk.files: missing or empty".into());
        &empty
    });
    let modified = manifest["modified_since_import"]
        .as_object()
        .unwrap_or(&empty);
    let unknown: BTreeSet<_> = modified.keys().filter(|k| !core.contains_key(*k)).collect();
    for path in unknown {
        found.push(format!(
            "{path}: listed as modified but not imported from Core"
        ));
    }
    for (path, core_hash) in core {
        let file = root.join(path);
        if !file.is_file() {
            found.push(format!("{path}: imported file is missing"));
            continue;
        }
        let same = Some(sha256(&file)?.as_str()) == core_hash.as_str();
        if let Some(reason) = modified.get(path) {
            if reason.as_str().is_none_or(|r| r.trim().is_empty()) {
                found.push(format!("{path}: modified_since_import needs a reason"));
            }
            if same {
                found.push(format!(
                    "{path}: matches Core again; remove it from modified_since_import"
                ));
            }
        } else if !same {
            found.push(format!(
                "{path}: differs from Core; restore it or list it in \
                 modified_since_import with a reason"
            ));
        }
    }
    for (name, font) in fonts {
        let shown = format!("{FONT_DIR}/{name}");
        let file = root.join(FONT_DIR).join(name);
        if !file.is_file() {
            found.push(format!("{shown}: pinned font file is missing"));
        } else if Some(std::fs::metadata(&file)?.len()) != font["bytes"].as_u64()
            || Some(sha256(&file)?.as_str()) != font["sha256"].as_str()
        {
            found.push(format!(
                "{shown}: size or hash differs from provenance.json"
            ));
        }
    }
    Ok(found)
}

/// Each case: name, manifest, expected number of problems.
fn self_test_cases() -> Vec<(&'static str, Value, usize)> {
    let absent = "0".repeat(64);
    let core = json!({
        "kept.rs": sha256_bytes(b"kept\n"),
        "changed.rs": sha256_bytes(b"original\n"),
    });
    let font_hash = sha256_bytes(b"font");
    let font = json!({"font.otf": {"sha256": font_hash, "bytes": 4}});
    let listed = json!({"changed.rs": "local edit"});
    let manifest = |modified: Value, fonts: Option<Value>, sources: Value| {
        let mut case =
            json!({"original_source_sha256": sources, "modified_since_import": modified});
        if let Some(fonts) = fonts {
            case["noto_cjk"] = json!({ "files": fonts });
        }
        case
    };
    let with = |base: &Value, key: &str, value: Value| {
        let mut out = base.clone();
        out[key] = value;
        out
    };
    let clean = || manifest(listed.clone(), Some(font.clone()), core.clone());
    vec![
        ("clean tree passes", clean(), 0),
        (
            "unlisted change fails",
            manifest(json!({}), Some(font.clone()), core.clone()),
            1,
        ),
        (
            "stale listing fails",
            manifest(
                with(&listed, "kept.rs", json!("none")),
                Some(font.clone()),
                core.clone(),
            ),
            1,
        ),
        (
            "blank reason fails",
            manifest(json!({"changed.rs": " "}), Some(font.clone()), core.clone()),
            1,
        ),
        (
            "null reason fails",
            manifest(
                json!({"changed.rs": null}),
                Some(font.clone()),
                core.clone(),
            ),
            1,
        ),
        (
            "false reason fails",
            manifest(
                json!({"changed.rs": false}),
                Some(font.clone()),
                core.clone(),
            ),
            1,
        ),
        (
            "unknown listing fails",
            manifest(
                with(&listed, "other.rs", json!("typo")),
                Some(font.clone()),
                core.clone(),
            ),
            1,
        ),
        (
            "missing Core file fails",
            manifest(
                listed.clone(),
                Some(font.clone()),
                with(&core, "absent.rs", json!(absent)),
            ),
            1,
        ),
        (
            "empty Core map fails",
            manifest(json!({}), Some(font.clone()), json!({})),
            1,
        ),
        (
            "font size drift fails",
            manifest(
                listed.clone(),
                Some(json!({"font.otf": {"sha256": font_hash, "bytes": 5}})),
                core.clone(),
            ),
            1,
        ),
        (
            "font hash drift fails",
            manifest(
                listed.clone(),
                Some(json!({"font.otf": {"sha256": absent, "bytes": 4}})),
                core.clone(),
            ),
            1,
        ),
        (
            "missing font fails",
            manifest(
                listed.clone(),
                Some(with(
                    &font,
                    "absent.otf",
                    json!({"sha256": absent, "bytes": 4}),
                )),
                core.clone(),
            ),
            1,
        ),
        (
            "empty font map fails",
            manifest(listed.clone(), Some(json!({})), core.clone()),
            1,
        ),
        (
            "missing noto_cjk fails",
            manifest(listed.clone(), None, core.clone()),
            1,
        ),
    ]
}

/// Run every self-test case on a temporary tree; returns the case count.
pub fn self_test() -> Result<usize> {
    let root = std::env::temp_dir().join(format!(
        "surface-provenance-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let result = (|| {
        std::fs::create_dir_all(root.join(FONT_DIR))?;
        std::fs::write(root.join("kept.rs"), "kept\n")?;
        std::fs::write(root.join("changed.rs"), "changed\n")?;
        std::fs::write(root.join(FONT_DIR).join("font.otf"), b"font")?;
        let cases = self_test_cases();
        let mut failures = Vec::new();
        for (name, case, expected) in &cases {
            let got = problems(&root, case)?.len();
            if got != *expected {
                failures.push(format!(
                    "self-test {name}: {got} problems, expected {expected}"
                ));
            }
        }
        if !failures.is_empty() {
            bail!("{}", failures.join("\n"));
        }
        Ok(cases.len())
    })();
    let _ = std::fs::remove_dir_all(&root);
    result
}

pub fn run(ws: &Workspace) -> Result<()> {
    let cases = self_test()?;
    let path = ws.root.join(MANIFEST);
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| format!("reading {MANIFEST}"))?,
    )?;
    let found = problems(&ws.root, &manifest)?;
    if !found.is_empty() {
        let mut message = format!("{MANIFEST} does not match the imported files:");
        for line in found {
            message.push_str("\n  ");
            message.push_str(&line);
        }
        bail!("{message}");
    }
    // problems() has already rejected a missing or empty map.
    let core = manifest["original_source_sha256"]
        .as_object()
        .map_or(0, Map::len);
    let modified = manifest["modified_since_import"]
        .as_object()
        .map_or(0, Map::len);
    let fonts = manifest["noto_cjk"]["files"]
        .as_object()
        .map_or(0, Map::len);
    println!(
        "provenance: {} of {core} Core files unchanged, {modified} listed as modified, \
         {fonts} font files match ({cases} self-test cases passed)",
        core - modified
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_test_runs_all_fourteen_cases() {
        assert_eq!(self_test().unwrap(), 14);
    }

    #[test]
    fn repository_manifest_matches() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let manifest: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(MANIFEST)).unwrap()).unwrap();
        assert_eq!(problems(root, &manifest).unwrap(), Vec::<String>::new());
    }
}
