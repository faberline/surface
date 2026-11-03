# Core and Jet migration

Surface has a new home: `faberline/surface`. This repository does not depend
on Core or Jet. Their working trees and release tags were not changed.

The existing `surface` and `ui-runtime` sources were imported from Core
commit `41c352eaae687a1914087d279f36adbe9e9b4af5`. Their public names remain
available. New APIs are additive. Imported shell-recursive EC wrappers were
replaced with direct Rust behavior tests suitable for this repository.

Core still contains its old packages. Jet still pins Core `v0.4.14`. That
is a staged migration, not an automatic dependency switch. Do not mix a
Core `surface::Element` value with the standalone crate's value in one
dependency graph: Cargo treats them as different Rust types.

## Follow-up sequence

1. Version and validate the standalone package API with consumers.
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
