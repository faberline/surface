# 0002: Surface devtools protocol

Status: Accepted (decided by the owner 2026-11-14)
Date: 2026-11-14

This record keeps the owner's decisions on observability and the defaults
accepted with the plan. The design is in [observability.md](../observability.md).
Line numbers below are from `origin/main` at `aff5844`.

## Context

A running Surface app is much less observable than a web page under browser
DevTools. A web page has a DOM, a console, network and performance traces,
screenshots and input injection. Surface has the first part only.

What exists:

- The semantic export `DomExport` v1, schema `surface.dom.v1`
  (`crates/surface-app/src/snapshot.rs:10-68`).
- A Serde `DisplayList` (`crates/surface/src/paint.rs:157`).
- Hit testing and change detection on `ComputedDom`
  (`crates/surface/src/layout.rs:103-127`).
- Seven jet-bridge operations: `capabilities`, `snapshot`, `query`,
  `activate`, `fill`, `set_value`, `key`. They run in
  `Runtime::execute_bridge` (`crates/surface-app/src/bridge.rs:14`), pinned
  at jet-bridge rev `a21f934` (`docs/application-bridge.md`).

What is missing is the time dimension and the pixels:

- No event or message trace, no frame timing, no `log` or `tracing`.
- No screenshot of a native app. Offscreen readback exists only in a test
  (`crates/surface-wgpu/tests/it.rs:336-374`).
- No paint-op to node mapping: `PaintOp` carries no node id
  (`crates/surface/src/paint.rs:18-60`).
- No waiting, no pointer or drag injection, no replay.

The current bridge is the Jet command set. Surface depends on it only
through `jet-bridge-protocol`, its one Jet-named dependency
(`docs/migration.md:49-51`). New operations such as trace, screenshot,
paint and input do not fit that set, and Surface cannot extend it alone.

Open issue #68 asks for a headless driver and evidence format. It is
re-split onto the work in this record (see observability.md).

## Decisions by the owner

All decided on 2026-11-14.

1. **Both tracks are planned.** Observability is the first track that can
   start. The gaps between Surface and egui/iced get an order (see
   `docs/roadmap.md`). Only the first wave is split into issues now.
2. **Agents and tests first.** Observability serves AI agents and automated
   tests first. The interface is machine-readable (CLI and MCP). A human
   inspector UI is out of scope for now.
3. **Surface owns its devtools protocol.** New operations do not depend on
   the jet-bridge command set.
4. **Timing.** Observability work starts after the desktop milestone S1
   (PR #118 key handler, #48 native IME) and before migration steps 13 and
   9. Steps 13 and 9 come after the first batch, O1 to O5.

## Defaults accepted with the plan

| # | Question | Default |
| --- | --- | --- |
| P1 | When does the jet-bridge compat adapter go? | It stays until migration step 23 is decided. The three `bridge-*` e2e suites (`bridge-native`, `bridge-native-compact`, `bridge-web`) stay unchanged and are the adapter's acceptance test. |
| P2 | How are messages logged? | A default method `Application::describe(&self, &Self::Message) -> Option<String>`, returning `None` by default. No `Debug` bound. A bound behind a feature would break every app, because `scripts/check.sh` builds with `--all-features`. |
| P3 | Which logging crate? | The `log` facade. It is smaller than `tracing` and works on wasm. |
| P4 | Web devtools? | Native first. Decision 0001 D-B limits web work to migration steps 9-15 and bug fixes until D1 reports. The web transport and web screenshot are separate items that wait for D-B to lift. |
| P5 | Feature name? | Reuse `devtools`, off by default. Native also needs an environment variable. |

## Consequences

### Effect on Jet

- Migration step 19 (`docs/migration.md:338`) is not affected while
  `surface.dom.v1` stays byte-identical.
- Migration step 23 (`docs/migration.md:369-373`) is affected. Its gate names
  the Surface bridge suites. Removing the adapter would break that gate.
  Jet tooling that speaks the seven jet-bridge operations keeps working
  through the adapter. Trace, screenshot, paint and input appear only in the
  new protocol.
- The Jet side is cross-repo. It goes through the planner and
  aw-service-desk. This record does not change Jet.

### Risks of owning the protocol

- Two protocols exist until the adapter goes. They can drift. The unchanged
  bridge suites are the guard.
- Surface owns a client binary and an MCP surface.
- Versioning discipline moves to Surface.
- Jet tooling may lag behind the new operations.
- A loopback token gives full input control. The feature is off by default,
  binds 127.0.0.1 only and checks the token on every connection.
- Five issues edit `crates/surface-platform/src/lib.rs`. They must run one
  at a time.

### Effect on other work

- Decision 0001 is unchanged. D-A, D-B and D-C stand.
- Web observability waits for D-B to lift.
- Migration steps 13 and 9 move behind O1 to O5.

## Left for the issue spec stage

These are not decided here. Each issue spec asks the owner.

- How the CLI reaches a web app.
- Whether the MCP server is hand-written or built on a crate.
- Whether Linux CI devtools runs count as evidence.
