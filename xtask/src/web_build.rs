//! `cargo xtask web build`: cargo, wasm-bindgen, wasm-opt, hashed files,
//! the HTML shell, compressed sidecars and the manifest.
//!
//! Every step writes into a staging directory next to the output. The
//! output is replaced only after all steps succeed, so a failed build
//! leaves the previous build in place.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::apps::App;
use crate::manifest::{self, FileEntry, Manifest, Tools};
use crate::tools;
use crate::workspace::Workspace;

pub const RUSTC_TARGET: &str = "wasm32-unknown-unknown";

/// Placeholder in `web/index.html` for the hashed JS module URL.
pub const JS_PLACEHOLDER: &str = "%SURFACE_JS%";

/// Size-tuned wasm-opt passes. The feature list matches what the
/// wasm32-unknown-unknown target enables by default in current Rust, so
/// wasm-opt validates the input and does not lower it.
pub const WASM_OPT_ARGS: &[&str] = &[
    "-Oz",
    "--enable-bulk-memory",
    "--enable-mutable-globals",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "--enable-reference-types",
    "--enable-multivalue",
    "--strip-debug",
    "--strip-producers",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// `wasm-release` cargo profile plus wasm-opt.
    Release,
    /// `dev` cargo profile, no wasm-opt.
    Dev,
}

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Profile::Release => "release",
            Profile::Dev => "dev",
        }
    }

    pub fn cargo_profile(self) -> &'static str {
        match self {
            Profile::Release => "wasm-release",
            Profile::Dev => "dev",
        }
    }

    /// Directory under `target/<triple>/` that cargo writes for the profile.
    pub fn cargo_dir(self) -> &'static str {
        match self {
            Profile::Release => "wasm-release",
            Profile::Dev => "debug",
        }
    }
}

pub struct BuildOptions {
    pub app: &'static App,
    pub profile: Profile,
    pub features: Vec<String>,
    pub out: PathBuf,
    pub compress: bool,
}

pub struct Report {
    pub out: PathBuf,
    pub manifest: Manifest,
}

impl Report {
    pub fn print(&self) {
        println!(
            "built {} ({}) -> {}",
            self.manifest.app,
            self.manifest.profile,
            self.out.display()
        );
        for path in [&self.manifest.wasm, &self.manifest.js, &self.manifest.entry] {
            if let Some(file) = self.manifest.file(path) {
                println!("  {}", size_line(file));
            }
        }
    }
}

fn size_line(file: &FileEntry) -> String {
    let mut line = format!("{:<44} {:>12}", file.path, bytes(file.bytes));
    if let Some(br) = file.br_bytes {
        line.push_str(&format!("  br {:>10}", bytes(br)));
    }
    if let Some(gzip) = file.gzip_bytes {
        line.push_str(&format!("  gzip {:>10}", bytes(gzip)));
    }
    line
}

fn bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.2} MiB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KiB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

pub fn build(ws: &Workspace, options: &BuildOptions) -> Result<Report> {
    let app = options.app;
    let template_path = ws.root.join(app.dir).join("web/index.html");
    let template = std::fs::read_to_string(&template_path)
        .with_context(|| format!("reading {}", template_path.display()))?;
    // Fail on a bad template before the long cargo build.
    check_template(&template).with_context(|| template_path.display().to_string())?;

    tools::check_rust_target(RUSTC_TARGET)?;
    let wasm_bindgen = tools::check_wasm_bindgen(&ws.root.join("Cargo.lock"))?;
    let wasm_opt = match options.profile {
        Profile::Release => Some(tools::wasm_opt_version()?),
        Profile::Dev => None,
    };
    let rustc = tools::rustc_version()?;

    cargo_build(ws, options)?;
    let cargo_wasm = ws
        .target
        .join(RUSTC_TARGET)
        .join(options.profile.cargo_dir())
        .join(format!("{}.wasm", app.lib_stem()));

    let staging = sibling(&options.out, "staging")?;
    remove_dir_if_present(&staging)?;
    std::fs::create_dir_all(&staging).with_context(|| format!("creating {}", staging.display()))?;
    let result = (|| {
        let manifest = assemble(
            options,
            &staging,
            &cargo_wasm,
            &template,
            Tools {
                rustc,
                wasm_bindgen,
                wasm_opt_args: match wasm_opt {
                    Some(_) => WASM_OPT_ARGS.iter().map(|arg| arg.to_string()).collect(),
                    None => Vec::new(),
                },
                wasm_opt,
            },
        )?;
        replace_dir(&staging, &options.out)?;
        Ok(manifest)
    })();
    match result {
        Ok(manifest) => Ok(Report {
            out: options.out.clone(),
            manifest,
        }),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging);
            Err(error)
        }
    }
}

fn cargo_build(ws: &Workspace, options: &BuildOptions) -> Result<()> {
    let mut command = Command::new(ws.cargo());
    command
        .current_dir(&ws.root)
        .args(["build", "--locked", "--target", RUSTC_TARGET, "--lib"])
        .args(["--package", options.app.package])
        .args(["--profile", options.profile.cargo_profile()]);
    if !options.features.is_empty() {
        command.args(["--features", &options.features.join(",")]);
    }
    run(&mut command, "cargo build")
}

fn run(command: &mut Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("starting {what}: {command:?}"))?;
    if !status.success() {
        bail!("{what} failed with {status}");
    }
    Ok(())
}

/// Runs wasm-bindgen and wasm-opt into `staging`, hashes, writes the shell,
/// sidecars and manifest.
fn assemble(
    options: &BuildOptions,
    staging: &Path,
    cargo_wasm: &Path,
    template: &str,
    tools: Tools,
) -> Result<Manifest> {
    let app = options.app;
    let stem = app.lib_stem();
    let bindgen_dir = staging.join(".bindgen");
    run(
        Command::new(tools::wasm_bindgen_bin())
            .args(["--target", "web", "--no-typescript", "--out-name", &stem])
            .arg("--out-dir")
            .arg(&bindgen_dir)
            .arg(cargo_wasm),
        "wasm-bindgen",
    )?;

    let bindgen_wasm = bindgen_dir.join(format!("{stem}_bg.wasm"));
    if options.profile == Profile::Release {
        let optimized = bindgen_dir.join(format!("{stem}_opt.wasm"));
        run(
            Command::new(tools::wasm_opt_bin())
                .args(WASM_OPT_ARGS)
                .arg(&bindgen_wasm)
                .arg("-o")
                .arg(&optimized),
            "wasm-opt",
        )?;
        std::fs::rename(&optimized, &bindgen_wasm)?;
    }

    let wasm_bytes = std::fs::read(&bindgen_wasm)?;
    let wasm_name = manifest::hashed_name(
        &format!("{stem}_bg"),
        "wasm",
        &manifest::sha256_hex(&wasm_bytes),
    );
    let js = std::fs::read_to_string(bindgen_dir.join(format!("{stem}.js")))?;
    let js = point_js_at_wasm(&js, &format!("{stem}_bg.wasm"), &wasm_name)?;
    let js_name = manifest::hashed_name(&stem, "js", &manifest::sha256_hex(js.as_bytes()));
    std::fs::write(staging.join(&wasm_name), &wasm_bytes)?;
    std::fs::write(staging.join(&js_name), &js)?;

    // Inline JS snippets keep wasm-bindgen's relative paths. Their names
    // are not content hashes, so they are not marked immutable.
    let snippets = bindgen_dir.join("snippets");
    if snippets.is_dir() {
        copy_dir(&snippets, &staging.join("snippets"))?;
    }
    std::fs::remove_dir_all(&bindgen_dir)?;

    std::fs::write(
        staging.join("index.html"),
        render_shell(template, &js_name, &wasm_name)?,
    )?;

    let mut files = Vec::new();
    for path in list_files(staging)? {
        let bytes = std::fs::read(staging.join(&path))?;
        let immutable = path == js_name || path == wasm_name;
        let (br_bytes, gzip_bytes) = if options.compress {
            let (br, gzip) = write_sidecars(&staging.join(&path), &bytes)?;
            (Some(br), Some(gzip))
        } else {
            (None, None)
        };
        files.push(FileEntry {
            sha256: manifest::sha256_hex(&bytes),
            bytes: bytes.len() as u64,
            path,
            immutable,
            br_bytes,
            gzip_bytes,
        });
    }

    let manifest = Manifest {
        schema_version: manifest::SCHEMA_VERSION,
        app: app.name.into(),
        package: app.package.into(),
        profile: options.profile.name().into(),
        cargo_profile: options.profile.cargo_profile().into(),
        rustc_target: RUSTC_TARGET.into(),
        features: options.features.clone(),
        tools,
        entry: "index.html".into(),
        js: js_name,
        wasm: wasm_name,
        files,
    };
    manifest.write(staging)?;
    Ok(manifest)
}

fn check_template(template: &str) -> Result<()> {
    match template.matches(JS_PLACEHOLDER).count() {
        1 => {}
        n => bail!("the HTML template must contain {JS_PLACEHOLDER} exactly once; found {n}"),
    }
    if !template.contains("<script") {
        bail!("the HTML template has no <script> element");
    }
    Ok(())
}

/// Fills the JS placeholder and preloads the module and the WASM binary
/// before the first script, so the download does not wait for the module.
pub fn render_shell(template: &str, js: &str, wasm: &str) -> Result<String> {
    check_template(template)?;
    let preload = format!(
        "<link rel=\"modulepreload\" href=\"./{js}\">\n\
         <link rel=\"preload\" href=\"./{wasm}\" as=\"fetch\" type=\"application/wasm\" crossorigin>\n"
    );
    let at = template.find("<script").expect("checked above");
    let mut html = String::with_capacity(template.len() + preload.len());
    html.push_str(&template[..at]);
    html.push_str(&preload);
    html.push_str(&template[at..]);
    Ok(html.replace(JS_PLACEHOLDER, &format!("./{js}")))
}

/// wasm-bindgen's `--target web` glue loads `'<stem>_bg.wasm'` relative to
/// the module. Point it at the hashed name. Anything other than exactly one
/// reference means the glue format changed, so fail.
pub fn point_js_at_wasm(js: &str, original: &str, hashed: &str) -> Result<String> {
    let quoted = format!("'{original}'");
    match js.matches(&quoted).count() {
        1 => Ok(js.replace(&quoted, &format!("'{hashed}'"))),
        n => bail!(
            "expected one reference to {quoted} in the wasm-bindgen glue, found {n}; \
             the glue format changed"
        ),
    }
}

fn write_sidecars(path: &Path, bytes: &[u8]) -> Result<(u64, u64)> {
    let mut br = Vec::new();
    {
        let params = brotli::enc::BrotliEncoderParams {
            quality: 11,
            lgwin: 24,
            ..Default::default()
        };
        brotli::BrotliCompress(&mut &bytes[..], &mut br, &params)?;
    }
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gzip.write_all(bytes)?;
    let gzip = gzip.finish()?;
    std::fs::write(with_suffix(path, ".br"), &br)?;
    std::fs::write(with_suffix(path, ".gz"), &gzip)?;
    Ok((br.len() as u64, gzip.len() as u64))
}

pub fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Relative `/`-separated paths of all files under `dir`, sorted.
fn list_files(dir: &Path) -> Result<Vec<String>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                let relative = path.strip_prefix(root)?;
                let parts: Vec<_> = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let target = to.join(path.file_name().context("unnamed entry")?);
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// `target/web/dev/showcase` -> `target/web/dev/.showcase.<label>`.
fn sibling(out: &Path, label: &str) -> Result<PathBuf> {
    let name = out
        .file_name()
        .with_context(|| format!("{} has no directory name", out.display()))?
        .to_string_lossy();
    let parent = out.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    Ok(parent.join(format!(".{name}.{label}")))
}

fn remove_dir_if_present(dir: &Path) -> Result<()> {
    match std::fs::remove_dir_all(dir) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(error).with_context(|| format!("removing {}", dir.display()))
        }
        _ => Ok(()),
    }
}

/// Moves `staging` to `out`. The old output is renamed aside first, so a
/// server reading `out` sees either the old or the new build.
fn replace_dir(staging: &Path, out: &Path) -> Result<()> {
    if !staging.is_dir() {
        bail!(
            "{} is missing; keeping the previous build",
            staging.display()
        );
    }
    let old = sibling(out, "old")?;
    remove_dir_if_present(&old)?;
    let had_old = out.exists();
    if had_old {
        std::fs::rename(out, &old).with_context(|| format!("moving {} aside", out.display()))?;
    }
    if let Err(error) = std::fs::rename(staging, out) {
        if had_old {
            let _ = std::fs::rename(&old, out);
        }
        return Err(error).with_context(|| format!("writing {}", out.display()));
    }
    remove_dir_if_present(&old)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "<!doctype html>\n<title>t</title>\n\
        <script type=\"module\">import init from '%SURFACE_JS%'; await init();</script>\n";

    #[test]
    fn shell_preloads_and_imports_the_hashed_module() {
        let html = render_shell(TEMPLATE, "app-0123.js", "app_bg-4567.wasm").unwrap();
        assert!(html.contains("import init from './app-0123.js'"));
        assert!(html.contains("<link rel=\"modulepreload\" href=\"./app-0123.js\">"));
        assert!(html.contains("href=\"./app_bg-4567.wasm\" as=\"fetch\""));
        assert!(html.find("modulepreload").unwrap() < html.find("<script").unwrap());
        assert!(!html.contains(JS_PLACEHOLDER));
    }

    #[test]
    fn shell_rejects_missing_or_repeated_placeholder() {
        assert!(render_shell("<script></script>", "a.js", "a.wasm").is_err());
        let twice = format!("<script>'{JS_PLACEHOLDER}' '{JS_PLACEHOLDER}'</script>");
        assert!(render_shell(&twice, "a.js", "a.wasm").is_err());
        assert!(render_shell(JS_PLACEHOLDER, "a.js", "a.wasm").is_err());
    }

    #[test]
    fn glue_is_pointed_at_the_hashed_wasm_exactly_once() {
        let js = "module_or_path = new URL('app_bg.wasm', import.meta.url);";
        assert_eq!(
            point_js_at_wasm(js, "app_bg.wasm", "app_bg-89ab.wasm").unwrap(),
            "module_or_path = new URL('app_bg-89ab.wasm', import.meta.url);"
        );
        assert!(point_js_at_wasm("no url", "app_bg.wasm", "x").is_err());
        assert!(point_js_at_wasm("'app_bg.wasm' 'app_bg.wasm'", "app_bg.wasm", "x").is_err());
    }

    #[test]
    fn templates_in_the_repository_render() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for app in crate::apps::ALL {
            let template =
                std::fs::read_to_string(root.join(app.dir).join("web/index.html")).unwrap();
            render_shell(&template, "a.js", "a.wasm")
                .unwrap_or_else(|error| panic!("{}: {error:#}", app.name));
        }
    }

    #[test]
    fn replace_dir_swaps_and_keeps_old_build_on_missing_staging() {
        let base = std::env::temp_dir().join(format!("xtask-replace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let out = base.join("app");
        let staging = sibling(&out, "staging").unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("index.html"), "old").unwrap();

        // No staging directory: the swap fails and the old build stays.
        assert!(replace_dir(&staging, &out).is_err());
        assert_eq!(
            std::fs::read_to_string(out.join("index.html")).unwrap(),
            "old"
        );

        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("index.html"), "new").unwrap();
        replace_dir(&staging, &out).unwrap();
        assert_eq!(
            std::fs::read_to_string(out.join("index.html")).unwrap(),
            "new"
        );
        assert!(!staging.exists());
        assert!(!sibling(&out, "old").unwrap().exists());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn profiles_map_to_cargo_profiles_and_directories() {
        assert_eq!(Profile::Release.cargo_profile(), "wasm-release");
        assert_eq!(Profile::Release.cargo_dir(), "wasm-release");
        assert_eq!(Profile::Dev.cargo_dir(), "debug");
    }
}
