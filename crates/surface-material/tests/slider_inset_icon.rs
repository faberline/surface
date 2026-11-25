use surface::{
    Align, Color, ExtendedDisplayList, ExtendedPaintOp, Length, Node, PathFill, Rect, Role, Style,
    TextMeasurer,
};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection, SliderInsetIcon, StandardSliderMode, StandardSliderOptions,
    StandardSliderSize, Theme, VectorIcon, VerticalStandardSliderPlacement,
};

const ID: &str = "volume";
const ACTIVE: &str = "volume-inset-icon-active";
const INACTIVE: &str = "volume-inset-icon-inactive";
const NORMAL: [[f32; 2]; 3] = [[0., 0.], [24., 4.], [7., 24.]];
const MUTED: [[f32; 2]; 3] = [[0., 6.], [24., 0.], [24., 24.]];
const OPTIONS: StandardSliderOptions<'static> = StandardSliderOptions {
    id: ID,
    name: "音量",
    min: 0.,
    max: 100.,
    value: 50.,
    mode: StandardSliderMode::Continuous { keyboard_step: 5. },
    size: StandardSliderSize::Medium,
    direction: LayoutDirection::LeftToRight,
    disabled: false,
    engaged: false,
    value_label: None,
};

struct Fixture {
    options: StandardSliderOptions<'static>,
    vertical: bool,
    height: f32,
    decorate: bool,
    dark: bool,
    alternate: bool,
    scale: f32,
    values: Vec<f64>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            options: OPTIONS,
            vertical: false,
            height: 300.,
            decorate: true,
            dark: false,
            alternate: true,
            scale: 1.,
            values: Vec::new(),
        }
    }
}

impl Fixture {
    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }
}

impl Application for Fixture {
    type Message = f64;

    fn update(&mut self, message: f64) {
        self.options.value = message;
        self.values.push(message);
    }

    fn view(&self) -> View<f64> {
        let theme = self.theme();
        let normal = VectorIcon::new(NORMAL).unwrap();
        let muted = VectorIcon::new(MUTED).unwrap();
        let icon = SliderInsetIcon {
            icon: &normal,
            at_minimum: self.alternate.then_some(&muted),
        };
        let placement = VerticalStandardSliderPlacement {
            height: self.height,
            label_room: 120.,
        };
        let field = match (self.vertical, self.decorate) {
            (false, false) => theme.standard_slider(self.options, |value| value),
            (false, true) => {
                theme.standard_slider_with_inset_icon(self.options, icon, |value| value)
            }
            (true, false) => theme.vertical_standard_slider(self.options, placement, |value| value),
            (true, true) => theme.vertical_standard_slider_with_inset_icon(
                self.options,
                placement,
                icon,
                |value| value,
            ),
        }
        .unwrap();
        View::new(Node::new(Role::Window, "").styled(Style::column().align(Align::Center)))
            .children([
                View::new(Node::new(Role::Group, "").styled(Style::row().height(Length::Px(120.)))),
                field,
            ])
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = size * text.chars().count() as f32 * 0.6;
        let width = width.unwrap_or(natural).min(natural);
        let lines = (natural / width.max(1.)).ceil().max(1.);
        (width, size * 1.25 * lines)
    }
}

fn frame(model: Fixture, width: f32) -> (Runtime<Fixture>, ExtendedDisplayList) {
    let mut runtime = Runtime::new(model).unwrap();
    let list = runtime.render_extended(width, 600., &mut Metrics).unwrap();
    (runtime, list)
}

fn box_for(runtime: &Runtime<Fixture>, key: &str) -> Rect {
    let id = runtime.dom().by_id(key).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn icon_box(runtime: &Runtime<Fixture>) -> Rect {
    let active = box_for(runtime, ACTIVE);
    let inactive = box_for(runtime, INACTIVE);
    assert!(!has_area(active) || !has_area(inactive));
    if has_area(active) {
        active
    } else {
        inactive
    }
}

fn paths(paint: &ExtendedDisplayList) -> Vec<&PathFill> {
    paint
        .ops()
        .iter()
        .filter_map(|op| match op {
            ExtendedPaintOp::FillPath(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn close(left: f32, right: f32) {
    assert!((left - right).abs() < 0.002, "{left} != {right}");
}

fn has_area(rect: Rect) -> bool {
    rect.width > 0. && rect.height > 0.
}

fn glyph(paint: &ExtendedDisplayList, rect: Rect, ink: Color, points: [[f32; 2]; 3]) {
    let found = paths(paint);
    assert_eq!(found.len(), 1);
    let path = found[0];
    assert_eq!(path.rect(), rect);
    assert_eq!(path.clip().intersect(rect), rect);
    assert_eq!(path.color(), ink);
    for (point, [x, y]) in path.points().iter().zip(points) {
        close(point[0], rect.x + x * rect.width / 24.);
        close(point[1], rect.y + y * rect.height / 24.);
    }
}

#[test]
fn token_sizes_mirror_horizontally_with_an_upright_glyph_at_both_track_ends() {
    use LayoutDirection::{LeftToRight, RightToLeft};
    use StandardSliderSize::{ExtraLarge, Large, Medium};
    for (size, side, padding) in [(Medium, 24., 6.), (Large, 24., 6.), (ExtraLarge, 32., 8.)] {
        for value in [0., 50., 100.] {
            let active = value > 0.;
            let mut old = None;
            for direction in [LeftToRight, RightToLeft] {
                let (runtime, paint) = frame(
                    Fixture {
                        options: StandardSliderOptions {
                            direction,
                            size,
                            value,
                            ..OPTIONS
                        },
                        ..Fixture::default()
                    },
                    360.,
                );
                let rect = icon_box(&runtime);
                let colors = runtime.app().theme().colors;
                let ink = if active {
                    colors.on_primary
                } else {
                    colors.on_secondary_container
                };
                assert_eq!((rect.width, rect.height), (side, side));
                glyph(&paint, rect, ink, if active { NORMAL } else { MUTED });
                let track = box_for(
                    &runtime,
                    if active {
                        "volume-active"
                    } else {
                        "volume-inactive"
                    },
                );
                close(rect.y, track.y + (track.height - side) / 2.);
                assert_eq!(rect.intersect(track), rect);
                if direction == LeftToRight {
                    if active {
                        close(rect.x - track.x, padding);
                    } else {
                        let stop = box_for(&runtime, "volume-stop");
                        close(stop.x - rect.x - side, padding);
                    }
                    old = Some(rect);
                } else {
                    close(rect.x, 360. - old.unwrap().x - side);
                }
            }
        }
    }
}

#[test]
fn vertical_glyph_stays_upright_and_the_same_size_on_both_layout_directions() {
    for value in [0., 50.] {
        let mut old = None;
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let (runtime, paint) = frame(
                Fixture {
                    options: StandardSliderOptions {
                        direction,
                        value,
                        ..OPTIONS
                    },
                    vertical: true,
                    dark: true,
                    ..Fixture::default()
                },
                320.,
            );
            let rect = icon_box(&runtime);
            if let Some(old) = old {
                assert_eq!(old, rect);
            }
            old = Some(rect);
            assert_eq!((rect.width, rect.height), (24., 24.));
            let colors = runtime.app().theme().colors;
            if value != 0. {
                let track = box_for(&runtime, "volume-active");
                close(track.y + track.height - rect.y - rect.height, 6.);
                glyph(&paint, rect, colors.on_primary, NORMAL);
            } else {
                let stop = box_for(&runtime, "volume-stop");
                close(rect.y - stop.y - stop.height, 6.);
                glyph(&paint, rect, colors.on_secondary_container, MUTED);
            }
        }
    }
}

#[test]
fn measured_clearance_relocates_or_omits_the_icon_without_dropping_any_track_marks() {
    for (width, mode, drawn) in [
        (360., StandardSliderMode::Discrete { intervals: 8 }, true),
        (270., StandardSliderMode::Discrete { intervals: 32 }, false),
        (70., OPTIONS.mode, false),
        (360., OPTIONS.mode, true),
    ] {
        let options = StandardSliderOptions { mode, ..OPTIONS };
        let (new, new_paint) = frame(
            Fixture {
                options,
                ..Fixture::default()
            },
            width,
        );
        let (old, old_paint) = frame(
            Fixture {
                options,
                decorate: false,
                ..Fixture::default()
            },
            width,
        );
        let count = usize::from(drawn);
        assert_eq!(paths(&new_paint).len(), count);
        assert_eq!(paths(&old_paint).len(), 0);
        let new_track_ops: Vec<_> = new_paint
            .ops()
            .iter()
            .filter(|op| !matches!(op, ExtendedPaintOp::FillPath(_)))
            .cloned()
            .collect();
        assert_eq!(new_track_ops, old_paint.ops());
        if drawn {
            let icon = icon_box(&new);
            for part in ["volume-stop", "volume-focus", "volume-handle"] {
                assert!(!has_area(icon.intersect(box_for(&new, part))));
            }
            if let StandardSliderMode::Discrete { intervals } = mode {
                for step in 0..intervals {
                    let part = format!("volume-tick-{step}");
                    assert_eq!(box_for(&new, &part), box_for(&old, &part));
                    assert!(!has_area(icon.intersect(box_for(&new, &part))));
                }
            }
        } else {
            assert!(!has_area(icon_box(&new)));
        }
    }

    let (runtime, paint) = frame(
        Fixture {
            options: StandardSliderOptions {
                value: 8.,
                ..OPTIONS
            },
            alternate: false,
            ..Fixture::default()
        },
        360.,
    );
    let icon = icon_box(&runtime);
    assert_eq!(icon.intersect(box_for(&runtime, "volume-inactive")), icon);
    glyph(
        &paint,
        icon,
        runtime.app().theme().colors.on_secondary_container,
        NORMAL,
    );

    // Layout alone can retire and restore paint without changing the value.
    let (mut runtime, _) = frame(Fixture::default(), 360.);
    let icon = runtime.dom().by_id(ACTIVE).unwrap();
    let narrow = runtime.render_extended(70., 600., &mut Metrics).unwrap();
    assert!(paths(&narrow).is_empty());
    let wide = runtime.render_extended(360., 600., &mut Metrics).unwrap();
    assert_eq!(runtime.dom().by_id(ACTIVE), Some(icon));
    assert_eq!(paths(&wide).len(), 1);
}

#[test]
fn small_sizes_keep_the_standard_display_list_and_disabled_ink_stays_passive() {
    for size in [StandardSliderSize::ExtraSmall, StandardSliderSize::Small] {
        for vertical in [false, true] {
            let options = StandardSliderOptions { size, ..OPTIONS };
            let (on, decorated) = frame(
                Fixture {
                    options,
                    vertical,
                    ..Fixture::default()
                },
                320.,
            );
            let (_, plain) = frame(
                Fixture {
                    options,
                    vertical,
                    decorate: false,
                    ..Fixture::default()
                },
                320.,
            );
            assert_eq!(decorated, plain);
            assert!(on.dom().by_id(ACTIVE).is_none() && on.dom().by_id(INACTIVE).is_none());
        }
    }
    for value in [0., 50.] {
        let (runtime, paint) = frame(
            Fixture {
                options: StandardSliderOptions {
                    value,
                    disabled: true,
                    ..OPTIONS
                },
                dark: true,
                ..Fixture::default()
            },
            320.,
        );
        let colors = runtime.app().theme().colors;
        let (ink, points) = if value == 0. {
            (colors.on_surface, MUTED)
        } else {
            (colors.inverse_on_surface, NORMAL)
        };
        glyph(
            &paint,
            icon_box(&runtime),
            Color(ink.0, ink.1, ink.2, 97),
            points,
        );
        let field = runtime.dom().by_id(ID).unwrap();
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .adjustment_target(field)
            .is_none());
    }
}

#[test]
fn the_glyph_has_no_ax_node_and_focus_label_and_input_match_the_existing_slider() {
    for vertical in [false, true] {
        let options = StandardSliderOptions {
            engaged: true,
            value_label: Some("50％"),
            direction: LayoutDirection::RightToLeft,
            ..OPTIONS
        };
        let fixture = |decorate| Fixture {
            options,
            vertical,
            decorate,
            dark: true,
            scale: 2.,
            ..Fixture::default()
        };
        let (mut decorated, _) = frame(fixture(true), 320.);
        let (mut plain, _) = frame(fixture(false), 320.);
        let space = icon_box(&decorated);
        for part in [
            ID,
            "volume-rail",
            "volume-focus",
            "volume-label",
            "volume-label-text",
        ] {
            let area = box_for(&decorated, part);
            assert_eq!(box_for(&plain, part), area);
            if part.contains("label") || part.contains("focus") {
                assert!(has_area(area) && !has_area(area.intersect(space)));
            }
        }
        let frame = decorated.accessibility_frame().unwrap();
        let target = decorated.dom().by_id(ID).unwrap();
        assert!(frame.adjustment_target(target).is_some());
        assert_eq!(frame.node(target).unwrap().node.name(), "音量");
        for id in [
            ACTIVE,
            INACTIVE,
            "volume-inset-icon-active-glyph",
            "volume-inset-icon-inactive-glyph",
        ] {
            assert!(frame.node(decorated.dom().by_id(id).unwrap()).is_none());
        }
        let (x, y) = (space.x + space.width / 2., space.y + space.height / 2.);
        for app in [&mut decorated, &mut plain] {
            for event in [
                Event::FocusNext { reverse: false },
                Event::PointerDown { x, y },
                Event::PointerUp { x, y },
                Event::Edit {
                    key: EditKey::Up,
                    extend: false,
                    by_word: false,
                },
            ] {
                assert!(app.dispatch(event).unwrap());
            }
            assert_eq!(app.focused(), Some(ID));
        }
        assert_eq!(decorated.app().values, plain.app().values);
        assert_eq!(decorated.app().values.len(), 2);
    }
}
