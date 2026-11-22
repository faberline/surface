use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    Application, Event, HorizontalDrag, HorizontalDragSession, Runtime, VerticalDrag,
    VerticalDragSession, View,
};

#[derive(Clone)]
enum Message {
    Press,
    Grow,
    ReplaceSession,
    Horizontal(HorizontalDrag),
    Vertical(VerticalDrag, Rect),
}

struct App {
    handle_height: f32,
    vertical: VerticalDragSession,
    horizontal: Option<HorizontalDragSession>,
    vertical_events: Vec<(VerticalDrag, Rect)>,
    horizontal_events: Vec<HorizontalDrag>,
    presses: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press => self.presses += 1,
            Message::Grow => self.handle_height = 72.,
            Message::ReplaceSession => self.vertical = VerticalDragSession::new(),
            Message::Horizontal(drag) => self.horizontal_events.push(drag),
            Message::Vertical(drag, bounds) => self.vertical_events.push((drag, bounds)),
        }
    }

    fn view(&self) -> View<Message> {
        let mut handle = View::new(
            Node::new(Role::Button, "Resize sheet").id("handle").styled(
                Style::default()
                    .width(Length::Px(160.))
                    .height(Length::Px(self.handle_height)),
            ),
        )
        .on_press(Message::Press)
        .on_vertical_drag_with_bounds(self.vertical.clone(), Message::Vertical);
        if let Some(session) = &self.horizontal {
            handle = handle.on_horizontal_drag(session.clone(), Message::Horizontal);
        }

        let content = View::new(
            Node::new(Role::Group, "Content").id("content").styled(
                Style::column()
                    .width(Length::Px(160.))
                    .height(Length::Px(80.))
                    .clip(true),
            ),
        )
        .children((0..8).map(|_| {
            View::new(
                Node::new(Role::Group, "Row").styled(Style::default().height(Length::Px(30.))),
            )
        }));

        View::new(Node::new(Role::Window, "Gallery").styled(Style::column()))
            .children([handle, content])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(320., 240., &mut Metrics).unwrap();
}

fn runtime(both_axes: bool) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        handle_height: 48.,
        vertical: VerticalDragSession::new(),
        horizontal: both_axes.then(HorizontalDragSession::new),
        vertical_events: Vec::new(),
        horizontal_events: Vec::new(),
        presses: 0,
    })
    .unwrap();
    render(&mut runtime);
    runtime
}

fn pointer(runtime: &mut Runtime<App>, event: Event, now_ms: f64) {
    runtime.set_time(now_ms);
    runtime.dispatch(event).unwrap();
    render(runtime);
}

#[test]
fn dominant_axis_stays_claimed_and_uses_the_original_bounds_and_vertical_velocity() {
    let mut runtime = runtime(true);
    pointer(&mut runtime, Event::PointerDown { x: 20., y: 20. }, 0.);
    pointer(&mut runtime, Event::PointerMove { x: 21., y: 16. }, 10.);
    assert!(runtime.app().vertical_events.is_empty());

    pointer(&mut runtime, Event::PointerMove { x: 22., y: 12. }, 20.);
    runtime.apply(Message::Grow).unwrap();
    render(&mut runtime);
    pointer(&mut runtime, Event::PointerUp { x: 200., y: -8. }, 40.);

    let bounds = Rect::new(0., 0., 160., 48.);
    assert_eq!(
        runtime.app().vertical_events,
        [
            (VerticalDrag::Move { delta_y: -8. }, bounds),
            (
                VerticalDrag::End {
                    delta_y: -28.,
                    velocity_y: -1_000.,
                },
                bounds,
            ),
        ],
    );
    assert!(runtime.app().horizontal_events.is_empty());
    assert_eq!(runtime.app().presses, 0);

    pointer(&mut runtime, Event::PointerDown { x: 20., y: 20. }, 80.);
    pointer(&mut runtime, Event::PointerUp { x: 40., y: 19. }, 100.);
    assert_eq!(
        runtime.app().horizontal_events,
        [HorizontalDrag::End {
            delta_x: 20.,
            velocity_x: 1_000.,
        }]
    );
}

#[test]
fn a_wrong_axis_gesture_cannot_press_the_handle_but_a_tap_can() {
    let mut runtime = runtime(false);
    pointer(&mut runtime, Event::PointerDown { x: 20., y: 20. }, 0.);
    pointer(&mut runtime, Event::PointerMove { x: 35., y: 22. }, 20.);
    pointer(&mut runtime, Event::PointerUp { x: 35., y: 40. }, 40.);
    assert_eq!(runtime.app().presses, 0);
    assert!(runtime.app().vertical_events.is_empty());

    pointer(&mut runtime, Event::PointerDown { x: 20., y: 20. }, 60.);
    pointer(&mut runtime, Event::PointerUp { x: 21., y: 22. }, 80.);
    assert_eq!(runtime.app().presses, 1);
}

#[test]
fn resize_or_replacement_cannot_deliver_a_stale_release() {
    let mut runtime = runtime(false);
    pointer(&mut runtime, Event::PointerDown { x: 20., y: 20. }, 0.);
    pointer(&mut runtime, Event::PointerMove { x: 20., y: 35. }, 20.);
    assert!(runtime.pointer_geometry_changed().unwrap());
    assert_eq!(runtime.app().vertical_events[1].0, VerticalDrag::Cancel);
    runtime
        .dispatch(Event::PointerDown { x: 20., y: 20. })
        .unwrap();
    runtime
        .dispatch(Event::PointerMove { x: 20., y: 38. })
        .unwrap();
    pointer(&mut runtime, Event::PointerUp { x: 20., y: 40. }, 40.);
    assert_eq!(runtime.app().vertical_events.len(), 2);

    pointer(&mut runtime, Event::PointerDown { x: 20., y: 20. }, 60.);
    pointer(&mut runtime, Event::PointerMove { x: 20., y: 36. }, 80.);
    runtime.apply(Message::ReplaceSession).unwrap();
    render(&mut runtime);
    pointer(&mut runtime, Event::PointerUp { x: 20., y: 40. }, 100.);
    assert_eq!(runtime.app().vertical_events.len(), 3);
    assert_eq!(
        runtime.app().vertical_events[2].0,
        VerticalDrag::Move { delta_y: 16. }
    );
    assert_eq!(runtime.app().presses, 0);
}

#[test]
fn scrolling_content_beside_the_handle_does_not_dispatch_a_drag() {
    let mut runtime = runtime(false);
    pointer(&mut runtime, Event::PointerDown { x: 20., y: 70. }, 0.);
    pointer(&mut runtime, Event::PointerMove { x: 20., y: 96. }, 20.);
    pointer(&mut runtime, Event::PointerUp { x: 20., y: 96. }, 40.);
    pointer(
        &mut runtime,
        Event::Scroll {
            x: 20.,
            y: 80.,
            delta_x: 0.,
            delta_y: 30.,
        },
        60.,
    );
    assert_eq!(runtime.scroll_metrics("content").unwrap().offset.1, 30.);
    assert!(runtime.app().vertical_events.is_empty());
}
