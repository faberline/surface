use surface::{AdjustableValue, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment as Action, AccessibilityAdjustmentResult as Outcome, Application,
    EditKey, Event, ImeEvent, Key, KeyPress, Queue, Roving, Runtime, View,
};

#[derive(Clone)]
enum Message {
    Set(f64),
    Open,
    Close,
    Disabled,
    Hide,
    Edit,
}

#[derive(Default)]
struct App {
    value: Option<f64>,
    writes: usize,
    modal: bool,
    disabled: bool,
    hidden: bool,
    hide_on_write: bool,
    queue: Option<Queue<Message>>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Set(value) => {
                self.value = Some(value);
                self.writes += 1;
            }
            Message::Open => self.modal = true,
            Message::Close => self.modal = false,
            Message::Disabled => self.disabled = true,
            Message::Hide => self.hidden = true,
            Message::Edit => {}
        }
    }

    fn attach_queue(&mut self, queue: Queue<Message>) {
        self.queue = Some(queue);
    }

    fn view(&self) -> View<Message> {
        let queue = self.queue.clone().filter(|_| self.hide_on_write);
        let adjust = View::new(
            Node::new(Role::Group, "分鐘")
                .id("minute")
                .disabled(self.disabled)
                .adjustable(AdjustableValue {
                    min: 0.,
                    max: 59.,
                    step: 1.,
                    value: self.value,
                    value_text: Some(
                        self.value
                            .map_or_else(|| "未設定".into(), |value| format!("{value:02}")),
                    ),
                })
                .styled(target()),
        )
        .on_value(move |value| {
            if let Some(queue) = &queue {
                queue.post(Message::Hide);
            }
            Message::Set(value)
        });
        let adjust = if self.hidden {
            adjust.without_interaction(None)
        } else {
            adjust
        };
        let editor = View::new(
            Node::new(Role::TextInput, "Editor")
                .id("editor")
                .text("value")
                .styled(target()),
        )
        .on_input(|_| Message::Edit);
        let opener = View::new(Node::new(Role::Button, "Open").id("open").styled(target()))
            .on_press(Message::Open);
        let panel = self.modal.then(|| {
            View::new(
                Node::new(Role::Dialog, "Panel")
                    .id("panel")
                    .styled(target()),
            )
            .modal_overlay(Message::Close)
        });
        View::new(Node::new(Role::Window, "Test").id("root"))
            .children([opener, editor, adjust].into_iter().chain(panel))
            .roving(Roving::horizontal(false))
    }
}

fn target() -> Style {
    Style::row().width(Length::Px(120.)).height(Length::Px(48.))
}

struct Text;
impl TextMeasurer for Text {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (24., 24.)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    runtime.render(400., 400., &mut Text).unwrap();
}

fn runtime(value: Option<f64>) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        value,
        ..App::default()
    })
    .unwrap();
    draw(&mut runtime);
    runtime
}

fn control(runtime: &Runtime<App>) -> surface_app::AccessibilityAdjustmentTarget {
    let id = runtime.dom().by_id("minute").unwrap();
    runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id)
        .unwrap()
}

#[test]
fn adjusts_discrete_values_without_focusing_then_rejects_old_tree_tokens() {
    let mut runtime = runtime(None);
    let first = control(&runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&first, Action::Increment)
            .unwrap(),
        Outcome::Adjusted
    );
    assert_eq!(runtime.app().value, Some(0.));
    assert_eq!(runtime.focused(), None);
    assert_eq!(
        runtime
            .accessibility_adjust(&first, Action::SetValue(25.))
            .unwrap(),
        Outcome::Rejected
    );
    draw(&mut runtime);
    let current = control(&runtime);
    for rejected in [
        Action::SetValue(f64::NAN),
        Action::SetValue(f64::INFINITY),
        Action::Decrement,
    ] {
        assert_eq!(
            runtime.accessibility_adjust(&current, rejected).unwrap(),
            Outcome::Rejected
        );
    }
    assert_eq!(runtime.app().writes, 1);
    assert_eq!(
        runtime
            .accessibility_adjust(&current, Action::SetValue(7.9))
            .unwrap(),
        Outcome::Adjusted
    );
    assert_eq!(runtime.app().value, Some(8.));
    draw(&mut runtime);
    let current = control(&runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&current, Action::SetValue(90.))
            .unwrap(),
        Outcome::Adjusted
    );
    assert_eq!(runtime.app().value, Some(59.));
}

#[test]
fn native_or_projected_arrow_steps_keep_the_same_control_focused() {
    let mut runtime = runtime(Some(7.));
    let id = runtime.dom().by_id("minute").unwrap();
    let focus = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(id)
        .unwrap();
    runtime.accessibility_focus(&focus).unwrap();
    draw(&mut runtime);
    for (key, expected) in [
        (EditKey::Up, 8.),
        (EditKey::Down, 7.),
        (EditKey::End, 59.),
        (EditKey::Home, 0.),
        (EditKey::Down, 0.),
    ] {
        assert!(runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false
            })
            .unwrap());
        assert_eq!(runtime.app().value, Some(expected));
        assert_eq!(runtime.focused(), Some("minute"));
    }
    assert_eq!(runtime.app().writes, 4);

    let key = EditKey::Left;
    assert!(runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false
        })
        .unwrap());
    assert_eq!(runtime.focused(), Some("editor"));
    assert_eq!(runtime.app().writes, 4);
}

#[test]
fn projected_keyboard_action_uses_the_rendered_focus_target() {
    let mut runtime = runtime(Some(7.));
    runtime.activate("minute").unwrap();
    draw(&mut runtime);
    let id = runtime.dom().by_id("minute").unwrap();
    let source = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(id)
        .unwrap();
    let press = KeyPress::new(Key::ArrowUp, Default::default());
    assert!(runtime
        .accessibility_key(
            &source,
            &press,
            Event::Edit {
                key: EditKey::Up,
                extend: false,
                by_word: false,
            }
        )
        .unwrap());
    assert_eq!(runtime.app().value, Some(8.));
    assert_eq!(runtime.focused(), Some("minute"));
}

#[test]
fn a_callback_that_hides_the_target_cannot_dispatch_the_old_adjustment() {
    let mut runtime = Runtime::new(App {
        value: Some(7.),
        hide_on_write: true,
        ..App::default()
    })
    .unwrap();
    draw(&mut runtime);
    let initial = control(&runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&initial, Action::Decrement)
            .unwrap(),
        Outcome::Rebuilt
    );
    assert!(runtime.app().hidden);
    assert_eq!(runtime.app().value, Some(7.));
    assert_eq!(runtime.app().writes, 0);

    draw(&mut runtime);
    let id = runtime.dom().by_id("minute").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(id).is_none());
    assert!(frame.adjustment_target(id).is_none());
    assert_eq!(
        runtime
            .accessibility_adjust(&initial, Action::Increment)
            .unwrap(),
        Outcome::Rejected
    );
}

#[test]
fn composition_modal_scope_disabled_and_held_pointer_block_adjustments() {
    let mut runtime = runtime(Some(7.));
    runtime.activate("editor").unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    draw(&mut runtime);
    let before_preedit = control(&runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&before_preedit, Action::Increment)
            .unwrap(),
        Outcome::BlockedByComposition
    );
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "注".into(),
            selection: None,
        }))
        .unwrap();
    draw(&mut runtime);
    let before = control(&runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&before, Action::Increment)
            .unwrap(),
        Outcome::BlockedByComposition
    );
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    draw(&mut runtime);
    let id = runtime.dom().by_id("open").unwrap();
    let Rect { x, y, .. } = runtime.computed().unwrap().nodes[id.index()].bounds;
    runtime
        .dispatch(Event::PointerDown {
            x: x + 4.,
            y: y + 4.,
        })
        .unwrap();
    draw(&mut runtime);
    let pressed = control(&runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&pressed, Action::Increment)
            .unwrap(),
        Outcome::BlockedByPointer
    );
    runtime.cancel_pointer().unwrap();

    draw(&mut runtime);
    let start = control(&runtime);
    runtime.apply(Message::Open).unwrap();
    draw(&mut runtime);
    let id = runtime.dom().by_id("minute").unwrap();
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id)
        .is_none());
    assert_eq!(
        runtime
            .accessibility_adjust(&start, Action::Increment)
            .unwrap(),
        Outcome::Rejected
    );

    runtime.apply(Message::Close).unwrap();
    runtime.apply(Message::Disabled).unwrap();
    draw(&mut runtime);
    let id = runtime.dom().by_id("minute").unwrap();
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id)
        .is_none());
    assert_eq!(runtime.app().writes, 0);
}

#[cfg(feature = "devtools")]
#[test]
fn semantic_up_key_adjusts_the_focused_opted_in_control() {
    use serde_json::json;
    use surface_devtools_protocol::{Request, VERSION};

    let mut runtime = runtime(Some(7.));
    runtime.activate("minute").unwrap();
    draw(&mut runtime);
    let reply = runtime.execute_devtools(Request {
        v: VERSION,
        id: 1,
        method: "Action.key".into(),
        params: json!({"key": "up"}),
    });
    assert!(reply.ok, "{:?}", reply.error);
    assert_eq!(
        reply.result.unwrap(),
        json!({"changed": true, "focused": "minute"})
    );
    assert_eq!(runtime.app().value, Some(8.));
}
