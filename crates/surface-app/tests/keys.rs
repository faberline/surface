//! App key handler for text fields (issue #103). Headless: the hosts call
//! `Runtime::key` first and run the field's own key handling only when it
//! returns `false`.
use std::{cell::Cell, rc::Rc};
use surface::{Node, Role};
use surface_app::{
    Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, TextAction, TextOptions, View,
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Text(String),
    Send,
    MenuUp,
    MenuDown,
    MenuPick,
    MenuClose,
}

#[derive(Default)]
struct Chat {
    draft: String,
    sent: Vec<String>,
    menu: Option<usize>,
    log: Vec<Msg>,
    /// No `on_key` at all when false.
    handler: bool,
    calls: Rc<Cell<u32>>,
}
impl Chat {
    fn new(handler: bool) -> Self {
        Self {
            handler,
            ..Self::default()
        }
    }
}
impl Application for Chat {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        self.log.push(message.clone());
        match message {
            Msg::Text(text) => {
                self.menu = text.starts_with('/').then_some(0);
                self.draft = text;
            }
            // A real app would also clear the draft. Keeping it shows that
            // the consumed Enter inserted nothing.
            Msg::Send => self.sent.push(self.draft.clone()),
            Msg::MenuUp => self.menu = self.menu.map(|i| i.saturating_sub(1)),
            Msg::MenuDown => self.menu = self.menu.map(|i| i + 1),
            Msg::MenuPick | Msg::MenuClose => self.menu = None,
        }
    }
    fn view(&self) -> View<Msg> {
        let field = View::new(
            Node::new(Role::TextInput, "Message")
                .id("chat")
                .text(&self.draft),
        )
        .text_options(TextOptions {
            multiline: true,
            ..TextOptions::default()
        })
        .on_input(Msg::Text);
        let field = if self.handler {
            let menu_open = self.menu.is_some();
            let calls = self.calls.clone();
            field.on_key(move |press| {
                calls.set(calls.get() + 1);
                if menu_open && press.modifiers.is_empty() {
                    return match press.key {
                        Key::ArrowUp => Some(Msg::MenuUp),
                        Key::ArrowDown => Some(Msg::MenuDown),
                        Key::Enter => Some(Msg::MenuPick),
                        Key::Escape => Some(Msg::MenuClose),
                        _ => None,
                    };
                }
                match press.text_action() {
                    Some(TextAction::Submit) => Some(Msg::Send),
                    _ => None,
                }
            })
        } else {
            field
        };
        View::new(Node::new(Role::Window, "Chat")).children([
            field,
            View::new(Node::new(Role::Button, "Other").id("other")),
        ])
    }
}

fn press(key: Key) -> KeyPress {
    KeyPress::new(key, Modifiers::default())
}
fn shift(key: Key) -> KeyPress {
    KeyPress::new(
        key,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    )
}
fn focused(handler: bool, text: &str) -> Runtime<Chat> {
    let mut rt = Runtime::new(Chat::new(handler)).unwrap();
    rt.activate("chat").unwrap();
    rt.dispatch(Event::Text(text.into())).unwrap();
    assert_eq!(rt.app().draft, text);
    rt
}
/// What a host does after `Runtime::key` passes Enter in a multiline field.
fn field_enter(rt: &mut Runtime<Chat>, press: &KeyPress) {
    if !rt.key(press).unwrap() {
        rt.dispatch(Event::Text("\n".into())).unwrap();
    }
}

#[test]
fn enter_reaches_the_handler_and_a_consumed_enter_inserts_no_newline() {
    let mut rt = focused(true, "hi");
    field_enter(&mut rt, &press(Key::Enter));
    assert_eq!(rt.app().sent, ["hi"]);
    assert_eq!(rt.app().draft, "hi", "no newline after a consumed Enter");
    assert_eq!(rt.input_snapshot().unwrap().value, "hi");
    assert_eq!(rt.app().calls.get(), 1);
}

#[test]
fn shift_enter_passes_to_the_field_and_inserts_a_newline() {
    let mut rt = focused(true, "hi");
    field_enter(&mut rt, &shift(Key::Enter));
    assert!(rt.app().sent.is_empty());
    assert_eq!(rt.app().draft, "hi\n");
    assert_eq!(rt.app().calls.get(), 1, "the handler saw Shift+Enter");
}

#[test]
fn default_mappings_are_submit_for_enter_and_newline_for_shift_enter() {
    assert_eq!(press(Key::Enter).text_action(), Some(TextAction::Submit));
    assert_eq!(shift(Key::Enter).text_action(), Some(TextAction::NewLine));
    for modifiers in [
        Modifiers {
            meta: true,
            ..Modifiers::default()
        },
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
        Modifiers {
            alt: true,
            ..Modifiers::default()
        },
        Modifiers {
            shift: true,
            meta: true,
            ..Modifiers::default()
        },
    ] {
        assert_eq!(KeyPress::new(Key::Enter, modifiers).text_action(), None);
    }
    assert_eq!(press(Key::Escape).text_action(), None);
    assert_eq!(press(Key::Character("a".into())).text_action(), None);
}

#[test]
fn an_open_menu_consumes_up_down_enter_and_escape_without_changing_the_field() {
    let mut rt = focused(true, "/he");
    assert_eq!(rt.app().menu, Some(0));
    let before = rt.input_snapshot().unwrap();
    for key in [Key::ArrowDown, Key::ArrowDown, Key::ArrowUp] {
        assert!(rt.key(&press(key)).unwrap());
    }
    assert_eq!(rt.app().menu, Some(1));
    assert!(rt.key(&press(Key::Enter)).unwrap());
    assert_eq!(rt.app().menu, None);
    assert!(rt.app().sent.is_empty(), "Enter picked, it did not send");
    // Open the menu again, then close it with Escape.
    rt.dispatch(Event::Text("x".into())).unwrap();
    rt.dispatch(Event::InputValue {
        value: "/he".into(),
        anchor: 3,
        cursor: 3,
    })
    .unwrap();
    assert_eq!(rt.app().menu, Some(0));
    assert!(rt.key(&press(Key::Escape)).unwrap());
    assert_eq!(rt.app().menu, None);
    let after = rt.input_snapshot().unwrap();
    assert_eq!(after.value, before.value);
    assert_eq!(after.selection, before.selection);
    assert_eq!(rt.focused(), Some("chat"), "Escape did not blur the field");
}

#[test]
fn enter_during_a_composition_never_calls_the_handler() {
    let mut rt = focused(true, "hi");
    rt.dispatch(Event::Ime(ImeEvent::Preedit {
        text: "ㄓㄨㄥ".into(),
        selection: None,
    }))
    .unwrap();
    assert!(rt.input_snapshot().unwrap().composing);
    assert!(!rt.key(&press(Key::Enter)).unwrap());
    assert_eq!(rt.app().calls.get(), 0, "not called during composition");
    assert!(rt.app().sent.is_empty());
    // The IME commits the candidate. The next Enter sends.
    rt.dispatch(Event::Ime(ImeEvent::Commit("中".into())))
        .unwrap();
    assert_eq!(rt.app().draft, "hi中");
    assert!(rt.key(&press(Key::Enter)).unwrap());
    assert_eq!(rt.app().sent, ["hi中"]);
}

#[test]
fn with_no_handler_key_passes_and_the_field_behaves_as_before() {
    let mut rt = focused(false, "hi");
    let log = rt.app().log.len();
    for key in [Key::Enter, Key::Escape, Key::ArrowUp] {
        assert!(!rt.key(&press(key)).unwrap());
    }
    assert_eq!(rt.app().log.len(), log, "no message ran");
    field_enter(&mut rt, &press(Key::Enter));
    assert_eq!(rt.app().draft, "hi\n");
}

#[test]
fn a_handler_on_an_unfocused_field_is_not_called() {
    let mut rt = Runtime::new(Chat::new(true)).unwrap();
    assert!(!rt.key(&press(Key::Enter)).unwrap(), "nothing focused");
    rt.activate("other").unwrap();
    assert!(!rt.key(&press(Key::Enter)).unwrap());
    assert_eq!(rt.app().calls.get(), 0);
}

#[test]
fn a_passed_key_runs_no_message() {
    let mut rt = focused(true, "hi");
    let log = rt.app().log.len();
    assert!(!rt.key(&press(Key::Character("a".into()))).unwrap());
    assert_eq!(rt.app().log.len(), log);
    assert_eq!(rt.app().calls.get(), 1);
}
