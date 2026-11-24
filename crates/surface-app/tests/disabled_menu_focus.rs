use surface::{Color, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, AccessibilityFocusResult, Application, EditKey, Event, Key,
    KeyPress, Modifiers, Roving, Runtime, View,
};

const FOCUS_INK: Color = Color(200, 20, 45, 255);

#[derive(Clone, Copy)]
enum Message {
    Show,
    Dismiss,
    Press,
    Key,
    Context,
    SetDisabled(bool),
    Rebuild,
}

#[derive(Default)]
struct App {
    open: bool,
    automatic: bool,
    disabled: bool,
    initial: Option<&'static str>,
    actions: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Show => self.open = true,
            Message::Dismiss => self.open = false,
            Message::Press | Message::Key | Message::Context => self.actions += 1,
            Message::SetDisabled(disabled) => self.disabled = disabled,
            Message::Rebuild => {}
        }
    }

    fn view(&self) -> View<Message> {
        let menu = self.open.then(|| {
            let menu = View::new(
                Node::new(Role::Menu, "Commands").id("commands").styled(
                    Style::column()
                        .width(Length::Px(140.))
                        .height(Length::Px(56.))
                        .clip(true),
                ),
            )
            .children([
                item("first", Role::MenuItem, false, false),
                item("legacy", Role::MenuItem, true, false),
                item("wrong-role", Role::Button, true, true),
                View::new(Node::new(Role::Group, "").disabled(true)).children([item(
                    "inherited",
                    Role::MenuItem,
                    true,
                    true,
                )]),
                item("unavailable", Role::MenuItem, self.disabled, true)
                    .restore_focus_on_removal("last"),
                item("last", Role::MenuItem, false, false),
            ])
            .roving_retained(Roving::vertical(self.automatic))
            .modal_overlay(Message::Dismiss);
            if let Some(initial) = self.initial {
                menu.modal_initial_focus(initial)
            } else {
                menu
            }
        });
        View::new(Node::new(Role::Window, "Test").styled(Style::column())).children(
            std::iter::once(
                View::new(Node::new(Role::Button, "Open").id("open")).on_press(Message::Show),
            )
            .chain(menu),
        )
    }
}

fn item(id: &str, role: Role, disabled: bool, focusable_when_disabled: bool) -> View<Message> {
    let mut style = Style::row().width(Length::Px(140.)).height(Length::Px(48.));
    style.min_height = 48.;
    let row = View::new(
        Node::new(role, id)
            .id(id)
            .disabled(disabled)
            .styled(style)
            .focus_color(FOCUS_INK)
            .focus_outline_width(3.),
    )
    .on_press(Message::Press)
    .on_key(|press| (press.key == Key::Enter).then_some(Message::Key))
    .on_context_menu(|_| Message::Context)
    .accessible_activation();
    if focusable_when_disabled {
        row.focus_disabled_menu_item()
    } else {
        row
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(220., 180., &mut Metrics).unwrap();
}

fn open(disabled: bool, automatic: bool, initial: Option<&'static str>) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        disabled,
        automatic,
        initial,
        ..App::default()
    })
    .unwrap();
    runtime.activate("open").unwrap();
    render(&mut runtime);
    runtime
}

fn edit(key: EditKey) -> Event {
    Event::Edit {
        key,
        extend: false,
        by_word: false,
    }
}

fn bounds(runtime: &Runtime<App>, id: &str) -> (Rect, Rect) {
    let id = runtime.dom().by_id(id).unwrap();
    let node = &runtime.computed().unwrap().nodes[id.index()];
    (node.bounds, node.bounds.intersect(node.clip))
}

#[test]
fn roving_reaches_and_reveals_only_the_opted_in_disabled_item_without_pressing_it() {
    for automatic in [false, true] {
        let mut runtime = open(true, automatic, None);
        assert_eq!(runtime.focused(), Some("first"));
        assert_eq!(bounds(&runtime, "unavailable").1.height, 0.);
        assert!(runtime.dispatch(edit(EditKey::Down)).unwrap());
        assert_eq!(runtime.focused(), Some("unavailable"));
        assert_eq!(runtime.app().actions, 0);
        let paint = runtime.render(220., 180., &mut Metrics).unwrap();
        let (row, visible) = bounds(&runtime, "unavailable");
        assert!(paint.ops.iter().any(|op| matches!(
            op,
            PaintOp::Stroke { rect, color, .. } if *rect == row && *color == FOCUS_INK
        )));
        assert_eq!(row, visible);
        assert_eq!(visible.height, 48.);
        let menu_id = runtime.dom().by_id("commands").unwrap();
        let target_id = runtime.dom().by_id("unavailable").unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(frame.focused(), Some(target_id));
        assert!(frame.focus_target(target_id).is_some());
        assert!(frame.activation_target(target_id).is_none());
        assert!(!runtime.dom().get(target_id).unwrap().is_focusable());
        assert!(
            runtime.computed().unwrap().nodes[menu_id.index()]
                .bounds
                .height
                < 2. * row.height
        );

        runtime.apply(Message::Rebuild).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), Some("unavailable"));
        assert_eq!(bounds(&runtime, "unavailable").1, visible);

        assert!(runtime.dispatch(edit(EditKey::Down)).unwrap());
        assert_eq!(runtime.focused(), Some("last"));
        assert_eq!(runtime.app().actions, usize::from(automatic));
    }
}

#[test]
fn disabled_focus_blocks_pointer_touch_and_every_activation_route() {
    let mut runtime = open(true, false, Some("unavailable"));
    assert_eq!(runtime.focused(), Some("unavailable"));
    let (row, visible) = bounds(&runtime, "unavailable");
    assert_eq!(row, visible);
    let (x, y) = (row.x + row.width / 2., row.y + row.height / 2.);
    for key in [Key::Enter, Key::Space] {
        let press = KeyPress::new(key, Modifiers::default());
        assert!(runtime.key(&press).unwrap());
        assert!(!runtime.dispatch(Event::Activate).unwrap());
    }
    assert!(matches!(
        runtime.activate("unavailable"),
        Err(surface_app::AppError::MissingControl(_))
    ));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    runtime.dispatch(Event::SecondaryDown { x, y }).unwrap();
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().actions, 0);
    assert_eq!(runtime.focused(), Some("unavailable"));

    let target = runtime.dom().by_id("unavailable").unwrap();
    let source = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(target)
        .unwrap();
    assert!(runtime
        .accessibility_key(
            &source,
            &KeyPress::new(Key::Enter, Modifiers::default()),
            Event::Activate,
        )
        .unwrap());
    assert_eq!(runtime.app().actions, 0);
    assert!(runtime
        .accessibility_key(
            &source,
            &KeyPress::new(Key::ArrowDown, Modifiers::default()),
            edit(EditKey::Down),
        )
        .unwrap());
    assert_eq!(runtime.focused(), Some("last"));
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert_eq!(runtime.focused(), Some("open"));
    assert!(!runtime.app().open);
}

#[test]
fn changing_to_disabled_keeps_focus_and_retires_previously_issued_press_tokens() {
    let mut runtime = open(false, false, Some("unavailable"));
    let id = runtime.dom().by_id("unavailable").unwrap();
    let press = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id)
        .unwrap();
    runtime.apply(Message::SetDisabled(true)).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("unavailable"));
    assert_eq!(
        runtime.accessibility_activate(&press).unwrap(),
        AccessibilityActivationResult::Rejected
    );
    let id = runtime.dom().by_id("unavailable").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.activation_target(id).is_none());
    let focus = frame.focus_target(id).unwrap();
    assert_eq!(
        runtime.accessibility_focus(&focus).unwrap(),
        AccessibilityFocusResult::Unchanged
    );
    // An already visible next row may take focus. The previous disabled
    // item can take it back through the same host focus lane.
    assert!(runtime.dispatch(edit(EditKey::Down)).unwrap());
    assert_eq!(runtime.focused(), Some("last"));
    assert_eq!(
        runtime.accessibility_focus(&focus).unwrap(),
        AccessibilityFocusResult::Moved
    );
    assert_eq!(runtime.focused(), Some("unavailable"));
    assert_eq!(runtime.app().actions, 0);
}

#[cfg(feature = "devtools")]
#[test]
fn devtools_reports_disabled_for_a_direct_action_but_keys_can_navigate() {
    use serde_json::json;
    use surface_devtools_protocol::{ErrorCode, Request, VERSION};

    let mut runtime = open(true, false, None);
    let mut request = |method: &str, params| {
        runtime.execute_devtools(Request {
            v: VERSION,
            id: 1,
            method: method.into(),
            params,
        })
    };
    let down = request("Action.key", json!({ "key": "down" }));
    assert_eq!(down.result.unwrap()["focused"], "unavailable");
    let direct = request("Action.activate", json!({ "id": "unavailable" }));
    assert_eq!(direct.error.unwrap().code, ErrorCode::Disabled);
    let enter = request("Action.key", json!({ "key": "enter" }));
    let result = enter.result.unwrap();
    assert_eq!(result["focused"], "unavailable");
    assert_eq!(result["changed"], false);
    assert_eq!(runtime.app().actions, 0);
}
