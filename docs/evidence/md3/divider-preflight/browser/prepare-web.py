"""Seal the stock Divider E2E and prepare a focused runner copy."""

from difflib import unified_diff
from hashlib import sha256
from pathlib import Path
import re
import shutil

root = Path(__file__).resolve().parent
product = root / "source"
stock = root / "web-source"
focused = root / "web-focused"
runner_path = "e2e/src/material.rs"
case_path = "e2e/src/material_divider.rs"

incoming = [line.split("  ", 1)[1] for line in (root / "source.sha256").read_text().splitlines()]
paths = sorted(set(incoming) | {case_path})
assert len(paths) == 561
assert {name for name in paths if not (product / name).exists() or (product / name).read_bytes() != (stock / name).read_bytes()} == {runner_path, case_path}

with (root / "web-source.sha256").open("w") as manifest:
    for name in paths:
        manifest.write(f"{sha256((stock / name).read_bytes()).hexdigest()}  {name}\n")

with (root / "material-divider-e2e.patch").open("w") as patch:
    for name in [runner_path, case_path]:
        previous = (product / name).read_text().splitlines(keepends=True) if name in incoming else []
        current = (stock / name).read_text().splitlines(keepends=True)
        patch.writelines(unified_diff(
            previous, current, fromfile=(f"a/{name}" if name in incoming else "/dev/null"), tofile=f"b/{name}"
        ))

if focused.exists():
    raise RuntimeError("Choose a new focused runner directory")
shutil.copytree(stock, focused)
original = (stock / runner_path).read_text()
common = original[original.index("use anyhow::Result;"):original.index("async fn focus(")]
common = common.replace("use serde_json::{json, Value};", "use serde_json::Value;")
common = common.replace("use crate::check::step;\n", "").replace("use crate::{check, check_eq, Env};", "use crate::Env;")
common = re.sub(r"pub const STEPS: &\[&str\] = &\[.*?\];",
                'pub const STEPS: &[&str] = &["M262", "M263"];', common, count=1, flags=re.S)
header = """//! Focused Divider runner. The stock registration stays in web-source.
#![allow(dead_code)]
#[path = "material_divider.rs"]
mod material_divider;
#[path = "material_time_input.rs"]
mod material_time_input;

"""
end = """pub async fn run(env: &Env, chrome: &Chrome) -> Result<()> {
    let (_server, url) = env.serve("material-gallery")?;
    material_divider::run(env, chrome, &url).await
}
"""
(focused / runner_path).write_text(header + common + end)
