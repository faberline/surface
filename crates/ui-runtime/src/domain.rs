//! The component runtime: fibers and their hook slots, the thread-local
//! runtime that renders them, the hooks components call, mounting and the
//! update scheduler.

mod fiber;
mod fiber_id;
mod hook_slot;
mod mount_handle;
mod runtime;
mod update_scheduler;
mod use_callback;
mod use_effect_once;
mod use_memo;
mod use_reducer;
mod use_ref;
mod use_state;

#[cfg(feature = "debug")]
mod debug;

pub use fiber_id::FiberId;
pub use mount_handle::{mount, MountHandle};
pub use update_scheduler::set_update_scheduler;
pub use use_callback::use_callback;
pub use use_effect_once::use_effect_once;
pub use use_memo::{hash_dep, use_memo, MemoDepHash};
pub use use_reducer::{use_reducer, DispatchHandle};
pub use use_ref::{use_ref, RefHandle};
pub use use_state::{use_state, StateSetter};

#[cfg(feature = "debug")]
pub use debug::{debug_snapshot_fibers, debug_snapshot_hooks, DebugFiberMeta, DebugHookSummary};
