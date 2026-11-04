use surface::{Direction, Insets, Length, Node, RangeValue, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Roving, Runtime, View};

#[derive(Default)]
struct App {
    radio: usize,
    tab: usize,
    value: f64,
    dialog: bool,
    nested: bool,
    opens: usize,
}
#[derive(Clone)]
enum Message {
    Radio(usize),
    Tab(usize),
    Value(f64),
    Open,
    Close,
    Nested(bool),
}
fn button(id: &str, message: Message) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .text(id)
            .styled(Style::default().height(Length::Px(32.))),
    )
    .on_press(message)
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, m: Message) {
        match m {
            Message::Radio(i) => self.radio = i,
            Message::Tab(i) => self.tab = i,
            Message::Value(v) => self.value = v,
            Message::Open => {
                self.dialog = true;
                self.opens += 1;
            }
            Message::Close => {
                self.dialog = false;
                self.nested = false;
            }
            Message::Nested(v) => self.nested = v,
        }
    }
    fn view(&self) -> View<Message> {
        let mut children = vec![
            button("open", Message::Open),
            View::new(
                Node::new(Role::RadioGroup, "Choice")
                    .id("radios")
                    .styled(Style::row()),
            )
            .roving(Roving::vertical(true))
            .children((0..3).map(|i| {
                View::new(
                    Node::new(Role::Radio, format!("radio{i}"))
                        .id(format!("radio{i}"))
                        .text(format!("R{i}"))
                        .checked(self.radio == i)
                        .disabled(i == 1),
                )
                .on_press(Message::Radio(i))
            })),
            View::new(
                Node::new(Role::TabList, "Tabs")
                    .id("tabs")
                    .tab_index(-1)
                    .styled(Style::row()),
            )
            .roving(Roving {
                direction: Direction::Row,
                automatic: false,
            })
            .children((0..3).map(|i| {
                View::new(
                    Node::new(Role::Tab, format!("tab{i}"))
                        .id(format!("tab{i}"))
                        .text(format!("T{i}"))
                        .selected(self.tab == i)
                        .disabled(i == 1),
                )
                .on_press(Message::Tab(i))
            })),
            View::new(
                Node::new(Role::Slider, "Volume")
                    .id("slider")
                    .range(RangeValue {
                        min: 10.,
                        max: 20.,
                        step: 2.,
                        value: self.value,
                    })
                    .styled(
                        Style::default()
                            .width(Length::Px(200.))
                            .height(Length::Px(48.))
                            .padding(10.),
                    ),
            )
            .on_value(Message::Value),
        ];
        let panel_style = |left: f32| {
            Style::column()
                .width(Length::Px(240.))
                .height(Length::Px(180.))
                .absolute(Insets {
                    left: Length::Px(left),
                    top: Length::Px(30.),
                    ..Default::default()
                })
        };
        if self.dialog {
            children.push(
                View::new(
                    Node::new(Role::Dialog, "Dialog")
                        .id("dialog")
                        .tab_index(-1)
                        .styled(panel_style(40.)),
                )
                .children([
                    button("close", Message::Close),
                    button("nested-open", Message::Nested(true)),
                ])
                .modal(Message::Close),
            );
        }
        if self.nested {
            children.push(
                View::new(
                    Node::new(Role::Dialog, "Nested dialog")
                        .id("nested")
                        .tab_index(-1)
                        .styled(panel_style(50.)),
                )
                .children([button("nested-close", Message::Nested(false))])
                .modal(Message::Nested(false)),
            );
        }
        View::new(Node::new(Role::Window, "Controls").styled(Style::column().gap(8.)))
            .children(children)
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, s: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (s.len() as f32 * size / 2., size * 1.5)
    }
}
fn runtime() -> Runtime<App> {
    let mut r = Runtime::new(App {
        value: 12.,
        ..Default::default()
    })
    .unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r
}
fn key(r: &mut Runtime<App>, key: EditKey) {
    r.dispatch(Event::Edit {
        key,
        extend: false,
        by_word: false,
    })
    .unwrap();
}
fn tab(r: &mut Runtime<App>) {
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
}

#[test]
fn groups_have_one_tab_stop_skip_disabled_and_separate_focus_from_selection() {
    let mut r = runtime();
    tab(&mut r);
    assert_eq!(r.focused(), Some("open"));
    tab(&mut r);
    assert_eq!(r.focused(), Some("radio0"));
    key(&mut r, EditKey::Down);
    assert_eq!(r.focused(), Some("radio2"));
    assert_eq!(r.app().radio, 2);
    tab(&mut r);
    assert_eq!(r.focused(), Some("tab0"));
    key(&mut r, EditKey::Right);
    assert_eq!(r.focused(), Some("tab2"));
    assert_eq!(r.app().tab, 0);
    r.dispatch(Event::Activate).unwrap();
    assert_eq!(r.app().tab, 2);
    tab(&mut r);
    assert_eq!(r.focused(), Some("slider"));
    key(&mut r, EditKey::Left);
    assert_eq!(r.app().value, 10.);
    key(&mut r, EditKey::End);
    assert_eq!(r.app().value, 20.);
    key(&mut r, EditKey::Right);
    assert_eq!(r.app().value, 20.);
    key(&mut r, EditKey::Home);
    assert_eq!(r.app().value, 10.);
}
#[test]
fn arrows_enter_a_focused_group_container_without_activating_a_tab() {
    let mut r = runtime();
    r.activate("tabs").unwrap();
    key(&mut r, EditKey::Right);
    assert_eq!(r.focused(), Some("tab0"));
    r.activate("tabs").unwrap();
    key(&mut r, EditKey::Left);
    assert_eq!(r.focused(), Some("tab2"));
    assert_eq!(r.app().tab, 0);
}
#[test]
fn slider_drag_keeps_capture_across_rebuilds_and_clamps_outside_release() {
    let mut r = runtime();
    let id = r.dom().by_id("slider").unwrap();
    let b = r.computed().unwrap().nodes[id.index()].bounds;
    r.dispatch(Event::PointerDown {
        x: b.x + 10.,
        y: b.y + 24.,
    })
    .unwrap();
    assert_eq!(r.app().value, 10.);
    assert!(
        r.computed().is_none(),
        "state update invalidated the old layout"
    );
    r.dispatch(Event::PointerMove {
        x: b.x + b.width * 0.5,
        y: b.y + 24.,
    })
    .unwrap();
    assert_eq!(r.app().value, 16.);
    r.dispatch(Event::PointerUp {
        x: b.x + b.width + 500.,
        y: 999.,
    })
    .unwrap();
    assert_eq!(r.app().value, 20.);
    assert_eq!(r.app().opens, 0);
    r.set_value("slider", 13.).unwrap();
    assert_eq!(r.app().value, 14.);
    assert!(!r.set_value("slider", f64::NAN).unwrap());
    assert_eq!(r.app().value, 14.);
}
#[test]
fn nested_modals_trap_focus_block_background_and_restore_each_opener() {
    let mut r = runtime();
    r.activate("open").unwrap();
    assert_eq!(r.focused(), Some("close"));
    assert!(r.activate("radio2").is_err());
    assert!(r.set_value("slider", 18.).is_err());
    tab(&mut r);
    assert_eq!(r.focused(), Some("nested-open"));
    tab(&mut r);
    assert_eq!(r.focused(), Some("close"));
    r.activate("nested-open").unwrap();
    assert_eq!(r.focused(), Some("nested-close"));
    tab(&mut r);
    assert_eq!(r.focused(), Some("nested-close"));
    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().nested);
    assert!(r.app().dialog);
    assert_eq!(r.focused(), Some("nested-open"));
    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().dialog);
    assert_eq!(r.focused(), Some("open"));
}
#[test]
fn outside_click_dismisses_without_activating_the_control_under_it() {
    let mut r = runtime();
    r.activate("open").unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r.dispatch(Event::PointerDown { x: 1., y: 1. }).unwrap();
    r.dispatch(Event::PointerUp { x: 1., y: 1. }).unwrap();
    assert!(!r.app().dialog);
    assert_eq!(r.app().opens, 1);
    assert_eq!(r.focused(), Some("open"));
}
#[test]
fn invalid_numeric_contract_is_rejected_before_it_reaches_layout() {
    assert!(Runtime::new(App {
        value: f64::NAN,
        ..Default::default()
    })
    .is_err());
    assert!(!RangeValue {
        min: -1e308,
        max: 1e308,
        value: 0.,
        step: 1.
    }
    .is_valid());
}
