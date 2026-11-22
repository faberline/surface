use std::cell::Cell;
use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    Application, Event, HorizontalScroll, HorizontalScrollResponse, Runtime, VerticalScroll,
    VerticalScrollResponse, View,
};

#[derive(Clone, Copy, Default)]
enum Wheel {
    #[default]
    Observe,
    Managed(f32),
    Stock,
    Unmarked,
}

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Horizontal(HorizontalScroll, Option<String>),
    Vertical(&'static str, VerticalScroll),
    Close,
}

#[derive(Default)]
struct App {
    wheel: Wheel,
    child_scroller: bool,
    parent_mask: bool,
    own_y: bool,
    modal: bool,
    views: Cell<usize>,
    events: Vec<Message>,
}

fn clip(id: &str, width: f32, height: f32) -> View<Message> {
    View::new(
        Node::new(Role::Group, id).id(id).styled(
            Style::column()
                .width(Length::Px(width))
                .height(Length::Px(height))
                .clip(true),
        ),
    )
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        self.events.push(message);
    }

    fn view(&self) -> View<Message> {
        self.views.set(self.views.get() + 1);
        let card = View::new(
            Node::new(Role::Button, "Card")
                .id("card")
                .styled(Style::default().height(Length::Px(20.))),
        );
        let overflow = self.child_scroller.then(|| {
            View::new(
                Node::new(Role::Group, "More").styled(Style::default().height(Length::Px(80.))),
            )
        });
        let mask = clip("mask", 220., 38.).children(std::iter::once(card).chain(overflow));
        let mut carousel = clip("carousel", 96., 60.).children([mask]);
        let wheel = self.wheel;
        if !matches!(wheel, Wheel::Unmarked) {
            carousel = carousel.on_horizontal_scroll(move |scroll, focus| {
                let offset = match wheel {
                    Wheel::Observe => None,
                    Wheel::Managed(offset) => Some(offset),
                    Wheel::Stock | Wheel::Unmarked => return None,
                };
                Some(HorizontalScrollResponse {
                    offset,
                    message: Message::Horizontal(scroll, focus.map(str::to_owned)),
                })
            });
        }
        if self.own_y {
            carousel = carousel.on_vertical_scroll(|scroll| {
                Some(VerticalScrollResponse {
                    consumed: scroll.delta,
                    message: Message::Vertical("carousel", scroll),
                })
            });
        }
        if self.parent_mask {
            carousel = clip("parent-mask", 120., 60.).children([carousel]);
        }
        let body = clip("body", 120., 100.)
            .children([
                carousel,
                View::new(
                    Node::new(Role::Group, "Rest")
                        .styled(Style::default().height(Length::Px(240.))),
                ),
            ])
            .on_vertical_scroll(|scroll| {
                Some(VerticalScrollResponse {
                    consumed: 5.,
                    message: Message::Vertical("body", scroll),
                })
            });
        let modal = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Modal").id("modal")).modal_overlay(Message::Close)
        });
        View::new(Node::new(Role::Window, "Test").styled(Style::column()))
            .children(std::iter::once(body).chain(modal))
    }
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

fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
}

fn wheel(runtime: &mut Runtime<App>, delta_x: f32, delta_y: f32) -> bool {
    runtime
        .dispatch(Event::Scroll {
            x: 20.,
            y: 30.,
            delta_x,
            delta_y,
        })
        .unwrap()
}

fn offset(runtime: &Runtime<App>, id: &str) -> (f32, f32) {
    runtime.scroll_metrics(id).unwrap().offset
}

#[test]
fn masked_view_observes_physical_x_with_clock_and_focus() {
    let mut runtime = runtime(App::default());
    runtime.activate("card").unwrap();
    render(&mut runtime);
    runtime.set_time(75.);
    assert!(wheel(&mut runtime, -16., 0.));
    render(&mut runtime);
    assert_eq!(offset(&runtime, "carousel"), (0., 0.));
    assert_eq!(offset(&runtime, "body"), (0., 0.));
    assert_eq!(
        runtime.app().events,
        [Message::Horizontal(
            HorizontalScroll {
                delta: -16.,
                offset: 0.,
                max_offset: 124.,
                now_ms: Some(75.),
            },
            Some("card".into())
        )]
    );
}

#[test]
fn a_diagonal_event_saves_both_axes_before_one_view_rebuild() {
    let mut runtime = runtime(App {
        wheel: Wheel::Managed(20.),
        ..App::default()
    });
    let views_before = runtime.app().views.get();
    assert!(wheel(&mut runtime, 7., 19.));
    assert_eq!(runtime.app().views.get(), views_before + 1);
    render(&mut runtime);
    assert_eq!(offset(&runtime, "carousel"), (20., 0.));
    assert_eq!(offset(&runtime, "body"), (0., 14.));
    assert!(matches!(
        runtime.app().events.as_slice(),
        [
            Message::Horizontal(HorizontalScroll { delta: 7., .. }, None),
            Message::Vertical("body", VerticalScroll { delta: 19., .. }),
        ]
    ));

    assert!(wheel(&mut runtime, 0., 10.));
    render(&mut runtime);
    assert_eq!(offset(&runtime, "body"), (0., 19.));
    assert!(matches!(
        runtime.app().events.last(),
        Some(Message::Vertical(
            "body",
            VerticalScroll {
                delta: 10.,
                offset: 14.,
                ..
            }
        ))
    ));
}

#[test]
fn vertical_input_can_reach_a_real_child_or_an_explicit_same_view_hook() {
    let mut child = runtime(App {
        child_scroller: true,
        ..App::default()
    });
    assert!(wheel(&mut child, 0., 20.));
    render(&mut child);
    assert_eq!(offset(&child, "mask"), (0., 20.));
    assert_eq!(offset(&child, "body"), (0., 0.));
    assert!(child.app().events.is_empty());

    let mut both = runtime(App {
        own_y: true,
        ..App::default()
    });
    assert!(wheel(&mut both, 0., 20.));
    render(&mut both);
    assert_eq!(offset(&both, "carousel"), (0., 0.));
    assert_eq!(offset(&both, "body"), (0., 0.));
    assert!(matches!(
        both.app().events.as_slice(),
        [Message::Vertical("carousel", _)]
    ));
}

#[test]
fn a_clipped_wrapper_does_not_take_y_from_the_scrollable_parent() {
    let mut runtime = runtime(App {
        parent_mask: true,
        ..App::default()
    });
    assert!(wheel(&mut runtime, 0., 20.));
    render(&mut runtime);
    assert_eq!(offset(&runtime, "parent-mask"), (0., 0.));
    assert_eq!(offset(&runtime, "body"), (0., 15.));
}

#[test]
fn unmarked_routing_stays_unchanged_and_modal_scope_blocks_the_background() {
    let mut unmarked = runtime(App {
        wheel: Wheel::Unmarked,
        ..App::default()
    });
    assert!(wheel(&mut unmarked, 14., 20.));
    render(&mut unmarked);
    assert_eq!(offset(&unmarked, "carousel"), (0., 0.));
    assert_eq!(offset(&unmarked, "body"), (0., 0.));
    assert!(unmarked.app().events.is_empty());

    let mut modal = runtime(App {
        modal: true,
        ..App::default()
    });
    assert!(!wheel(&mut modal, 14., 20.));
    assert!(modal.app().events.is_empty());
}

#[test]
fn stock_and_finite_managed_x_offsets_are_bounded() {
    for (policy, expected) in [
        (Wheel::Stock, 16.),
        (Wheel::Managed(500.), 124.),
        (Wheel::Managed(f32::NAN), 0.),
    ] {
        let mut runtime = runtime(App {
            wheel: policy,
            ..App::default()
        });
        assert!(!wheel(&mut runtime, f32::NAN, 5.));
        assert!(wheel(&mut runtime, 16., 0.));
        render(&mut runtime);
        assert_eq!(offset(&runtime, "carousel").0, expected);
    }
}
