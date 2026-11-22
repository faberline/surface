use std::{cell::RefCell, rc::Rc};
use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult as Outcome, AccessibilityActivationTarget, Application, Event,
    ImeEvent, Queue, Runtime, ScrollTarget, View,
};

#[derive(Clone, Copy, Default)]
enum OnClone {
    #[default]
    Idle,
    Replace,
    Close,
    Scroll,
}

type SharedQueue = Rc<RefCell<Option<Queue<Message>>>>;

enum Message {
    Press(SharedQueue, OnClone),
    Open(bool),
    Replace,
    Edit(String),
}

impl Clone for Message {
    fn clone(&self) -> Self {
        match self {
            Self::Press(queue, effect) => {
                if let Some(queue) = queue.borrow().as_ref() {
                    match effect {
                        OnClone::Idle => {}
                        OnClone::Replace => queue.post(Self::Replace),
                        OnClone::Close => queue.close(),
                        OnClone::Scroll => queue.scroll_to("root", ScrollTarget::Top),
                    }
                }
                Self::Press(queue.clone(), *effect)
            }
            Self::Open(open) => Self::Open(*open),
            Self::Replace => Self::Replace,
            Self::Edit(text) => Self::Edit(text.clone()),
        }
    }
}

#[derive(Default)]
struct App {
    queue: SharedQueue,
    effect: OnClone,
    modal: bool,
    replaced: bool,
    presses: usize,
    text: String,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press(_, _) => self.presses += 1,
            Message::Open(open) => self.modal = open,
            Message::Replace => self.replaced = true,
            Message::Edit(text) => self.text = text,
        }
    }

    fn view(&self) -> View<Message> {
        let press = || Message::Press(self.queue.clone(), self.effect);
        let mut style = Style::default().height(Length::Px(40.));
        style.min_height = 40.;
        let button =
            |id, label| View::new(Node::new(Role::Button, label).id(id).styled(style.clone()));
        let mut children = vec![
            View::new(
                Node::new(Role::TextInput, "Field")
                    .id("field")
                    .text(&self.text)
                    .styled(style.clone()),
            )
            .on_input(Message::Edit)
            .on_press(press())
            .accessible_activation(),
            button(
                "button",
                if self.replaced {
                    "Replacement"
                } else {
                    "Original"
                },
            )
            .on_press(press())
            .accessible_activation(),
            button("legacy", "Legacy").on_press(press()),
            button("no-message", "No press").accessible_activation(),
            View::new(Node::new(Role::Group, "Disabled group").disabled(true)).children([button(
                "disabled", "Disabled",
            )
            .on_press(press())
            .accessible_activation()]),
        ];
        if self.modal {
            children.push(
                View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                    .modal(Message::Open(false))
                    .children([button("close", "Close")
                        .on_press(Message::Open(false))
                        .accessible_activation()]),
            );
        }
        View::new(
            Node::new(Role::Window, "Test")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children(children)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}

fn runtime(effect: OnClone) -> Runtime<App> {
    let mut rt = Runtime::new(App {
        effect,
        ..App::default()
    })
    .unwrap();
    rt.app().queue.replace(Some(rt.queue()));
    render(&mut rt, 340.);
    rt
}

fn render(rt: &mut Runtime<App>, height: f32) {
    rt.render(240., height, &mut Metrics).unwrap();
}

fn target(rt: &Runtime<App>, stable_id: &str) -> Option<AccessibilityActivationTarget> {
    let id = rt.dom().by_id(stable_id).unwrap();
    rt.accessibility_frame()?.activation_target(id)
}

#[test]
fn activation_requires_opt_in_enabled_press_role_and_a_message() {
    let mut rt = runtime(OnClone::Idle);
    for id in ["legacy", "disabled", "no-message", "field"] {
        assert!(target(&rt, id).is_none(), "{id}");
    }
    let action = target(&rt, "button").unwrap();
    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        Outcome::Activated
    );
    assert_eq!(rt.focused(), Some("button"));
    assert_eq!(rt.app().presses, 1);
    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        Outcome::Rejected
    );
    assert_eq!(rt.app().presses, 1);
    rt.activate("legacy").unwrap();
    assert_eq!(rt.app().presses, 2);
}

#[test]
fn old_activation_cannot_cross_runtimes_replacements_or_modals() {
    let mut rt = runtime(OnClone::Idle);
    let previous_id = rt.dom().by_id("button");
    let action = target(&rt, "button").unwrap();
    let mut other = runtime(OnClone::Idle);
    assert_ne!(target(&other, "button").unwrap(), action);
    assert_eq!(
        other.accessibility_activate(&action).unwrap(),
        Outcome::Rejected
    );

    rt.apply(Message::Replace).unwrap();
    render(&mut rt, 340.);
    assert_eq!(rt.dom().by_id("button"), previous_id);
    let replacement = target(&rt, "button").unwrap();
    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        Outcome::Rejected
    );

    rt.apply(Message::Open(true)).unwrap();
    render(&mut rt, 340.);
    assert!(target(&rt, "button").is_none());
    assert_eq!(
        rt.accessibility_activate(&replacement).unwrap(),
        Outcome::Rejected
    );
    let close = target(&rt, "close").unwrap();
    assert_eq!(
        rt.accessibility_activate(&close).unwrap(),
        Outcome::Activated
    );
    assert!(!rt.app().modal);
    assert_eq!(rt.app().presses, 0);
}

#[test]
fn clipping_retires_a_target_even_when_the_control_returns() {
    let mut rt = runtime(OnClone::Idle);
    let action = target(&rt, "button").unwrap();
    render(&mut rt, 35.);
    assert!(target(&rt, "button").is_none());
    render(&mut rt, 340.);
    assert_ne!(target(&rt, "button").unwrap(), action);
    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        Outcome::Rejected
    );
}

#[test]
fn queued_updates_before_or_during_clone_do_not_run_the_old_press() {
    for effect in [OnClone::Idle, OnClone::Replace] {
        let mut rt = runtime(effect);
        let action = target(&rt, "button").unwrap();
        if matches!(effect, OnClone::Idle) {
            rt.queue().post(Message::Replace);
        }
        assert_eq!(
            rt.accessibility_activate(&action).unwrap(),
            Outcome::Rebuilt
        );
        assert!(rt.app().replaced);
        assert_eq!(rt.app().presses, 0);
        assert!(rt.focused().is_none());
        render(&mut rt, 340.);
        assert_eq!(
            rt.accessibility_activate(&action).unwrap(),
            Outcome::Rejected
        );
    }
}

#[test]
fn clone_cannot_press_while_close_or_scroll_is_pending() {
    for effect in [OnClone::Close, OnClone::Scroll] {
        let mut rt = runtime(effect);
        let action = target(&rt, "button").unwrap();
        assert_eq!(
            rt.accessibility_activate(&action).unwrap(),
            Outcome::Rejected
        );
        assert_eq!(rt.app().presses, 0);
        assert!(rt.focused().is_none());
        assert_eq!(rt.take_close(), matches!(effect, OnClone::Close));
    }
}

#[test]
fn activation_preserves_ime_and_does_not_interrupt_a_pointer_press() {
    let mut rt = runtime(OnClone::Idle);
    rt.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rt.focused(), Some("field"));
    rt.dispatch(Event::Text("原本".into())).unwrap();
    rt.dispatch(Event::Select {
        anchor: 0,
        cursor: "原".len(),
    })
    .unwrap();
    render(&mut rt, 340.);
    let action = target(&rt, "button").unwrap();
    for text in ["ㄒ", ""] {
        rt.dispatch(Event::Ime(ImeEvent::Preedit {
            text: text.into(),
            selection: None,
        }))
        .unwrap();
        assert_eq!(
            rt.accessibility_activate(&action).unwrap(),
            Outcome::BlockedByComposition
        );
    }
    rt.queue().post(Message::Replace);
    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        Outcome::BlockedByComposition
    );
    assert!(!rt.app().replaced);
    rt.dispatch(Event::Ime(ImeEvent::Commit("新".into())))
        .unwrap();
    assert_eq!(rt.app().text, "新本");
    render(&mut rt, 340.);
    let action = target(&rt, "button").unwrap();

    rt.dispatch(Event::PointerDown { x: 5., y: 5. }).unwrap();
    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        Outcome::BlockedByPointer
    );
    assert_eq!(rt.focused(), Some("field"));
    rt.dispatch(Event::PointerUp { x: 230., y: 330. }).unwrap();
    assert_eq!(rt.app().presses, 0);
}
