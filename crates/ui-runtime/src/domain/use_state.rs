//! `use_state`: a positional per-fiber state cell and its setter.

use super::fiber_id::FiberId;
use super::hook_slot::HookSlot;
use super::runtime::RUNTIME;
use super::update_scheduler::notify_update_scheduled;

/// `useState` — positional per-fiber state cell. Returns the current
/// value and a setter. The setter, when invoked, marks the owning
/// fiber dirty; the next commit re-renders it.
///
/// Matches React's semantics: setter accepts a new value OR a
/// function that receives the current value and returns the new
/// value. For v0 we only support the direct-value form — the
/// functional form lands when we implement `useReducer` (same
/// shape).
pub fn use_state<T: Clone + 'static>(initial: T) -> (T, StateSetter<T>) {
    RUNTIME.with(|r| {
        let mut rt = r.borrow_mut();
        let id = rt
            .current
            .expect("hook called outside a render — rules-of-hooks violation");
        let fiber = rt.fiber_mut(id);
        let idx = fiber.hook_cursor;
        fiber.hook_cursor += 1;
        if fiber.hooks.len() <= idx {
            fiber.hooks.push(HookSlot::State(Box::new(initial)));
        }
        let HookSlot::State(any_value) = &fiber.hooks[idx] else {
            panic!(
                "hook slot {idx} type mismatch: expected State, got another kind — \
                 rules-of-hooks violation OR transpiler bug"
            );
        };
        let value: &T = any_value
            .downcast_ref()
            .expect("hook slot type mismatch — rules-of-hooks violation OR transpiler bug");
        let value = value.clone();
        let setter = StateSetter {
            fiber_id: id,
            idx,
            _marker: std::marker::PhantomData,
        };
        (value, setter)
    })
}

/// Setter handle returned from `use_state`. Clone-friendly so it can
/// be moved into event-handler closures; updating schedules a
/// re-render of the owning fiber.
pub struct StateSetter<T: Clone + 'static> {
    fiber_id: FiberId,
    idx: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Clone + 'static> Clone for StateSetter<T> {
    fn clone(&self) -> Self {
        Self {
            fiber_id: self.fiber_id,
            idx: self.idx,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + 'static> StateSetter<T> {
    /// Write a new value. Marks the owning fiber dirty. Re-render
    /// happens on the next `flush_updates` call.
    pub fn set(&self, new_value: T) {
        RUNTIME.with(|r| {
            let mut rt = r.borrow_mut();
            let fiber = rt.fiber_mut(self.fiber_id);
            fiber.hooks[self.idx] = HookSlot::State(Box::new(new_value));
            fiber.dirty = true;
        });
        notify_update_scheduled();
    }
}
