#!/usr/bin/env python3
"""Build only a focused Text Field Counter runner in the private E2E checkout."""

from hashlib import sha256
import argparse
import os
from pathlib import Path
import re
from shutil import copy2
import subprocess


batch = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--out", type=Path, default=batch / "focused-bin")
args = parser.parse_args()
out = args.out.resolve()
assert out.is_relative_to(batch) and not out.exists()
source = batch / "source"
module = source / "e2e/src/material.rs"
original = module.read_bytes()
entered = original.decode()
entered, count = re.subn(
    r"pub const STEPS: &\[&str\] = &\[.*?\];",
    'pub const STEPS: &[&str] = &["M337", "M338", "M339", "M340"];',
    entered,
    count=1,
    flags=re.S,
)
assert count == 1
serve = '    let (_server, url) = env.serve("material-gallery")?;'
assert entered.count(serve) == 1
entered = entered.replace(serve, serve + """
    if std::env::var_os("SURFACE_TEXT_FIELD_COUNTER_FOCUSED").is_some() {
        return material_text_field_counter::run(env, chrome, &url).await;
    }""")
target = Path(os.environ["CARGO_TARGET_DIR"])
assert target.is_absolute() and target.parent == Path("/tmp/cargo-target")
out.mkdir()
(out / "scope-selector.rs").write_text(entered)
try:
    module.write_text(entered)
    subprocess.run(["cap", "cargo", "build", "--locked", "-p", "surface-e2e"], cwd=source, check=True)
    runner = out / "surface-e2e"
    copy2(target / "debug/surface-e2e", runner)
    (out / "runner.sha256").write_text(sha256(runner.read_bytes()).hexdigest() + "\n")
finally:
    module.write_bytes(original)
