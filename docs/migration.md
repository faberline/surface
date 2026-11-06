# Core and Jet migration

Surface has a new home: `faberline/surface`. This repository does not depend
on Core or Jet. Their working trees and release tags were not changed.

The existing `surface` and `ui-runtime` sources were imported from Core
commit `41c352eaae687a1914087d279f36adbe9e9b4af5`. Their public names remain
available. New APIs are additive. Imported shell-recursive EC wrappers were
replaced with direct Rust behavior tests suitable for this repository.

Core's and Jet's main branches still use the old dependency arrangement.
The reviewable migration proposals are:

- [Core PR #95](https://github.com/faberline/core/pull/95), implementation commit
  `23e614ca3e85e7ef77ae5563bb9c7a1fbdc15120`: compatibility packages
  re-export the standalone types and hooks, including debug support.
- [Jet PR #75](https://github.com/faberline/jet/pull/75), implementation commit
  `41ecdf95aeaa2723f862d1007fe2d1eb6c7718e1`: both UI dependencies and
  generated Rust use standalone Surface. `cli-std` stays on Core v0.4.14.

Both pin `1754a2e96e487ea693be558041abdf21ffc63d27`. Surface is public.
Its Linux CI passes at that revision. Neither consumer PR is merged.

Core's mixed-source behavior/type tests pass locally. Core CI stops at
draft/issue preflight; old EC metadata and independent acceptance remain
open. Jet's complete local advanced WASM gate passes, including MUI/AntD
live parity. Its hosted parity job reports success by skipping an unwired
oracle, so it is not hosted parity acceptance. Three existing Clippy
findings also reproduce on Jet's untouched base.

Until the facades are adopted, old Core types and standalone types have
different Cargo identities. Move both UI packages together.

## Follow-up sequence

1. Version and validate the standalone package API with consumers.
   [`consumer_contract.rs`](../crates/ui-runtime/tests/consumer_contract.rs)
   pins the `surface` and `ui-runtime` items Jet and Core use. Change it
   only with a consumer migration. The release tag is not cut yet.
2. Move Jet's `surface` and `ui-runtime` dependencies together to one pinned
   commit or release in this repository. Run Jet's transpiler and WASM
   parity gates. Changing only one dependency would split the types.
3. Update Core consumers together. Remove its old packages or replace them
   with compatibility re-exports after all owning applications migrate.
4. Reconcile the existing Core issues #10–#13 and Jet #41 against the
   accepted standalone implementation. Keep the issues open until their
   full criteria have independent evidence.

The new runtime covers a useful part of those plans. It does not complete
their full transform/image/damage, drag/trace, and desktop E2E contracts.
