use surface::{
    Align, Color, Corners, DisplayList, Node, NodeId, PaintOp, Rect, Role, Style, TextMeasurer,
};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, Application, EditKey, Event, Runtime,
    View,
};
use surface_material::{
    shape::LayoutDirection, StandardSliderMode, StandardSliderOptions, StandardSliderSize, Theme,
    VerticalStandardSliderPlacement,
};

const ID: &str = "volume";
const OPTIONS: StandardSliderOptions<'static> = StandardSliderOptions {
    id: ID,
    name: "音量",
    min: 0.,
    max: 100.,
    value: 25.,
    mode: StandardSliderMode::Continuous { keyboard_step: 5. },
    size: StandardSliderSize::ExtraSmall,
    direction: LayoutDirection::LeftToRight,
    disabled: false,
    engaged: false,
    value_label: None,
};
const PLACEMENT: VerticalStandardSliderPlacement = VerticalStandardSliderPlacement {
    height: 200.,
    label_room: 96.,
};

#[derive(Clone)]
enum Message {
    Value(f64),
    Engage,
}

struct Model {
    options: StandardSliderOptions<'static>,
    placement: VerticalStandardSliderPlacement,
    dark: bool,
    scale: f32,
    requests: Vec<f64>,
}

impl Model {
    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Value(value) => {
                self.options.value = value;
                self.requests.push(value);
            }
            Message::Engage => self.options.engaged = true,
        }
    }

    fn view(&self) -> View<Message> {
        let slider = self
            .theme()
            .vertical_standard_slider(self.options, self.placement, Message::Value)
            .expect("valid fixture");
        View::new(
            Node::new(Role::Window, "").styled(Style::column().padding(12.).align(Align::Center)),
        )
        .children([slider])
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let advance = size * 0.75;
        let count = text.chars().count() as f32;
        let limit = width.unwrap_or(f32::INFINITY);
        let per_line = (limit / advance).floor().max(1.);
        (
            (advance * count.min(per_line)).min(limit),
            (count / per_line).ceil().max(1.) * size * 1.5,
        )
    }
}

fn runtime(
    options: StandardSliderOptions<'static>,
    placement: VerticalStandardSliderPlacement,
    dark: bool,
    scale: f32,
) -> Runtime<Model> {
    Runtime::new(Model {
        options,
        placement,
        dark,
        scale,
        requests: Vec::new(),
    })
    .unwrap()
}

fn paint(runtime: &mut Runtime<Model>) -> DisplayList {
    runtime.render(320., 400., &mut Metrics).unwrap()
}

fn id(runtime: &Runtime<Model>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn bounds(runtime: &Runtime<Model>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn right(rect: Rect) -> f32 {
    rect.x + rect.width
}
fn bottom(rect: Rect) -> f32 {
    rect.y + rect.height
}
fn center_y(rect: Rect) -> f32 {
    rect.y + rect.height / 2.
}

fn has_fill(list: &DisplayList, area: Rect, ink: Color, corners: Corners) -> bool {
    list.ops.iter().any(|op| match op {
        PaintOp::Shape {
            rect,
            color,
            corners: shape,
            ..
        } => *rect == area && *color == ink && *shape == corners,
        PaintOp::Quad {
            rect,
            color,
            radius,
            ..
        } => *rect == area && *color == ink && Corners::all(*radius) == corners,
        _ => false,
    })
}

#[test]
fn five_presets_rotate_track_gaps_stop_and_palette_in_both_directions() {
    use StandardSliderSize::*;
    for (size, track, span, radius) in [
        (ExtraSmall, 16., 44_f32, 8.),
        (Small, 24., 44., 8.),
        (Medium, 40., 52., 12.),
        (Large, 56., 68., 16.),
        (ExtraLarge, 96., 108., 28.),
    ] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            for dark in [false, true] {
                let mut runtime = runtime(
                    StandardSliderOptions {
                        size,
                        direction,
                        ..OPTIONS
                    },
                    PLACEMENT,
                    dark,
                    1.,
                );
                let list = paint(&mut runtime);
                let colors = runtime.app().theme().colors;
                let hit = bounds(&runtime, ID);
                let rail = bounds(&runtime, "volume-rail");
                let active = bounds(&runtime, "volume-active");
                let inactive = bounds(&runtime, "volume-inactive");
                let handle = bounds(&runtime, "volume-handle");
                let stop = bounds(&runtime, "volume-stop");
                assert_eq!((hit.width, hit.height), (span.max(48.), 200.));
                assert_eq!(rail.y, hit.y + 12.);
                assert_eq!(rail.height, hit.height - 24.);
                assert_eq!(center_y(handle), bottom(rail) - rail.height / 4.);
                assert_eq!((handle.width, handle.height), (span, 4.));
                assert_eq!((active.width, inactive.width), (track, track));
                assert_eq!(bottom(active), bottom(rail));
                assert_eq!(bottom(handle) + 6., active.y);
                assert_eq!(bottom(inactive) + 6., handle.y);
                assert_eq!(inactive.y, rail.y);
                assert_eq!((stop.width, stop.height), (4., 4.));
                assert_eq!(stop.y, rail.y + 4.);
                assert!(has_fill(
                    &list,
                    active,
                    colors.primary,
                    Corners::new(2., 2., radius, radius).resolve(active.width, active.height)
                ));
                assert!(has_fill(
                    &list,
                    inactive,
                    colors.secondary_container,
                    Corners::new(radius, radius, 2., 2.)
                ));
                assert!(has_fill(&list, handle, colors.primary, Corners::all(2.)));
                assert!(has_fill(
                    &list,
                    stop,
                    colors.on_secondary_container,
                    Corners::all(2.)
                ));
            }
        }
    }
}

#[test]
fn rtl_upright_scaled_label_switches_side_and_fits_both_endpoints() {
    let placement = VerticalStandardSliderPlacement {
        label_room: 120.,
        ..PLACEMENT
    };
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        for (value, value_label) in [(0., "目前音量 0％"), (100., "目前音量 100％")] {
            let mut runtime = runtime(
                StandardSliderOptions {
                    value,
                    direction,
                    value_label: Some(value_label),
                    ..OPTIONS
                },
                placement,
                true,
                2.,
            );
            paint(&mut runtime);
            let hit = bounds(&runtime, ID);
            let rail = bounds(&runtime, "volume-rail");
            assert!(runtime.dom().by_id("volume-label").is_none());
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            runtime.apply(Message::Engage).unwrap();
            let list = paint(&mut runtime);
            let handle = bounds(&runtime, "volume-handle");
            let ring = bounds(&runtime, "volume-focus");
            let label = bounds(&runtime, "volume-label");
            let text = bounds(&runtime, "volume-label-text");
            assert_eq!(runtime.focused(), Some(ID));
            assert_eq!(bounds(&runtime, ID), hit);
            assert_eq!(bounds(&runtime, "volume-rail"), rail);
            assert_eq!(handle.height, 2.);
            assert_eq!(ring.height, 12.);
            assert_eq!(
                center_y(handle),
                if value == 0. { bottom(rail) } else { rail.y }
            );
            assert!(label.height > 64.);
            assert_eq!(label.intersect(text), text);
            assert!(label.y >= hit.y && bottom(label) <= bottom(hit));
            if direction == LayoutDirection::LeftToRight {
                assert_eq!(label.x, right(ring) + 4.);
                assert!(right(label) <= right(hit) + placement.label_room);
            } else {
                assert_eq!(right(label) + 4., ring.x);
                assert!(label.x >= hit.x - placement.label_room);
            }
            assert!(label.x >= 0. && right(label) <= 320.);
            let colors = runtime.app().theme().colors;
            assert!(list.ops.iter().any(|op| matches!(op, PaintOp::StyledText {
                text, style, color, ..
            } if text == value_label && style.font_size == 28. && *color == colors.inverse_on_surface)));
            assert!(list.ops.iter().any(|op| matches!(op, PaintOp::Stroke {
                rect, width: 3., color, ..
            } if *rect == ring && *color == colors.secondary)));
            let frame = runtime.accessibility_frame().unwrap();
            let slider = id(&runtime, ID);
            assert_eq!(frame.children(slider).count(), 0);
            let node = frame.node(slider).unwrap().node;
            assert_eq!(node.slider_value_text(), Some(value_label));
            assert!(node.is_vertical_slider());
        }
    }
}

#[test]
fn tight_label_room_keeps_accessibility_and_disabled_has_no_binding() {
    let value_label = "現在音量百分之二十五，共有十個合理的音量選擇";
    for placement in [
        VerticalStandardSliderPlacement {
            label_room: 0.,
            ..PLACEMENT
        },
        VerticalStandardSliderPlacement {
            height: 48.,
            label_room: 60.,
        },
    ] {
        let mut runtime = runtime(
            StandardSliderOptions {
                engaged: true,
                value_label: Some(value_label),
                ..OPTIONS
            },
            placement,
            true,
            2.,
        );
        let list = paint(&mut runtime);
        assert!(!list.ops.iter().any(|op| matches!(op, PaintOp::StyledText {
            text, ..
        } if text == value_label)));
        let frame = runtime.accessibility_frame().unwrap();
        let slider = frame.node(id(&runtime, ID)).unwrap();
        assert_eq!(slider.node.slider_value_text(), Some(value_label));
        assert!(slider.node.is_vertical_slider());
    }

    let mut runtime = runtime(
        StandardSliderOptions {
            disabled: true,
            engaged: true,
            value_label: Some(value_label),
            ..OPTIONS
        },
        PLACEMENT,
        false,
        1.,
    );
    let list = paint(&mut runtime);
    let id = id(&runtime, ID);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(id).unwrap().node.is_vertical_slider());
    assert!(frame.focus_target(id).is_none());
    assert!(frame.adjustment_target(id).is_none());
    let colors = runtime.app().theme().colors;
    let Color(r, g, b, _) = colors.on_surface;
    let handle = bounds(&runtime, "volume-handle");
    assert_eq!((handle.width, handle.height), (44., 4.));
    assert!(has_fill(
        &list,
        handle,
        Color(r, g, b, 97),
        Corners::all(2.)
    ));
    assert!(runtime.dom().by_id("volume-label").is_none());
}

#[test]
fn discrete_real_y_input_keys_and_adjustment_keep_same_selection_lattice() {
    let mut runtime = runtime(
        StandardSliderOptions {
            direction: LayoutDirection::RightToLeft,
            mode: StandardSliderMode::Discrete { intervals: 8 },
            ..OPTIONS
        },
        PLACEMENT,
        false,
        1.,
    );
    let list = paint(&mut runtime);
    let hit = bounds(&runtime, ID);
    let rail = bounds(&runtime, "volume-rail");
    let tick = bounds(&runtime, "volume-tick-1");
    assert_eq!((tick.width, tick.height), (4., 4.));
    assert!(has_fill(
        &list,
        tick,
        runtime.app().theme().colors.on_primary,
        Corners::all(2.)
    ));
    let x = hit.x + hit.width / 2.;
    let near_bottom = bottom(rail) - rail.height * 0.13;
    for event in [
        Event::PointerDown { x, y: near_bottom },
        Event::PointerMove {
            x: x + 20.,
            y: near_bottom,
        },
        Event::PointerMove { x, y: rail.y },
        Event::PointerUp { x, y: rail.y },
    ] {
        runtime.dispatch_touch(event).unwrap();
        paint(&mut runtime);
    }
    assert_eq!(runtime.app().requests, [12.5, 100.]);
    for (key, expected) in [(EditKey::Home, 0.), (EditKey::Up, 12.5)] {
        runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false,
            })
            .unwrap();
        assert_eq!(runtime.app().options.value, expected);
    }
    paint(&mut runtime);
    let token = runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id(&runtime, ID))
        .unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&token, AccessibilityAdjustment::SetValue(66.))
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted
    );
    assert_eq!(runtime.app().options.value, 62.5);
}

#[test]
fn ranges_and_target_height_and_room_are_checked() {
    let theme = Theme::light();
    for placement in [
        VerticalStandardSliderPlacement {
            height: 47.,
            ..PLACEMENT
        },
        VerticalStandardSliderPlacement {
            height: f32::INFINITY,
            ..PLACEMENT
        },
        VerticalStandardSliderPlacement {
            label_room: -1.,
            ..PLACEMENT
        },
        VerticalStandardSliderPlacement {
            label_room: f32::NAN,
            ..PLACEMENT
        },
    ] {
        assert!(theme
            .vertical_standard_slider(OPTIONS, placement, Message::Value)
            .is_none());
    }
    let invalid = StandardSliderOptions {
        mode: StandardSliderMode::Discrete { intervals: 0 },
        ..OPTIONS
    };
    assert!(theme
        .vertical_standard_slider(invalid, PLACEMENT, Message::Value)
        .is_none());
}
