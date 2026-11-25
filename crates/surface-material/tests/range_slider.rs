use surface::{
    Color, Corners, DisplayList, Length, Node, PaintOp, RangeValue, Rect, Role, Style, TextMeasurer,
};
use surface_app::{AppError, Application, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection, RangeSliderChange, RangeSliderOptions, RangeSliderThumb,
    StandardSliderMode, StandardSliderSize, Theme,
};

const OPTIONS: RangeSliderOptions<'static> = RangeSliderOptions {
    id: "distance",
    name: "距離區間",
    lower_name: "最短距離",
    upper_name: "最遠距離",
    min: -10.,
    max: 90.,
    lower: 15.,
    upper: 65.,
    mode: StandardSliderMode::Continuous { keyboard_step: 5. },
    size: StandardSliderSize::ExtraSmall,
    direction: LayoutDirection::LeftToRight,
    disabled: false,
    engaged: None,
    lower_label: Some("最短距離十五公里"),
    upper_label: Some("最遠距離六十五公里"),
};

struct Model {
    options: RangeSliderOptions<'static>,
    dark: bool,
    scale: f32,
    requests: Vec<RangeSliderChange>,
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
    type Message = RangeSliderChange;

    fn update(&mut self, change: RangeSliderChange) {
        self.options.lower = change.lower;
        self.options.upper = change.upper;
        self.requests.push(change);
    }

    fn view(&self) -> View<RangeSliderChange> {
        View::new(Node::new(Role::Window, "").styled(Style::column().padding(18.).gap(8.)))
            .children([
                View::new(
                    Node::new(Role::Group, "").styled(Style::column().height(Length::Px(156.))),
                ),
                self.theme()
                    .range_slider(self.options, std::convert::identity)
                    .expect("valid test range slider"),
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
        let glyphs = text.chars().count() as f32;
        let available = width.unwrap_or(f32::INFINITY);
        let columns = (available / advance).floor().max(1.);
        (
            (advance * glyphs.min(columns)).min(available),
            (glyphs / columns).ceil().max(1.) * font_size * 1.5,
        )
    }
}

fn runtime(options: RangeSliderOptions<'static>, dark: bool, scale: f32) -> Runtime<Model> {
    Runtime::new(Model {
        options,
        dark,
        scale,
        requests: Vec::new(),
    })
    .unwrap()
}

fn paint(runtime: &mut Runtime<Model>, width: f32) -> DisplayList {
    runtime.render(width, 640., &mut Metrics).unwrap()
}

fn bounds(runtime: &Runtime<Model>, key: &str) -> Rect {
    let id = runtime.dom().by_id(key).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn center(rect: Rect) -> f32 {
    rect.x + rect.width / 2.
}

fn right(rect: Rect) -> f32 {
    rect.x + rect.width
}

fn bottom(rect: Rect) -> f32 {
    rect.y + rect.height
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

#[test]
fn five_sizes_paint_two_handles_between_rounded_inactive_ends() {
    for (size, track_height, handle_height, radius) in [
        (StandardSliderSize::ExtraSmall, 16., 44_f32, 8.),
        (StandardSliderSize::Small, 24., 44., 8.),
        (StandardSliderSize::Medium, 40., 52., 12.),
        (StandardSliderSize::Large, 56., 68., 16.),
        (StandardSliderSize::ExtraLarge, 96., 108., 28.),
    ] {
        for dark in [false, true] {
            let mut runtime = runtime(RangeSliderOptions { size, ..OPTIONS }, dark, 1.);
            let colors = runtime.app().theme().colors;
            for width in [320., 960.] {
                let list = paint(&mut runtime, width);
                let owner = bounds(&runtime, "distance");
                let rail = bounds(&runtime, "distance-rail");
                let lower_target = bounds(&runtime, "distance-lower");
                let upper_target = bounds(&runtime, "distance-upper");
                let lower = bounds(&runtime, "distance-lower-handle");
                let upper = bounds(&runtime, "distance-upper-handle");
                let active = bounds(&runtime, "distance-active");
                let min = bounds(&runtime, "distance-inactive-min");
                let max = bounds(&runtime, "distance-inactive-max");
                let stop_min = bounds(&runtime, "distance-stop-min");
                let stop_max = bounds(&runtime, "distance-stop-max");
                assert_eq!(rail.x, owner.x + 24.);
                assert_eq!(rail.width, owner.width - 48.);
                assert_eq!(center(lower), rail.x + rail.width * 0.25);
                assert_eq!(center(upper), rail.x + rail.width * 0.75);
                for (target, handle) in [(lower_target, lower), (upper_target, upper)] {
                    assert_eq!((target.width, target.height), (48., handle_height.max(48.)));
                    assert_eq!((handle.width, handle.height), (4., handle_height));
                    assert_eq!(center(target), center(handle));
                    assert!(has_fill(&list, handle, colors.primary, Corners::all(2.)));
                }
                assert_eq!(active.height, track_height);
                assert_eq!(min.height, track_height);
                assert_eq!(max.height, track_height);
                assert_eq!(right(min) + 6., lower.x);
                assert_eq!(right(lower) + 6., active.x);
                assert_eq!(right(active) + 6., upper.x);
                assert_eq!(right(upper) + 6., max.x);
                assert_eq!(stop_min.x, rail.x + 4.);
                assert_eq!(right(stop_max), right(rail) - 4.);
                for stop in [stop_min, stop_max] {
                    assert_eq!((stop.width, stop.height), (4., 4.));
                    assert!(has_fill(
                        &list,
                        stop,
                        colors.on_secondary_container,
                        Corners::all(2.)
                    ));
                }
                assert!(has_fill(&list, active, colors.primary, Corners::all(2.)));
                assert!(has_fill(
                    &list,
                    min,
                    colors.secondary_container,
                    Corners::new(radius, 2., 2., radius).resolve(min.width, min.height)
                ));
                assert!(has_fill(
                    &list,
                    max,
                    colors.secondary_container,
                    Corners::new(2., radius, radius, 2.).resolve(max.width, max.height)
                ));
            }
        }
    }
}

#[test]
fn rtl_mirrors_art_and_preserves_the_named_lower_then_upper_focus_order() {
    let options = RangeSliderOptions {
        mode: StandardSliderMode::Discrete { intervals: 8 },
        ..OPTIONS
    };
    let mut ltr = runtime(options, false, 1.);
    let mut rtl = runtime(
        RangeSliderOptions {
            direction: LayoutDirection::RightToLeft,
            ..options
        },
        true,
        1.,
    );
    paint(&mut ltr, 320.);
    let list = paint(&mut rtl, 320.);
    let owner = bounds(&rtl, "distance");
    for key in [
        "distance-rail",
        "distance-active",
        "distance-inactive-min",
        "distance-inactive-max",
        "distance-stop-min",
        "distance-stop-max",
        "distance-lower",
        "distance-upper",
        "distance-lower-handle",
        "distance-upper-handle",
        "distance-tick-4",
    ] {
        let a = bounds(&ltr, key);
        let b = bounds(&rtl, key);
        assert_eq!(a.x + right(b), owner.x + right(owner), "{key}");
        assert_eq!((a.y, a.width, a.height), (b.y, b.width, b.height), "{key}");
    }
    let theme = rtl.app().theme();
    assert!(has_fill(
        &list,
        bounds(&rtl, "distance-inactive-min"),
        theme.colors.secondary_container,
        Corners::new(2., 8., 8., 2.),
    ));
    assert!(has_fill(
        &list,
        bounds(&rtl, "distance-tick-4"),
        theme.colors.on_primary,
        Corners::all(2.),
    ));
    assert!(has_fill(
        &list,
        bounds(&rtl, "distance-tick-1"),
        theme.colors.on_secondary_container,
        Corners::all(2.),
    ));

    let dom = rtl.dom();
    let pair = dom.by_id("distance").unwrap();
    assert_eq!(dom.get(pair).unwrap().name(), "距離區間");
    let sliders = dom
        .children(pair)
        .filter_map(|id| {
            let node = dom.get(id).unwrap();
            (node.role() == Role::Slider).then_some(node)
        })
        .collect::<Vec<_>>();
    assert_eq!(sliders.len(), 2);
    for (node, name, value, label) in [
        (sliders[0], OPTIONS.lower_name, 15., OPTIONS.lower_label),
        (sliders[1], OPTIONS.upper_name, 65., OPTIONS.upper_label),
    ] {
        assert_eq!(node.name(), name);
        assert_eq!(node.slider_value_text(), label);
        assert_eq!(
            node.range_value(),
            Some(RangeValue {
                min: -10.,
                max: 90.,
                value,
                step: 12.5
            })
        );
    }
    rtl.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rtl.focused(), Some("distance-lower"));
    rtl.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rtl.focused(), Some("distance-upper"));
    rtl.dispatch(Event::FocusNext { reverse: true }).unwrap();
    assert_eq!(rtl.focused(), Some("distance-lower"));
}

#[test]
fn equality_has_no_active_fill_and_both_real_targets_stay_at_the_endpoints() {
    for value in [-10., 40., 90.] {
        let mut runtime = runtime(
            RangeSliderOptions {
                lower: value,
                upper: value,
                engaged: Some(RangeSliderThumb::Lower),
                ..OPTIONS
            },
            false,
            1.,
        );
        let list = paint(&mut runtime, 320.);
        let owner = bounds(&runtime, "distance");
        let rail = bounds(&runtime, "distance-rail");
        let target = bounds(&runtime, "distance-lower");
        let active = bounds(&runtime, "distance-active");
        let lower = bounds(&runtime, "distance-lower-handle");
        let upper = bounds(&runtime, "distance-upper-handle");
        let expected_position = rail.x + rail.width * ((value as f32 + 10.) / 100.);
        assert_eq!(active.width, 0.);
        assert_eq!(target, bounds(&runtime, "distance-upper"));
        assert_eq!((target.width, target.height), (48., 48.));
        assert_eq!(center(target), expected_position);
        assert!(target.x >= owner.x && right(target) <= right(owner));
        assert_eq!((lower.width, upper.width), (2., 4.));
        assert!(has_fill(
            &list,
            lower,
            runtime.app().theme().colors.primary,
            Corners::all(1.)
        ));
        assert!(!has_fill(
            &list,
            upper,
            runtime.app().theme().colors.primary,
            Corners::all(2.)
        ));
        assert!(!has_fill(
            &list,
            active,
            runtime.app().theme().colors.primary,
            Corners::all(0.)
        ));
        if value == -10. {
            assert_eq!(bounds(&runtime, "distance-stop-min").width, 0.);
        }
        if value == 90. {
            assert_eq!(bounds(&runtime, "distance-stop-max").width, 0.);
        }
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("distance-lower"));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("distance-upper"));
    }
}

#[test]
fn enlarged_rtl_label_clamps_at_the_owner_edge_and_clears_the_focus_outline() {
    let mut runtime = runtime(
        RangeSliderOptions {
            direction: LayoutDirection::RightToLeft,
            lower: OPTIONS.min,
            engaged: Some(RangeSliderThumb::Lower),
            ..OPTIONS
        },
        true,
        2.,
    );
    paint(&mut runtime, 320.);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    let list = paint(&mut runtime, 320.);
    let owner = bounds(&runtime, "distance");
    let label = bounds(&runtime, "distance-lower-label");
    let label_text = bounds(&runtime, "distance-lower-label-text");
    let ring = bounds(&runtime, "distance-lower-focus");
    let handle = bounds(&runtime, "distance-lower-handle");
    let colors = runtime.app().theme().colors;
    assert_eq!(right(label), right(owner));
    assert!(label.height > 44. && label.y > 0.);
    assert!(label_text.x >= label.x && right(label_text) <= right(label));
    assert_eq!(ring.x, handle.x - 5.);
    assert_eq!(ring.width, 12.);
    assert_eq!(ring.height, 54.);
    assert_eq!(bottom(label) + 4., ring.y);
    assert!(has_fill(
        &list,
        label,
        colors.inverse_surface,
        Corners::full().resolve(label.width, label.height)
    ));
    assert!(runtime.dom().by_id("distance-upper-label").is_none());
    assert!(list.ops.iter().any(|op| matches!(op,
        PaintOp::Stroke { rect, color, width, .. }
        if *rect == ring && *color == colors.secondary && *width == 3.
    )));
}

#[test]
fn controlled_pair_changes_keep_the_common_lattice_and_resolve_at_the_peer() {
    let mut runtime = runtime(
        RangeSliderOptions {
            mode: StandardSliderMode::Discrete { intervals: 8 },
            ..OPTIONS
        },
        false,
        1.,
    );
    paint(&mut runtime, 320.);
    for (id, value) in [
        ("distance-lower", 23.),
        ("distance-upper", 66.),
        ("distance-lower", 88.),
        ("distance-upper", -8.),
        ("distance-upper", 88.),
    ] {
        runtime
            .dispatch(Event::SetValue {
                id: id.into(),
                value,
            })
            .unwrap();
        paint(&mut runtime, 320.);
    }
    assert_eq!(
        runtime.app().requests,
        [
            RangeSliderChange {
                thumb: RangeSliderThumb::Lower,
                lower: 15. + 12.5,
                upper: 65.
            },
            RangeSliderChange {
                thumb: RangeSliderThumb::Lower,
                lower: 65.,
                upper: 65.
            },
            RangeSliderChange {
                thumb: RangeSliderThumb::Upper,
                lower: 65.,
                upper: 90.
            },
        ]
    );
}

#[test]
fn disabled_and_invalid_options_are_inert_or_rejected_before_building_the_pair() {
    for options in [
        RangeSliderOptions {
            id: "  ",
            ..OPTIONS
        },
        RangeSliderOptions {
            name: "",
            ..OPTIONS
        },
        RangeSliderOptions {
            upper_name: " 最短距離 ",
            ..OPTIONS
        },
        RangeSliderOptions {
            upper: 14.,
            ..OPTIONS
        },
        RangeSliderOptions {
            upper: 100.,
            ..OPTIONS
        },
        RangeSliderOptions {
            lower: f64::NAN,
            ..OPTIONS
        },
        RangeSliderOptions {
            mode: StandardSliderMode::Discrete { intervals: 0 },
            ..OPTIONS
        },
    ] {
        assert!(Theme::light()
            .range_slider(options, std::convert::identity)
            .is_none());
    }
    let mut runtime = runtime(
        RangeSliderOptions {
            disabled: true,
            engaged: Some(RangeSliderThumb::Lower),
            ..OPTIONS
        },
        false,
        1.,
    );
    let list = paint(&mut runtime, 320.);
    let ink = runtime.app().theme().colors.on_surface;
    assert!(has_fill(
        &list,
        bounds(&runtime, "distance-active"),
        Color(ink.0, ink.1, ink.2, 97),
        Corners::all(2.),
    ));
    assert!(runtime.dom().by_id("distance-lower-label").is_none());
    for id in ["distance-lower", "distance-upper"] {
        let node = runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap();
        assert!(node.is_disabled() && !node.is_tab_stop());
        assert!(matches!(
            runtime.dispatch(Event::SetValue { id: id.into(), value: 51. }),
            Err(AppError::MissingControl(missing)) if missing == id
        ));
    }
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), None);
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn fractional_values_keep_both_targets_and_all_gaps_on_the_same_rail() {
    let mut runtime = runtime(
        RangeSliderOptions {
            lower: 23.3,
            upper: 67.1,
            engaged: Some(RangeSliderThumb::Upper),
            ..OPTIONS
        },
        false,
        1.,
    );
    for width in [320., 321., 959., 960.] {
        paint(&mut runtime, width);
        let rail = bounds(&runtime, "distance-rail");
        let lower = bounds(&runtime, "distance-lower-handle");
        let upper = bounds(&runtime, "distance-upper-handle");
        let lower_target = bounds(&runtime, "distance-lower");
        let upper_target = bounds(&runtime, "distance-upper");
        let active = bounds(&runtime, "distance-active");
        let min = bounds(&runtime, "distance-inactive-min");
        let max = bounds(&runtime, "distance-inactive-max");
        let near = |a: f32, b: f32| assert!((a - b).abs() < 0.0002, "{a} vs {b} at {width}");
        near(center(lower), rail.x + rail.width * 0.333);
        near(center(upper), rail.x + rail.width * 0.771);
        near(center(lower), center(lower_target));
        near(center(upper), center(upper_target));
        near(lower.x - right(min), 6.);
        near(active.x - right(lower), 6.);
        near(upper.x - right(active), 6.);
        near(max.x - right(upper), 6.);
    }
}
