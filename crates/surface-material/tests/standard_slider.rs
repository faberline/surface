use surface::{
    Color, Corners, DisplayList, Node, NodeId, PaintOp, RangeValue, Rect, Role, Style, TextMeasurer,
};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, Application, Event, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, StandardSliderMode, StandardSliderOptions, StandardSliderSize, Theme,
};

const ID: &str = "volume";
const OPTIONS: StandardSliderOptions<'static> = StandardSliderOptions {
    id: ID,
    name: "音量",
    min: 0.,
    max: 100.,
    value: 25.,
    mode: StandardSliderMode::Continuous { keyboard_step: 1. },
    size: StandardSliderSize::ExtraSmall,
    direction: LayoutDirection::LeftToRight,
    disabled: false,
    engaged: false,
    value_label: None,
};

#[derive(Clone)]
enum Message {
    Value(f64),
    Engage(bool),
}

struct Model {
    options: StandardSliderOptions<'static>,
    neutral: Option<f64>,
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
            Message::Engage(engaged) => self.options.engaged = engaged,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let slider = match self.neutral {
            Some(value) => theme.centered_slider(self.options, value, Message::Value),
            None => theme.standard_slider(self.options, Message::Value),
        }
        .expect("valid test slider");
        View::new(Node::new(Role::Window, "").styled(Style::column().padding(18.).gap(8.)))
            .children([
                // Leave room above the fixed cross-axis slider hit target.
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::column().height(surface::Length::Px(155.))),
                ),
                slider,
            ])
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, font_size: f32, width: Option<f32>) -> (f32, f32) {
        let advance = font_size * 0.75;
        let count = text.chars().count() as f32;
        let limit = width.unwrap_or(f32::INFINITY);
        let per_line = (limit / advance).floor().max(1.);
        (
            (advance * count.min(per_line)).min(limit),
            (count / per_line).ceil().max(1.) * font_size * 1.5,
        )
    }
}

fn runtime(options: StandardSliderOptions<'static>, dark: bool, scale: f32) -> Runtime<Model> {
    Runtime::new(Model {
        options,
        neutral: None,
        dark,
        scale,
        requests: Vec::new(),
    })
    .unwrap()
}

fn centered_runtime(
    options: StandardSliderOptions<'static>,
    neutral: f64,
    dark: bool,
    scale: f32,
) -> Runtime<Model> {
    Runtime::new(Model {
        options,
        neutral: Some(neutral),
        dark,
        scale,
        requests: Vec::new(),
    })
    .unwrap()
}

fn paint(runtime: &mut Runtime<Model>, width: f32) -> DisplayList {
    runtime.render(width, 640., &mut Metrics).unwrap()
}

fn id(runtime: &Runtime<Model>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn bounds(runtime: &Runtime<Model>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn center(rect: Rect) -> f32 {
    rect.x + rect.width / 2.
}

fn right(rect: Rect) -> f32 {
    rect.x + rect.width
}

fn has_fill(list: &DisplayList, area: Rect, ink: Color, corners: Corners) -> bool {
    list.ops.iter().any(|op| match op {
        PaintOp::Shape {
            rect,
            color,
            corners: c,
            ..
        } => *rect == area && *color == ink && *c == corners,
        PaintOp::Quad {
            rect,
            color,
            radius,
            ..
        } => *rect == area && *color == ink && Corners::all(*radius) == corners,
        _ => false,
    })
}

fn alpha(color: Color, a: u8) -> Color {
    Color(color.0, color.1, color.2, a)
}

#[test]
fn both_themes_paint_all_five_exact_size_rows_and_responsive_rails() {
    for (size, track_height, handle_height, radius) in [
        (StandardSliderSize::ExtraSmall, 16., 44_f32, 8.),
        (StandardSliderSize::Small, 24., 44., 8.),
        (StandardSliderSize::Medium, 40., 52., 12.),
        (StandardSliderSize::Large, 56., 68., 16.),
        (StandardSliderSize::ExtraLarge, 96., 108., 28.),
    ] {
        for dark in [false, true] {
            let mut runtime = runtime(StandardSliderOptions { size, ..OPTIONS }, dark, 1.);
            let colors = runtime.app().theme().colors;
            for width in [320., 960.] {
                let list = paint(&mut runtime, width);
                let touch = bounds(&runtime, ID);
                let rail = bounds(&runtime, "volume-rail");
                let active = bounds(&runtime, "volume-active");
                let inactive = bounds(&runtime, "volume-inactive");
                let handle = bounds(&runtime, "volume-handle");
                let stop = bounds(&runtime, "volume-stop");
                assert_eq!(touch.height, handle_height.max(48.));
                assert_eq!(touch.width, width - 36.);
                assert_eq!(rail.x, touch.x + 12.);
                assert_eq!(rail.width, touch.width - 24.);
                assert_eq!(center(handle), rail.x + rail.width / 4.);
                assert_eq!((handle.width, handle.height), (4., handle_height));
                assert_eq!(
                    (active.height, inactive.height),
                    (track_height, track_height)
                );
                assert_eq!(active.x, rail.x);
                assert_eq!(right(active) + 6., handle.x);
                assert_eq!(right(handle) + 6., inactive.x);
                assert_eq!(right(inactive), right(rail));
                assert_eq!((stop.width, stop.height), (4., 4.));
                assert_eq!(right(stop) + 4., right(inactive));
                assert!(has_fill(
                    &list,
                    active,
                    colors.primary,
                    Corners::new(radius, 2., 2., radius)
                ));
                assert!(has_fill(
                    &list,
                    inactive,
                    colors.secondary_container,
                    Corners::new(2., radius, radius, 2.)
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
fn rtl_mirrors_all_parts_and_endpoints_keep_short_caps_inside_the_rail() {
    for value in [0., 4., 25., 97., 100.] {
        let mut ltr = runtime(StandardSliderOptions { value, ..OPTIONS }, false, 1.);
        let mut rtl = runtime(
            StandardSliderOptions {
                value,
                direction: LayoutDirection::RightToLeft,
                ..OPTIONS
            },
            false,
            1.,
        );
        paint(&mut ltr, 320.);
        let painted = paint(&mut rtl, 320.);
        let rail = bounds(&ltr, "volume-rail");
        let touch = bounds(&ltr, ID);
        for key in [
            "volume-active",
            "volume-inactive",
            "volume-handle",
            "volume-stop",
        ] {
            let nominal = bounds(&ltr, key);
            let mirrored = bounds(&rtl, key);
            assert_eq!(
                (nominal.y, nominal.width, nominal.height),
                (mirrored.y, mirrored.width, mirrored.height)
            );
            if nominal.width > 0. {
                assert!((center(mirrored) + center(nominal) - 2. * center(touch)).abs() < 0.001);
            }
        }
        let handle = bounds(&rtl, "volume-handle");
        let active = bounds(&rtl, "volume-active");
        let inactive = bounds(&rtl, "volume-inactive");
        let stop = bounds(&rtl, "volume-stop");
        let expected = match value {
            0. => right(rail),
            100. => rail.x,
            value => rail.x + rail.width * (1. - value as f32 / 100.),
        };
        assert!((center(handle) - expected).abs() < 0.001);
        if active.width > 0. {
            assert!((active.x - right(handle) - 6.).abs() < 0.001);
            let corners = Corners::new(2., 8., 8., 2.).resolve(active.width, active.height);
            assert!(has_fill(
                &painted,
                active,
                rtl.app().theme().colors.primary,
                corners
            ));
        }
        if inactive.width > 0. {
            assert!(inactive.width >= 8.);
            assert_eq!(stop.x, inactive.x + 4.);
            assert!((handle.x - right(inactive) - 6.).abs() < 0.001);
        } else {
            assert_eq!(stop.width, 0.);
        }
    }
}

#[test]
fn discrete_ties_snap_to_logical_stops_and_dense_dots_do_not_overlap() {
    for (current, snapped) in [(0., 0.), (12.5, 25.), (37.5, 50.), (100., 100.)] {
        let mut runtime = runtime(
            StandardSliderOptions {
                mode: StandardSliderMode::Discrete { intervals: 4 },
                value: current,
                ..OPTIONS
            },
            false,
            1.,
        );
        paint(&mut runtime, 320.);
        assert_eq!(
            runtime.dom().get(id(&runtime, ID)).unwrap().range_value(),
            Some(RangeValue {
                min: 0.,
                max: 100.,
                value: snapped,
                step: 25.
            })
        );
        let rail = bounds(&runtime, "volume-rail");
        assert_eq!(
            center(bounds(&runtime, "volume-handle")),
            rail.x + rail.width * snapped as f32 / 100.
        );
    }
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut runtime = runtime(
            StandardSliderOptions {
                mode: StandardSliderMode::Discrete {
                    intervals: u16::MAX,
                },
                direction,
                ..OPTIONS
            },
            true,
            1.,
        );
        for width in [80., 320.] {
            let list = paint(&mut runtime, width);
            let colors = runtime.app().theme().colors;
            let active = bounds(&runtime, "volume-active");
            let inactive = bounds(&runtime, "volume-inactive");
            let end = bounds(&runtime, "volume-stop");
            let ticks: Vec<_> = runtime
                .dom()
                .iter()
                .filter_map(|(id, node)| {
                    node.stable_id()?.strip_prefix("volume-tick-")?;
                    Some(runtime.computed().unwrap().nodes[id.index()].bounds)
                })
                .collect();
            assert_eq!(ticks.len(), 32);
            let mut visible: Vec<_> = ticks.into_iter().filter(|b| b.width > 0.).collect();
            if end.width > 0. {
                visible.push(end);
            }
            visible.sort_by(|a, b| a.x.total_cmp(&b.x));
            assert!(visible.len() <= ((width - 36. - 24.) / 8.).ceil() as usize);
            assert!(visible
                .windows(2)
                .all(|pair| pair[1].x - pair[0].x >= 7.999));
            for dot in visible {
                assert_eq!((dot.width, dot.height), (4., 4.));
                let inside =
                    |outer: Rect| dot.x >= outer.x - 0.001 && right(dot) <= right(outer) + 0.001;
                let color = if inside(active) {
                    colors.on_primary
                } else {
                    assert!(inside(inactive));
                    colors.on_secondary_container
                };
                assert!(has_fill(&list, dot, color, Corners::all(2.)));
            }
        }
    }
}

#[test]
fn focus_engages_a_two_dp_handle_and_measured_label_without_moving_the_target() {
    let label = "媒體音量五十百分比以及目前的播放輸出";
    let mut runtime = runtime(
        StandardSliderOptions {
            value: 0.,
            direction: LayoutDirection::RightToLeft,
            value_label: Some(label),
            ..OPTIONS
        },
        true,
        2.,
    );
    paint(&mut runtime, 320.);
    let before = bounds(&runtime, ID);
    let rail = bounds(&runtime, "volume-rail");
    assert!(runtime.dom().by_id("volume-label").is_none());
    assert_eq!(bounds(&runtime, "volume-handle").width, 4.);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ID));
    runtime.apply(Message::Engage(true)).unwrap();
    let list = paint(&mut runtime, 320.);
    let handle = bounds(&runtime, "volume-handle");
    let focus = bounds(&runtime, "volume-focus");
    let indicator = bounds(&runtime, "volume-label");
    let text = bounds(&runtime, "volume-label-text");
    let colors = runtime.app().theme().colors;
    assert_eq!(bounds(&runtime, ID), before);
    assert_eq!(bounds(&runtime, "volume-rail"), rail);
    assert_eq!((handle.width, center(handle)), (2., right(rail)));
    assert_eq!(focus.width, 12.);
    assert_eq!(indicator.y + indicator.height + 4., focus.y);
    assert!(indicator.height > 64.);
    assert!(indicator.x >= before.x && right(indicator) <= right(before));
    assert!(text.x >= indicator.x && right(text) <= right(indicator));
    assert!(list.ops.iter().any(|op| matches!(op, PaintOp::Stroke {
        rect, width: 3., color, ..
    } if *rect == focus && *color == colors.secondary)));
    assert!(list.ops.iter().any(|op| matches!(op, PaintOp::StyledText {
        text: painted, style, color, ..
    } if painted == label && style.font_size == 28. && *color == colors.inverse_on_surface)));
    let ax = runtime.accessibility_frame().unwrap();
    assert_eq!(ax.children(id(&runtime, ID)).count(), 0);
    assert!(ax.node(id(&runtime, "volume-label-text")).is_none());
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn one_slider_exposes_the_numeric_value_and_disabled_ink_has_no_binding() {
    for disabled in [false, true] {
        let mut runtime = runtime(
            StandardSliderOptions {
                disabled,
                engaged: true,
                value_label: Some("   "),
                ..OPTIONS
            },
            true,
            1.,
        );
        let list = paint(&mut runtime, 320.);
        let colors = runtime.app().theme().colors;
        assert!(runtime.dom().by_id("volume-label").is_none());
        let slider = id(&runtime, ID);
        let ax = runtime.accessibility_frame().unwrap();
        let node = ax.node(slider).unwrap().node;
        assert_eq!(
            (node.role(), node.name(), node.is_disabled()),
            (Role::Slider, "音量", disabled)
        );
        assert_eq!(
            node.range_value(),
            Some(RangeValue {
                min: 0.,
                max: 100.,
                value: 25.,
                step: 1.
            })
        );
        assert_eq!(ax.children(slider).count(), 0);
        if disabled {
            let handle = bounds(&runtime, "volume-handle");
            let active = bounds(&runtime, "volume-active");
            let inactive = bounds(&runtime, "volume-inactive");
            let stop = bounds(&runtime, "volume-stop");
            assert_eq!(handle.width, 4.);
            assert!(has_fill(
                &list,
                handle,
                alpha(colors.on_surface, 97),
                Corners::all(2.)
            ));
            assert!(has_fill(
                &list,
                active,
                alpha(colors.on_surface, 97),
                Corners::new(8., 2., 2., 8.)
            ));
            assert!(has_fill(
                &list,
                inactive,
                alpha(colors.on_surface, 31),
                Corners::new(2., 8., 8., 2.)
            ));
            assert!(has_fill(
                &list,
                stop,
                alpha(colors.on_surface, 97),
                Corners::all(2.)
            ));
            assert!(ax.focus_target(slider).is_none());
            assert!(runtime.set_value(ID, 26.).is_err());
            assert!(runtime.app().requests.is_empty());
        } else {
            assert!(ax.focus_target(slider).is_some());
            assert!(runtime.set_value(ID, 26.).unwrap());
            assert!(!runtime.set_value(ID, 26.).unwrap());
            assert_eq!(runtime.app().requests, [26.]);
        }
    }
}

#[test]
fn closely_spaced_discrete_values_keep_their_exact_logical_placement() {
    let epsilon = f64::EPSILON;
    let value = 1. + 3. * epsilon;
    let mut runtime = runtime(
        StandardSliderOptions {
            min: 1.,
            max: 1. + 4. * epsilon,
            value,
            mode: StandardSliderMode::Discrete { intervals: 3 },
            ..OPTIONS
        },
        false,
        1.,
    );
    paint(&mut runtime, 320.);
    let rail = bounds(&runtime, "volume-rail");
    let handle = bounds(&runtime, "volume-handle");
    assert!((center(handle) - (rail.x + rail.width * (2. / 3.))).abs() < 0.001);
    assert!(runtime
        .dom()
        .get(id(&runtime, ID))
        .unwrap()
        .range_value()
        .is_some_and(|range| range.value == value));
}

#[test]
fn invalid_ranges_or_names_never_build_a_slider() {
    let theme = Theme::light();
    for invalid in [
        StandardSliderOptions { id: "", ..OPTIONS },
        StandardSliderOptions {
            name: " ",
            ..OPTIONS
        },
        StandardSliderOptions {
            min: f64::NAN,
            ..OPTIONS
        },
        StandardSliderOptions {
            min: -f64::MAX,
            max: f64::MAX,
            ..OPTIONS
        },
        StandardSliderOptions { max: 0., ..OPTIONS },
        StandardSliderOptions {
            value: 101.,
            ..OPTIONS
        },
        StandardSliderOptions {
            mode: StandardSliderMode::Continuous { keyboard_step: 0. },
            ..OPTIONS
        },
        StandardSliderOptions {
            mode: StandardSliderMode::Continuous {
                keyboard_step: f64::EPSILON,
            },
            ..OPTIONS
        },
        StandardSliderOptions {
            mode: StandardSliderMode::Discrete { intervals: 0 },
            ..OPTIONS
        },
        StandardSliderOptions {
            // The six raw positions are distinct, but one rounded value
            // would select its adjacent logical stop on the next rebuild.
            min: 1. - 2. * f64::EPSILON,
            max: 1. + 3. * f64::EPSILON,
            value: 1.,
            mode: StandardSliderMode::Discrete { intervals: 6 },
            ..OPTIONS
        },
        StandardSliderOptions {
            min: 1.,
            max: 1. + 3. * f64::EPSILON,
            value: 1.,
            mode: StandardSliderMode::Discrete { intervals: 4 },
            ..OPTIONS
        },
        StandardSliderOptions {
            min: 1.,
            max: 1. + f64::EPSILON,
            value: 1.,
            mode: StandardSliderMode::Discrete { intervals: 100 },
            ..OPTIONS
        },
    ] {
        assert!(
            theme.standard_slider(invalid, |_| ()).is_none(),
            "{invalid:?}"
        );
    }
}

const SIGNED: StandardSliderOptions<'static> = StandardSliderOptions {
    min: -100.,
    max: 100.,
    value: 0.,
    ..OPTIONS
};

#[test]
fn centered_spans_merge_near_neutral_and_remain_mirrored_at_both_endpoints() {
    // The 300 dp viewport leaves a 240 dp rail after page and slider padding.
    let cases = [
        (-100., [(0., 0.), (8., 118.), (122., 240.)]),
        (-50., [(0., 52.), (68., 118.), (122., 240.)]),
        (-5., [(0., 106.), (0., 0.), (122., 240.)]),
        (0., [(0., 112.), (0., 0.), (128., 240.)]),
        (5., [(0., 118.), (0., 0.), (134., 240.)]),
        (50., [(0., 118.), (122., 172.), (188., 240.)]),
        (100., [(0., 118.), (122., 232.), (0., 0.)]),
    ];
    let sizes = [
        (StandardSliderSize::ExtraSmall, 16., 44., 8.),
        (StandardSliderSize::Small, 24., 44., 8.),
        (StandardSliderSize::Medium, 40., 52., 12.),
        (StandardSliderSize::Large, 56., 68., 16.),
        (StandardSliderSize::ExtraLarge, 96., 108., 28.),
    ];
    for (direction, dark) in [
        (LayoutDirection::LeftToRight, false),
        (LayoutDirection::RightToLeft, true),
    ] {
        let rtl = direction == LayoutDirection::RightToLeft;
        for (size, thickness, handle_height, outer) in sizes {
            for (value, spans) in cases {
                let mut runtime = centered_runtime(
                    StandardSliderOptions {
                        value,
                        direction,
                        size,
                        ..SIGNED
                    },
                    0.,
                    dark,
                    1.,
                );
                let colors = runtime.app().theme().colors;
                let list = paint(&mut runtime, 300.);
                let rail = bounds(&runtime, "volume-rail");
                assert_eq!(rail.width, 240.);
                let caps = |left, right| {
                    if rtl {
                        Corners::new(right, left, left, right)
                    } else {
                        Corners::new(left, right, right, left)
                    }
                };
                let parts = [
                    (
                        "volume-inactive-min",
                        colors.secondary_container,
                        caps(outer, 2.),
                    ),
                    ("volume-active", colors.primary, Corners::all(2.)),
                    (
                        "volume-inactive-max",
                        colors.secondary_container,
                        caps(2., outer),
                    ),
                ];
                for ((part, color, corners), (start, end)) in parts.into_iter().zip(spans) {
                    let area = bounds(&runtime, part);
                    assert!(
                        (area.width - (end - start)).abs() < 0.001,
                        "{part} at {value}"
                    );
                    if end > start {
                        let left = rail.x + if rtl { rail.width - end } else { start };
                        assert!((area.x - left).abs() < 0.001, "{part} at {value}");
                        assert_eq!(area.height, thickness);
                        assert!(
                            has_fill(&list, area, color, corners.resolve(area.width, area.height)),
                            "{part} at {value}"
                        );
                    }
                }
                for (index, name) in ["volume-stop-min", "volume-stop-max"]
                    .into_iter()
                    .enumerate()
                {
                    let logical_minimum = index == 0;
                    let stop = bounds(&runtime, name);
                    let endpoint = if logical_minimum { -100. } else { 100. };
                    if value == endpoint {
                        assert_eq!(stop.width, 0.);
                    } else {
                        let physical_left = logical_minimum != rtl;
                        let x = if physical_left {
                            rail.x + 4.
                        } else {
                            right(rail) - 8.
                        };
                        assert_eq!((stop.x, stop.width, stop.height), (x, 4., 4.));
                        assert!(has_fill(
                            &list,
                            stop,
                            colors.on_secondary_container,
                            Corners::all(2.)
                        ));
                    }
                }
                let handle = bounds(&runtime, "volume-handle");
                assert_eq!((handle.width, handle.height), (4., handle_height));
                assert!(has_fill(&list, handle, colors.primary, Corners::all(2.)));
            }
        }
    }
}

#[test]
fn centered_discrete_dots_take_their_actual_span_color_and_skip_both_cuts() {
    let options = StandardSliderOptions {
        mode: StandardSliderMode::Discrete { intervals: 8 },
        ..SIGNED
    };
    for (value, active, inactive) in [(-50., 3, 5), (50., 5, 3)] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut runtime = centered_runtime(
                StandardSliderOptions {
                    value,
                    direction,
                    ..options
                },
                0.,
                true,
                1.,
            );
            let list = paint(&mut runtime, 300.);
            let colors = runtime.app().theme().colors;
            assert!(runtime.dom().by_id("volume-tick-0").is_none());
            assert!(runtime.dom().by_id("volume-tick-8").is_none());
            assert_eq!(bounds(&runtime, "volume-tick-4").width, 0.);
            let current = if value < 0. { 2 } else { 6 };
            assert_eq!(
                bounds(&runtime, &format!("volume-tick-{current}")).width,
                0.
            );
            let active = bounds(&runtime, &format!("volume-tick-{active}"));
            let inactive = bounds(&runtime, &format!("volume-tick-{inactive}"));
            assert_eq!((active.width, inactive.width), (4., 4.));
            assert!(has_fill(&list, active, colors.on_primary, Corners::all(2.)));
            assert!(has_fill(
                &list,
                inactive,
                colors.on_secondary_container,
                Corners::all(2.)
            ));
        }
    }
}

#[test]
fn centered_checks_the_neutral_without_changing_the_existing_lattice() {
    let theme = Theme::light();
    for invalid in [-100., 100., 101., f64::NAN, f64::INFINITY] {
        assert!(theme.centered_slider(SIGNED, invalid, |_| ()).is_none());
    }
    let signed_steps = StandardSliderOptions {
        min: -1.,
        max: 1.,
        mode: StandardSliderMode::Discrete { intervals: 10 },
        ..SIGNED
    };
    assert!(theme.centered_slider(signed_steps, 0.2, |_| ()).is_some());
    assert!(theme.centered_slider(signed_steps, 0.19, |_| ()).is_none());
    let odd = StandardSliderOptions {
        mode: StandardSliderMode::Discrete { intervals: 3 },
        ..signed_steps
    };
    assert!(theme.centered_slider(odd, 0., |_| ()).is_none());

    let mut runtime = centered_runtime(
        StandardSliderOptions {
            value: 25.,
            ..SIGNED
        },
        -25.,
        false,
        1.,
    );
    paint(&mut runtime, 300.);
    let rail = bounds(&runtime, "volume-rail");
    let active = bounds(&runtime, "volume-active");
    assert_eq!((active.x - rail.x, active.width), (92., 50.));

    let mut narrow = centered_runtime(SIGNED, 0., false, 1.);
    paint(&mut narrow, 80.);
    assert_eq!(bounds(&narrow, ID).width, 48.);
    for part in [
        "volume-inactive-min",
        "volume-inactive-max",
        "volume-active",
        "volume-stop-min",
        "volume-stop-max",
    ] {
        assert_eq!(bounds(&narrow, part).width, 0.);
    }
    assert_eq!(bounds(&narrow, "volume-handle").width, 4.);
}

#[test]
fn centered_keeps_one_adjustable_target_and_signed_values_through_the_shared_path() {
    let mut runtime = centered_runtime(
        StandardSliderOptions {
            engaged: true,
            value_label: Some("−37％"),
            direction: LayoutDirection::RightToLeft,
            ..SIGNED
        },
        0.,
        true,
        2.,
    );
    paint(&mut runtime, 320.);
    let owner = bounds(&runtime, ID);
    let rail = bounds(&runtime, "volume-rail");
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ID));
    assert!(runtime.set_value(ID, -37.25).unwrap());
    assert!(!runtime.set_value(ID, -37.25).unwrap());
    paint(&mut runtime, 320.);
    let frame = runtime.accessibility_frame().unwrap();
    let slider = id(&runtime, ID);
    let node = frame.node(slider).unwrap().node;
    assert_eq!(
        (node.role(), node.name(), node.slider_value_text()),
        (Role::Slider, "音量", Some("−37％"))
    );
    assert_eq!(
        node.range_value(),
        Some(RangeValue {
            min: -100.,
            max: 100.,
            value: -37.25,
            step: 1.
        })
    );
    assert_eq!(frame.children(slider).count(), 0);
    let action = frame.adjustment_target(slider).unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&action, AccessibilityAdjustment::SetValue(200.))
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted
    );
    paint(&mut runtime, 320.);
    assert_eq!(runtime.app().requests, [-37.25, 100.]);
    assert_eq!(runtime.focused(), Some(ID));
    assert_eq!(bounds(&runtime, ID), owner);
    assert_eq!(bounds(&runtime, "volume-rail"), rail);
    assert_eq!(center(bounds(&runtime, "volume-handle")), rail.x);
}

#[test]
fn disabled_centered_keeps_both_muted_anchors_without_an_action_or_indicator() {
    let mut runtime = centered_runtime(
        StandardSliderOptions {
            disabled: true,
            engaged: true,
            value_label: Some("零"),
            ..SIGNED
        },
        0.,
        true,
        1.,
    );
    let list = paint(&mut runtime, 320.);
    let colors = runtime.app().theme().colors;
    for part in ["volume-stop-min", "volume-stop-max"] {
        assert!(has_fill(
            &list,
            bounds(&runtime, part),
            alpha(colors.on_surface, 97),
            Corners::all(2.),
        ));
    }
    assert_eq!(bounds(&runtime, "volume-active").width, 0.);
    assert_eq!(bounds(&runtime, "volume-handle").width, 4.);
    assert!(runtime.dom().by_id("volume-label").is_none());
    let frame = runtime.accessibility_frame().unwrap();
    let slider = id(&runtime, ID);
    assert!(frame.node(slider).unwrap().node.is_disabled());
    assert!(frame.focus_target(slider).is_none());
    assert!(frame.adjustment_target(slider).is_none());
    assert!(runtime.set_value(ID, 10.).is_err());
}
