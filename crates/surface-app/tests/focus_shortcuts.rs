use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, View};

const SHIFT: Modifiers = Modifiers {
    shift: true,
    control: false,
    alt: false,
    meta: false,
};

#[derive(Clone)]
enum Message {
    Close,
    Edit(String),
    Local,
    Press,
}

struct App {
    open: bool,
    local_handler: bool,
    month_target: &'static str,
    text: String,
    local_calls: usize,
    presses: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            open: true,
            local_handler: false,
            month_target: "month",
            text: "原本".to_owned(),
            local_calls: 0,
            presses: 0,
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Close => self.open = false,
            Message::Edit(text) => self.text = text,
            Message::Local => self.local_calls += 1,
            Message::Press => self.presses += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let field = View::new(
            Node::new(Role::TextInput, "Date")
                .id("field")
                .text(&self.text)
                .styled(Style::default().height(Length::Px(24.))),
        )
        .on_input(Message::Edit);
        let outside = button("outside");
        let root = View::new(Node::new(Role::Window, "Fixture"));
        if !self.open {
            return root.children([outside, field]);
        }

        let mut day = button("day");
        if self.local_handler {
            day = day.on_key(|press| (press == &chord("M")).then_some(Message::Local));
        }
        let popup = View::new(
            Node::new(Role::Group, "Calendar")
                .id("popup")
                .styled(Style::column().height(Length::Px(64.)).clip(true)),
        )
        .focus_shortcut_within(Key::Character("m".into()), SHIFT, self.month_target)
        .focus_shortcut_within(Key::Character("Y".into()), SHIFT, "year")
        .children([
            day,
            field,
            View::new(
                Node::new(Role::Group, "Spacer").styled(Style::default().height(Length::Px(96.))),
            ),
            button("month").on_press(Message::Press),
            button("year").on_press(Message::Press),
            View::new(
                Node::new(Role::Button, "Disabled")
                    .id("disabled")
                    .disabled(true),
            ),
            View::new(
                Node::new(Role::Button, "Collapsed")
                    .id("collapsed")
                    .styled(Style::default().height(Length::Px(0.))),
            ),
        ]);
        let dialog = View::new(Node::new(Role::Dialog, "Picker").id("picker"))
            .modal_overlay(Message::Close)
            .modal_initial_focus("day")
            .children([button("opener"), popup]);
        root.focus_shortcut_within(Key::Character("o".into()), SHIFT, "month")
            .children([outside, dialog])
    }
}

fn button(id: &str) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .styled(Style::default().height(Length::Px(24.))),
    )
}

fn chord(letter: &str) -> KeyPress {
    KeyPress::new(Key::Character(letter.into()), SHIFT)
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    runtime.render(240., 240., &mut Metrics).unwrap();
}

#[test]
fn shift_letters_focus_and_reveal_without_pressing_targets() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.focused(), Some("day"));

    for modifiers in [
        Modifiers::default(),
        Modifiers {
            control: true,
            ..SHIFT
        },
        Modifiers { alt: true, ..SHIFT },
        Modifiers {
            meta: true,
            ..SHIFT
        },
    ] {
        assert!(!runtime
            .key(&KeyPress::new(Key::Character("M".into()), modifiers))
            .unwrap());
    }
    assert!(runtime.key(&chord("M").with_repeat(true)).unwrap());
    assert_eq!(runtime.focused(), Some("day"));

    let source = runtime.dom().by_id("day").unwrap();
    let projected = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(source)
        .unwrap();
    assert!(runtime
        .accessibility_key(&projected, &chord("M"), None)
        .unwrap());
    assert_eq!(runtime.focused(), Some("month"));
    draw(&mut runtime);
    assert!(runtime.scroll_metrics("popup").unwrap().offset.1 > 0.);

    assert!(runtime.key(&chord("y")).unwrap());
    assert_eq!(runtime.focused(), Some("year"));
    assert_eq!(runtime.app().presses, 0);
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().presses, 1);
}

#[test]
fn rejects_out_of_scope_sources_and_disabled_or_out_of_subtree_targets() {
    for month_target in ["missing", "disabled", "opener", "outside"] {
        let mut runtime = Runtime::new(App {
            month_target,
            ..App::default()
        })
        .unwrap();
        assert!(!runtime.key(&chord("M")).unwrap(), "target {month_target}");
        assert_eq!(runtime.focused(), Some("day"));
    }
    let mut runtime = Runtime::new(App::default()).unwrap();
    assert!(
        !runtime.key(&chord("O")).unwrap(),
        "outer owner is outside the modal"
    );
    assert_eq!(runtime.focused(), Some("day"));

    runtime.activate("opener").unwrap();
    assert!(
        !runtime.key(&chord("Y")).unwrap(),
        "opener is outside the popup subtree"
    );
    runtime.apply(Message::Close).unwrap();
    runtime.activate("outside").unwrap();
    assert!(!runtime.key(&chord("Y")).unwrap(), "the calendar is closed");
    assert_eq!(runtime.focused(), Some("outside"));
}

#[test]
fn rejects_a_measured_zero_size_target() {
    let mut runtime = Runtime::new(App {
        month_target: "collapsed",
        ..App::default()
    })
    .unwrap();
    draw(&mut runtime);
    let id = runtime.dom().by_id("collapsed").unwrap();
    assert_eq!(
        runtime.computed().unwrap().nodes[id.index()].bounds.height,
        0.
    );
    assert!(!runtime.key(&chord("M")).unwrap());
    assert_eq!(runtime.focused(), Some("day"));
}

#[test]
fn local_handler_wins_and_composition_keeps_its_replacement() {
    let mut runtime = Runtime::new(App {
        local_handler: true,
        ..App::default()
    })
    .unwrap();
    assert!(runtime.key(&chord("M")).unwrap());
    assert_eq!(runtime.focused(), Some("day"));
    assert_eq!(runtime.app().local_calls, 1);

    runtime.activate("field").unwrap();
    assert!(!runtime.key(&chord("Y")).unwrap());
    assert_eq!(runtime.focused(), Some("field"));
    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: "原".len(),
        })
        .unwrap();
    draw(&mut runtime);
    let day = runtime.dom().by_id("day").unwrap();
    let projected = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(day)
        .unwrap();
    for text in ["ㄒ", ""] {
        runtime
            .dispatch(Event::Ime(ImeEvent::Preedit {
                text: text.into(),
                selection: None,
            }))
            .unwrap();
        assert!(!runtime.key(&chord("Y")).unwrap());
        assert!(!runtime
            .accessibility_key(&projected, &chord("Y"), None)
            .unwrap());
        assert_eq!(runtime.focused(), Some("field"));
    }
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("新".into())))
        .unwrap();
    assert_eq!(runtime.app().text, "新本");
    assert_eq!(runtime.app().presses, 0);
}
