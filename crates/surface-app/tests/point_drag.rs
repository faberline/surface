use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, Application, Event, FrameInput, HorizontalDragSession, InputAnimationUpdate,
    PointDrag, PointDragSession, Runtime, View,
};

#[derive(Clone)]
enum Message {
    Point(PointDrag, Rect),
    Press(&'static str),
    Axis,
    Modal(bool),
    Replace,
    Widen,
}

#[derive(Default)]
struct App {
    session: PointDragSession,
    axis: HorizontalDragSession,
    points: Vec<(PointDrag, Rect)>,
    presses: Vec<&'static str>,
    axis_events: usize,
    modal: bool,
    wide: bool,
    ignore_point_messages: bool,
    suppress_touch_activation: bool,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Point(event, bounds) => self.points.push((event, bounds)),
            Message::Press(id) => self.presses.push(id),
            Message::Axis => self.axis_events += 1,
            Message::Modal(open) => self.modal = open,
            Message::Replace => self.session = PointDragSession::new(),
            Message::Widen => self.wide = true,
        }
    }

    fn view(&self) -> View<Message> {
        let use_message = !self.ignore_point_messages;
        let dial = View::new(
            Node::new(Role::Group, "Dial").id("dial").styled(
                Style::row()
                    .width(Length::Px(if self.wide { 128. } else { 96. }))
                    .height(Length::Px(80.)),
            ),
        )
        .on_horizontal_drag(self.axis.clone(), |_| Message::Axis)
        .on_point_drag_with_bounds(self.session.clone(), move |point, bounds| {
            use_message.then_some(Message::Point(point, bounds))
        })
        .children([button("dial-option")]);
        let modal = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Modal").id("dialog"))
                .children([button("dialog-action")])
                .modal_overlay(Message::Modal(false))
        });
        View::new(Node::new(Role::Window, "Fixture").styled(Style::row().gap(8.))).children(
            [
                dial,
                button("neighbor").on_horizontal_drag(self.axis.clone(), |_| Message::Axis),
            ]
            .into_iter()
            .chain(modal),
        )
    }

    fn observes_frame_input(&self) -> bool {
        self.suppress_touch_activation
    }

    fn animation_frame_with_input(
        &mut self,
        _: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            suppress_touch_activation: self.suppress_touch_activation && input.touch.is_some(),
            ..InputAnimationUpdate::default()
        }
    }
}

fn button(id: &'static str) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .styled(Style::row().width(Length::Px(72.)).height(Length::Px(72.))),
    )
    .on_press(Message::Press(id))
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    runtime.render(320., 160., &mut Metrics).unwrap();
}

fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    draw(&mut runtime);
    runtime
}

fn rect(runtime: &Runtime<App>, target: &str) -> Rect {
    let id = runtime.dom().by_id(target).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn down(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
}

fn moved(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
}

fn up(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn tap_delivers_coordinates_without_child_press_and_preserves_semantic_activation() {
    let mut runtime = runtime(App::default());
    let bounds = rect(&runtime, "dial");
    down(&mut runtime, (24., 24.));
    moved(&mut runtime, (25., 25.));
    up(&mut runtime, (25., 25.));
    up(&mut runtime, (25., 25.));
    assert_eq!(
        runtime.app().points,
        [(
            PointDrag::Tap {
                origin: (24., 24.),
                position: (25., 25.)
            },
            bounds
        )]
    );
    assert!(runtime.app().presses.is_empty());
    assert!(runtime.activate("dial-option").unwrap());
    assert_eq!(runtime.app().presses, ["dial-option"]);
}

#[test]
fn exact_diagonal_claims_both_coordinates_and_keeps_bounds_across_rebuild() {
    let mut runtime = runtime(App::default());
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    assert_eq!(runtime.focused(), Some("neighbor"));
    let bounds = rect(&runtime, "dial");
    let origin = (24., 24.);
    down(&mut runtime, origin);
    moved(&mut runtime, (29., 29.));
    runtime.apply(Message::Widen).unwrap();
    draw(&mut runtime);
    assert!(rect(&runtime, "dial").width > bounds.width);
    moved(&mut runtime, (29., 48.));
    draw(&mut runtime);
    let neighbor = rect(&runtime, "neighbor");
    let end = (neighbor.x + 16., neighbor.y + 16.);
    up(&mut runtime, end);
    assert_eq!(
        runtime.app().points,
        [
            (
                PointDrag::Move {
                    origin,
                    position: (29., 29.)
                },
                bounds
            ),
            (
                PointDrag::Move {
                    origin,
                    position: (29., 48.)
                },
                bounds
            ),
            (
                PointDrag::End {
                    origin,
                    position: end
                },
                bounds
            ),
        ]
    );
    assert_eq!(runtime.app().axis_events, 0);
    assert!(runtime.app().presses.is_empty());
    assert_eq!(runtime.focused(), Some("neighbor"));
}

#[test]
fn release_only_claims_once_and_under_slop_release_requires_the_same_visible_owner() {
    let mut runtime = runtime(App::default());
    let bounds = rect(&runtime, "dial");
    let from_edge = (bounds.x + bounds.width - 1., bounds.y + 20.);
    down(&mut runtime, from_edge);
    up(&mut runtime, (from_edge.0 + 3., from_edge.1));
    assert!(runtime.app().points.is_empty());

    let origin = (24., 24.);
    down(&mut runtime, origin);
    up(&mut runtime, (29., 29.));
    assert_eq!(
        runtime.app().points,
        [(
            PointDrag::End {
                origin,
                position: (29., 29.)
            },
            bounds
        )]
    );
    assert!(runtime.app().presses.is_empty());

    draw(&mut runtime);
    let sibling = rect(&runtime, "neighbor");
    let point = (sibling.x + 16., sibling.y + 16.);
    down(&mut runtime, point);
    up(&mut runtime, point);
    assert_eq!(runtime.app().presses, ["neighbor"]);
    assert_eq!(runtime.app().points.len(), 1);
}

#[test]
fn rejected_callback_and_suppressed_touch_cannot_invoke_stock_actions() {
    let mut rejected = runtime(App {
        ignore_point_messages: true,
        ..App::default()
    });
    down(&mut rejected, (24., 24.));
    up(&mut rejected, (24., 24.));
    assert!(rejected.app().presses.is_empty());

    let mut touch = runtime(App {
        suppress_touch_activation: true,
        ..App::default()
    });
    for (x, y) in [(24., 24.), (29., 29.)] {
        touch
            .dispatch_touch(Event::PointerDown { x: 24., y: 24. })
            .unwrap();
        draw(&mut touch);
        touch.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    }
    assert!(touch.app().points.is_empty() && touch.app().presses.is_empty());

    touch
        .dispatch_touch(Event::PointerDown { x: 24., y: 24. })
        .unwrap();
    draw(&mut touch);
    touch
        .dispatch_touch(Event::PointerMove { x: 29., y: 29. })
        .unwrap();
    touch
        .dispatch_touch(Event::PointerUp { x: 30., y: 30. })
        .unwrap();
    assert!(matches!(
        touch.app().points.as_slice(),
        [(PointDrag::Move { .. }, _), (PointDrag::End { .. }, _)]
    ));
    assert!(touch.app().presses.is_empty());
}

#[test]
fn scope_loss_or_resize_cancels_matching_preview_once() {
    for loss in [Some(Message::Modal(true)), None] {
        let mut runtime = runtime(App::default());
        let bounds = rect(&runtime, "dial");
        down(&mut runtime, (24., 24.));
        moved(&mut runtime, (29., 29.));
        match loss {
            Some(message) => runtime.apply(message).unwrap(),
            None => {
                assert!(runtime.pointer_geometry_changed().unwrap());
            }
        }
        up(&mut runtime, (29., 29.));
        assert_eq!(runtime.app().points.len(), 2);
        assert_eq!(runtime.app().points[1], (PointDrag::Cancel, bounds));
        assert!(runtime.app().presses.is_empty());
    }
}

#[test]
fn replacing_the_session_cannot_deliver_a_stale_terminal() {
    let mut runtime = runtime(App::default());
    down(&mut runtime, (24., 24.));
    moved(&mut runtime, (29., 29.));
    runtime.apply(Message::Replace).unwrap();
    draw(&mut runtime);
    up(&mut runtime, (29., 29.));
    assert_eq!(runtime.app().points.len(), 1);
    assert!(runtime.app().presses.is_empty());

    down(&mut runtime, (24., 24.));
    up(&mut runtime, (24., 24.));
    assert_eq!(runtime.app().points.len(), 2);
    assert!(matches!(runtime.app().points[1].0, PointDrag::Tap { .. }));
}

#[test]
fn host_loss_and_invalid_release_cancel_without_second_terminal() {
    for host_loss in [true, false] {
        let mut runtime = runtime(App::default());
        let bounds = rect(&runtime, "dial");
        down(&mut runtime, (24., 24.));
        moved(&mut runtime, (29., 29.));
        if host_loss {
            assert!(runtime.cancel_point_drag().unwrap());
            assert!(!runtime.cancel_point_drag().unwrap());
        } else {
            up(&mut runtime, (f32::NAN, 29.));
        }
        up(&mut runtime, (29., 29.));
        assert_eq!(runtime.app().points.len(), 2);
        assert_eq!(runtime.app().points[1], (PointDrag::Cancel, bounds));
        assert!(runtime.app().presses.is_empty());
    }
}

#[test]
fn host_loss_clears_an_unclaimed_point_without_retiring_an_axis_drag() {
    let mut runtime = runtime(App::default());
    down(&mut runtime, (24., 24.));
    assert!(runtime.has_point_drag());
    assert!(runtime.cancel_point_drag().unwrap());
    assert!(!runtime.has_point_drag());
    up(&mut runtime, (24., 24.));
    assert!(runtime.app().points.is_empty());
    assert!(runtime.app().presses.is_empty());

    let neighbor = rect(&runtime, "neighbor");
    let origin = (neighbor.x + 16., neighbor.y + 16.);
    down(&mut runtime, origin);
    assert!(!runtime.has_point_drag());
    assert!(!runtime.cancel_point_drag().unwrap());
    moved(&mut runtime, (origin.0 + 14., origin.1));
    assert!(!runtime.cancel_point_drag().unwrap());
    up(&mut runtime, (origin.0 + 20., origin.1));
    assert_eq!(runtime.app().axis_events, 2);
    assert!(runtime.app().points.is_empty());
    assert!(runtime.app().presses.is_empty());
}
