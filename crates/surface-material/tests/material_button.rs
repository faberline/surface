use surface::{
    Align, Color, ExtendedDisplayList, ExtendedPaintOp, Node, NodeId, PaintOp, PathFill, Rect,
    Role, Style, TextAlignment, TextMeasurer,
};
use surface_app::{
    AccessibilityActivationResult, Application, Event, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, typography, ButtonAppearance, ButtonIcon, ButtonOptions, ButtonShape,
    ButtonSize, Elevation, Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const ID: &str = "action";
const FACE: &str = "action-face";
const LABEL: &str = "action-label";
const ICON: &str = "action-icon";
const FOCUS: &str = "action-focus";
const AFTER: &str = "after";

#[derive(Clone, Copy)]
enum Message {
    Invoke,
    Checked(bool),
    Disable,
    Other,
}

#[derive(Clone, Copy)]
enum Art {
    None,
    Leading,
    Trailing,
}

struct Model {
    options: ButtonOptions<'static>,
    art: Art,
    dark: bool,
    requests: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            options: ButtonOptions::new(ID, "Go", 280.),
            art: Art::None,
            dark: false,
            requests: 0,
        }
    }
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
            Message::Invoke => self.requests += 1,
            Message::Checked(value) => self.options.checked = Some(value),
            Message::Disable => self.options.disabled = true,
            Message::Other => {}
        }
    }

    fn text_scale(&self) -> f32 {
        self.options.text_scale
    }

    fn view(&self) -> View<Message> {
        let art = VectorIcon::new([[0., 0.], [24., 6.], [8., 24.]]).unwrap();
        let icon = match self.art {
            Art::None => None,
            Art::Leading => Some(ButtonIcon::Leading(&art)),
            Art::Trailing => Some(ButtonIcon::Trailing(&art)),
        };
        let options = ButtonOptions {
            icon,
            ..self.options
        };
        let theme = self.theme();
        View::new(
            Node::new(Role::Window, "Buttons")
                .styled(Style::column().align(Align::Start).gap(16.).padding(20.)),
        )
        .children([
            theme.button_with_options(options, Message::Invoke).unwrap(),
            theme.button(AFTER, "After", Message::Other),
        ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        let width = width.unwrap_or(natural).min(natural);
        (
            width,
            (natural / width.max(1.)).ceil().max(1.) * (size * 1.5),
        )
    }
}

fn draw(runtime: &mut Runtime<Model>) -> ExtendedDisplayList {
    runtime.render_extended(320., 720., &mut Metrics).unwrap()
}

fn id(runtime: &Runtime<Model>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn node<'a>(runtime: &'a Runtime<Model>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn rect(runtime: &Runtime<Model>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn alpha(color: Color, opacity: u8) -> Color {
    Color(color.0, color.1, color.2, opacity)
}

fn fill(list: &ExtendedDisplayList, area: Rect, ink: Color) -> Option<f32> {
    list.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::Legacy(PaintOp::Quad {
            rect,
            radius,
            color,
            ..
        }) if *rect == area && *color == ink => Some(*radius),
        _ => None,
    })
}

fn stroke(list: &ExtendedDisplayList, area: Rect, ink: Color) -> Option<(f32, f32)> {
    list.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::Legacy(PaintOp::Stroke {
            rect,
            radius,
            width,
            color,
            ..
        }) if *rect == area && *color == ink => Some((*radius, *width)),
        _ => None,
    })
}

fn paths(list: &ExtendedDisplayList) -> impl Iterator<Item = &PathFill> {
    list.ops().iter().filter_map(|op| match op {
        ExtendedPaintOp::FillPath(path) => Some(path),
        _ => None,
    })
}

fn text_run(list: &ExtendedDisplayList, wanted: Rect) -> Option<(&PaintOp, TextAlignment)> {
    list.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::AlignedText(text) if text.run().rect() == wanted => {
            Some((text.run(), text.alignment()))
        }
        _ => None,
    })
}

fn projected_key(runtime: &mut Runtime<Model>, press: Key, repeated: bool) {
    draw(runtime);
    let frame = runtime.accessibility_frame().unwrap();
    let focused = runtime.dom().by_id(runtime.focused().unwrap()).unwrap();
    let focus = frame.focus_target(focused).unwrap();
    let press = KeyPress::new(press, Modifiers::default()).with_repeat(repeated);
    assert!(runtime
        .accessibility_key(&focus, &press, Event::Activate)
        .unwrap());
}

fn tab(runtime: &mut Runtime<Model>, target: &str) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(target));
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 0.01, "{actual} != {expected}");
}

#[test]
fn exact_token_sizes_cover_label_only_and_icon_label_widths() {
    use ButtonSize::{ExtraLarge, ExtraSmall, Large, Medium, Small};
    for (size, height, side, icon_size, gap, square, outline, font) in [
        (
            ExtraSmall,
            32.,
            12.,
            20.,
            4.,
            12.,
            1.,
            typography::LABEL_LARGE,
        ),
        (Small, 40., 16., 20., 8., 12., 1., typography::LABEL_LARGE),
        (Medium, 56., 24., 24., 8., 16., 1., typography::TITLE_MEDIUM),
        (
            Large,
            96.,
            48.,
            32.,
            12.,
            28.,
            2.,
            typography::HEADLINE_SMALL,
        ),
        (
            ExtraLarge,
            136.,
            64.,
            40.,
            16.,
            28.,
            3.,
            typography::HEADLINE_LARGE,
        ),
    ] {
        for (art, extra) in [(Art::None, 0.), (Art::Leading, icon_size + gap)] {
            let options = ButtonOptions {
                size,
                appearance: ButtonAppearance::Outlined,
                shape: ButtonShape::Square,
                ..ButtonOptions::new(ID, "Go", 280.)
            };
            let mut runtime = Runtime::new(Model {
                options,
                art,
                ..Model::default()
            })
            .unwrap();
            let list = draw(&mut runtime);
            let (hit, face, label, ring) = (
                rect(&runtime, ID),
                rect(&runtime, FACE),
                rect(&runtime, LABEL),
                rect(&runtime, FOCUS),
            );
            let width = 2. * font.font_size + 2. * side + extra;
            assert_eq!((face.width, face.height), (width, height), "{size:?}");
            assert_eq!((hit.width, hit.height), (width.max(48.), height.max(48.)));
            assert_eq!(
                (label.width, label.height),
                (2. * font.font_size, font.line_height_px())
            );
            assert_eq!(
                ring,
                Rect::new(
                    face.x - 5.,
                    face.y - 5.,
                    face.width + 10.,
                    face.height + 10.
                )
            );
            let outline_ink = runtime.app().theme().colors.outline_variant;
            assert_eq!(stroke(&list, face, outline_ink), Some((square, outline)));
            let (run, alignment) = text_run(&list, label).unwrap();
            let PaintOp::StyledText { style, text, .. } = run else {
                panic!("styled label")
            };
            assert_eq!(
                (style, text.as_str(), alignment),
                (&font, "Go", TextAlignment::Center)
            );
            if matches!(art, Art::Leading) {
                let icon = rect(&runtime, ICON);
                assert_eq!((icon.width, icon.height), (icon_size, icon_size));
                close(icon.x - face.x, side);
                close(label.x - icon.x - icon.width, gap);
                assert_eq!(paths(&list).next().unwrap().rect(), icon);
            } else {
                assert!(runtime.dom().by_id(ICON).is_none());
                close(label.x - face.x, side);
            }
            close(face.x + face.width - label.x - label.width, side);
        }
    }
}

#[test]
fn all_appearances_keep_their_command_toggle_and_disabled_paints() {
    use ButtonAppearance::{Elevated, Filled, Outlined, Text, Tonal};
    for dark in [false, true] {
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let c = theme.colors;
        for (appearance, checked, enabled_fill, enabled_ink) in [
            (Elevated, None, c.surface_container_low, c.primary),
            (Elevated, Some(false), c.surface_container_low, c.primary),
            (Elevated, Some(true), c.primary, c.on_primary),
            (Filled, None, c.primary, c.on_primary),
            (
                Filled,
                Some(false),
                c.surface_container,
                c.on_surface_variant,
            ),
            (Filled, Some(true), c.primary, c.on_primary),
            (Tonal, None, c.secondary_container, c.on_secondary_container),
            (
                Tonal,
                Some(false),
                c.secondary_container,
                c.on_secondary_container,
            ),
            (Tonal, Some(true), c.secondary, c.on_secondary),
            (Outlined, None, Color::TRANSPARENT, c.on_surface_variant),
            (
                Outlined,
                Some(false),
                Color::TRANSPARENT,
                c.on_surface_variant,
            ),
            (
                Outlined,
                Some(true),
                c.inverse_surface,
                c.inverse_on_surface,
            ),
            (Text, None, Color::TRANSPARENT, c.primary),
        ] {
            for disabled in [false, true] {
                let options = ButtonOptions {
                    appearance,
                    checked,
                    disabled,
                    ..ButtonOptions::new(ID, "Go", 280.)
                };
                let mut runtime = Runtime::new(Model {
                    options,
                    dark,
                    art: Art::Leading,
                    ..Model::default()
                })
                .unwrap();
                let list = draw(&mut runtime);
                let face = rect(&runtime, FACE);
                let expected_fill = if disabled {
                    alpha(c.on_surface, 26)
                } else {
                    enabled_fill
                };
                let ink = if disabled {
                    alpha(c.on_surface, 97)
                } else {
                    enabled_ink
                };
                let radius = if checked == Some(true) {
                    12.
                } else {
                    face.height / 2.
                };
                assert_eq!(node(&runtime, FACE).style().background, expected_fill);
                assert_eq!(
                    fill(&list, face, expected_fill),
                    (expected_fill.3 > 0).then_some(radius)
                );
                assert_eq!(
                    stroke(&list, face, c.outline_variant),
                    (appearance == Outlined && checked != Some(true)).then_some((radius, 1.))
                );
                let (label, _) = text_run(&list, rect(&runtime, LABEL)).unwrap();
                assert!(matches!(label, PaintOp::StyledText { color, .. } if *color == ink));
                assert_eq!(paths(&list).next().unwrap().color(), ink);
                let level = if appearance == Elevated && !disabled {
                    Elevation::Level1
                } else {
                    Elevation::Level0
                };
                assert_eq!(
                    node(&runtime, FACE).box_shadow_layers(),
                    theme.elevation(level)
                );
                if disabled {
                    let button = runtime.accessibility_frame().unwrap();
                    assert!(button.activation_target(id(&runtime, ID)).is_none());
                }
            }
        }
    }
}

#[test]
fn one_named_action_preserves_the_callers_checked_state_until_a_model_update() {
    let options = ButtonOptions {
        label: "收藏",
        accessible_name: Some("收藏目前對話"),
        checked: Some(false),
        ..ButtonOptions::new(ID, "Go", 280.)
    };
    let mut runtime = Runtime::new(Model {
        options,
        art: Art::Trailing,
        ..Model::default()
    })
    .unwrap();
    draw(&mut runtime);
    let action = id(&runtime, ID);
    let frame = runtime.accessibility_frame().unwrap();
    let button = frame.node(action).unwrap().node;
    assert_eq!(
        (button.role(), button.name(), button.is_checked()),
        (Role::Button, "收藏目前對話", Some(false))
    );
    assert_eq!(button.is_selected(), None);
    assert_eq!(frame.children(action).count(), 0);
    for part in [FACE, LABEL, ICON, FOCUS] {
        assert!(frame.node(id(&runtime, part)).is_none());
    }
    tab(&mut runtime, ID);
    for key in [Key::Enter, Key::Space] {
        projected_key(&mut runtime, key.clone(), false);
        projected_key(&mut runtime, key, true);
    }
    assert_eq!(runtime.app().requests, 2);
    assert_eq!(runtime.app().options.checked, Some(false));
    draw(&mut runtime);
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(action)
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().requests, 3);
    runtime.apply(Message::Checked(true)).unwrap();
    draw(&mut runtime);
    assert_eq!(id(&runtime, ID), action);
    let button = node(&runtime, ID);
    assert_eq!(
        (button.name(), button.is_checked()),
        ("收藏目前對話", Some(true))
    );
    assert_eq!(runtime.focused(), Some(ID));
    let stale = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(action)
        .unwrap();
    runtime.apply(Message::Disable).unwrap();
    let _ = draw(&mut runtime);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(action).unwrap().node.is_disabled());
    assert_eq!(frame.node(action).unwrap().node.is_checked(), Some(true));
    assert!(frame.activation_target(action).is_none() && frame.focus_target(action).is_none());
    assert_ne!(
        runtime.accessibility_activate(&stale).unwrap(),
        AccessibilityActivationResult::Activated
    );
    let hit = rect(&runtime, ID);
    runtime
        .dispatch(Event::PointerDown {
            x: hit.x + 2.,
            y: hit.y + 2.,
        })
        .unwrap();
    runtime
        .dispatch(Event::PointerUp {
            x: hit.x + 2.,
            y: hit.y + 2.,
        })
        .unwrap();
    tab(&mut runtime, AFTER);
    projected_key(&mut runtime, Key::Enter, false);
    assert_eq!(runtime.app().requests, 3);
}

fn corner_distance([x, y]: [f32; 2], face: Rect, radius: f32) -> f32 {
    let dx = (face.x + radius - x)
        .max(x - (face.x + face.width - radius))
        .max(0.);
    let dy = (face.y + radius - y)
        .max(y - (face.y + face.height - radius))
        .max(0.);
    dx.hypot(dy)
}

#[test]
fn the_face_and_exterior_focus_track_the_same_spring_and_clip_the_held_ripple() {
    let options = ButtonOptions {
        size: ButtonSize::ExtraSmall,
        checked: Some(false),
        ..ButtonOptions::new(ID, "Go", 280.)
    };
    let mut runtime = Runtime::new(Model {
        options,
        art: Art::Leading,
        ..Model::default()
    })
    .unwrap();
    runtime.set_time(0.);
    draw(&mut runtime);
    tab(&mut runtime, ID);
    let (hit, face, ring) = (
        rect(&runtime, ID),
        rect(&runtime, FACE),
        rect(&runtime, FOCUS),
    );
    let c = runtime.app().theme().colors;
    let radius = |list: &ExtendedDisplayList, ink| {
        let face_radius = fill(list, face, ink).expect("face fill");
        let (ring_radius, width) = stroke(list, ring, c.secondary).expect("exterior focus");
        close(ring_radius, face_radius + 5.);
        close(width, 3.);
        face_radius
    };
    let (x, y) = (face.x + 1., hit.y + 1.);
    assert!(y < face.y);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let first = draw(&mut runtime);
    assert_eq!(radius(&first, c.surface_container), 16.);
    assert!(runtime.needs_frame());
    runtime.set_time(50.);
    let mid = draw(&mut runtime);
    let pressed_radius = radius(&mid, c.surface_container);
    let angular_rate = 800_f32.sqrt();
    let decay = (-angular_rate * 0.6 * 0.05).exp();
    let (sin, cos) = (angular_rate * 0.8 * 0.05).sin_cos();
    let expected = 8. + 8. * decay * (cos + 0.75 * sin);
    close(pressed_radius, expected);
    assert!(pressed_radius.is_finite() && pressed_radius > 8. && pressed_radius < 16.);
    let wave = paths(&mid)
        .find(|path| path.color() == alpha(c.on_surface_variant, 17))
        .unwrap();
    assert!(wave.points().iter().all(|&point| {
        face.contains(point[0], point[1])
            && corner_distance(point, face, pressed_radius) <= pressed_radius + 0.1
    }));
    let wave_order = mid
        .ops()
        .iter()
        .position(|op| matches!(op, ExtendedPaintOp::FillPath(path) if path == wave))
        .unwrap();
    let text_order = mid
        .ops()
        .iter()
        .position(|op| matches!(op, ExtendedPaintOp::AlignedText(_)))
        .unwrap();
    assert!(wave_order < text_order);
    assert_eq!(runtime.app().requests, 0);

    // The caller can change the controlled value while a stable button is held.
    runtime.apply(Message::Checked(true)).unwrap();
    close(radius(&draw(&mut runtime), c.primary), pressed_radius);
    assert_eq!(rect(&runtime, ID), hit);
    assert_eq!(node(&runtime, ID).is_checked(), Some(true));
    assert!(runtime.cancel_pointer().unwrap());
    let cancelled = draw(&mut runtime);
    close(radius(&cancelled, c.primary), pressed_radius);
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, 0);
    runtime.set_time(60.);
    assert!(radius(&draw(&mut runtime), c.primary) < pressed_radius);

    runtime.set_reduced_motion(true);
    assert_eq!(radius(&draw(&mut runtime), c.primary), 12.);
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    let reduced = draw(&mut runtime);
    assert_eq!(radius(&reduced, c.primary), 8.);
    assert_eq!(fill(&reduced, face, alpha(c.on_primary, 26)), Some(8.));
    assert_eq!(
        paths(&reduced).count(),
        1,
        "the decorative icon is the only path"
    );
    assert!(!runtime.needs_frame());
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, 1);
    assert_eq!(radius(&draw(&mut runtime), c.primary), 12.);
    assert_eq!(runtime.focused(), Some(ID));
}

#[test]
fn compact_rtl_full_text_and_focus_fit_with_the_icon_at_either_logical_end() {
    let label = "分享目前對話並選擇方式";
    let mut fonts = TextSystem::default();
    for art in [Art::Leading, Art::Trailing] {
        let options = ButtonOptions {
            size: ButtonSize::ExtraLarge,
            direction: LayoutDirection::RightToLeft,
            text_scale: 2.,
            ..ButtonOptions::new(ID, label, 276.)
        };
        let mut runtime = Runtime::new(Model {
            options,
            art,
            dark: true,
            ..Model::default()
        })
        .unwrap();
        let viewport = Rect::new(0., 0., 320., 900.);
        runtime
            .render_extended(viewport.width, viewport.height, &mut fonts)
            .unwrap();
        tab(&mut runtime, ID);
        let list = runtime
            .render_extended(viewport.width, viewport.height, &mut fonts)
            .unwrap();
        let hit = rect(&runtime, ID);
        let face = rect(&runtime, FACE);
        let text = rect(&runtime, LABEL);
        let icon = rect(&runtime, ICON);
        let ring = rect(&runtime, FOCUS);
        assert_eq!(hit.width, 276.);
        assert!(hit.height >= 48. && face.height > 136.);
        assert_eq!((icon.width, icon.height), (40., 40.));
        assert_eq!(icon.intersect(text).width, 0.);
        if matches!(art, Art::Leading) {
            close(icon.x - text.x - text.width, 16.);
        } else {
            close(text.x - icon.x - icon.width, 16.);
        }
        assert_eq!(ring.intersect(viewport), ring);
        assert_eq!(text.intersect(face), text);
        assert_eq!(icon.intersect(face), icon);
        let c = runtime.app().theme().colors;
        let radius = fill(&list, face, c.primary).unwrap();
        assert_eq!(stroke(&list, ring, c.secondary), Some((radius + 5., 3.)));
        for content in [text, icon] {
            for x in [content.x, content.x + content.width] {
                for y in [content.y, content.y + content.height] {
                    assert!(
                        corner_distance([x, y], face, radius) <= radius + 0.01,
                        "content outside rounded face"
                    );
                }
            }
        }
        let (paint, alignment) = text_run(&list, text).unwrap();
        let PaintOp::StyledText {
            color,
            style,
            text: painted,
            ..
        } = paint
        else {
            panic!("styled text")
        };
        assert_eq!(
            (
                *color,
                style.font_size,
                style.line_height,
                painted.as_str(),
                alignment
            ),
            (c.on_primary, 64., Some(80.), label, TextAlignment::Center)
        );
        assert_eq!(paint.clip(), text);
        let measured = fonts.text_layout_styled(label, style, Some(text.width));
        assert_eq!(
            measured
                .clusters
                .iter()
                .map(|cluster| cluster.bytes.end)
                .max(),
            Some(label.len())
        );
        assert!(measured.height <= text.height && measured.height > 80.);
        assert_eq!(node(&runtime, ID).name(), label);
    }
}

#[test]
fn an_icon_stacks_in_the_minimum_width_without_changing_its_logical_order() {
    for art in [Art::Leading, Art::Trailing] {
        let options = ButtonOptions {
            direction: LayoutDirection::RightToLeft,
            text_scale: 2.,
            ..ButtonOptions::new(ID, "分享", 48.)
        };
        let mut runtime = Runtime::new(Model {
            options,
            art,
            ..Model::default()
        })
        .unwrap();
        let mut fonts = TextSystem::default();
        let paint = runtime.render_extended(320., 600., &mut fonts).unwrap();
        let (action, face, label, icon) = (
            rect(&runtime, ID),
            rect(&runtime, FACE),
            rect(&runtime, LABEL),
            rect(&runtime, ICON),
        );
        assert_eq!(action.width, 48.);
        assert_eq!((icon.width, icon.height), (20., 20.));
        assert_eq!(face.intersect(icon), icon);
        assert_eq!(face.intersect(label), label);
        if matches!(art, Art::Leading) {
            close(label.y - icon.y - icon.height, 8.);
        } else {
            close(icon.y - label.y - label.height, 8.);
        }
        assert_eq!(text_run(&paint, label).unwrap().0.clip(), label);
    }
}

#[test]
fn invalid_width_scale_and_text_toggle_are_rejected_before_mount() {
    let theme = Theme::light();
    let valid = ButtonOptions::new(ID, "Go", 280.);
    for options in [
        ButtonOptions { id: " ", ..valid },
        ButtonOptions {
            label: " ",
            ..valid
        },
        ButtonOptions {
            max_width: 47.,
            ..valid
        },
        ButtonOptions {
            max_width: f32::INFINITY,
            ..valid
        },
        ButtonOptions {
            text_scale: f32::NAN,
            ..valid
        },
        ButtonOptions {
            text_scale: f32::MAX,
            ..valid
        },
        ButtonOptions {
            text_scale: 0.,
            ..valid
        },
        ButtonOptions {
            appearance: ButtonAppearance::Text,
            checked: Some(false),
            ..valid
        },
    ] {
        assert!(theme
            .button_with_options(options, Message::Invoke)
            .is_none());
    }
    assert!(theme
        .button_with_options(
            ButtonOptions {
                max_width: 48.,
                ..valid
            },
            Message::Invoke
        )
        .is_some());
}
