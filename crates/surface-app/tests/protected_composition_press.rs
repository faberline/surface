use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, ImeEvent, Runtime, View};

struct App {
    text: String,
    protected: bool,
    disabled: bool,
    activations: usize,
}

#[derive(Clone)]
enum Message {
    Text(String),
    Action,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Text(text) => self.text = text,
            Message::Action => self.activations += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let style = Style::default().height(Length::Px(40.));
        let button = View::new(
            Node::new(Role::Button, "Read")
                .id("reader")
                .disabled(self.disabled)
                .styled(style.clone()),
        )
        .on_press(Message::Action);
        let button = if self.protected {
            button.protect_composition_on_press()
        } else {
            button
        };
        View::new(Node::new(Role::Window, "Test").styled(Style::column())).children([
            View::new(
                Node::new(Role::TextInput, "Message")
                    .id("message")
                    .text(&self.text)
                    .styled(style.clone()),
            )
            .on_input(Message::Text),
            button,
            View::new(
                Node::new(Role::Button, "Unprotected")
                    .id("other")
                    .styled(style),
            )
            .on_press(Message::Action),
        ])
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size)
    }
}

fn start(protected: bool, disabled: bool) -> Runtime<App> {
    let mut rt = Runtime::new(App {
        text: "abc".into(),
        protected,
        disabled,
        activations: 0,
    })
    .unwrap();
    rt.activate("message").unwrap();
    rt.render(240., 240., &mut Metrics).unwrap();
    rt
}

fn begin<A: Application>(rt: &mut Runtime<A>, text: &str) {
    rt.dispatch(Event::Select {
        anchor: 1,
        cursor: 2,
    })
    .unwrap();
    rt.dispatch(Event::Ime(ImeEvent::Preedit {
        text: text.into(),
        selection: None,
    }))
    .unwrap();
    assert!(rt.computed().is_none());
    assert!(rt.editor("message").unwrap().composition().is_some());
}

#[test]
fn primary_and_secondary_before_redraw_leave_protected_preedit_intact() {
    for text in ["ㄓ", ""] {
        for press in [
            Event::PointerDown { x: 20., y: 55. },
            Event::SecondaryDown { x: 20., y: 55. },
        ] {
            let mut rt = start(true, false);
            begin(&mut rt, text);
            assert!(rt.pointer_preserves_composition(20., 55.));
            assert!(!rt.dispatch(press).unwrap());
            assert_eq!(rt.focused(), Some("message"));
            assert!(rt.editor("message").unwrap().composition().is_some());
            assert_eq!(rt.app().activations, 0);
            rt.dispatch(Event::Ime(ImeEvent::Commit("字".into())))
                .unwrap();
            assert_eq!(rt.app().text, "a字c");
        }
    }
}

#[test]
fn prepaint_presses_keep_legacy_cancellation_without_an_eligible_guard() {
    for (protected, disabled) in [(false, false), (true, true)] {
        for press in [
            Event::PointerDown { x: 20., y: 55. },
            Event::SecondaryDown { x: 20., y: 55. },
        ] {
            let mut rt = start(protected, disabled);
            begin(&mut rt, "ㄓ");
            assert!(!rt.pointer_preserves_composition(20., 55.));
            rt.dispatch(press).unwrap();
            assert!(rt.editor("message").unwrap().composition().is_none());
        }
    }
}

#[test]
fn current_hit_geometry_limits_protection_to_the_opted_in_target() {
    let mut rt = start(true, false);
    begin(&mut rt, "ㄓ");
    rt.render(240., 240., &mut Metrics).unwrap();
    assert!(rt.pointer_preserves_composition(20., 55.));
    assert!(!rt.pointer_preserves_composition(20., 95.));
    assert!(!rt.dispatch(Event::PointerDown { x: 20., y: 55. }).unwrap());
    assert!(rt.editor("message").unwrap().composition().is_some());
    rt.dispatch(Event::PointerDown { x: 20., y: 95. }).unwrap();
    assert!(rt.editor("message").unwrap().composition().is_none());
}

#[test]
fn escape_clears_the_pending_range_before_dismissing_a_modal() {
    struct Dialog(bool);
    impl Application for Dialog {
        type Message = ();

        fn update(&mut self, _: ()) {
            self.0 = false;
        }

        fn view(&self) -> View<()> {
            let root = View::new(Node::new(Role::Window, "Test"));
            if !self.0 {
                return root;
            }
            root.children([View::new(Node::new(Role::Dialog, "Edit").id("modal"))
                .modal(())
                .children([View::new(
                    Node::new(Role::TextInput, "Message")
                        .id("message")
                        .text("abc"),
                )
                .on_input(|_| ())])])
        }
    }

    let mut rt = Runtime::new(Dialog(true)).unwrap();
    assert_eq!(rt.focused(), Some("message"));
    begin(&mut rt, "");
    assert!(!rt.editor("message").unwrap().is_composing());
    assert!(rt.dispatch(Event::Cancel).unwrap());
    assert!(
        rt.app().0,
        "the first Escape cancels only the pending replacement"
    );
    assert!(rt.editor("message").unwrap().composition().is_none());
    assert!(rt.dispatch(Event::Cancel).unwrap());
    assert!(!rt.app().0, "the second Escape dismisses the modal");
}
