//! Window close hook (issue #104). Headless: the host only forwards the
//! window's close request to `Runtime::request_close`.
use std::{cell::RefCell, rc::Rc};
use surface::{Node, Role};
use surface_app::{Application, CloseDecision, Queue, Runtime, View};

#[derive(Clone)]
enum Msg {
    Bump,
    DiscardAndClose,
}

#[derive(Default)]
struct Shared {
    drops: u32,
    queue: Option<Queue<Msg>>,
}

struct Editor {
    n: i32,
    asked: u32,
    veto: bool,
    shared: Rc<RefCell<Shared>>,
}
impl Editor {
    fn new(veto: bool) -> (Self, Rc<RefCell<Shared>>) {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let app = Self {
            n: 0,
            asked: 0,
            veto,
            shared: shared.clone(),
        };
        (app, shared)
    }
}
impl Drop for Editor {
    fn drop(&mut self) {
        self.shared.borrow_mut().drops += 1;
    }
}
impl Application for Editor {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        match message {
            Msg::Bump => self.n += 1,
            Msg::DiscardAndClose => {
                self.veto = false;
                if let Some(queue) = &self.shared.borrow().queue {
                    queue.close();
                }
            }
        }
    }
    fn close_requested(&mut self) -> CloseDecision {
        self.asked += 1;
        if self.veto {
            CloseDecision::Cancel
        } else {
            CloseDecision::Allow
        }
    }
    fn view(&self) -> View<Msg> {
        View::new(Node::new(Role::Window, "Editor")).children([View::new(
            Node::new(Role::Text, format!("n={}", self.n)).id("n"),
        )])
    }
}

#[test]
fn a_close_request_calls_the_hook_exactly_once_before_exit() {
    let (app, _) = Editor::new(false);
    let mut runtime = Runtime::new(app).unwrap();
    assert_eq!(runtime.app().asked, 0, "no hook call before a request");
    let close = runtime.request_close().unwrap();
    assert!(close, "an allowed request closes");
    assert_eq!(runtime.app().asked, 1);
}

#[test]
fn a_cancelled_close_keeps_the_window_and_later_events_still_run() {
    let (app, shared) = Editor::new(true);
    let mut runtime = Runtime::new(app).unwrap();
    runtime.apply(Msg::Bump).unwrap();
    assert!(!runtime.request_close().unwrap(), "veto keeps the window");
    assert!(!runtime.take_close(), "a veto is not a close command");
    assert_eq!(runtime.app().n, 1, "state is kept");
    assert_eq!(shared.borrow().drops, 0, "state is not dropped");
    runtime.apply(Msg::Bump).unwrap();
    assert_eq!(runtime.app().n, 2, "later events still run");
    assert!(runtime.export_dom().unwrap().contains("n=2"));
    assert!(
        !runtime.request_close().unwrap(),
        "asked again, vetoed again"
    );
    assert_eq!(runtime.app().asked, 2);
}

#[test]
fn the_app_close_command_ends_the_loop_after_a_veto() {
    let (app, shared) = Editor::new(true);
    let mut runtime = Runtime::new(app).unwrap();
    shared.borrow_mut().queue = Some(runtime.queue());
    assert!(!runtime.request_close().unwrap());
    assert!(!runtime.take_close(), "nothing asked to close yet");
    runtime.apply(Msg::DiscardAndClose).unwrap();
    assert!(
        runtime.take_close(),
        "the close command is seen by the host"
    );
    assert!(!runtime.take_close(), "it is reported once");
    assert_eq!(runtime.app().asked, 1, "the command skips the hook");
}

#[test]
fn without_a_hook_the_default_allows_the_close() {
    #[derive(Clone)]
    enum Never {}
    struct Plain;
    impl Application for Plain {
        type Message = Never;
        fn update(&mut self, message: Never) {
            match message {}
        }
        fn view(&self) -> View<Never> {
            View::new(Node::new(Role::Window, "Plain"))
        }
    }
    let mut runtime = Runtime::new(Plain).unwrap();
    assert!(runtime.request_close().unwrap());
    assert!(!runtime.take_close());
}

#[test]
fn the_app_state_is_dropped_when_the_runtime_ends() {
    let (app, shared) = Editor::new(false);
    let runtime = Runtime::new(app).unwrap();
    shared.borrow_mut().queue = Some(runtime.queue());
    assert_eq!(shared.borrow().drops, 0);
    drop(runtime);
    // Queue clones hold no app, so the app state is gone exactly once.
    assert_eq!(shared.borrow().drops, 1);
}
