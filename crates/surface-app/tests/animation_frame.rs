use std::{cell::Cell, rc::Rc};
use surface::{Color, ExtendedPaintOp, Length, Node, PaintOp, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, Event, Runtime, ToggleMotion, ToggleSpring, View,
};

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

struct Animated {
    active: bool,
    color: Color,
    clicks: usize,
    fail: Rc<Cell<bool>>,
}

impl Animated {
    fn new(fail: Rc<Cell<bool>>) -> Self {
        Self {
            active: true,
            color: Color::rgb(0, 0, 0),
            clicks: 0,
            fail,
        }
    }
}

impl Application for Animated {
    type Message = ();

    fn update(&mut self, _: ()) {
        self.active = !self.active;
        self.clicks += 1;
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        let Some(now_ms) = frame.now_ms else {
            return AnimationUpdate::default();
        };
        let request_next = self.active && !frame.reduced_motion;
        let color = Color::rgb(if request_next { now_ms as u8 } else { 0 }, 0, 0);
        let rebuild = self.color != color;
        self.color = color;
        AnimationUpdate {
            rebuild,
            request_next,
        }
    }

    fn view(&self) -> View<()> {
        let button = || {
            View::new(
                Node::new(Role::Button, "Pause").id("pause").styled(
                    Style::default()
                        .height(Length::Px(48.))
                        .background(self.color),
                ),
            )
            .on_press(())
        };
        let window = View::new(Node::new(Role::Window, "Animation"));
        if self.fail.get() {
            window.children([button(), button()])
        } else {
            window.children([button()])
        }
    }
}

#[test]
fn an_animation_frame_during_a_held_click_preserves_the_pause_action() {
    let mut runtime = Runtime::new(Animated::new(Rc::new(Cell::new(false)))).unwrap();
    runtime.render(120., 96., &mut Metrics).unwrap();
    assert!(!runtime.needs_frame(), "no clock");
    runtime.set_time(10.);
    let frame = runtime.render_extended(120., 96., &mut Metrics).unwrap();
    assert!(runtime.needs_frame());
    assert!(
        matches!(&frame.ops()[0], ExtendedPaintOp::Legacy(PaintOp::Quad { color, .. }) if *color == Color::rgb(10, 0, 0))
    );

    let point = (24., 24.);
    runtime
        .dispatch(Event::PointerDown {
            x: point.0,
            y: point.1,
        })
        .unwrap();
    runtime.set_time(26.);
    runtime.render_extended(120., 96., &mut Metrics).unwrap();
    runtime
        .dispatch(Event::PointerUp {
            x: point.0,
            y: point.1,
        })
        .unwrap();
    runtime.render_extended(120., 96., &mut Metrics).unwrap();
    assert_eq!(runtime.app().clicks, 1);
    assert!(!runtime.needs_frame(), "pause stops drawing");
}

#[test]
fn failed_animation_rebuild_retries_and_reduce_motion_stops_redraws() {
    let fail = Rc::new(Cell::new(false));
    let mut runtime = Runtime::new(Animated::new(fail.clone())).unwrap();
    runtime.render(120., 96., &mut Metrics).unwrap();
    fail.set(true);
    runtime.set_time(16.);
    assert!(runtime.render(120., 96., &mut Metrics).is_err());
    assert!(!runtime.needs_frame());
    fail.set(false);
    let paint = runtime.render(120., 96., &mut Metrics).unwrap();
    assert!(matches!(&paint.ops[0], PaintOp::Quad { color, .. } if *color == Color::rgb(16, 0, 0)));
    assert!(runtime.needs_frame());
    runtime.set_reduced_motion(true);
    let paint = runtime.render(120., 96., &mut Metrics).unwrap();
    assert!(matches!(&paint.ops[0], PaintOp::Quad { color, .. } if *color == Color::rgb(0, 0, 0)));
    assert!(!runtime.needs_frame());
}

#[test]
fn standard_spring_can_settle_from_a_released_gesture() {
    let spring = ToggleSpring::StandardDefaultSpatial;
    let mut launched = ToggleMotion::start_from(false, 0.375, -1.5, 100., spring);
    let mut stationary = ToggleMotion::start_from(false, 0.375, 0., 100., spring);

    assert_eq!(
        launched.sample(false, Some(100.), false, spring),
        (0.375, true)
    );
    let moving = launched.sample(false, Some(108.), false, spring);
    let still = stationary.sample(false, Some(108.), false, spring);
    assert!(moving.1 && moving.0 < still.0);
    assert_eq!(launched.sample(true, Some(108.), false, spring), moving);
    assert_eq!(launched.sample(true, Some(109.), true, spring), (1., false));

    let mut invalid = ToggleMotion::start_from(false, 0.375, f32::NAN, 100., spring);
    assert_eq!(
        invalid.sample(false, Some(100.), false, spring),
        (0., false)
    );
}
