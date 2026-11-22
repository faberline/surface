use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityFocusResult as Result, AccessibilityFocusTarget, Application, EditKey, Event,
    ImeEvent, Key, KeyPress, Modifiers, Roving, Runtime, TextOptions, View,
};

#[derive(Clone)]
enum Message {
    Open(bool),
    Replace,
    BreakView(bool),
    Press,
    Edit(String),
}

#[derive(Default)]
struct App {
    modal: bool,
    replaced: bool,
    broken: bool,
    presses: u32,
    text: String,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open(open) => self.modal = open,
            Message::Replace => self.replaced = true,
            Message::BreakView(broken) => self.broken = broken,
            Message::Press => self.presses += 1,
            Message::Edit(text) => self.text = text,
        }
    }

    fn view(&self) -> View<Message> {
        let mut field_style = Style::default().height(Length::Px(40.));
        field_style.min_height = 40.;
        let mut children = vec![
            View::new(
                Node::new(Role::TextInput, "Field")
                    .id("field")
                    .text(&self.text)
                    .styled(field_style.clone()),
            )
            .text_options(TextOptions {
                multiline: true,
                ..Default::default()
            })
            .on_input(Message::Edit),
            View::new(
                Node::new(
                    Role::Button,
                    if self.replaced {
                        "Replacement"
                    } else {
                        "Original"
                    },
                )
                .id("button")
                .styled(field_style.clone()),
            )
            .on_press(Message::Press)
            .on_key(|press| {
                (press.key == Key::Character("k".into()) && press.modifiers.alt)
                    .then_some(Message::Press)
            }),
            View::new(
                Node::new(Role::Button, "Disabled")
                    .id("disabled")
                    .disabled(true)
                    .styled(field_style.clone()),
            ),
        ];
        if self.broken {
            children.push(View::new(Node::new(Role::Button, "Duplicate").id("button")));
        }
        if self.modal {
            children.push(
                View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                    .modal(Message::Open(false))
                    .children([View::new(
                        Node::new(Role::Button, "Close")
                            .id("close")
                            .styled(field_style),
                    )
                    .on_press(Message::Open(false))]),
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

fn runtime() -> Runtime<App> {
    let mut runtime = Runtime::new(App::default()).unwrap();
    assert!(runtime.accessibility_frame().is_none());
    runtime.render(240., 240., &mut Metrics).unwrap();
    runtime
}

fn target(runtime: &Runtime<App>, stable_id: &str) -> AccessibilityFocusTarget {
    let node = runtime.dom().by_id(stable_id).unwrap();
    runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(node)
        .unwrap()
}

#[test]
fn custom_projected_keys_need_no_fallback_and_reject_stale_targets() {
    let mut runtime = runtime();
    let projected = target(&runtime, "button");
    assert_eq!(
        runtime.accessibility_focus(&projected).unwrap(),
        Result::Moved
    );
    let key = Key::Character("k".into());
    let plain = KeyPress::new(key.clone(), Modifiers::default());
    let custom = KeyPress::new(
        key,
        Modifiers {
            alt: true,
            ..Modifiers::default()
        },
    );
    assert!(!runtime.accessibility_key(&projected, &plain, None).unwrap());
    assert!(runtime
        .accessibility_key(&projected, &custom, None)
        .unwrap());
    assert_eq!(runtime.app().presses, 1);
    assert!(!runtime
        .accessibility_key(&projected, &custom, None)
        .unwrap());
    assert_eq!(runtime.app().presses, 1);
}

#[test]
fn rapid_projected_keys_follow_current_focus_before_the_next_frame() {
    #[derive(Default)]
    struct Toolbar(Option<&'static str>);

    impl Application for Toolbar {
        type Message = &'static str;

        fn update(&mut self, message: Self::Message) {
            self.0 = Some(message);
        }

        fn view(&self) -> View<Self::Message> {
            View::new(
                Node::new(Role::Group, "Tools")
                    .id("tools")
                    .styled(Style::row()),
            )
            .children(["cut", "save"].map(|id| {
                View::new(
                    Node::new(Role::Button, id)
                        .id(id)
                        .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
                )
                .on_press(id)
            }))
            .roving(Roving::horizontal(false))
        }
    }

    let mut runtime = Runtime::new(Toolbar::default()).unwrap();
    runtime.render(240., 240., &mut Metrics).unwrap();
    let cut = runtime.dom().by_id("cut").unwrap();
    let projected = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(cut)
        .unwrap();
    assert_eq!(
        runtime.accessibility_focus(&projected).unwrap(),
        Result::Moved
    );
    assert!(runtime.accessibility_frame().is_none());
    assert!(runtime
        .accessibility_key(
            &projected,
            &KeyPress::new(Key::End, Modifiers::default()),
            Event::Edit {
                key: EditKey::End,
                extend: false,
                by_word: false
            },
        )
        .unwrap());
    assert_eq!(runtime.focused(), Some("save"));
    assert_eq!(runtime.app().0, None);
    assert!(runtime
        .accessibility_key(
            &projected,
            &KeyPress::new(Key::Enter, Modifiers::default()),
            Event::Activate,
        )
        .unwrap());
    assert_eq!(runtime.app().0, Some("save"));
    // The press rebuilt the view. Another queued key from that browser
    // control cannot address either this view or a control reusing its ID.
    assert!(!runtime
        .accessibility_key(
            &projected,
            &KeyPress::new(Key::Enter, Modifiers::default()),
            Event::Activate,
        )
        .unwrap());
}

#[test]
fn projected_keys_do_not_apply_after_pending_view_replacement_or_modal_change() {
    let mut runtime = runtime();
    let old = target(&runtime, "button");
    assert_eq!(runtime.accessibility_focus(&old).unwrap(), Result::Moved);
    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    runtime.queue().post(Message::Replace);
    assert!(runtime
        .accessibility_key(&old, &enter, Event::Activate)
        .unwrap());
    assert!(runtime.app().replaced);
    assert_eq!(runtime.app().presses, 0);
    runtime.render(240., 240., &mut Metrics).unwrap();
    assert!(!runtime
        .accessibility_key(&old, &enter, Event::Activate)
        .unwrap());

    let replaced = target(&runtime, "button");
    runtime.apply(Message::Open(true)).unwrap();
    runtime.render(240., 240., &mut Metrics).unwrap();
    assert_eq!(runtime.focused(), Some("close"));
    assert!(!runtime
        .accessibility_key(&replaced, &enter, Event::Activate)
        .unwrap());
    assert!(runtime.app().modal);
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn focus_does_not_activate_and_ordinary_frames_keep_identity() {
    let mut runtime = runtime();
    let button = runtime.dom().by_id("button").unwrap();
    let field = runtime.dom().by_id("field").unwrap();
    let disabled = runtime.dom().by_id("disabled").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let generation = frame.generation();
    assert!(frame.node(field).unwrap().text_options.unwrap().multiline);
    assert!(frame.focus_target(disabled).is_none());
    let field_focus = frame.focus_target(field).unwrap();
    let button_focus = frame.focus_target(button).unwrap();
    assert_eq!(
        runtime.accessibility_focus(&field_focus).unwrap(),
        Result::Moved
    );
    runtime.render(240., 240., &mut Metrics).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.generation(), generation);
    assert_eq!(frame.focused(), Some(field));
    assert_eq!(
        runtime.accessibility_focus(&field_focus).unwrap(),
        Result::Unchanged
    );
    assert_eq!(
        runtime.accessibility_focus(&button_focus).unwrap(),
        Result::Moved
    );
    assert_eq!(runtime.focused(), Some("button"));
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn modal_projection_and_replacement_retire_previous_targets() {
    let mut runtime = runtime();
    let button = runtime.dom().by_id("button").unwrap();
    let root = runtime.dom().root();
    let old = target(&runtime, "button");
    let mut other = crate::runtime();
    assert_ne!(target(&other, "button"), old);
    assert_eq!(other.accessibility_focus(&old).unwrap(), Result::Rejected);

    runtime.apply(Message::Open(true)).unwrap();
    assert_eq!(runtime.accessibility_focus(&old).unwrap(), Result::Rejected);
    runtime.render(240., 240., &mut Metrics).unwrap();
    let modal = runtime.dom().by_id("modal").unwrap();
    let close = runtime.dom().by_id("close").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.modal_root(), Some(modal));
    assert_eq!(frame.root(), modal);
    assert!(frame.node(button).is_none());
    assert!(frame.children(root).next().is_none());
    assert_eq!(frame.children(modal).collect::<Vec<_>>(), vec![close]);

    runtime.apply(Message::Open(false)).unwrap();
    runtime.apply(Message::Replace).unwrap();
    runtime.render(240., 240., &mut Metrics).unwrap();
    assert_eq!(runtime.dom().by_id("button"), Some(button));
    assert_eq!(runtime.accessibility_focus(&old).unwrap(), Result::Rejected);
    assert_eq!(
        runtime
            .accessibility_focus(&target(&runtime, "button"))
            .unwrap(),
        Result::Moved
    );
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn queue_is_applied_before_focus_and_invalid_view_never_reuses_old_targets() {
    let mut runtime = runtime();
    let old = target(&runtime, "button");
    runtime.queue().post(Message::Replace);
    assert!(runtime.accessibility_frame().is_none());
    assert_eq!(runtime.accessibility_focus(&old).unwrap(), Result::Rebuilt);
    assert!(runtime.app().replaced);
    assert!(runtime.focused().is_none());
    runtime.render(240., 240., &mut Metrics).unwrap();
    let changed = target(&runtime, "button");
    runtime.queue().post(Message::BreakView(true));
    assert!(runtime.accessibility_focus(&changed).is_err());
    runtime.render(240., 240., &mut Metrics).unwrap();
    assert!(runtime.accessibility_frame().is_none());
    assert_eq!(
        runtime.accessibility_focus(&changed).unwrap(),
        Result::Rejected
    );
    runtime.apply(Message::BreakView(false)).unwrap();
    runtime.render(240., 240., &mut Metrics).unwrap();
    assert_ne!(target(&runtime, "button"), changed);
}

#[test]
fn focus_preserves_the_replacement_range_across_empty_preedit() {
    let mut runtime = runtime();
    let field = target(&runtime, "field");
    assert_eq!(runtime.accessibility_focus(&field).unwrap(), Result::Moved);
    runtime.dispatch(Event::Text("原本".into())).unwrap();
    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: "原".len(),
        })
        .unwrap();
    runtime.render(240., 240., &mut Metrics).unwrap();
    let button = target(&runtime, "button");
    let field = target(&runtime, "field");
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄒ".into(),
            selection: None,
        }))
        .unwrap();
    assert_eq!(
        runtime.accessibility_focus(&button).unwrap(),
        Result::BlockedByComposition
    );
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: String::new(),
            selection: None,
        }))
        .unwrap();
    // Processing this queued message would move focus into the modal and
    // cancel the pending replacement. An AX request must not trigger it.
    runtime.queue().post(Message::Open(true));
    assert_eq!(
        runtime.accessibility_focus(&button).unwrap(),
        Result::BlockedByComposition
    );
    assert!(!runtime
        .accessibility_key(
            &button,
            &KeyPress::new(Key::Enter, Modifiers::default()),
            Event::Activate,
        )
        .unwrap());
    assert!(!runtime
        .accessibility_key(
            &field,
            &KeyPress::new(Key::Enter, Modifiers::default()),
            Event::Activate,
        )
        .unwrap());
    assert!(!runtime.app().modal);
    assert_eq!(runtime.focused(), Some("field"));
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("新".into())))
        .unwrap();
    assert_eq!(runtime.app().text, "新本");
}

#[test]
fn a_focus_request_cannot_interrupt_an_active_pointer_press() {
    let mut runtime = runtime();
    let field = target(&runtime, "field");
    let button = runtime.dom().by_id("button").unwrap();
    let bounds = runtime
        .accessibility_frame()
        .unwrap()
        .node(button)
        .unwrap()
        .layout
        .bounds;
    let (x, y) = (bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    assert_eq!(
        runtime.accessibility_focus(&field).unwrap(),
        Result::BlockedByPointer
    );
    assert!(!runtime
        .accessibility_key(
            &field,
            &KeyPress::new(Key::Enter, Modifiers::default()),
            Event::Activate,
        )
        .unwrap());
    assert_eq!(runtime.app().presses, 0);
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().presses, 1);
    assert_eq!(runtime.focused(), Some("button"));
}

#[test]
fn clipping_and_render_failure_retire_a_target_without_a_view_rebuild() {
    let mut runtime = runtime();
    let button = runtime.dom().by_id("button").unwrap();
    let old = target(&runtime, "button");
    runtime.render(240., 30., &mut Metrics).unwrap();
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(button)
        .is_none());
    assert_eq!(runtime.accessibility_focus(&old).unwrap(), Result::Rejected);
    runtime.render(240., 240., &mut Metrics).unwrap();
    let current = target(&runtime, "button");
    assert_ne!(old, current);
    assert!(runtime.render(240., 0., &mut Metrics).is_err());
    assert_eq!(
        runtime.accessibility_focus(&current).unwrap(),
        Result::Rejected
    );
    assert!(runtime.accessibility_frame().is_none());
    runtime.render(240., 240., &mut Metrics).unwrap();
    assert_ne!(current, target(&runtime, "button"));
}
