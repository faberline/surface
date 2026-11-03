//! `use_reducer`: a state slot whose transitions run through a reducer.

use std::rc::Rc;

use super::fiber_id::FiberId;
use super::hook_slot::HookSlot;
use super::runtime::RUNTIME;
use super::update_scheduler::notify_update_scheduled;

type Reducer<S, A> = Rc<dyn Fn(&S, A) -> S>;

/// Dispatch handle returned from `use_reducer`.
pub struct DispatchHandle<S: Clone + 'static, A: 'static> {
    fiber_id: FiberId,
    idx: usize,
    reducer: Reducer<S, A>,
    _marker: std::marker::PhantomData<(S, A)>,
}

impl<S: Clone + 'static, A: 'static> Clone for DispatchHandle<S, A> {
    fn clone(&self) -> Self {
        Self {
            fiber_id: self.fiber_id,
            idx: self.idx,
            reducer: self.reducer.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Clone + 'static, A: 'static> DispatchHandle<S, A> {
    pub fn dispatch(&self, action: A) {
        RUNTIME.with(|r| {
            let mut rt = r.borrow_mut();
            let fiber = rt.fiber_mut(self.fiber_id);
            let HookSlot::State(any_value) = &fiber.hooks[self.idx] else {
                panic!(
                    "hook slot {} type mismatch: expected State (reducer), got another kind",
                    self.idx
                );
            };
            let current: &S = any_value
                .downcast_ref()
                .expect("reducer slot type mismatch — rules-of-hooks OR transpiler bug");
            let new_state = (self.reducer)(current, action);
            fiber.hooks[self.idx] = HookSlot::State(Box::new(new_state));
            fiber.dirty = true;
        });
        notify_update_scheduled();
    }
}

/// `useReducer` — same slot as `useState`, but transitions driven
/// by a pure reducer. Reducer is stable across renders.
pub fn use_reducer<S: Clone + 'static, A: 'static, F: Fn(&S, A) -> S + 'static>(
    reducer: F,
    initial: S,
) -> (S, DispatchHandle<S, A>) {
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
            panic!("hook slot {idx} type mismatch: expected State (reducer), got another kind");
        };
        let state: &S = any_value
            .downcast_ref()
            .expect("reducer slot type mismatch — rules-of-hooks OR transpiler bug");
        (
            state.clone(),
            DispatchHandle {
                fiber_id: id,
                idx,
                reducer: Rc::new(reducer),
                _marker: std::marker::PhantomData,
            },
        )
    })
}
