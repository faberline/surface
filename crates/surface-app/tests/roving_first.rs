use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Key, KeyPress, Modifiers, Roving, Runtime, View};

#[derive(Clone)]
enum Message {
    Press,
    Rebuild,
}

struct App {
    first: bool,
    rtl: bool,
    width: f32,
    presses: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        if matches!(message, Message::Press) {
            self.presses += 1;
        }
    }

    fn view(&self) -> View<Message> {
        let mut row = Node::new(Role::Group, "Options").id("row").styled(
            Style::row()
                .width(Length::Px(self.width))
                .height(Length::Px(32.))
                .clip(true),
        );
        if self.rtl {
            row = row.reverse_row();
        }
        let options = Roving::horizontal(false);
        let row = View::new(row).children([
            member("disabled", true, false),
            member("first", false, false),
            member("checked", false, true),
        ]);
        let row = if self.first {
            row.roving_first(options)
        } else {
            row.roving(options)
        };
        View::new(Node::new(Role::Window, "Test").styled(Style::column())).children([
            member("before", false, false),
            row,
            member("after", false, false),
        ])
    }
}

fn member(id: &str, disabled: bool, checked: bool) -> View<Message> {
    let mut style = Style::default()
        .width(Length::Px(48.))
        .height(Length::Px(32.));
    style.min_width = 48.;
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .disabled(disabled)
            .checked(checked)
            .styled(style),
    )
    .on_press(Message::Press)
    .accessible_activation()
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(240., 140., &mut Metrics).unwrap();
}

fn runtime(first: bool, rtl: bool, width: f32) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        first,
        rtl,
        width,
        presses: 0,
    })
    .unwrap();
    render(&mut runtime);
    runtime
}

fn key(key: EditKey) -> Event {
    Event::Edit {
        key,
        extend: false,
        by_word: false,
    }
}

fn tab(runtime: &mut Runtime<App>, reverse: bool, expected: &str) {
    assert!(runtime.dispatch(Event::FocusNext { reverse }).unwrap());
    assert_eq!(runtime.focused(), Some(expected));
}

#[test]
fn first_enabled_entry_works_both_ways_and_keeps_the_current_group_tab_stop() {
    for rtl in [false, true] {
        let mut runtime = runtime(true, rtl, 180.);
        tab(&mut runtime, false, "before");
        tab(&mut runtime, false, "first");
        let direction = if rtl { EditKey::Left } else { EditKey::Right };
        assert!(runtime.dispatch(key(direction)).unwrap());
        assert_eq!(runtime.focused(), Some("checked"));
        assert_eq!(runtime.app().presses, 0);
        tab(&mut runtime, false, "after");
        tab(&mut runtime, true, "first");
        tab(&mut runtime, true, "before");
    }
}

#[test]
fn ordinary_roving_still_enters_at_the_checked_member() {
    let mut runtime = runtime(false, false, 180.);
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "checked");
    tab(&mut runtime, false, "after");
    tab(&mut runtime, true, "checked");
}

#[test]
fn projected_arrows_reveal_a_clipped_member_and_rebuild_retains_focus() {
    let mut runtime = runtime(true, false, 96.);
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "first");
    render(&mut runtime);
    let row = runtime.dom().by_id("row").unwrap();
    let first = runtime.dom().by_id("first").unwrap();
    let checked = runtime.dom().by_id("checked").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.is_named_roving_group(row));
    assert!(frame.focus_target(checked).is_none());
    let source = frame.focus_target(first).unwrap();
    assert!(runtime
        .accessibility_key(
            &source,
            &KeyPress::new(Key::ArrowRight, Modifiers::default()),
            key(EditKey::Right)
        )
        .unwrap());
    assert_eq!(runtime.focused(), Some("checked"));
    render(&mut runtime);
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.focused(), Some(checked));
    assert!(frame.focus_target(checked).is_some());
    assert!(frame.activation_target(checked).is_some());
    assert_eq!(runtime.scroll_metrics("row").unwrap().offset.0, 48.);
    runtime.apply(Message::Rebuild).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("checked"));
    assert_eq!(runtime.scroll_metrics("row").unwrap().offset.0, 48.);
    assert_eq!(runtime.app().presses, 0);
}
