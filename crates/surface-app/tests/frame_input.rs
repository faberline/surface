use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, AppError, Application, Event, FrameInput, HorizontalDrag,
    HorizontalDragSession, InputAnimationUpdate, Runtime, TouchContact, View,
};

#[derive(Clone)]
enum Message {
    Press,
    Drag(HorizontalDrag),
    Dismiss,
}

#[derive(Default)]
struct App {
    observe: bool,
    suppress_long_press: bool,
    suppression_sent: bool,
    presses: usize,
    drags: Vec<HorizontalDrag>,
    drag_session: Option<HorizontalDragSession>,
    transient: bool,
    modal: bool,
    pointer: Option<(f32, f32)>,
    focused: Option<String>,
    touch: Option<TouchContact>,
    touch_cancelled: bool,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press => self.presses += 1,
            Message::Drag(drag) => self.drags.push(drag),
            Message::Dismiss => self.modal = false,
        }
    }

    fn observes_frame_input(&self) -> bool {
        self.observe
    }

    fn animation_frame_with_input(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        self.pointer = input.pointer;
        self.focused = input.focused.map(str::to_owned);
        self.touch = input.touch;
        self.touch_cancelled = input.touch_cancelled;
        let held_long_enough = input
            .touch
            .and_then(|touch| touch.started_ms)
            .zip(frame.now_ms)
            .is_some_and(|(start, now)| now - start >= 500.);
        let suppress = self.suppress_long_press && !self.suppression_sent && held_long_enough;
        self.suppression_sent |= suppress;
        InputAnimationUpdate {
            suppress_touch_activation: suppress,
            ..Default::default()
        }
    }

    fn cancel_transient(&mut self) -> bool {
        std::mem::take(&mut self.transient)
    }

    fn view(&self) -> View<Message> {
        let mut button = View::new(
            Node::new(Role::Button, "Activate").id("button").styled(
                Style::default()
                    .width(Length::Px(120.))
                    .height(Length::Px(40.)),
            ),
        )
        .on_press(Message::Press);
        if let Some(session) = &self.drag_session {
            button = button.on_horizontal_drag(session.clone(), Message::Drag);
        }
        let passive = View::new(
            Node::new(Role::Group, if self.transient { "Shown" } else { "Hidden" })
                .id("passive")
                .styled(Style::default().height(Length::Px(40.))),
        );
        let modal = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Modal").id("modal")).modal_overlay(Message::Dismiss)
        });
        View::new(Node::new(Role::Window, "Test").styled(Style::column()))
            .children([button, passive].into_iter().chain(modal))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>, now: f64) {
    runtime.set_time(now);
    runtime.render(240., 160., &mut Metrics).unwrap();
}

fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime, 0.);
    runtime
}

fn down() -> Event {
    Event::PointerDown { x: 20., y: 20. }
}

fn up() -> Event {
    Event::PointerUp { x: 20., y: 20. }
}

#[test]
fn passive_pointer_motion_requests_a_frame_only_for_observers() {
    for observe in [false, true] {
        let mut runtime = runtime(App {
            observe,
            ..App::default()
        });
        let passive = runtime.dom().by_id("passive").unwrap();
        assert!(!runtime.dom().get(passive).unwrap().is_focusable());
        let event = Event::PointerMove { x: 24., y: 64. };
        assert_eq!(runtime.dispatch(event.clone()).unwrap(), observe);
        assert!(!runtime.dispatch(event).unwrap());
        render(&mut runtime, 10.);
        assert_eq!(runtime.app().pointer, Some((24., 64.)));
        assert_eq!(runtime.app().focused, None);
        assert!(runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap());
        render(&mut runtime, 20.);
        assert_eq!(runtime.app().focused.as_deref(), Some("button"));
        assert_eq!(runtime.dispatch(Event::PointerLeave).unwrap(), observe);
        render(&mut runtime, 30.);
        assert_eq!(runtime.app().pointer, None);
    }
}

#[test]
fn long_press_consumes_only_the_current_touch_tap() {
    let mut runtime = runtime(App {
        observe: true,
        suppress_long_press: true,
        ..App::default()
    });
    runtime.set_time(10.);
    assert!(runtime.dispatch_touch(down()).unwrap());
    assert!(runtime
        .dispatch_touch(Event::PointerMove { x: 21., y: 22. })
        .unwrap());
    render(&mut runtime, 509.);
    assert!(!runtime.app().suppression_sent);
    assert_eq!(runtime.app().pointer, None);
    assert_eq!(
        runtime.app().touch,
        Some(TouchContact {
            origin: (20., 20.),
            position: (21., 22.),
            started_ms: Some(10.),
        })
    );
    render(&mut runtime, 510.);
    assert!(runtime.app().suppression_sent);
    render(&mut runtime, 600.);
    assert!(runtime.dispatch_touch(up()).unwrap());
    render(&mut runtime, 610.);
    assert_eq!(runtime.app().presses, 0);
    assert_eq!(runtime.app().touch, None);
    assert!(!runtime.app().touch_cancelled);

    assert!(runtime.dispatch_touch(down()).unwrap());
    assert!(runtime.dispatch_touch(up()).unwrap());
    render(&mut runtime, 620.);
    assert!(runtime.dispatch(down()).unwrap());
    assert!(runtime.dispatch(up()).unwrap());
    assert_eq!(runtime.app().presses, 2);
}

#[test]
fn touch_cancellation_is_reported_once_and_retires_the_pending_press() {
    type Interrupt = fn(&mut Runtime<App>) -> Result<bool, AppError>;
    let interrupts: [Interrupt; 4] = [
        Runtime::cancel_pointer,
        Runtime::pointer_geometry_changed,
        |runtime| runtime.dispatch(Event::Blur),
        |runtime| runtime.dispatch(Event::PointerMove { x: 10., y: 30. }),
    ];
    for interrupt in interrupts {
        let mut runtime = runtime(App {
            observe: true,
            ..App::default()
        });
        assert!(runtime.dispatch_touch(down()).unwrap());
        render(&mut runtime, 10.);
        assert!(runtime.app().touch.is_some());
        assert!(!runtime.app().touch_cancelled);

        assert!(interrupt(&mut runtime).unwrap());
        assert!(!runtime.dispatch_touch(up()).unwrap());
        render(&mut runtime, 20.);
        assert!(runtime.app().touch_cancelled);
        assert_eq!(runtime.app().touch, None);
        assert_eq!(runtime.app().presses, 0);
        render(&mut runtime, 30.);
        assert!(!runtime.app().touch_cancelled);
    }
}

#[test]
fn a_suppressed_touch_can_still_finish_a_drag() {
    let mut runtime = runtime(App {
        observe: true,
        suppress_long_press: true,
        drag_session: Some(HorizontalDragSession::new()),
        ..App::default()
    });
    assert!(runtime.dispatch_touch(down()).unwrap());
    render(&mut runtime, 600.);
    assert!(runtime.app().suppression_sent);
    runtime
        .dispatch_touch(Event::PointerMove { x: 50., y: 20. })
        .unwrap();
    runtime
        .dispatch_touch(Event::PointerUp { x: 60., y: 20. })
        .unwrap();
    assert_eq!(runtime.app().presses, 0);
    assert!(matches!(
        runtime.app().drags.as_slice(),
        [
            HorizontalDrag::Move { delta_x: 30. },
            HorizontalDrag::End { delta_x: 40., .. }
        ]
    ));
}

#[test]
fn escape_rebuilds_transient_ui_without_focus_after_modal_precedence() {
    let mut runtime = runtime(App {
        observe: true,
        transient: true,
        modal: true,
        ..App::default()
    });
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert!(!runtime.app().modal);
    assert!(runtime.app().transient);
    assert_eq!(runtime.focused(), None);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    let passive = runtime.dom().by_id("passive").unwrap();
    assert_eq!(runtime.dom().get(passive).unwrap().name(), "Hidden");
}
