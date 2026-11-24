use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    Application, EditKey, Event, FocusNavigation, ImeEvent, Key, KeyPress, Modifiers, Runtime, View,
};

#[derive(Clone, Copy, Default)]
enum Destination {
    #[default]
    Enabled,
    Disabled,
    OutsideModal,
}

#[derive(Clone)]
enum Message {
    Mount,
    Press,
    Close,
}

#[derive(Default)]
struct App {
    destination: Destination,
    mounted: bool,
    modal: bool,
    presses: usize,
    year_step: bool,
    key_step: bool,
    key_first: bool,
    editor_arrow_binding: Option<bool>,
    preferred_modal_focus: Option<&'static str>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Mount => {
                self.mounted = true;
                self.modal = matches!(self.destination, Destination::OutsideModal);
            }
            Message::Press => self.presses += 1,
            Message::Close => self.modal = false,
        }
    }

    fn view(&self) -> View<Message> {
        let navigation = || FocusNavigation {
            target: "next".into(),
            message: Message::Mount,
        };
        let start = if let Some(opted_in) = self.editor_arrow_binding {
            let editor = View::new(
                Node::new(Role::TextInput, "Search")
                    .id("start")
                    .styled(Style::default().height(Length::Px(24.))),
            )
            .on_input(|_| Message::Press);
            let arrow = move |press: &KeyPress| (press.key == Key::ArrowDown).then(navigation);
            if opted_in {
                editor.on_text_input_arrow_navigation(arrow)
            } else {
                editor.on_key_focus_navigation(arrow)
            }
        } else if self.key_step {
            let key_first = self.key_first;
            button("start")
                .on_key(move |press| {
                    (key_first && press.key == Key::Enter && press.modifiers.is_empty())
                        .then_some(Message::Press)
                })
                .on_key_focus_navigation(move |press| {
                    (press.key == Key::Enter && press.modifiers.is_empty()).then(navigation)
                })
        } else if self.year_step {
            button("start").on_focus_navigation_with_extend(move |key, extend| {
                (key == EditKey::PageDown && extend).then(navigation)
            })
        } else {
            button("start").on_focus_navigation(move |key| (key == EditKey::Right).then(navigation))
        };
        let next = self.mounted.then(|| {
            [
                View::new(
                    Node::new(Role::Group, "Spacer")
                        .styled(Style::default().height(Length::Px(100.))),
                ),
                View::new(
                    Node::new(Role::Button, "Next")
                        .id("next")
                        .disabled(matches!(self.destination, Destination::Disabled))
                        .styled(Style::default().height(Length::Px(24.))),
                )
                .on_press(Message::Press),
            ]
        });
        let body = View::new(
            Node::new(Role::Group, "Viewport")
                .id("body")
                .styled(Style::column().height(Length::Px(64.)).clip(true)),
        )
        .children(std::iter::once(start).chain(next.into_iter().flatten()));
        let modal = self.modal.then(|| {
            let panel = View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                .children([
                    button("close").on_press(Message::Close),
                    button("preferred").on_press(Message::Press),
                    View::new(
                        Node::new(Role::Button, "Disabled")
                            .id("disabled")
                            .disabled(true),
                    ),
                ])
                .modal_overlay(Message::Close);
            if let Some(target) = self.preferred_modal_focus {
                panel.modal_initial_focus(target)
            } else {
                panel
            }
        });
        View::new(Node::new(Role::Window, "Test")).children(std::iter::once(body).chain(modal))
    }
}

fn button(id: &str) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .styled(Style::default().height(Length::Px(24.))),
    )
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(200., 150., &mut Metrics).unwrap();
}

fn runtime(destination: Destination) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        destination,
        ..App::default()
    })
    .unwrap();
    runtime.activate("start").unwrap();
    render(&mut runtime);
    runtime
}

fn right(runtime: &mut Runtime<App>) -> bool {
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Right,
            extend: false,
            by_word: false,
        })
        .unwrap()
}

fn key_runtime(destination: Destination, key_first: bool) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        destination,
        key_step: true,
        key_first,
        ..App::default()
    })
    .unwrap();
    runtime.activate("start").unwrap();
    render(&mut runtime);
    runtime
}

fn enter() -> KeyPress {
    KeyPress::new(Key::Enter, Modifiers::default())
}

fn editor_runtime(destination: Destination, opted_in: bool) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        destination,
        editor_arrow_binding: Some(opted_in),
        ..App::default()
    })
    .unwrap();
    render(&mut runtime);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("start"));
    runtime
}

#[test]
fn editor_arrow_moves_to_result_without_consuming_editing_and_ime_keys() {
    let down = KeyPress::new(Key::ArrowDown, Modifiers::default());
    let shifted = KeyPress::new(
        Key::ArrowDown,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    let mut legacy = editor_runtime(Destination::Enabled, false);
    assert!(!legacy.key(&down).unwrap());
    assert!(!legacy.app().mounted);
    let mut runtime = editor_runtime(Destination::Enabled, true);
    assert!(!runtime.key(&shifted).unwrap());
    assert!(runtime.key(&down.clone().with_repeat(true)).unwrap());
    assert!(!runtime.app().mounted);
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄅ".into(),
            selection: Some((3, 3)),
        }))
        .unwrap();
    assert!(!runtime.key(&down).unwrap());
    assert!(runtime.editor("start").unwrap().is_composing());
    assert_eq!(runtime.focused(), Some("start"));
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    assert!(runtime.key(&down).unwrap());
    assert_eq!(runtime.focused(), Some("next"));
    assert_eq!(runtime.app().presses, 0);
    render(&mut runtime);
    assert!(runtime.scroll_metrics("body").unwrap().offset.1 > 0.);
}

#[cfg(feature = "devtools")]
#[test]
fn headless_arrow_uses_the_same_opted_in_editor_route() {
    use serde_json::json;
    use surface_devtools_protocol::{Request, VERSION};

    let mut runtime = editor_runtime(Destination::Enabled, true);
    let reply = runtime.execute_devtools(Request {
        v: VERSION,
        id: 1,
        method: "Action.key".into(),
        params: json!({"key": "down"}),
    });
    assert!(reply.ok, "{:?}", reply.error);
    assert_eq!(
        reply.result.unwrap(),
        json!({"changed": true, "focused": "next"})
    );
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn key_navigation_applies_once_and_does_not_activate_the_destination() {
    let mut runtime = key_runtime(Destination::Enabled, false);
    assert!(runtime.key(&enter().with_repeat(true)).unwrap());
    assert!(!runtime.app().mounted);
    assert!(runtime.key(&enter()).unwrap());
    assert!(runtime.app().mounted);
    assert_eq!(runtime.focused(), Some("next"));
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn key_navigation_preserves_key_handler_priority_and_disallows_cross_scope_focus() {
    let mut runtime = key_runtime(Destination::Enabled, true);
    assert!(runtime.key(&enter()).unwrap());
    assert!(!runtime.app().mounted);
    assert_eq!(runtime.focused(), Some("start"));
    assert_eq!(runtime.app().presses, 1);

    for (destination, focused) in [
        (Destination::Disabled, "start"),
        (Destination::OutsideModal, "close"),
    ] {
        let mut runtime = key_runtime(destination, false);
        assert!(runtime.key(&enter()).unwrap());
        assert!(runtime.app().mounted);
        assert_eq!(runtime.focused(), Some(focused));
        assert_eq!(runtime.app().presses, 0);
    }
}

#[test]
fn projected_key_navigation_consumes_activation_fallback() {
    let mut runtime = key_runtime(Destination::Enabled, false);
    let start = runtime.dom().by_id("start").unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(start)
        .unwrap();
    assert!(runtime
        .accessibility_key(&target, &enter(), Event::Activate)
        .unwrap());
    assert_eq!(runtime.focused(), Some("next"));
    assert_eq!(runtime.app().presses, 0);
}

#[cfg(feature = "devtools")]
#[test]
fn semantic_enter_uses_opted_in_key_navigation() {
    use serde_json::json;
    use surface_devtools_protocol::{Request, VERSION};

    for (dedicated_handler, focus, presses) in [(false, "next", 0), (true, "start", 1)] {
        let mut runtime = key_runtime(Destination::Enabled, dedicated_handler);
        let reply = runtime.execute_devtools(Request {
            v: VERSION,
            id: 1,
            method: "Action.key".into(),
            params: json!({"key": "enter"}),
        });
        assert!(reply.ok, "{:?}", reply.error);
        assert_eq!(
            reply.result.unwrap(),
            json!({"changed": true, "focused": focus})
        );
        assert_eq!(runtime.app().presses, presses);
    }
}

#[test]
fn navigation_mounts_and_reveals_the_target_without_pressing_it() {
    let mut runtime = runtime(Destination::Enabled);
    assert_eq!(runtime.focused(), Some("start"));
    assert!(runtime.dom().by_id("next").is_none());
    assert!(right(&mut runtime));
    assert_eq!(runtime.focused(), Some("next"));
    assert_eq!(runtime.app().presses, 0);
    render(&mut runtime);
    assert!(runtime.scroll_metrics("body").unwrap().offset.1 > 0.);
}

#[test]
fn navigation_cannot_focus_a_disabled_target_or_cross_the_new_modal_scope() {
    for (destination, focused) in [
        (Destination::Disabled, "start"),
        (Destination::OutsideModal, "close"),
    ] {
        let mut runtime = runtime(destination);
        assert!(right(&mut runtime));
        assert!(runtime.dom().by_id("next").is_some());
        assert_eq!(runtime.focused(), Some(focused));
        assert_eq!(runtime.app().presses, 0);
    }
}

#[test]
fn extended_navigation_preserves_shift_on_page_keys() {
    let mut runtime = Runtime::new(App {
        year_step: true,
        ..App::default()
    })
    .unwrap();
    runtime.activate("start").unwrap();
    for extend in [false, true] {
        assert_eq!(
            runtime
                .dispatch(Event::Edit {
                    key: EditKey::PageDown,
                    extend,
                    by_word: false,
                })
                .unwrap(),
            extend
        );
    }
    assert_eq!(runtime.focused(), Some("next"));
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn modal_entry_uses_an_enabled_descendant_and_falls_back() {
    for (target, expected) in [
        ("preferred", "preferred"),
        ("disabled", "close"),
        ("missing", "close"),
        ("start", "close"),
    ] {
        let mut runtime = Runtime::new(App {
            destination: Destination::OutsideModal,
            preferred_modal_focus: Some(target),
            ..App::default()
        })
        .unwrap();
        runtime.activate("start").unwrap();
        assert!(right(&mut runtime));
        assert_eq!(runtime.focused(), Some(expected));
        assert!(runtime.dispatch(Event::Cancel).unwrap());
        assert_eq!(runtime.focused(), Some("start"));
    }
}
