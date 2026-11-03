//! Mounting a root component and flushing its pending updates.

use std::cell::RefCell;

use surface::{Component, Element};

use super::fiber_id::FiberId;
use super::runtime::{render_fiber, RUNTIME};

/// Mount point — runs a component once and returns its initial
/// rendered tree + a handle for subsequent event dispatch and
/// updates.
pub fn mount(component: Component) -> MountHandle {
    let fiber_id = RUNTIME.with(|r| r.borrow_mut().new_fiber());
    let tree = render_fiber(fiber_id, component.clone());
    MountHandle {
        fiber_id,
        component,
        tree: RefCell::new(tree),
    }
}

/// Returned from `mount`. Holds the live fiber + the last rendered
/// tree so tests / the WebGPU renderer can inspect it.
pub struct MountHandle {
    pub fiber_id: FiberId,
    pub component: Component,
    pub tree: RefCell<Element>,
}

impl MountHandle {
    /// Returns a clone of the currently-mounted element tree.
    pub fn snapshot(&self) -> Element {
        self.tree.borrow().clone()
    }

    /// Synchronously flush any pending state updates. Re-runs the
    /// component function for each dirty fiber and replaces this
    /// handle's tree if the root was affected. Returns whether a
    /// re-render happened.
    pub fn flush(&self) -> bool {
        let re_render = RUNTIME.with(|r| {
            let rt = r.borrow();
            rt.fibers.iter().any(|f| f.id == self.fiber_id && f.dirty)
        });
        if !re_render {
            return false;
        }
        let new_tree = render_fiber(self.fiber_id, self.component.clone());
        *self.tree.borrow_mut() = new_tree;
        RUNTIME.with(|r| {
            let mut rt = r.borrow_mut();
            rt.fiber_mut(self.fiber_id).dirty = false;
        });
        true
    }
}
