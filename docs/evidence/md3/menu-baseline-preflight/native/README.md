# Native command menu proof

This proof uses a stock Surface Gallery built from the 590-file interim menu
candidate. The source version is 0.1.45. The manifest hash is
`5eeeb4ef56134d28edc731f137fd648871e199edb2708bc54accaff38bbc9519`.
Use this source identity when comparing later release candidates.

Run the two scripts from a fresh copy of this evidence folder. Supply the
exact source directory and its manifest to the build. The build script checks
the source before and after compilation. The replay checks both programs
before and after use.

```sh
sh build-pinned.sh "$source" "$manifest" 5eeeb4ef56134d28edc731f137fd648871e199edb2708bc54accaff38bbc9519
PYTHON_BIN=/opt/homebrew/opt/python@3.14/bin/python3.14 sh replay-native.sh "$output"
```

Use a new output directory below `/tmp`. The tests run on macOS with the
native offscreen GPU, private loopback, and Reduce Motion. They sample the
running Gallery's on-screen window count through CoreGraphics. The native
key API drives semantic input. Browser pointer behavior and live VoiceOver
remain separate from this proof.

The 960 × 800 light replay opens below the trigger, visits the disabled
command, tests Enter and Space, rejects a direct activation request, and
chooses the next enabled command. It verifies that Tab, Shift+Tab, and
Escape return focus to the right control. The 320 × 480 dark replay
opens above a bottom trigger in right-to-left mode with 200% text.
It uses End to reveal the clipped last command. Each replay checks panel
and page colors, a visible text sample, and pixels of the focused row's
outline. Each screenshot must match across Page.screenshot, CLI and MCP.

For the independent Web focus diagnostic, use the earlier stock Web source
manifest `2a41d9838ff4043bd6836cd1b96a6b19a5c64e32dd204af22bce64b7f5d8c202`.
The pinned WASM hash is
`cc720f832755435037ef839f179e6c3e1fef98e64a537af5330d946d68a3ab5a`.
Use a fresh diagnostic output directory.

```sh
node diagnose-menu-focus.mjs "$gallery_web" "$diagnostic_output"
```

The Web diagnostic checks projected focus before the real click, Runtime
focus after the click sends browser focus to the canvas, and projected
focus after explicit reentry. It runs Chrome in headless mode over a
private debugging pipe.
