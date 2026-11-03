//! Debug-only views of the runtime for tooling, behind the `debug` feature.
//! They expose just enough shape for `JetDebug` to serialize while `Fiber`
//! stays `pub(crate)`.

mod fibers;
mod hooks;

pub use fibers::{debug_snapshot_fibers, DebugFiberMeta};
pub use hooks::{debug_snapshot_hooks, DebugHookSummary};

use super::mount_handle::MountHandle;
use super::runtime::RUNTIME;

impl MountHandle {
    /// Debug-only: force the root fiber dirty so the next `flush()`
    /// re-renders even when no state changed. Used by `JetDebug::force_rerender`.
    pub fn mark_root_dirty(&self) {
        RUNTIME.with(|r| {
            let mut rt = r.borrow_mut();
            rt.fiber_mut(self.fiber_id).dirty = true;
        });
    }
}

#[cfg(test)]
mod tests;
