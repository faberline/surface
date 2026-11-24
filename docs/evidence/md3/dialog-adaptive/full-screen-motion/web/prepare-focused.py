#!/usr/bin/env python3
"""Make a scratch-only Material selection runner without changing the stock patch."""
from pathlib import Path
import re
import shutil

root = Path(__file__).resolve().parent
source = root / 'source'
focused = root / 'focused-source'

if not focused.exists():
    shutil.copytree(source, focused)

original = (source / 'e2e/src/material.rs').read_text()
parent = original[original.index('use anyhow::Result;'):original.index('async fn focus(')]
parent = parent.replace('use serde_json::{json, Value};', 'use serde_json::Value;')
parent = parent.replace('use crate::check::step;\n', '').replace('use crate::{check, check_eq, Env};', 'use crate::Env;')
parent = re.sub(r'pub const STEPS: &\[&str\] = &\[.*?\];',
                'pub const STEPS: &[&str] = &["M256", "M257", "M258", "M259", "M260", "M261", "M262", "M263"];',
                parent, count=1, flags=re.S)
header = '''//! Private headless Dialog selection. Stock code in source/ is unchanged.
#![allow(dead_code)]
#[path = "material_dialog.rs"]
mod material_dialog;
#[path = "material_time_input.rs"]
mod material_time_input;

'''
end = '''pub async fn run(env: &Env, chrome: &Chrome) -> Result<()> {
    let (_server, url) = env.serve("material-gallery")?;
    material_dialog::run(env, chrome, &url).await
}
'''
(focused / 'e2e/src/material.rs').write_text(header + parent + end)
for name in ('material_dialog.rs', 'material_dialog_motion.rs', 'material_dialog_transform.rs'):
    shutil.copyfile(source / 'e2e/src' / name, focused / 'e2e/src' / name)
