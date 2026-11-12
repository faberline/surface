//! Host decisions for a window close. Pure, so they run without a window.
use surface_app::{AppError, Application, Runtime};

/// What the event loop does after a close request.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HostStep {
    Exit,
    Stay,
    Fail(String),
}

/// Map the result of `Runtime::request_close` to a host step.
pub(crate) fn step_from(result: Result<bool, AppError>, closing: &mut bool) -> HostStep {
    let _ = (result, closing);
    HostStep::Stay // STUB (red)
}

/// A window close request. A request after the loop began to exit is ignored,
/// because winit can still deliver one in the same iteration.
pub(crate) fn close_request_step<A: Application>(
    runtime: &mut Runtime<A>,
    closing: &mut bool,
) -> HostStep {
    let _ = (runtime, closing);
    HostStep::Stay // STUB (red)
}

/// True once the app asked to close with `Queue::close`.
pub(crate) fn should_exit<A: Application>(runtime: &mut Runtime<A>, closing: &mut bool) -> bool {
    let _ = (runtime, closing);
    false // STUB (red)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use surface::{Node, Role};
    use surface_app::{CloseDecision, Queue, View};

    #[derive(Clone)]
    enum Msg {}
    struct App {
        asked: Rc<Cell<u32>>,
        veto: bool,
        close_in_hook: bool,
        queue: Rc<std::cell::RefCell<Option<Queue<Msg>>>>,
    }
    impl Application for App {
        type Message = Msg;
        fn update(&mut self, message: Msg) {
            match message {}
        }
        fn close_requested(&mut self) -> CloseDecision {
            self.asked.set(self.asked.get() + 1);
            if self.close_in_hook {
                if let Some(q) = self.queue.borrow().as_ref() {
                    q.close();
                }
            }
            if self.veto {
                CloseDecision::Cancel
            } else {
                CloseDecision::Allow
            }
        }
        fn view(&self) -> View<Msg> {
            View::new(Node::new(Role::Window, "App"))
        }
    }
    fn runtime(veto: bool, close_in_hook: bool) -> (Runtime<App>, Rc<Cell<u32>>) {
        let asked = Rc::new(Cell::new(0));
        let queue = Rc::new(std::cell::RefCell::new(None));
        let rt = Runtime::new(App {
            asked: asked.clone(),
            veto,
            close_in_hook,
            queue: queue.clone(),
        })
        .unwrap();
        *queue.borrow_mut() = Some(rt.queue());
        (rt, asked)
    }

    #[test]
    fn an_allowed_request_exits_and_calls_the_hook_once() {
        let (mut rt, asked) = runtime(false, false);
        let mut closing = false;
        assert_eq!(close_request_step(&mut rt, &mut closing), HostStep::Exit);
        assert!(closing);
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn a_vetoed_request_stays_and_is_asked_again_next_time() {
        let (mut rt, asked) = runtime(true, false);
        let mut closing = false;
        assert_eq!(close_request_step(&mut rt, &mut closing), HostStep::Stay);
        assert!(!closing);
        assert_eq!(close_request_step(&mut rt, &mut closing), HostStep::Stay);
        assert_eq!(asked.get(), 2);
    }

    #[test]
    fn a_second_request_after_exit_does_not_run_the_hook_again() {
        let (mut rt, asked) = runtime(false, false);
        let mut closing = false;
        assert_eq!(close_request_step(&mut rt, &mut closing), HostStep::Exit);
        assert_eq!(close_request_step(&mut rt, &mut closing), HostStep::Stay);
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn a_veto_with_an_immediate_close_command_exits_and_consumes_the_flag() {
        let (mut rt, _) = runtime(true, true);
        let mut closing = false;
        assert_eq!(close_request_step(&mut rt, &mut closing), HostStep::Exit);
        assert!(!rt.take_close(), "the flag was consumed by the request");
    }

    #[test]
    fn an_app_close_command_exits_once() {
        let (mut rt, asked) = runtime(true, false);
        let mut closing = false;
        assert!(!should_exit(&mut rt, &mut closing), "nothing asked");
        rt.queue().close();
        assert!(should_exit(&mut rt, &mut closing));
        assert!(closing);
        assert_eq!(asked.get(), 0, "the command skips the hook");
        assert!(!should_exit(&mut rt, &mut closing), "reported once");
    }

    #[test]
    fn an_app_error_fails_the_host() {
        let mut closing = false;
        let step = step_from(Err(AppError::MissingId("x".into())), &mut closing);
        assert!(matches!(step, HostStep::Fail(m) if m.contains('x')));
        assert!(!closing);
        assert_eq!(step_from(Ok(true), &mut closing), HostStep::Exit);
        assert!(closing);
    }
}
