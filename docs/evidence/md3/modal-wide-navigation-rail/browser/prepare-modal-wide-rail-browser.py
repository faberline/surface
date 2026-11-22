#!/usr/bin/env python3
"""Route the original browser fixtures to the two modal wide rail cases."""

import argparse
from pathlib import Path
import re
import subprocess


def focus(source, route, case):
    text = source.read_text()
    text, count = re.subn(
        r"pub const STEPS: &\[&str\] = &\[.*?\];",
        f'pub const STEPS: &[&str] = &["{case}"];',
        text,
        count=1,
        flags=re.S,
    )
    if count != 1:
        raise ValueError(f"Missing step list in {source}")
    text, count = re.subn(
        r"async fn journey\([^\n]+\) -> Result<\(\)> \{",
        lambda old: route + "\n\n#[allow(dead_code)]\n"
        + old.group().replace("fn journey", "fn earlier_journey", 1),
        text,
        count=1,
    )
    if count != 1:
        raise ValueError(f"Missing journey in {source}")
    source.write_text(text)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("focused_source", type=Path)
    args = parser.parse_args()
    if args.focused_source.exists():
        parser.error("the focused source destination must be absent")
    subprocess.run(["cp", "-cR", args.source, args.focused_source], check=True)
    e2e = args.focused_source / "e2e/src"
    focus(e2e / "material.rs", """async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    let _ = chrome;
    crate::smoke::ready(tab, "gallery").await?;
    modal_wide_navigation_rail::journey(env, tab).await
}""", "M180")
    focus(e2e / "web_accessibility.rs", """async fn journey(env: &Env, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    let evidence = modal_wide_navigation_rail::accessibility(env, tab).await?;
    env.write_json("browser-modal-wide-rail-accessibility.json", &evidence)
}""", "A70")


if __name__ == "__main__":
    main()
