use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Key, KeyPress, Modifiers, Roving, Runtime, View};

const A: &str = "open-a";
const PIN: &str = "pin-a";
const B: &str = "menu-b";
const C: &str = "open-c";

#[derive(Clone)]
enum Message {
    Press(&'static str),
    Reverse,
    RemoveLast,
}

#[derive(Default)]
struct App {
    rtl: bool,
    vertical_only: bool,
    reversed: bool,
    remove_last: bool,
    presses: Vec<&'static str>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press(id) => self.presses.push(id),
            Message::Reverse => self.reversed = !self.reversed,
            Message::RemoveLast => self.remove_last = true,
        }
    }

    fn view(&self) -> View<Message> {
        let row = |id, actions| {
            let node = Node::new(Role::ListItem, "").id(id).styled(Style::row());
            let node = if self.rtl { node.reverse_row() } else { node };
            View::new(node).children(actions)
        };
        let mut rows = vec![
            row(
                "row-a",
                vec![
                    action(A, false, 48.),
                    action("disabled-a", true, 48.),
                    action(PIN, false, 48.),
                ],
            ),
            row(
                "row-b",
                vec![
                    action("disabled-b", true, 48.),
                    action("collapsed-b", false, 0.),
                    action(B, false, 48.),
                ],
            ),
        ];
        if !self.remove_last {
            rows.push(row("row-c", vec![action(C, false, 48.)]));
        }
        if self.reversed {
            rows.reverse();
        }
        let list = View::new(
            Node::new(Role::List, "Items")
                .id("list")
                .styled(Style::column().height(Length::Px(72.)).clip(true)),
        )
        .children(rows);
        let list = if self.vertical_only {
            list.roving(Roving::vertical(false))
        } else {
            list.roving_all_arrows()
        };
        View::new(Node::new(Role::Window, "Test")).children([
            action("before", false, 40.),
            list,
            action("after", false, 40.),
        ])
    }
}

fn action(id: &'static str, disabled: bool, height: f32) -> View<Message> {
    let mut node = Node::new(Role::Button, id)
        .id(id)
        .disabled(disabled)
        .styled(
            Style::column()
                .width(Length::Px(48.))
                .height(Length::Px(height)),
        );
    if id == PIN {
        node = node.checked(true);
    }
    View::new(node)
        .on_press(Message::Press(id))
        .accessible_activation()
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(240., 180., &mut Metrics).unwrap();
}

fn edit(key: EditKey) -> Event {
    Event::Edit {
        key,
        extend: false,
        by_word: false,
    }
}

fn move_focus(runtime: &mut Runtime<App>, event: Event, expected: &str) {
    assert!(runtime.dispatch(event).unwrap());
    render(runtime);
    assert_eq!(runtime.focused(), Some(expected));
}

fn tab(reverse: bool) -> Event {
    Event::FocusNext { reverse }
}

#[test]
fn all_arrows_follow_authored_order_in_ltr_and_rtl_without_activating() {
    for rtl in [false, true] {
        let mut runtime = Runtime::new(App {
            rtl,
            ..App::default()
        })
        .unwrap();
        render(&mut runtime);
        move_focus(&mut runtime, tab(false), "before");
        move_focus(&mut runtime, tab(false), A);
        for (key, expected) in [
            (EditKey::Right, PIN),
            (EditKey::Down, B),
            (EditKey::Right, C),
            (EditKey::Down, A),
            (EditKey::Up, C),
            (EditKey::Left, B),
            (EditKey::Up, PIN),
            (EditKey::Left, A),
            (EditKey::Left, C),
            (EditKey::Home, A),
            (EditKey::End, C),
        ] {
            move_focus(&mut runtime, edit(key), expected);
        }
        assert!(runtime.app().presses.is_empty());
        assert!(runtime.dispatch(Event::Activate).unwrap());
        assert_eq!(runtime.app().presses, [C]);
        render(&mut runtime);
        move_focus(&mut runtime, tab(false), "after");
        move_focus(&mut runtime, tab(true), A);
        move_focus(&mut runtime, tab(true), "before");
    }
}

#[test]
fn projected_arrow_reveals_a_clipped_action_and_focus_survives_rebuilds() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    render(&mut runtime);
    move_focus(&mut runtime, tab(false), "before");
    move_focus(&mut runtime, tab(false), A);
    let first = runtime.dom().by_id(A).unwrap();
    let last = runtime.dom().by_id(C).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.focus_target(last).is_none());
    let projected = frame.focus_target(first).unwrap();
    assert!(runtime
        .accessibility_key(
            &projected,
            &KeyPress::new(Key::ArrowLeft, Modifiers::default()),
            edit(EditKey::Left)
        )
        .unwrap());
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(C));
    assert_eq!(runtime.accessibility_frame().unwrap().focused(), Some(last));
    assert!(runtime.scroll_metrics("list").unwrap().offset.1 > 0.);

    runtime.apply(Message::Reverse).unwrap();
    render(&mut runtime);
    let last = runtime.dom().by_id(C).unwrap();
    assert_eq!(runtime.scroll_metrics("list").unwrap().offset.1, 0.);
    assert_eq!(runtime.accessibility_frame().unwrap().focused(), Some(last));
    runtime.apply(Message::RemoveLast).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(B));
    assert!(runtime.app().presses.is_empty());
}

#[test]
fn deferred_fallback_defers_to_tab_and_blur_before_the_next_layout() {
    for (event, expected) in [(tab(true), Some("after")), (Event::Blur, None)] {
        let mut runtime = Runtime::new(App::default()).unwrap();
        render(&mut runtime);
        move_focus(&mut runtime, tab(false), "before");
        move_focus(&mut runtime, tab(false), A);
        move_focus(&mut runtime, edit(EditKey::End), C);
        runtime.apply(Message::RemoveLast).unwrap();
        assert_eq!(runtime.focused(), None);
        runtime.dispatch(event).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), expected);
        assert!(runtime.app().presses.is_empty());
    }
}

#[test]
fn existing_vertical_roving_still_enters_at_checked_and_ignores_horizontal_keys() {
    let mut runtime = Runtime::new(App {
        vertical_only: true,
        ..App::default()
    })
    .unwrap();
    render(&mut runtime);
    move_focus(&mut runtime, tab(false), "before");
    move_focus(&mut runtime, tab(false), PIN);
    for key in [EditKey::Left, EditKey::Right] {
        assert!(!runtime.dispatch(edit(key)).unwrap());
    }
    assert_eq!(runtime.focused(), Some(PIN));
    assert!(runtime.app().presses.is_empty());
}
