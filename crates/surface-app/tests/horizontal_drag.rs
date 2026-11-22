use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, Event, HorizontalDrag, HorizontalDragSession, Runtime, View};

#[derive(Clone)]
enum Message {
    Open,
    Dismiss,
    Press,
    Place { width: f32, slide: f32 },
    Drag(HorizontalDrag, Rect),
}

struct App {
    open: bool,
    width: f32,
    slide: f32,
    session: HorizontalDragSession,
    selected: usize,
    dismissals: usize,
    drag: Vec<(HorizontalDrag, Rect)>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Dismiss => {
                self.open = false;
                self.dismissals += 1;
            }
            Message::Press => self.selected += 1,
            Message::Place { width, slide } => {
                self.width = width;
                self.slide = slide;
            }
            Message::Drag(drag, bounds) => self.drag.push((drag, bounds)),
        }
    }

    fn view(&self) -> View<Message> {
        let opener = View::new(
            Node::new(Role::Button, "Show").id("open").styled(
                Style::default()
                    .width(Length::Px(80.))
                    .height(Length::Px(48.)),
            ),
        )
        .on_press(Message::Open);
        let panel = self.open.then(|| {
            let close = row("close", Message::Dismiss);
            let destination = row("destination", Message::Press);
            View::new(
                Node::new(Role::Dialog, "Navigation")
                    .id("dialog")
                    .slide_horizontally(self.slide)
                    .styled(
                        Style::column()
                            .fixed(Default::default())
                            .width(Length::Px(self.width))
                            .height(Length::Percent(1.))
                            .clip(true),
                    ),
            )
            .children([close, destination])
            .on_horizontal_drag_with_bounds(self.session.clone(), Message::Drag)
            .modal_with_backdrop(
                Node::new(Role::Group, "").id("scrim").styled(
                    Style::default()
                        .fixed(Default::default())
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.)),
                ),
                Message::Dismiss,
            )
        });
        View::new(Node::new(Role::Window, "Gallery")).children(std::iter::once(opener).chain(panel))
    }
}

fn row(id: &str, message: Message) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .styled(Style::default().height(Length::Px(48.))),
    )
    .on_press(message)
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render_extended(320., 240., &mut Metrics).unwrap();
}

fn runtime() -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        open: false,
        width: 152.,
        slide: -0.25,
        session: HorizontalDragSession::new(),
        selected: 0,
        dismissals: 0,
        drag: Vec::new(),
    })
    .unwrap();
    runtime.activate("open").unwrap();
    render(&mut runtime);
    runtime
}

fn pointer(runtime: &mut Runtime<App>, event: Event) {
    runtime.dispatch(event).unwrap();
    render(runtime);
}

#[test]
fn measured_bounds_survive_motion_release_and_cancellation() {
    let mut runtime = runtime();
    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    runtime
        .apply(Message::Place {
            width: 112.,
            slide: -0.5,
        })
        .unwrap();
    render(&mut runtime);
    pointer(&mut runtime, Event::PointerMove { x: 50., y: 69. });
    pointer(&mut runtime, Event::PointerUp { x: 90., y: 70. });
    let pressed = Rect::new(-38., 0., 152., 240.);
    assert_eq!(
        runtime.app().drag,
        [
            (HorizontalDrag::Move { delta_x: 20. }, pressed),
            (
                HorizontalDrag::End {
                    delta_x: 60.,
                    velocity_x: 0.,
                },
                pressed,
            ),
        ]
    );

    pointer(&mut runtime, Event::PointerDown { x: 24., y: 68. });
    pointer(&mut runtime, Event::PointerMove { x: 54., y: 70. });
    runtime
        .apply(Message::Place {
            width: 200.,
            slide: 0.,
        })
        .unwrap();
    render(&mut runtime);
    assert!(runtime.cancel_pointer().unwrap());
    let pressed = Rect::new(-56., 0., 112., 240.);
    assert_eq!(
        &runtime.app().drag[2..],
        [
            (HorizontalDrag::Move { delta_x: 30. }, pressed),
            (HorizontalDrag::Cancel, pressed),
        ]
    );
    assert_eq!(runtime.app().selected, 0);
    assert_eq!(runtime.focused(), Some("close"));
}

#[test]
fn a_tap_selects_once_and_a_vertical_gesture_cannot_become_a_press() {
    let mut runtime = runtime();
    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    pointer(&mut runtime, Event::PointerMove { x: 30., y: 82. });
    pointer(&mut runtime, Event::PointerUp { x: 30., y: 82. });
    assert!(runtime.app().drag.is_empty());
    assert_eq!(runtime.app().selected, 0);

    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    pointer(&mut runtime, Event::PointerUp { x: 32., y: 69. });
    assert_eq!(runtime.app().selected, 1);
    assert!(runtime.app().drag.is_empty());
}

#[test]
fn tab_cancels_a_live_drag_before_moving_focus_inside_the_modal() {
    let mut runtime = runtime();
    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    pointer(&mut runtime, Event::PointerMove { x: 50., y: 68. });
    pointer(&mut runtime, Event::FocusNext { reverse: false });
    assert_eq!(runtime.focused(), Some("destination"));
    assert_eq!(runtime.app().drag[1].0, HorizontalDrag::Cancel);
    pointer(&mut runtime, Event::PointerUp { x: 50., y: 68. });
    assert_eq!(runtime.app().drag.len(), 2);
    assert_eq!(runtime.app().selected, 0);
    assert_eq!(runtime.app().dismissals, 0);
}

#[test]
fn first_escape_cancels_the_swipe_second_escape_closes_the_modal() {
    let mut runtime = runtime();
    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    pointer(&mut runtime, Event::PointerMove { x: 50., y: 68. });
    pointer(&mut runtime, Event::Cancel);
    assert_eq!(runtime.app().drag[1].0, HorizontalDrag::Cancel);
    assert_eq!(runtime.app().dismissals, 0);
    assert_eq!(runtime.focused(), Some("close"));
    pointer(&mut runtime, Event::PointerUp { x: 50., y: 68. });
    pointer(&mut runtime, Event::Cancel);
    assert_eq!(runtime.app().dismissals, 1);
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn an_early_resize_cancels_capture_and_requires_new_layout_for_another_drag() {
    let mut runtime = runtime();
    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    pointer(&mut runtime, Event::PointerMove { x: 50., y: 68. });
    assert!(runtime.pointer_geometry_changed().unwrap());
    assert_eq!(runtime.app().drag[1].0, HorizontalDrag::Cancel);
    runtime
        .dispatch(Event::PointerUp { x: 50., y: 68. })
        .unwrap();
    runtime
        .dispatch(Event::PointerDown { x: 30., y: 68. })
        .unwrap();
    runtime
        .dispatch(Event::PointerMove { x: 70., y: 68. })
        .unwrap();
    assert_eq!(runtime.app().drag.len(), 2);
    render(&mut runtime);
    pointer(&mut runtime, Event::PointerDown { x: 30., y: 68. });
    pointer(&mut runtime, Event::PointerMove { x: 50., y: 68. });
    assert_eq!(runtime.app().drag.len(), 3);
    assert_eq!(runtime.app().selected, 0);
}
