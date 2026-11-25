use std::num::NonZeroU16;

use surface::{Length, Node, RangeValue, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, Application, EditKey, Event, Runtime,
    SliderInputOptions, SliderScale, View,
};

#[derive(Clone)]
enum Message {
    Value(f64),
    Move,
    Horizontal,
    Disable,
}

struct App {
    value: f64,
    history: Vec<f64>,
    scale: SliderScale,
    moved: bool,
    horizontal: bool,
    disabled: bool,
}

impl Application for App {
    type Message = Message;

    fn view(&self) -> View<Message> {
        let node = Node::new(Role::Slider, "Volume")
            .id("volume")
            .range(RangeValue {
                min: 0.,
                max: 100.,
                value: self.value,
                step: match self.scale {
                    SliderScale::Continuous => 7.,
                    SliderScale::Discrete(intervals) => 100. / f64::from(intervals.get()),
                },
            })
            .disabled(self.disabled)
            .styled(
                Style::row()
                    .width(Length::Px(48.))
                    .height(Length::Px(200.))
                    .padding(12.),
            );
        let slider = View::new(node)
            .on_value(Message::Value)
            .accessible_slider_adjustment();
        let slider = if self.horizontal {
            slider.with_slider_input(SliderInputOptions {
                scale: self.scale,
                reverse_horizontal: false,
            })
        } else {
            slider.with_vertical_slider_input(self.scale)
        };
        let spacer = self.moved.then(|| {
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(20.))))
        });
        View::new(Node::new(Role::Window, "Sliders").styled(Style::column()))
            .children(spacer.into_iter().chain([slider]))
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Value(value) => {
                self.value = value;
                self.history.push(value);
            }
            Message::Move => self.moved = true,
            Message::Horizontal => self.horizontal = true,
            Message::Disable => self.disabled = true,
        }
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn start(scale: SliderScale) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        value: 0.,
        history: Vec::new(),
        scale,
        moved: false,
        horizontal: false,
        disabled: false,
    })
    .unwrap();
    paint(&mut runtime);
    runtime
}

fn paint(runtime: &mut Runtime<App>) {
    runtime.render(240., 240., &mut Metrics).unwrap();
}

fn pointer(runtime: &mut Runtime<App>, event: Event) {
    runtime.dispatch(event).unwrap();
}

#[test]
fn continuous_mouse_uses_only_y_and_keeps_exact_endpoints_outside_the_target() {
    let mut runtime = start(SliderScale::Continuous);
    // The rail runs from y=12 to y=188. Mouse-down lands 3/16 up the rail.
    pointer(&mut runtime, Event::PointerDown { x: 24., y: 155. });
    assert_eq!(runtime.app().history, [18.75]);
    paint(&mut runtime);
    pointer(&mut runtime, Event::PointerMove { x: 120., y: 155. });
    assert_eq!(runtime.app().history, [18.75]);
    pointer(&mut runtime, Event::PointerMove { x: 120., y: 100. });
    assert_eq!(runtime.app().history, [18.75, 50.]);
    paint(&mut runtime);
    pointer(&mut runtime, Event::PointerMove { x: 120., y: 12. });
    assert_eq!(runtime.app().value, 100.);
    paint(&mut runtime);
    pointer(&mut runtime, Event::PointerUp { x: 120., y: 188. });
    assert_eq!(runtime.app().history, [18.75, 50., 100., 0.]);
    assert_eq!(runtime.focused(), Some("volume"));
}

#[test]
fn discrete_touch_snaps_live_cancels_and_rearms_only_for_a_new_contact() {
    let mut runtime = start(SliderScale::Discrete(NonZeroU16::new(4).unwrap()));
    runtime
        .dispatch_touch(Event::PointerDown { x: 24., y: 146. })
        .unwrap();
    assert_eq!(runtime.app().history, [25.]);
    paint(&mut runtime);
    runtime
        .dispatch_touch(Event::PointerMove { x: 42., y: 50. })
        .unwrap();
    assert_eq!(runtime.app().history, [25., 75.]);
    assert!(runtime.cancel_pointer().unwrap());
    paint(&mut runtime);
    runtime
        .dispatch_touch(Event::PointerUp { x: 24., y: 12. })
        .unwrap();
    assert_eq!(runtime.app().history, [25., 75.]);
    for event in [
        Event::PointerDown { x: 24., y: 12. },
        Event::PointerUp { x: 24., y: 12. },
    ] {
        runtime.dispatch_touch(event).unwrap();
    }
    assert_eq!(runtime.app().history, [25., 75., 100.]);
}

#[test]
fn vertical_drag_retires_after_geometry_or_axis_changes() {
    let mut runtime = start(SliderScale::Continuous);
    for change in [None, Some(Message::Move), Some(Message::Horizontal)] {
        let y = if runtime.app().moved { 120. } else { 100. };
        pointer(&mut runtime, Event::PointerDown { x: 24., y });
        assert_eq!(runtime.app().value, 50.);
        if let Some(change) = change {
            runtime.apply(change).unwrap();
        } else {
            assert!(runtime.pointer_geometry_changed().unwrap());
        }
        paint(&mut runtime);
        pointer(&mut runtime, Event::PointerMove { x: 40., y: 40. });
        pointer(&mut runtime, Event::PointerUp { x: 40., y: 40. });
        assert_eq!(runtime.app().history, [50.]);
    }
    let id = runtime.dom().by_id("volume").unwrap();
    assert!(!runtime.dom().get(id).unwrap().is_vertical_slider());
    pointer(&mut runtime, Event::PointerDown { x: 36., y: 120. });
    assert_eq!(runtime.app().history, [50., 100.]);
}

#[test]
fn vertical_keys_and_scoped_accessible_adjustments_use_logical_values() {
    let mut runtime = start(SliderScale::Continuous);
    let id = runtime.dom().by_id("volume").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(id).unwrap().node.is_vertical_slider());
    let target = frame.adjustment_target(id).unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&target, AccessibilityAdjustment::SetValue(11.25))
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted
    );
    assert_eq!(runtime.focused(), None);
    pointer(&mut runtime, Event::FocusNext { reverse: false });
    for (key, expected) in [
        (EditKey::Right, 18.25),
        (EditKey::Up, 25.25),
        (EditKey::Left, 18.25),
        (EditKey::Down, 11.25),
        (EditKey::PageUp, 21.25),
        (EditKey::PageDown, 11.25),
        (EditKey::End, 100.),
        (EditKey::Home, 0.),
    ] {
        pointer(
            &mut runtime,
            Event::Edit {
                key,
                extend: false,
                by_word: false,
            },
        );
        assert_eq!(runtime.app().value, expected);
    }
    paint(&mut runtime);
    let id = runtime.dom().by_id("volume").unwrap();
    let token = runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id)
        .unwrap();
    runtime.apply(Message::Disable).unwrap();
    paint(&mut runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&token, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::Rejected
    );
    let node = runtime.dom().by_id("volume").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(node).unwrap().node.is_vertical_slider());
    assert!(frame.adjustment_target(node).is_none());
}
