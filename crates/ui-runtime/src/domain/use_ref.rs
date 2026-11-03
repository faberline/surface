//! `use_ref`: a mutable cell that survives re-renders.

use std::cell::RefCell;
use std::rc::Rc;

use super::hook_slot::HookSlot;
use super::runtime::RUNTIME;

/// Persistent mutable cell that survives re-renders. Mutating a
/// ref does NOT trigger a re-render.
pub struct RefHandle<T: 'static> {
    cell: Rc<RefCell<Box<dyn std::any::Any>>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> Clone for RefHandle<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + 'static> RefHandle<T> {
    pub fn current(&self) -> T {
        let b = self.cell.borrow();
        b.downcast_ref::<T>()
            .expect("RefHandle type mismatch — transpiler bug")
            .clone()
    }

    pub fn set(&self, new_value: T) {
        let mut b = self.cell.borrow_mut();
        *b = Box::new(new_value);
    }
}

impl<T: 'static> RefHandle<T> {
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut b = self.cell.borrow_mut();
        let t = b
            .downcast_mut::<T>()
            .expect("RefHandle type mismatch — transpiler bug");
        f(t)
    }
}

/// `useRef` — stable mutable container across renders.
pub fn use_ref<T: 'static>(initial: T) -> RefHandle<T> {
    RUNTIME.with(|r| {
        let mut rt = r.borrow_mut();
        let id = rt
            .current
            .expect("hook called outside a render — rules-of-hooks violation");
        let fiber = rt.fiber_mut(id);
        let idx = fiber.hook_cursor;
        fiber.hook_cursor += 1;
        if fiber.hooks.len() <= idx {
            let cell: Rc<RefCell<Box<dyn std::any::Any>>> =
                Rc::new(RefCell::new(Box::new(initial)));
            fiber.hooks.push(HookSlot::Ref(cell));
        }
        let HookSlot::Ref(cell) = &fiber.hooks[idx] else {
            panic!(
                "hook slot {idx} type mismatch: expected Ref, got another kind — \
                 rules-of-hooks OR transpiler bug"
            );
        };
        RefHandle {
            cell: cell.clone(),
            _marker: std::marker::PhantomData,
        }
    })
}
