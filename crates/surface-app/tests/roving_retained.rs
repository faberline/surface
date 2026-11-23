use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Roving, Runtime, ScrollTarget, View};

const ROWS: &[&str] = &["a", "b", "c", "d"];

#[derive(Clone)]
enum Message {
    Press,
    Rebuild,
    Reverse,
    Replace {
        rows: &'static [&'static str],
        checked: Option<&'static str>,
        disabled: Option<&'static str>,
    },
}

struct App {
    rows: &'static [&'static str],
    reversed: bool,
    retained: bool,
    checked: Option<&'static str>,
    disabled: Option<&'static str>,
    explicit_fallback: &'static str,
    presses: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press => self.presses += 1,
            Message::Rebuild => {}
            Message::Reverse => self.reversed = !self.reversed,
            Message::Replace {
                rows,
                checked,
                disabled,
            } => {
                self.rows = rows;
                self.checked = checked;
                self.disabled = disabled;
            }
        }
    }

    fn view(&self) -> View<Message> {
        let rows = (0..self.rows.len()).map(|index| {
            let id = self.rows[if self.reversed {
                self.rows.len() - 1 - index
            } else {
                index
            }];
            let row = View::new(
                Node::new(Role::Radio, id)
                    .id(id)
                    .checked(self.checked == Some(id))
                    .disabled(self.disabled == Some(id))
                    .styled(Style::column().height(Length::Px(40.))),
            )
            .on_press(Message::Press)
            .accessible_activation();
            if id == "c" {
                row.restore_focus_on_removal(self.explicit_fallback)
            } else {
                row
            }
        });
        let group = View::new(
            Node::new(Role::RadioGroup, "Choices")
                .id("choices")
                .styled(Style::column().height(Length::Px(64.)).clip(true)),
        )
        .children(rows);
        let options = Roving::vertical(false);
        let group = if self.retained {
            group.roving_retained(options)
        } else {
            group.roving(options)
        };
        let after = View::new(
            Node::new(Role::Button, "After")
                .id("after")
                .styled(Style::column().height(Length::Px(40.))),
        )
        .on_press(Message::Press);
        View::new(Node::new(Role::Window, "Test")).children([group, after])
    }
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

fn fixture(retained: bool, checked: &'static str, explicit_fallback: &'static str) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        rows: ROWS,
        reversed: false,
        retained,
        checked: Some(checked),
        disabled: None,
        explicit_fallback,
        presses: 0,
    })
    .unwrap();
    render(&mut runtime);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    render(&mut runtime);
    runtime
}

fn offset(runtime: &Runtime<App>) -> f32 {
    runtime.scroll_metrics("choices").unwrap().offset.1
}

fn projected_focus(runtime: &Runtime<App>, id: &str) -> bool {
    let id = runtime.dom().by_id(id).unwrap();
    runtime.accessibility_frame().unwrap().focused() == Some(id)
}

#[test]
fn reordering_reveals_the_same_previously_visible_member_only_when_opted_in() {
    for retained in [false, true] {
        let mut runtime = fixture(retained, "a", "b");
        assert!(projected_focus(&runtime, "a"));
        assert_eq!(offset(&runtime), 0.);
        runtime.apply(Message::Reverse).unwrap();
        render(&mut runtime);

        assert_eq!(runtime.focused(), Some("a"));
        assert_eq!(projected_focus(&runtime, "a"), retained);
        assert_eq!(offset(&runtime), if retained { 96. } else { 0. });
        assert_eq!(runtime.app().presses, 0);
    }
}

#[test]
fn same_order_rebuild_and_reordering_hidden_focus_preserve_manual_scroll() {
    let mut runtime = fixture(true, "a", "b");
    runtime
        .dispatch(Event::Scroll {
            x: 10.,
            y: 10.,
            delta_x: 0.,
            delta_y: 20.,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(offset(&runtime), 20.);
    assert!(projected_focus(&runtime, "a"));
    runtime.apply(Message::Rebuild).unwrap();
    render(&mut runtime);
    assert_eq!(offset(&runtime), 20.);
    assert_eq!(runtime.focused(), Some("a"));

    assert!(runtime.scroll_to("choices", ScrollTarget::Offset { x: 0., y: 48. }));
    render(&mut runtime);
    assert_eq!(offset(&runtime), 48.);
    assert!(!projected_focus(&runtime, "a"));
    runtime.apply(Message::Reverse).unwrap();
    render(&mut runtime);
    assert_eq!(offset(&runtime), 48.);
    assert_eq!(runtime.focused(), Some("a"));
    assert!(!projected_focus(&runtime, "a"));
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn current_group_fallback_survives_simultaneous_removal_and_prefers_selection() {
    for (retained, fallback, expected) in [
        (true, "a", Some("d")),
        (true, "after", Some("after")),
        (false, "a", None),
    ] {
        let mut runtime = fixture(retained, "c", fallback);
        assert!(projected_focus(&runtime, "c"));
        runtime
            .apply(Message::Replace {
                rows: &["b", "d"],
                checked: Some("d"),
                disabled: None,
            })
            .unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), expected);
        if let Some(expected) = expected {
            assert!(projected_focus(&runtime, expected));
        }
        assert_eq!(runtime.app().presses, 0);
    }
}

#[test]
fn disabling_the_focused_row_and_removing_its_peer_uses_the_first_enabled_member() {
    let mut runtime = fixture(true, "c", "a");
    runtime
        .apply(Message::Replace {
            rows: &["b", "c", "d"],
            checked: Some("c"),
            disabled: Some("c"),
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("b"));
    assert!(projected_focus(&runtime, "b"));
    assert_eq!(runtime.app().presses, 0);
}
