use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Key, KeyPress, Modifiers, Roving, Runtime, View};

#[derive(Clone)]
enum Message {
    Press,
    Custom,
}

#[derive(Default)]
struct App {
    presses: usize,
    custom_calls: usize,
    custom_handler: bool,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press => self.presses += 1,
            Message::Custom => self.custom_calls += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let control = |role, id| {
            View::new(
                Node::new(role, id)
                    .id(id)
                    .styled(Style::column().height(Length::Px(40.))),
            )
        };
        let mut guarded = control(Role::Checkbox, "guarded")
            .on_press(Message::Press)
            .accessible_activation()
            .suppress_repeated_activation();
        if self.custom_handler {
            guarded = guarded.on_key(|press| {
                (press.key == Key::Space && press.repeat).then_some(Message::Custom)
            });
        }
        let choices = View::new(Node::new(Role::Group, "Choices").id("choices"))
            .children([
                guarded,
                control(Role::Button, "legacy").on_press(Message::Press),
            ])
            .roving(Roving::vertical(false));
        View::new(Node::new(Role::Window, "Test")).children([
            choices,
            control(Role::Button, "without-press").suppress_repeated_activation(),
            control(Role::TextInput, "editor")
                .on_input(|_| Message::Custom)
                .on_press(Message::Press)
                .suppress_repeated_activation(),
        ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn fixture(custom_handler: bool) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        custom_handler,
        ..App::default()
    })
    .unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("guarded"));
    runtime
}

fn key(key: Key) -> KeyPress {
    KeyPress::new(key, Modifiers::default())
}

fn select_next() -> Event {
    Event::Edit {
        key: EditKey::Down,
        extend: false,
        by_word: false,
    }
}

#[test]
fn normal_key_route_keeps_one_press_per_hold_and_leaves_legacy_and_arrows_alone() {
    for activation in [Key::Enter, Key::Space] {
        let mut runtime = fixture(false);
        let press = key(activation);
        assert!(!runtime.key(&press).unwrap());
        assert!(runtime.dispatch(Event::Activate).unwrap());
        let repeat = press.with_repeat(true);
        assert!(runtime.key(&repeat).unwrap());
        assert!(runtime.key(&repeat).unwrap());
        assert_eq!(runtime.app().presses, 1);
        assert_eq!(runtime.focused(), Some("guarded"));

        assert!(!runtime.key(&key(Key::ArrowDown).with_repeat(true)).unwrap());
        assert!(runtime.dispatch(select_next()).unwrap());
        assert_eq!(runtime.focused(), Some("legacy"));
        assert!(!runtime.key(&repeat).unwrap());
        assert!(runtime.dispatch(Event::Activate).unwrap());
        assert_eq!(runtime.app().presses, 2);
    }
}

#[test]
fn projected_route_consumes_repeated_activation_with_a_current_target() {
    let mut runtime = fixture(false);
    let press = key(Key::Space);
    for repeat in [false, true, true] {
        runtime.render(240., 200., &mut Metrics).unwrap();
        let node = runtime.dom().by_id("guarded").unwrap();
        let target = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(node)
            .unwrap();
        assert!(runtime
            .accessibility_key(&target, &press.clone().with_repeat(repeat), Event::Activate)
            .unwrap());
        assert_eq!(runtime.app().presses, 1);
        assert_eq!(runtime.focused(), Some("guarded"));
    }
}

#[test]
fn explicit_handlers_win_and_other_keys_and_controls_pass() {
    let mut runtime = fixture(true);
    assert!(runtime.key(&key(Key::Space).with_repeat(true)).unwrap());
    assert_eq!(runtime.app().custom_calls, 1);
    assert_eq!(runtime.app().presses, 0);
    let chord = KeyPress::new(
        Key::Enter,
        Modifiers {
            meta: true,
            ..Modifiers::default()
        },
    )
    .with_repeat(true);
    assert!(!runtime.key(&chord).unwrap());
    assert!(!runtime.key(&key(Key::Escape).with_repeat(true)).unwrap());

    let repeat = key(Key::Enter).with_repeat(true);
    for target in ["without-press", "editor"] {
        runtime.activate(target).unwrap();
        assert!(!runtime.key(&repeat).unwrap(), "{target}");
    }
}
