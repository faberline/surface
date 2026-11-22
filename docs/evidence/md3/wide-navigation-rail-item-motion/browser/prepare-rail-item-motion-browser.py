#!/usr/bin/env python3
"""Replay the eight wide rail Chromium cases with the canonical case functions."""

import argparse
from pathlib import Path
import re
import subprocess


def focus(path, ids, route):
    source = path.read_text()
    steps = ", ".join(f'"{step}"' for step in ids)
    source, step_count = re.subn(
        r"pub const STEPS: &\[&str\] = &\[.*?\];",
        f"pub const STEPS: &[&str] = &[{steps}];",
        source,
        count=1,
        flags=re.S,
    )
    source, route_count = re.subn(
        r"async fn journey\([^\n]+\) -> Result<\(\)> \{",
        lambda previous: route + "\n\n#[allow(dead_code)]\n"
        + previous.group().replace("fn journey", "fn full_journey", 1),
        source,
        count=1,
    )
    if step_count != 1 or route_count != 1:
        raise ValueError(f"Could not route the existing suite in {path}")
    path.write_text(source)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    if args.scratch.exists():
        parser.error("the scratch directory must be absent")
    subprocess.run(["cp", "-cR", args.source, args.scratch], check=True)
    e2e = args.scratch / "e2e/src"
    focus(e2e / "material.rs", ["M179", "M180", "M181", "M182"], """async fn journey(env: &Env, _chrome: &Chrome, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    let before = crate::check::total();
    wide_navigation_rail::journey(env, tab).await?;
    println!("  M179 checks: {}", crate::check::total() - before);
    let before = crate::check::total();
    modal_wide_navigation_rail::journey(env, tab).await?;
    println!("  M180 checks: {}", crate::check::total() - before);
    let before = crate::check::total();
    wide_rail_motion::journey(env, tab).await?;
    println!("  M181 checks: {}", crate::check::total() - before);
    let before = crate::check::total();
    wide_rail_item_motion::journey(env, tab).await?;
    println!("  M182 checks: {}", crate::check::total() - before);
    let errors = tab.errors();
    check!(errors.is_empty(), "page errors: {:?}", errors);
    Ok(())
}""")
    focus(e2e / "web_accessibility.rs", ["A69", "A70", "A71", "A72"], """async fn journey(env: &Env, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    let before = crate::check::total();
    let wide = wide_navigation_rail::accessibility(env, tab).await?;
    println!("  A69 checks: {}", crate::check::total() - before);
    let before = crate::check::total();
    let modal = modal_wide_navigation_rail::accessibility(env, tab).await?;
    println!("  A70 checks: {}", crate::check::total() - before);
    let before = crate::check::total();
    let motion = wide_rail_motion::accessibility(tab).await?;
    println!("  A71 checks: {}", crate::check::total() - before);
    let before = crate::check::total();
    let items = wide_rail_item_motion::accessibility(tab).await?;
    println!("  A72 checks: {}", crate::check::total() - before);
    let errors = tab.errors();
    check!(errors.is_empty(), "page errors: {:?}", errors);
    env.write_json("browser-wide-rail-item-motion-accessibility.json", &json!({"inline":wide,"modal":modal,"motion":motion,"items":items}))
}""")


if __name__ == "__main__":
    main()
