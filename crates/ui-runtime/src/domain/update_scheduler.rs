//! The renderer's update scheduler, called on every state write.

use std::rc::Rc;

use super::runtime::UPDATE_SCHEDULER;

/// Register the renderer-side async update scheduler. State setters
/// are framework-level primitives, but only the mounted renderer knows
/// how to coalesce dirty fibers into an actual frame.
pub fn set_update_scheduler(scheduler: Option<Rc<dyn Fn()>>) {
    UPDATE_SCHEDULER.with(|slot| {
        *slot.borrow_mut() = scheduler;
    });
}

pub(crate) fn notify_update_scheduled() {
    UPDATE_SCHEDULER.with(|slot| {
        if let Some(schedule) = slot.borrow().as_ref().cloned() {
            schedule();
        }
    });
}
