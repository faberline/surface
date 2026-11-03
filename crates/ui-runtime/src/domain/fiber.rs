//! The storage of one mounted component.

use super::fiber_id::FiberId;
use super::hook_slot::HookSlot;

/// Per-component hook storage. The transpiler compiles each
/// `useState` / `useEffect` into a positional slot lookup that
/// points into this `Vec`. React's "rules of hooks" (call in the
/// same order every render) enforce the positional contract.
#[derive(Default)]
pub(crate) struct Fiber {
    pub id: FiberId,
    pub hooks: Vec<HookSlot>,
    /// Cursor incremented by each hook call during a single render.
    /// Reset to 0 at the start of every render.
    pub hook_cursor: usize,
    /// Marked dirty by a state setter; the scheduler picks dirty
    /// fibers and re-renders them on the next commit.
    pub dirty: bool,
}
