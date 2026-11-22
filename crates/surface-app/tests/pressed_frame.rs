use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, Application, Event, FrameInput, InputAnimationUpdate, Runtime, View,
};

#[derive(Clone, Copy, Default)]
enum Control {
    #[default]
    Ready,
    Disabled,
    Removed,
    Suppressed,
    Replaced,
}

#[derive(Clone)]
enum Message {
    Set(Control),
    Activate,
}

#[derive(Default)]
struct App {
    control: Control,
    pressed: Option<String>,
    activations: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Set(control) => self.control = control,
            Message::Activate => self.activations += 1,
        }
    }

    fn animation_frame_with_press(
        &mut self,
        _frame: AnimationFrame<'_>,
        _input: FrameInput<'_>,
        pressed: Option<&str>,
    ) -> InputAnimationUpdate {
        self.pressed = pressed.map(str::to_owned);
        InputAnimationUpdate::default()
    }

    fn view(&self) -> View<Message> {
        let child = (!matches!(self.control, Control::Removed)).then(|| {
            let role = if matches!(self.control, Control::Replaced) {
                Role::Checkbox
            } else {
                Role::Button
            };
            let view = View::new(
                Node::new(role, "Action")
                    .id("action")
                    .disabled(matches!(self.control, Control::Disabled))
                    .styled(
                        Style::default()
                            .width(Length::Px(80.))
                            .height(Length::Px(40.)),
                    ),
            )
            .on_press(Message::Activate);
            if matches!(self.control, Control::Suppressed) {
                view.without_interaction(None)
            } else {
                view
            }
        });
        View::new(Node::new(Role::Window, "Test")).children(child)
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(180., 140., &mut Metrics).unwrap();
}

fn runtime() -> Runtime<App> {
    let mut runtime = Runtime::new(App::default()).unwrap();
    render(&mut runtime);
    runtime
}

fn point(runtime: &mut Runtime<App>, touch: bool, down: bool) -> bool {
    let event = if down {
        Event::PointerDown { x: 20., y: 20. }
    } else {
        Event::PointerUp { x: 20., y: 20. }
    };
    if touch {
        runtime.dispatch_touch(event)
    } else {
        runtime.dispatch(event)
    }
    .unwrap()
}

#[test]
fn frames_observe_pending_mouse_and_touch_presses_through_a_rebuild() {
    for touch in [false, true] {
        let mut runtime = runtime();
        assert!(point(&mut runtime, touch, true));
        render(&mut runtime);
        assert_eq!(runtime.app().pressed.as_deref(), Some("action"));
        runtime.apply(Message::Set(Control::Ready)).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.app().pressed.as_deref(), Some("action"));
        assert!(point(&mut runtime, touch, false));
        render(&mut runtime);
        assert_eq!(runtime.app().pressed, None);
        assert_eq!(runtime.app().activations, 1);
    }
}

#[test]
fn a_cancel_before_the_frame_never_reports_a_pending_press() {
    for touch in [false, true] {
        let mut runtime = runtime();
        assert!(point(&mut runtime, touch, true));
        assert!(runtime.cancel_pointer().unwrap());
        render(&mut runtime);
        assert_eq!(runtime.app().pressed, None);
        point(&mut runtime, touch, false);
        assert_eq!(runtime.app().activations, 0);

        assert!(point(&mut runtime, touch, true));
        render(&mut runtime);
        assert_eq!(runtime.app().pressed.as_deref(), Some("action"));
        assert!(runtime.cancel_pointer().unwrap());
        render(&mut runtime);
        assert_eq!(runtime.app().pressed, None);
        point(&mut runtime, touch, false);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn a_lost_or_replaced_target_cannot_replay_an_old_press() {
    for control in [
        Control::Disabled,
        Control::Removed,
        Control::Suppressed,
        Control::Replaced,
    ] {
        let mut runtime = runtime();
        assert!(point(&mut runtime, false, true));
        runtime.apply(Message::Set(control)).unwrap();
        runtime.apply(Message::Set(Control::Ready)).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.app().pressed, None);
        point(&mut runtime, false, false);
        assert_eq!(runtime.app().activations, 0);
    }
}
