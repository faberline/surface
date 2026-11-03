//! The thread-local runtime: the fiber list, the fiber being rendered and
//! the registered update scheduler.
//!
//! Single-threaded by design — a React app is a single-threaded render
//! loop. When this ships to WASM the browser's main thread is our
//! thread, and there's no possibility of concurrent renders.

use std::cell::RefCell;
use std::rc::Rc;

use surface::{Component, Element};

use super::fiber::Fiber;
use super::fiber_id::FiberId;

thread_local! {
    pub(crate) static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
    pub(crate) static UPDATE_SCHEDULER: RefCell<Option<Rc<dyn Fn()>>> = RefCell::new(None);
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub(crate) fibers: Vec<Fiber>,
    pub(crate) current: Option<FiberId>,
    next_id: u64,
}

impl Runtime {
    pub(crate) fn new_fiber(&mut self) -> FiberId {
        let id = FiberId::new(self.next_id);
        self.next_id += 1;
        self.fibers.push(Fiber {
            id,
            ..Default::default()
        });
        id
    }

    pub(crate) fn fiber_mut(&mut self, id: FiberId) -> &mut Fiber {
        self.fibers
            .iter_mut()
            .find(|f| f.id == id)
            .expect("fiber id not found — scheduler bug")
    }

    fn begin_render(&mut self, id: FiberId) {
        let f = self.fiber_mut(id);
        f.hook_cursor = 0;
        self.current = Some(id);
    }

    fn end_render(&mut self) {
        self.current = None;
    }
}

#[allow(dead_code)]
fn with_current_fiber<R>(f: impl FnOnce(&mut Fiber) -> R) -> R {
    // Kept around — `use_effect` (deferred) will use this instead of
    // inlining the RUNTIME.with dance. Silence dead_code until then.
    RUNTIME.with(|r| {
        let mut rt = r.borrow_mut();
        let id = rt
            .current
            .expect("hook called outside a render — rules-of-hooks violation");
        let fiber = rt.fiber_mut(id);
        f(fiber)
    })
}

pub(crate) fn render_fiber(fiber_id: FiberId, component: Component) -> Element {
    RUNTIME.with(|r| r.borrow_mut().begin_render(fiber_id));
    let tree = component.render();
    RUNTIME.with(|r| r.borrow_mut().end_render());
    tree
}
