//! App-level keyboard shortcuts (issue #119). Headless: the hosts call
//! `Runtime::key` first, then `Runtime::shortcut`.
use surface::{Node, Role};
use surface_app::{
    Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, Shortcut, View,
};

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Text(String),
    Palette,
    PaletteShift,
    PlainK,
    SelectAll,
    Close,
    Handled,
}

#[derive(Default)]
struct App {
    draft: String,
    log: Vec<Msg>,
    /// `on_key` on the text field: consumes Cmd+K with `Handled` when true.
    consume: bool,
    /// `shortcuts()` returns the table when true, nothing when false.
    table: bool,
}
impl Application for App {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        self.log.push(message.clone());
        if let Msg::Text(text) = message {
            self.draft = text;
        }
    }
    fn shortcuts(&self) -> Vec<Shortcut<Msg>> {
        // Escape closes the "modal": the table is empty afterwards.
        if !self.table || self.log.contains(&Msg::Close) {
            return Vec::new();
        }
        vec![
            Shortcut::new(Key::Character("k".into()), cmd(), Msg::Palette),
            Shortcut::new(
                Key::Character("k".into()),
                Modifiers {
                    shift: true,
                    ..cmd()
                },
                Msg::PaletteShift,
            ),
            Shortcut::new(Key::Character("k".into()), none(), Msg::PlainK),
            Shortcut::new(Key::Character("a".into()), cmd(), Msg::SelectAll),
            Shortcut::new(Key::Escape, none(), Msg::Close),
        ]
    }
    fn view(&self) -> View<Msg> {
        let field = View::new(
            Node::new(Role::TextInput, "Message")
                .id("chat")
                .text(&self.draft),
        )
        .on_input(Msg::Text);
        let field = if self.consume {
            field.on_key(|press| {
                (press.key == Key::Character("k".into()) && press.modifiers == cmd())
                    .then_some(Msg::Handled)
            })
        } else {
            field
        };
        View::new(Node::new(Role::Window, "App")).children([
            field,
            View::new(Node::new(Role::Button, "Other").id("other")),
        ])
    }
}

fn none() -> Modifiers {
    Modifiers::default()
}
fn cmd() -> Modifiers {
    Modifiers {
        meta: true,
        ..none()
    }
}
fn ctrl() -> Modifiers {
    Modifiers {
        control: true,
        ..none()
    }
}
fn chord(c: &str, modifiers: Modifiers) -> KeyPress {
    KeyPress::new(Key::Character(c.into()), modifiers)
}
fn runtime(consume: bool, focus: Option<&str>) -> Runtime<App> {
    let mut rt = Runtime::new(App {
        consume,
        table: true,
        ..App::default()
    })
    .unwrap();
    if let Some(id) = focus {
        rt.activate(id).unwrap();
    }
    rt
}
/// What the router does: `key`, then `shortcut` when `key` passes.
fn route(rt: &mut Runtime<App>, press: &KeyPress) -> bool {
    rt.key(press).unwrap() || rt.shortcut(press).unwrap()
}
fn messages(rt: &Runtime<App>) -> Vec<Msg> {
    rt.app().log.clone()
}

#[test]
fn cmd_k_fires_on_a_button_on_a_field_and_on_nothing() {
    for focus in [Some("other"), Some("chat"), None] {
        let mut rt = runtime(false, focus);
        assert!(rt.shortcut(&chord("k", cmd())).unwrap(), "{focus:?}");
        assert_eq!(messages(&rt), [Msg::Palette], "{focus:?}");
    }
}

#[test]
fn ctrl_k_and_shift_case_match_but_extra_modifiers_do_not() {
    let mut rt = runtime(false, None);
    // winit reports "K" with Shift held; Cmd+Shift+K is its own entry.
    assert!(rt.shortcut(&chord("K", cmd())).unwrap());
    assert_eq!(messages(&rt), [Msg::Palette]);
    let shifted = Modifiers {
        shift: true,
        ..cmd()
    };
    assert!(rt.shortcut(&chord("K", shifted)).unwrap());
    assert_eq!(messages(&rt), [Msg::Palette, Msg::PaletteShift]);
    // Cmd+Ctrl+K matches nothing.
    let both = Modifiers {
        control: true,
        ..cmd()
    };
    assert!(!rt.shortcut(&chord("k", both)).unwrap());
    // Ctrl+K alone matches nothing (the table has Cmd+K).
    assert!(!rt.shortcut(&chord("k", ctrl())).unwrap());
    assert_eq!(messages(&rt).len(), 2);
}

#[test]
fn unbound_keys_send_nothing() {
    let mut rt = runtime(false, None);
    assert!(!rt.shortcut(&chord("j", cmd())).unwrap());
    assert!(!rt.shortcut(&KeyPress::new(Key::Enter, none())).unwrap());
    assert!(messages(&rt).is_empty());
}

#[test]
fn a_plain_entry_fires_with_nothing_focused_and_on_a_button() {
    for focus in [None, Some("other")] {
        let mut rt = runtime(false, focus);
        assert!(rt.shortcut(&chord("k", none())).unwrap());
        assert_eq!(messages(&rt), [Msg::PlainK]);
    }
}

#[test]
fn a_plain_key_in_a_text_field_is_never_a_shortcut() {
    let mut rt = runtime(false, Some("chat"));
    assert!(!rt.shortcut(&chord("k", none())).unwrap());
    assert!(!rt.shortcut(&KeyPress::new(Key::Escape, none())).unwrap());
    assert!(messages(&rt).is_empty());
}

#[test]
fn field_edit_chords_belong_to_the_field() {
    let mut rt = runtime(false, Some("chat"));
    for m in [cmd(), ctrl()] {
        for c in ["a", "c", "x", "v", "z", "y", "A", "Z"] {
            assert!(!rt.shortcut(&chord(c, m)).unwrap(), "{c}");
        }
    }
    let shifted = Modifiers {
        shift: true,
        ..cmd()
    };
    assert!(!rt.shortcut(&chord("Z", shifted)).unwrap());
    assert!(messages(&rt).is_empty());
    // Outside a text field Cmd+A is the app's.
    let mut rt = runtime(false, Some("other"));
    assert!(rt.shortcut(&chord("a", cmd())).unwrap());
    assert_eq!(messages(&rt), [Msg::SelectAll]);
}

#[test]
fn a_consumed_on_key_wins_over_the_shortcut() {
    let mut rt = runtime(true, Some("chat"));
    assert!(route(&mut rt, &chord("k", cmd())));
    assert_eq!(messages(&rt), [Msg::Handled]);
}

#[test]
fn a_passing_on_key_falls_through_to_the_shortcut() {
    let mut rt = runtime(true, Some("chat"));
    let shifted = Modifiers {
        shift: true,
        ..cmd()
    };
    assert!(route(&mut rt, &chord("k", shifted)));
    assert_eq!(messages(&rt), [Msg::PaletteShift]);
}

#[test]
fn no_shortcut_fires_during_a_composition_and_one_fires_after_commit() {
    let mut rt = runtime(false, Some("chat"));
    rt.dispatch(Event::Ime(ImeEvent::Preedit {
        text: "ㄓ".into(),
        selection: None,
    }))
    .unwrap();
    assert!(rt.input_snapshot().unwrap().composing);
    assert!(!rt.shortcut(&chord("k", cmd())).unwrap());
    assert!(messages(&rt).is_empty());
    rt.dispatch(Event::Ime(ImeEvent::Commit("中".into())))
        .unwrap();
    let before = messages(&rt).len();
    assert!(rt.shortcut(&chord("k", cmd())).unwrap());
    assert_eq!(messages(&rt)[before..], [Msg::Palette]);
}

#[test]
fn a_repeated_chord_is_consumed_without_a_message() {
    let mut rt = runtime(false, None);
    let held = chord("k", cmd()).with_repeat(true);
    assert!(held.repeat);
    assert!(rt.shortcut(&held).unwrap());
    assert!(messages(&rt).is_empty());
    assert!(rt.shortcut(&chord("k", cmd())).unwrap());
    assert_eq!(messages(&rt), [Msg::Palette]);
    // An unbound repeat is not consumed.
    assert!(!rt.shortcut(&chord("j", cmd()).with_repeat(true)).unwrap());
}

#[test]
fn on_key_sees_the_repeat_flag() {
    use std::{cell::RefCell, rc::Rc};
    #[derive(Clone)]
    struct Seen(Rc<RefCell<Vec<bool>>>);
    struct Probe(Seen);
    impl Application for Probe {
        type Message = ();
        fn update(&mut self, _: ()) {}
        fn view(&self) -> View<()> {
            let seen = (self.0).0.clone();
            View::new(Node::new(Role::TextInput, "F").id("f"))
                .on_input(|_| ())
                .on_key(move |p| {
                    seen.borrow_mut().push(p.repeat);
                    None
                })
        }
    }
    let seen = Seen(Rc::default());
    let mut rt = Runtime::new(Probe(seen.clone())).unwrap();
    rt.activate("f").unwrap();
    rt.key(&chord("a", none())).unwrap();
    rt.key(&chord("a", none()).with_repeat(true)).unwrap();
    assert_eq!(*seen.0.borrow(), [false, true]);
}

#[test]
fn a_new_press_has_repeat_false() {
    assert!(!KeyPress::new(Key::Enter, none()).repeat);
}

#[test]
fn an_app_with_no_shortcuts_override_matches_nothing() {
    struct Plain;
    impl Application for Plain {
        type Message = ();
        fn update(&mut self, _: ()) {}
        fn view(&self) -> View<()> {
            View::new(Node::new(Role::Window, "W"))
        }
    }
    let mut rt = Runtime::new(Plain).unwrap();
    for press in [
        chord("k", cmd()),
        chord("k", none()),
        KeyPress::new(Key::Escape, none()),
    ] {
        assert!(!rt.shortcut(&press).unwrap());
    }
}

#[test]
fn a_shortcut_reaches_update_once_and_the_view_rebuilds() {
    let mut rt = runtime(false, None);
    rt.shortcut(&chord("k", cmd())).unwrap();
    assert_eq!(messages(&rt), [Msg::Palette]);
    // The table is read per key: after Close it is empty.
    assert!(rt.shortcut(&KeyPress::new(Key::Escape, none())).unwrap());
    assert!(!rt.shortcut(&chord("k", cmd())).unwrap());
    assert_eq!(messages(&rt), [Msg::Palette, Msg::Close]);
}
