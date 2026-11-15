# Delivery stages

## Current checkpoint: surface@0.1.0

[Milestone](https://github.com/faberline/surface/milestone/1) · [Landing issue #5](https://github.com/faberline/surface/issues/5)

This stage lands UI = f(state), the shared native/WASM renderer, the current
partial Material controls, controlled Unicode editing, current DOM export,
opt-in CLI/MCP access and focus scrolling. It does not qualify the full
Material catalog or the physical native OS input-method journey.

The PR #4 landing at `8b4e205` passed 45 Rust cases, the native/WASM
checks, the compact macOS GPU and Chrome 157 WebGPU bridge journeys, the
existing Material browser journey and the 40-event browser composition
journey. Hosted CI passed there
([run 41807907931](https://github.com/faberline/surface/actions/runs/41807907931)).
Tag `v0.1.0` at `b3c4917` adds PR #55 and the PR #56 consumer contract.
Hosted [run 42172139938](https://github.com/faberline/surface/actions/runs/42172139938)
passed there: 117 passed, 0 failed, 4 ignored. The 4 ignored GPU tests did
not run. No crate is published.

## Full framework target: surface@0.2.0

[Milestone](https://github.com/faberline/surface/milestone/2) tracks the
[completion contract](completion-contract.md). Its issues cover the
Material foundations, the component families and the input, platform and
accessibility qualification. These are backlog items; this planning pass
starts no worker loop.

Native OS Zhuyin verification is deferred at the user request in
[issue #48](https://github.com/faberline/surface/issues/48). Semantic fill and
synthetic browser events do not qualify an OS candidate window.

The issue list lives on GitHub. Use the
[milestones page](https://github.com/faberline/surface/milestones) for the
current set of issues and their states. This file does not copy it.

## Consumer migration

[Core #95](https://github.com/faberline/core/pull/95) preserves the old
package names through forwarding crates. Its required development checks
must pass before landing; the old EC compiler/digest qualification is open.
The broader Core MicroDOM/rendering issues remain independently scoped.

[Jet #76](https://github.com/faberline/jet/pull/76), including #75, switches
to the public standalone dependencies. Its hosted parity wrapper currently
reports an unwired oracle. That exit is not hosted parity acceptance. The
local browser/WASM evidence and any required current-head gates stay visible.

Core #95 and Jet #75 pin `1754a2e96e487ea693be558041abdf21ffc63d27`, which
is not an ancestor of Surface main. Tag `v0.1.0` was cut from main at
`b3c4917`. The [agreed Jet and Surface split](migration.md#jet-and-surface-split)
and the plan in [migration.md](migration.md) replace the `1754a2e` pin with
that tag. Neither consumer PR is merged. Whether to rebase or replace the PRs
is an open decision there. Branches and the separate native IME experiment
remain preserved.
