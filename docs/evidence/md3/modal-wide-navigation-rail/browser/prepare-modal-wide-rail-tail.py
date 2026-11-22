#!/usr/bin/env python3
"""Route the Material runner to the canonical cases after M174."""

import argparse
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("scratch", type=Path)
    args = parser.parse_args()
    if args.scratch.exists():
        parser.error("the scratch destination must be absent")
    subprocess.run(["cp", "-cR", args.source, args.scratch], check=True)
    path = args.scratch / "e2e/src/material.rs"
    text = path.read_text()
    signature = "async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {"
    marker = "    search_view::journey(env, tab).await?;\n"
    cut = text.index(marker) + len(marker)
    end = text.index("\n\nconst COLLAPSIBLE_FAB", cut)
    tail = text[cut:end]
    steps = ("M125", "M126", "M135", "M136", "M137", "M175", "M176", "M177", "M178", "M179", "M180", "M22")
    step_list = ", ".join(f'"{step}"' for step in steps)
    text, count = re.subn(
        r"pub const STEPS: &\[&str\] = &\[.*?\];",
        f"pub const STEPS: &[&str] = &[{step_list}];",
        text,
        count=1,
        flags=re.S,
    )
    if count != 1 or text.count(signature) != 1:
        raise ValueError("expected one canonical Material route and step list")
    prefix = '    crate::smoke::ready(tab, "gallery").await?;\n'
    retired = "#[allow(dead_code)]\n" + signature.replace("fn journey", "fn full_journey", 1)
    path.write_text(text.replace(signature, f"{signature}\n{prefix}{tail}\n\n{retired}", 1))


if __name__ == "__main__":
    main()
