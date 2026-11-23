use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    Application, Event, Runtime, ScrollTarget, VerticalScroll, VerticalScrollResponse, View,
};

#[derive(Clone, Copy, Default)]
enum Hook {
    #[default]
    Absent,
    Decline,
    Partial(f32),
    Fixed(f32),
}

#[derive(Clone)]
enum Message {
    Quiet,
    OpenModal,
    Observed(&'static str, VerticalScroll),
}

#[derive(Default)]
struct App {
    hook: Hook,
    nested: bool,
    disabled: bool,
    modal_open: bool,
    move_inner_after_wheel: bool,
    observed: Vec<(&'static str, VerticalScroll)>,
}

fn scroller(id: &'static str, height: f32, hook: Hook, disabled: bool) -> View<Message> {
    let view = View::new(
        Node::new(Role::Group, id).id(id).disabled(disabled).styled(
            Style::column()
                .width(Length::Px(100.))
                .height(Length::Px(height))
                .clip(true),
        ),
    );
    if matches!(hook, Hook::Absent) {
        return view;
    }
    view.on_vertical_scroll(move |scroll| {
        let consumed = match hook {
            Hook::Absent | Hook::Decline => return None,
            Hook::Partial(amount) => scroll.delta.signum() * amount,
            Hook::Fixed(amount) => amount,
        };
        Some(VerticalScrollResponse {
            consumed,
            message: Message::Observed(id, scroll),
        })
    })
}

fn rows(prefix: &str) -> impl Iterator<Item = View<Message>> + '_ {
    (0..15).map(move |index| {
        let id = format!("{prefix}-{index}");
        View::new(
            Node::new(Role::Button, &id).id(&id).styled(
                Style::default()
                    .width(Length::Px(200.))
                    .height(Length::Px(20.)),
            ),
        )
        .on_press(Message::Quiet)
    })
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Quiet => {}
            Message::OpenModal => self.modal_open = true,
            Message::Observed(id, scroll) => self.observed.push((id, scroll)),
        }
    }

    fn view(&self) -> View<Message> {
        let inner = self
            .nested
            .then(|| scroller("inner", 60., self.hook, false).children(rows("inner")));
        let space = (self.move_inner_after_wheel && !self.observed.is_empty()).then(|| {
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(65.))))
        });
        let outer = scroller("outer", 100., self.hook, self.disabled)
            .children(space.into_iter().chain(inner).chain(rows("outer")));
        let modal = self.modal_open.then(|| {
            View::new(
                Node::new(Role::Dialog, "Modal")
                    .id("modal")
                    .styled(Style::default().fixed(Default::default())),
            )
            .modal(Message::Quiet)
        });
        View::new(Node::new(Role::Window, "Test").styled(Style::column()))
            .children(std::iter::once(outer).chain(modal))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(320., 200., &mut Metrics).unwrap();
}

fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
}

fn wheel(runtime: &mut Runtime<App>, dx: f32, dy: f32) -> bool {
    let handled = wheel_input(runtime, dx, dy);
    render(runtime);
    handled
}

fn wheel_input(runtime: &mut Runtime<App>, dx: f32, dy: f32) -> bool {
    runtime
        .dispatch(Event::Scroll {
            x: 30.,
            y: 30.,
            delta_x: dx,
            delta_y: dy,
        })
        .unwrap()
}

fn offset(runtime: &Runtime<App>, id: &str) -> (f32, f32) {
    runtime.scroll_metrics(id).unwrap().offset
}

#[test]
fn partial_consumption_moves_the_body_and_keeps_focus_through_the_message() {
    let mut runtime = runtime(App {
        hook: Hook::Partial(12.),
        ..App::default()
    });
    runtime.activate("outer-0").unwrap();
    render(&mut runtime);

    assert!(wheel(&mut runtime, 13., 40.));
    assert_eq!(offset(&runtime, "outer"), (13., 28.));
    assert!(wheel(&mut runtime, 0., -24.));
    assert_eq!(offset(&runtime, "outer"), (13., 16.));
    assert_eq!(runtime.focused(), Some("outer-0"));
    assert_eq!(
        runtime.app().observed,
        [
            (
                "outer",
                VerticalScroll {
                    delta: 40.,
                    offset: 0.,
                    max_offset: 200.
                }
            ),
            (
                "outer",
                VerticalScroll {
                    delta: -24.,
                    offset: 28.,
                    max_offset: 200.
                }
            ),
        ]
    );
}

#[test]
fn nested_scroll_reports_both_boundaries_and_does_not_pass_leftovers_up() {
    let mut runtime = runtime(App {
        hook: Hook::Partial(12.),
        nested: true,
        ..App::default()
    });

    assert!(wheel(&mut runtime, 0., -30.));
    assert_eq!(offset(&runtime, "inner").1, 0.);
    assert!(runtime.scroll_to("inner", ScrollTarget::Bottom));
    render(&mut runtime);
    assert!(wheel(&mut runtime, 0., 30.));
    assert_eq!(offset(&runtime, "inner").1, 240.);
    assert_eq!(offset(&runtime, "outer").1, 0.);
    assert_eq!(
        runtime.app().observed,
        [
            (
                "inner",
                VerticalScroll {
                    delta: -30.,
                    offset: 0.,
                    max_offset: 240.
                }
            ),
            (
                "inner",
                VerticalScroll {
                    delta: 30.,
                    offset: 240.,
                    max_offset: 240.
                }
            ),
        ]
    );
}

#[test]
fn consumption_is_limited_to_the_finite_delta_and_its_direction() {
    for (delta, consumed, expected) in [
        (20., 100., 80.),
        (-20., -100., 80.),
        (20., -5., 100.),
        (-20., 5., 60.),
        (20., f32::NAN, 100.),
        (-20., f32::NEG_INFINITY, 60.),
    ] {
        let mut runtime = runtime(App {
            hook: Hook::Fixed(consumed),
            ..App::default()
        });
        assert!(runtime.scroll_to("outer", ScrollTarget::Offset { x: 0., y: 80. }));
        render(&mut runtime);
        assert!(wheel(&mut runtime, 0., delta));
        assert_eq!(offset(&runtime, "outer").1, expected);
        assert_eq!(runtime.app().observed.len(), 1);
    }
}

#[test]
fn no_hook_declining_hook_and_disabled_hook_leave_normal_scrolling_intact() {
    for (hook, disabled) in [
        (Hook::Absent, false),
        (Hook::Decline, false),
        (Hook::Fixed(25.), true),
    ] {
        let mut runtime = runtime(App {
            hook,
            disabled,
            ..App::default()
        });
        assert!(wheel(&mut runtime, 13., 40.));
        assert_eq!(offset(&runtime, "outer"), (13., 40.));
        assert!(runtime.app().observed.is_empty());
    }
}

#[test]
fn hook_ignores_nonvertical_invalid_and_out_of_scope_events() {
    let mut runtime = runtime(App {
        hook: Hook::Fixed(25.),
        ..App::default()
    });
    assert!(wheel(&mut runtime, 17., 0.));
    assert!(!wheel(&mut runtime, 0., f32::NAN));
    assert!(!wheel(&mut runtime, f32::INFINITY, 25.));
    assert_eq!(offset(&runtime, "outer"), (17., 0.));

    runtime.apply(Message::OpenModal).unwrap();
    render(&mut runtime);
    assert!(!wheel(&mut runtime, 0., 30.));
    assert!(runtime.app().observed.is_empty());
    assert_eq!(offset(&runtime, "outer"), (17., 0.));
}

#[test]
fn a_wheel_burst_between_frames_keeps_each_sample_and_current_offsets() {
    let mut runtime = runtime(App {
        hook: Hook::Partial(5.),
        ..App::default()
    });
    for delta in [15., 12., -7.] {
        assert!(wheel_input(&mut runtime, 0., delta));
    }
    assert_eq!(runtime.app().observed.len(), 1);
    render(&mut runtime);
    assert_eq!(
        runtime
            .app()
            .observed
            .iter()
            .map(|(target, scroll)| (*target, scroll.delta, scroll.offset))
            .collect::<Vec<_>>(),
        [("outer", 15., 0.), ("outer", 12., 10.), ("outer", -7., 17.)]
    );
    assert_eq!(offset(&runtime, "outer"), (0., 15.));
}

#[test]
fn a_moved_or_newly_out_of_scope_owner_never_retargets_queued_wheel_samples() {
    let mut moved = runtime(App {
        hook: Hook::Partial(50.),
        nested: true,
        move_inner_after_wheel: true,
        ..App::default()
    });
    assert!(wheel_input(&mut moved, 0., 10.));
    assert!(wheel_input(&mut moved, 0., 10.));
    render(&mut moved);
    assert!(matches!(moved.app().observed.as_slice(), [("inner", _)]));
    assert!(wheel(&mut moved, 0., 10.));
    assert!(matches!(
        moved.app().observed.as_slice(),
        [("inner", _), ("outer", _)]
    ));

    let mut modal = runtime(App {
        hook: Hook::Partial(30.),
        ..App::default()
    });
    assert!(wheel_input(&mut modal, 0., 10.));
    assert!(wheel_input(&mut modal, 0., 10.));
    modal.apply(Message::OpenModal).unwrap();
    render(&mut modal);
    assert_eq!(modal.app().observed.len(), 1);
    assert!(!wheel(&mut modal, 0., 10.));
    assert_eq!(modal.app().observed.len(), 1);
}

#[test]
fn a_pending_app_update_can_replace_the_route_before_replay() {
    let mut runtime = runtime(App {
        hook: Hook::Partial(30.),
        ..App::default()
    });
    assert!(wheel_input(&mut runtime, 0., 10.));
    assert!(wheel_input(&mut runtime, 0., 10.));
    runtime.queue().post(Message::OpenModal);
    render(&mut runtime);
    runtime.flush().unwrap();
    render(&mut runtime);
    assert!(runtime.app().modal_open);
    assert_eq!(runtime.app().observed.len(), 1);
    assert!(!wheel(&mut runtime, 0., 10.));
}
