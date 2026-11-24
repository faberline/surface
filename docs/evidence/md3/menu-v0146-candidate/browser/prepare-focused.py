"""Create a private selector for the exact three stock command menu steps."""

from difflib import unified_diff
from hashlib import sha256
from pathlib import Path
import re
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parent
source, manifest = map(Path, sys.argv[1:3])
expected = sys.argv[3]
source = source.resolve()
manifest = manifest.resolve()
stock = root / "runner-stock"
focused = root / "focused-source"
assert sha256(manifest.read_bytes()).hexdigest() == expected
entries = manifest.read_text().splitlines()
assert len(entries) == 590
assert not stock.exists() and not focused.exists()
(root / "runner-stock.sha256").write_bytes(manifest.read_bytes())
for suffix, directory in [("incoming", source), ("copy", stock)]:
    if suffix == "copy":
        shutil.copytree(source, stock)
    with (root / f"runner-stock-{suffix}.log").open("wb") as log:
        subprocess.run(["shasum", "-a", "256", "-c", str(manifest)], cwd=directory,
                       stdout=log, check=True)

build_manifest = (root / "source.sha256").read_text().splitlines()
expected_previous = "e2e/src/material_command_menu.rs"
known_diff = [new.split("  ", 1)[1] for previous, new in zip(build_manifest, entries) if previous != new]
assert known_diff in ([], [expected_previous]), known_diff

shutil.copytree(stock, focused)
runner = "e2e/src/material.rs"
original = (stock / runner).read_text()
common = original[original.index("use anyhow::Result;"):original.index("async fn focus(")]
common = common.replace("use serde_json::{json, Value};", "use serde_json::Value;")
common = common.replace("use crate::check::step;\n", "").replace("use crate::{check, check_eq, Env};", "use crate::Env;")
common = re.sub(r"pub const STEPS: &\[&str\] = &\[.*?\];",
                'pub const STEPS: &[&str] = &["M267", "M268", "M269"];', common, count=1, flags=re.S)
header = """//! Private headless command menu selection. Stock tests remain unchanged.
#![allow(dead_code)]
#[path = "material_command_menu.rs"]
mod material_command_menu;
#[path = "material_time_input.rs"]
mod material_time_input;

"""
end = """pub async fn run(env: &Env, chrome: &Chrome) -> Result<()> {
    let (_server, url) = env.serve("material-gallery")?;
    material_command_menu::run(env, chrome, &url).await
}
"""
focused_text = header + common + end
(focused / runner).write_text(focused_text)
(root / "focused-selector.patch").write_text("".join(unified_diff(
    original.splitlines(True), focused_text.splitlines(True), f"a/{runner}", f"b/{runner}"
)))
with (root / "focused-source.sha256").open("w") as out:
    for entry in entries:
        checksum, name = entry.split("  ", 1)
        observed = sha256((focused / name).read_bytes()).hexdigest()
        assert name == runner or checksum == observed, name
        out.write(f"{observed}  {name}\n")
