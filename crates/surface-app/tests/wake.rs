//! Out-of-band updates (migration step 12, issue #102).
use std::{cell::RefCell, rc::Rc};
use surface::{Node, Role};
use surface_app::{Application, Queue, Runtime, View};

#[derive(Clone)]
enum Msg {
    Add(i32),
    Chain,
}

#[derive(Default)]
struct Counter {
    n: i32,
    woken: u32,
    log: Vec<String>,
    queue: Rc<RefCell<Option<Queue<Msg>>>>,
}

impl Application for Counter {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        match message {
            Msg::Add(by) => {
                self.log.push("add".into());
                self.n += by;
            }
            Msg::Chain => {
                self.log.push("chain:start".into());
                if let Some(queue) = self.queue.borrow().as_ref() {
                    queue.post(Msg::Add(1));
                }
                self.log.push("chain:end".into());
            }
        }
    }
    fn wake(&mut self) {
        self.woken += 1;
    }
    fn view(&self) -> View<Msg> {
        View::new(Node::new(Role::Window, "Counter")).children([
            View::new(Node::new(Role::Text, format!("n={}", self.n)).id("n")),
            View::new(Node::new(Role::Text, format!("woken={}", self.woken)).id("woken")),
        ])
    }
}

#[test]
fn outside_write_changes_the_dom_export() {
    let mut runtime = Runtime::new(Counter::default()).unwrap();
    let before = runtime.export_dom().unwrap();
    assert!(before.contains("n=0"));
    runtime.apply(Msg::Add(3)).unwrap();
    let after = runtime.export_dom().unwrap();
    assert_ne!(before, after);
    assert!(after.contains("n=3"), "{after}");
}

#[test]
fn a_call_made_during_update_is_queued_not_reentered() {
    let mut runtime = Runtime::new(Counter::default()).unwrap();
    *runtime.app().queue.borrow_mut() = Some(runtime.queue());
    runtime.apply(Msg::Chain).unwrap();
    assert_eq!(
        runtime.app().log,
        ["chain:start", "chain:end", "add"],
        "the posted message must run after the current update returns"
    );
    assert!(runtime.export_dom().unwrap().contains("n=1"));
}

#[test]
fn wake_runs_the_app_hook_and_rebuilds() {
    let mut runtime = Runtime::new(Counter::default()).unwrap();
    runtime.wake().unwrap();
    assert_eq!(runtime.app().woken, 1);
    assert!(runtime.export_dom().unwrap().contains("woken=1"));
}

#[test]
fn a_posted_message_waits_for_flush_and_wake_applies_it() {
    let mut runtime = Runtime::new(Counter::default()).unwrap();
    runtime.queue().post(Msg::Add(2));
    assert_eq!(runtime.app().n, 0);
    runtime.wake().unwrap();
    assert_eq!(runtime.app().n, 2);
}
