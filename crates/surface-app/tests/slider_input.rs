use std::num::NonZeroU16;

use surface::{Length, Node, RangeValue, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, Application, EditKey, Event, Runtime,
    SliderInputOptions, SliderScale, View,
};

#[derive(Clone)]
enum Message {
    Value(f64),
    Reverse,
}

struct App {
    current: f64,
    history: Vec<f64>,
    input: SliderInputOptions,
}

impl Application for App {
    type Message = Message;

    fn view(&self) -> View<Message> {
        let node = Node::new(Role::Slider, "Volume")
            .id("volume")
            .range(RangeValue {
                min: 0.,
                max: 100.,
                value: self.current,
                step: 7.,
            })
            .styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .height(Length::Px(48.))
                    .padding(12.),
            );
        View::new(Node::new(Role::Window, "Sliders").styled(Style::column())).children([View::new(
            node,
        )
        .on_value(Message::Value)
        .with_slider_input(self.input)
        .accessible_slider_adjustment()])
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Value(value) => {
                self.current = value;
                self.history.push(value);
            }
            Message::Reverse => self.input.reverse_horizontal = !self.input.reverse_horizontal,
        }
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn start(scale: SliderScale, rtl: bool) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        current: 0.,
        history: Vec::new(),
        input: SliderInputOptions {
            scale,
            reverse_horizontal: rtl,
        },
    })
    .unwrap();
    paint(&mut runtime, 200.);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("volume"));
    runtime
}

fn paint(runtime: &mut Runtime<App>, width: f32) {
    runtime.render(width, 64., &mut Metrics).unwrap();
}

fn key(runtime: &mut Runtime<App>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

fn pointer(runtime: &mut Runtime<App>, event: Event) {
    runtime.dispatch(event).unwrap();
}

#[test]
fn continuous_pointer_write_is_unsnapped_but_arrows_use_authored_step() {
    let mut runtime = start(SliderScale::Continuous, false);
    // The real rail runs from 12 to 188 dp. This is 3/16 along it.
    pointer(&mut runtime, Event::PointerDown { x: 45., y: 24. });
    assert_eq!(runtime.app().history, [18.75]);
    paint(&mut runtime, 200.);
    pointer(&mut runtime, Event::PointerMove { x: 100., y: 24. });
    assert_eq!(runtime.app().history, [18.75, 50.]);
    paint(&mut runtime, 200.);
    pointer(&mut runtime, Event::PointerUp { x: 100., y: 24. });
    key(&mut runtime, EditKey::Right);
    key(&mut runtime, EditKey::PageUp);
    key(&mut runtime, EditKey::End);
    assert_eq!(runtime.app().history, [18.75, 50., 57., 67., 100.]);
}

#[test]
fn rtl_mirrors_pointer_and_horizontal_arrows_but_keeps_logical_keys() {
    let mut runtime = start(SliderScale::Continuous, true);
    pointer(&mut runtime, Event::PointerDown { x: 45., y: 24. });
    pointer(&mut runtime, Event::PointerUp { x: 45., y: 24. });
    assert_eq!(runtime.app().history, [81.25]);
    key(&mut runtime, EditKey::Left);
    key(&mut runtime, EditKey::Right);
    key(&mut runtime, EditKey::Up);
    key(&mut runtime, EditKey::Home);
    assert_eq!(runtime.app().history, [81.25, 88.25, 81.25, 88.25, 0.]);
    paint(&mut runtime, 200.);
    for event in [
        Event::PointerDown { x: 3., y: 24. },
        Event::PointerUp { x: 3., y: 24. },
    ] {
        runtime.dispatch_touch(event).unwrap();
    }
    assert_eq!(runtime.app().current, 100.);
    assert_eq!(runtime.app().history.len(), 6);
}

#[test]
fn discrete_pointer_keyboard_and_absolute_requests_keep_both_exact_endpoints() {
    let mut runtime = start(SliderScale::Discrete(NonZeroU16::new(3).unwrap()), false);
    key(&mut runtime, EditKey::Right);
    assert!((runtime.app().current - 100. / 3.).abs() < 1e-10);
    runtime.set_value("volume", 60.).unwrap();
    assert!((runtime.app().current - 200. / 3.).abs() < 1e-10);
    key(&mut runtime, EditKey::End);
    assert_eq!(runtime.app().current, 100.);
    key(&mut runtime, EditKey::Home);
    assert_eq!(runtime.app().current, 0.);
    paint(&mut runtime, 200.);
    pointer(&mut runtime, Event::PointerDown { x: 192., y: 24. });
    pointer(&mut runtime, Event::PointerUp { x: 192., y: 24. });
    assert_eq!(runtime.app().current, 100.);
}

#[test]
fn changed_geometry_and_replaced_direction_cancel_a_live_drag() {
    let mut runtime = start(SliderScale::Continuous, false);
    pointer(&mut runtime, Event::PointerDown { x: 45., y: 24. });
    assert!(runtime.pointer_geometry_changed().unwrap());
    paint(&mut runtime, 300.);
    pointer(&mut runtime, Event::PointerMove { x: 250., y: 24. });
    pointer(&mut runtime, Event::PointerUp { x: 250., y: 24. });
    assert_eq!(runtime.app().history, [18.75]);
    pointer(&mut runtime, Event::PointerDown { x: 150., y: 24. });
    runtime.apply(Message::Reverse).unwrap();
    paint(&mut runtime, 300.);
    pointer(&mut runtime, Event::PointerMove { x: 80., y: 24. });
    pointer(&mut runtime, Event::PointerUp { x: 80., y: 24. });
    assert_eq!(runtime.app().history, [18.75, 50.]);
    pointer(&mut runtime, Event::PointerDown { x: 150., y: 24. });
    paint(&mut runtime, 400.);
    pointer(&mut runtime, Event::PointerMove { x: 250., y: 24. });
    assert_eq!(runtime.app().history, [18.75, 50.]);
}

#[test]
fn accesskit_tokens_use_continuous_requests_and_relative_steps() {
    let mut runtime = start(SliderScale::Continuous, true);
    let target = |runtime: &mut Runtime<App>| {
        paint(runtime, 200.);
        let id = runtime.dom().by_id("volume").unwrap();
        runtime
            .accessibility_frame()
            .unwrap()
            .adjustment_target(id)
            .unwrap()
    };
    let absolute = target(&mut runtime);
    let result = runtime
        .accessibility_adjust(&absolute, AccessibilityAdjustment::SetValue(18.75))
        .unwrap();
    assert_eq!(result, AccessibilityAdjustmentResult::Adjusted);
    assert_eq!(runtime.app().current, 18.75);
    let relative = target(&mut runtime);
    let result = runtime
        .accessibility_adjust(&relative, AccessibilityAdjustment::Increment)
        .unwrap();
    assert_eq!(result, AccessibilityAdjustmentResult::Adjusted);
    assert_eq!(runtime.app().current, 25.75);
    assert_eq!(runtime.focused(), Some("volume"));
}
