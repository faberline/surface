//! Renderer-neutral component runtime: fiber tree + hooks + mount/flush loop.
//!
//! The runtime owns React-like authoring semantics without depending on React
//! DOM, a browser, WASM, AppKit, or any concrete renderer. Components render
//! `surface::Element` trees; host adapters decide whether those trees are
//! painted by Jet WASM WebGPU, a native desktop backend, or a test recorder.
//!
//! This is the middle layer between the UI element model and renderer backends:
//!
//! ```text
//! Component/hooks -> Element tree -> layout/paint/backend
//! ```

mod domain;

pub use domain::{
    hash_dep, mount, set_update_scheduler, use_callback, use_effect_once, use_memo, use_reducer,
    use_ref, use_state, DispatchHandle, FiberId, MemoDepHash, MountHandle, RefHandle, StateSetter,
};

#[cfg(feature = "debug")]
pub use domain::{debug_snapshot_fibers, debug_snapshot_hooks, DebugFiberMeta, DebugHookSummary};
