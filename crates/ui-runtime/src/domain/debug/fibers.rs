//! A summary of every fiber's storage.

use crate::domain::fiber_id::FiberId;
use crate::domain::runtime::RUNTIME;

/// Debug-only summary of a fiber's storage. Feature-gated to keep
/// the `pub(crate)` visibility of `Fiber` intact — we expose just
/// enough shape for `JetDebug` to serialize.
pub struct DebugFiberMeta {
    pub id: FiberId,
    pub hook_count: usize,
    pub dirty: bool,
}

pub fn debug_snapshot_fibers() -> Vec<DebugFiberMeta> {
    RUNTIME.with(|r| {
        let rt = r.borrow();
        rt.fibers
            .iter()
            .map(|f| DebugFiberMeta {
                id: f.id,
                hook_count: f.hooks.len(),
                dirty: f.dirty,
            })
            .collect()
    })
}
