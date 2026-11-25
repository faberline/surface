use surface::{
    Align, Color, ExtendedDisplayList, ExtendedPaintOp, Length, Node, NodeId, PaintOp, PathFill,
    Rect, Role, Style, TextMeasurer,
};
use surface_app::{
    AccessibilityActivationResult, Application, Event, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    IconButtonKind, IconButtonOptions, IconButtonShape, IconButtonSize, IconButtonWidth,
    MaterialIcon, Theme, VectorIcon,
};

const ID: &str = "star";
const NAME: &str = "加星號";
const DESCRIPTION: &str = "star-hint";
const FACE: &str = "star-face";
const ICON: &str = "star-icon";
const FOCUS: &str = "star-focus";

const WIDTHS: [IconButtonWidth; 3] = [
    IconButtonWidth::Narrow,
    IconButtonWidth::Default,
    IconButtonWidth::Wide,
];

#[derive(Clone, Copy)]
enum Message {
    Invoke,
    Other,
    Set(bool),
}

#[derive(Clone, Copy)]
enum Hint {
    Tooltip,
    Text,
    Missing,
}

struct Model {
    kind: IconButtonKind,
    checked: Option<bool>,
    disabled: bool,
    dark: bool,
    expanded_text: bool,
    size: IconButtonSize,
    width: IconButtonWidth,
    shape: IconButtonShape,
    hint: Hint,
    requests: usize,
    off: VectorIcon,
    on: VectorIcon,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            kind: IconButtonKind::Filled,
            checked: None,
            disabled: false,
            dark: false,
            expanded_text: false,
            size: IconButtonSize::Small,
            width: IconButtonWidth::Default,
            shape: IconButtonShape::Round,
            hint: Hint::Missing,
            requests: 0,
            off: VectorIcon::new([[0., 0.], [24., 12.], [0., 24.]]).unwrap(),
            on: VectorIcon::new([[0., 0.], [24., 0.], [12., 24.]]).unwrap(),
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

    fn options(&self) -> IconButtonOptions<'static> {
        IconButtonOptions {
            id: ID,
            name: NAME,
            description: Some(DESCRIPTION),
            kind: self.kind,
            size: self.size,
            width: self.width,
            shape: self.shape,
            disabled: self.disabled,
        }
    }

    fn artwork(&self) -> MaterialIcon<'_> {
        match self.checked {
            Some(checked) => MaterialIcon::Toggle {
                checked,
                off: &self.off,
                on: &self.on,
            },
            None => MaterialIcon::Command(&self.off),
        }
    }
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Invoke => self.requests += 1,
            Message::Set(value) => self.checked = Some(value),
            Message::Other => {}
        }
    }

    fn text_scale(&self) -> f32 {
        if self.expanded_text {
            2.
        } else {
            1.
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let button = theme
            .icon_button_with_options(self.options(), self.artwork(), Message::Invoke)
            .unwrap();
        let hint = match self.hint {
            Hint::Tooltip => Some(View::new(
                Node::new(Role::Tooltip, "收藏對話")
                    .id(DESCRIPTION)
                    .styled(Style::row().width(Length::Px(80.)).height(Length::Px(32.))),
            )),
            Hint::Text => Some(View::new(
                Node::new(Role::Text, "收藏對話")
                    .id(DESCRIPTION)
                    .text("收藏對話"),
            )),
            Hint::Missing => None,
        };
        View::new(
            Node::new(Role::Window, "Icon Buttons")
                .styled(Style::column().align(Align::Start).gap(12.).padding(20.)),
        )
        .children(
            [button, theme.button("after", "After", Message::Other)]
                .into_iter()
                .chain(hint),
        )
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

fn paint(runtime: &mut Runtime<Model>) -> ExtendedDisplayList {
    runtime.render_extended(320., 400., &mut Metrics).unwrap()
}

fn node_id(runtime: &Runtime<Model>, id: &str) -> NodeId {
    runtime.dom().by_id(id).unwrap()
}

fn bounds(runtime: &Runtime<Model>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[node_id(runtime, id).index()].bounds
}

fn centered(outer: Rect, width: f32, height: f32) -> Rect {
    Rect {
        x: outer.x + (outer.width - width) / 2.,
        y: outer.y + (outer.height - height) / 2.,
        width,
        height,
    }
}

fn alpha(color: Color, opacity: u8) -> Color {
    Color(color.0, color.1, color.2, opacity)
}

fn quad(paint: &ExtendedDisplayList, area: Rect, ink: Color) -> Option<f32> {
    paint.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::Legacy(PaintOp::Quad {
            rect,
            radius,
            color,
            ..
        }) if *rect == area && *color == ink => Some(*radius),
        _ => None,
    })
}

fn stroke(paint: &ExtendedDisplayList, area: Rect, ink: Color) -> Option<(f32, f32)> {
    paint.ops().iter().find_map(|op| match op {
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

fn fills(paint: &ExtendedDisplayList) -> impl Iterator<Item = &PathFill> {
    paint.ops().iter().filter_map(|op| match op {
        ExtendedPaintOp::FillPath(path) => Some(path),
        _ => None,
    })
}

fn press(runtime: &mut Runtime<Model>, key: Key, repeat: bool) {
    paint(runtime);
    let focus = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(node_id(runtime, runtime.focused().unwrap()))
        .unwrap();
    let press = KeyPress::new(key, Modifiers::default()).with_repeat(repeat);
    assert!(runtime
        .accessibility_key(&focus, &press, Event::Activate)
        .unwrap());
}

#[test]
fn all_fifteen_footprints_and_focus_shapes_keep_art_at_token_size() {
    let tokens = [
        (IconButtonSize::ExtraSmall, [28., 32., 40.], 32., 20., 12.),
        (IconButtonSize::Small, [32., 40., 52.], 40., 24., 12.),
        (IconButtonSize::Medium, [48., 56., 72.], 56., 24., 16.),
        (IconButtonSize::Large, [64., 96., 128.], 96., 32., 28.),
        (
            IconButtonSize::ExtraLarge,
            [104., 136., 184.],
            136.,
            40.,
            28.,
        ),
    ];
    for (size, widths, height, icon, square) in tokens {
        for (width, face_width) in WIDTHS.into_iter().zip(widths) {
            for (shape, checked, is_round) in [
                (IconButtonShape::Round, None, true),
                (IconButtonShape::Square, None, false),
                (IconButtonShape::Round, Some(false), true),
                (IconButtonShape::Round, Some(true), false),
                (IconButtonShape::Square, Some(false), false),
                (IconButtonShape::Square, Some(true), true),
            ] {
                let mut runtime = Runtime::new(Model {
                    size,
                    width,
                    shape,
                    checked,
                    dark: true,
                    expanded_text: true,
                    ..Model::default()
                })
                .unwrap();
                let idle = paint(&mut runtime);
                let hit = bounds(&runtime, ID);
                let face = bounds(&runtime, FACE);
                let art = bounds(&runtime, ICON);
                let focus = bounds(&runtime, FOCUS);
                let radius = if is_round {
                    f32::min(face_width, height) / 2.
                } else {
                    square
                };
                let colors = runtime.app().theme().colors;
                let fill = if checked == Some(false) {
                    colors.surface_container
                } else {
                    colors.primary
                };
                assert_eq!(
                    (hit.width, hit.height),
                    (f32::max(face_width, 48.), f32::max(height, 48.))
                );
                assert_eq!(face, centered(hit, face_width, height));
                assert_eq!(art, centered(face, icon, icon));
                assert_eq!(focus, centered(face, face_width + 10., height + 10.));
                assert_eq!(quad(&idle, face, fill), Some(radius));
                assert_eq!(fills(&idle).next().unwrap().rect(), art);
                assert_eq!(stroke(&idle, focus, colors.secondary), None);

                runtime
                    .dispatch(Event::FocusNext { reverse: false })
                    .unwrap();
                let focused = paint(&mut runtime);
                assert_eq!(runtime.focused(), Some(ID));
                assert_eq!(
                    stroke(&focused, focus, colors.secondary),
                    Some((radius + 5., 3.))
                );
                assert_eq!(stroke(&focused, hit, colors.secondary), None);
                assert_eq!(bounds(&runtime, ID), hit);
            }
        }
    }
}

#[test]
fn colors_follow_command_off_on_and_disabled_roles_in_light_and_dark() {
    let transparent = Color::TRANSPARENT;
    for dark in [false, true] {
        let colors = Model {
            dark,
            ..Model::default()
        }
        .theme()
        .colors;
        let rows = [
            (
                IconButtonKind::Filled,
                None,
                colors.primary,
                colors.on_primary,
                false,
            ),
            (
                IconButtonKind::Filled,
                Some(false),
                colors.surface_container,
                colors.on_surface_variant,
                false,
            ),
            (
                IconButtonKind::Filled,
                Some(true),
                colors.primary,
                colors.on_primary,
                false,
            ),
            (
                IconButtonKind::Tonal,
                None,
                colors.secondary_container,
                colors.on_secondary_container,
                false,
            ),
            (
                IconButtonKind::Tonal,
                Some(false),
                colors.secondary_container,
                colors.on_secondary_container,
                false,
            ),
            (
                IconButtonKind::Tonal,
                Some(true),
                colors.secondary,
                colors.on_secondary,
                false,
            ),
            (
                IconButtonKind::Outlined,
                None,
                transparent,
                colors.on_surface_variant,
                true,
            ),
            (
                IconButtonKind::Outlined,
                Some(false),
                transparent,
                colors.on_surface_variant,
                true,
            ),
            (
                IconButtonKind::Outlined,
                Some(true),
                colors.inverse_surface,
                colors.inverse_on_surface,
                false,
            ),
            (
                IconButtonKind::Standard,
                None,
                transparent,
                colors.on_surface_variant,
                false,
            ),
            (
                IconButtonKind::Standard,
                Some(false),
                transparent,
                colors.on_surface_variant,
                false,
            ),
            (
                IconButtonKind::Standard,
                Some(true),
                transparent,
                colors.primary,
                false,
            ),
        ];
        for (kind, checked, fill, ink, outline) in rows {
            for disabled in [false, true] {
                let mut runtime = Runtime::new(Model {
                    kind,
                    checked,
                    dark,
                    disabled,
                    ..Model::default()
                })
                .unwrap();
                let list = paint(&mut runtime);
                let face = bounds(&runtime, FACE);
                let expected_fill = if disabled && fill != transparent {
                    alpha(colors.on_surface, 26)
                } else {
                    fill
                };
                let expected_ink = if disabled {
                    alpha(colors.on_surface, 97)
                } else {
                    ink
                };
                let art = fills(&list).next().unwrap();
                assert_eq!(art.color(), expected_ink);
                let radius = if checked == Some(true) { 12. } else { 20. };
                assert_eq!(
                    quad(&list, face, expected_fill),
                    (expected_fill != transparent).then_some(radius)
                );
                assert_eq!(
                    stroke(&list, face, colors.outline_variant),
                    outline.then_some((20., 1.))
                );
                assert_eq!(
                    runtime
                        .dom()
                        .get(node_id(&runtime, ID))
                        .unwrap()
                        .is_checked(),
                    checked
                );
            }
        }
    }
}

#[test]
fn outline_thickness_follows_all_five_sizes() {
    for (size, width) in [
        (IconButtonSize::ExtraSmall, 1.),
        (IconButtonSize::Small, 1.),
        (IconButtonSize::Medium, 1.),
        (IconButtonSize::Large, 2.),
        (IconButtonSize::ExtraLarge, 3.),
    ] {
        let mut runtime = Runtime::new(Model {
            size,
            kind: IconButtonKind::Outlined,
            ..Model::default()
        })
        .unwrap();
        let list = paint(&mut runtime);
        let face = bounds(&runtime, FACE);
        assert_eq!(
            stroke(&list, face, runtime.app().theme().colors.outline_variant),
            Some((face.height / 2., width)),
        );
    }
}

#[test]
fn controlled_toggle_is_one_named_action_with_one_description_and_distinct_art() {
    let mut runtime = Runtime::new(Model {
        checked: Some(false),
        hint: Hint::Tooltip,
        ..Model::default()
    })
    .unwrap();
    let off = paint(&mut runtime);
    let button = node_id(&runtime, ID);
    let frame = runtime.accessibility_frame().unwrap();
    let projected = frame.node(button).unwrap().node;
    assert_eq!(
        (projected.role(), projected.name(), projected.is_checked()),
        (Role::Button, NAME, Some(false))
    );
    assert_eq!(projected.is_selected(), None);
    assert_eq!(frame.children(button).count(), 0);
    for art in [FACE, ICON, FOCUS] {
        assert!(frame.node(node_id(&runtime, art)).is_none());
    }
    assert_eq!(
        frame.description_target(button),
        Some(node_id(&runtime, DESCRIPTION))
    );

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ID));
    for key in [Key::Space, Key::Enter] {
        press(&mut runtime, key.clone(), false);
        press(&mut runtime, key, true);
    }
    assert_eq!(runtime.app().requests, 2);
    paint(&mut runtime);
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(node_id(&runtime, ID))
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().requests, 3);
    assert_eq!(runtime.app().checked, Some(false));

    runtime.apply(Message::Set(true)).unwrap();
    let on = paint(&mut runtime);
    let frame = runtime.accessibility_frame().unwrap();
    let checked = frame.node(node_id(&runtime, ID)).unwrap().node;
    assert_eq!(
        (checked.role(), checked.name(), checked.is_checked()),
        (Role::Button, NAME, Some(true))
    );
    assert_eq!(runtime.focused(), Some(ID));
    assert_eq!(bounds(&runtime, ICON), fills(&on).next().unwrap().rect());
    assert_ne!(
        fills(&off).next().unwrap().points(),
        fills(&on).next().unwrap().points()
    );
    assert_eq!(runtime.app().requests, 3);

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("after"));
    for (hint, has_description) in [(Hint::Text, true), (Hint::Missing, false)] {
        let mut runtime = Runtime::new(Model {
            hint,
            ..Model::default()
        })
        .unwrap();
        paint(&mut runtime);
        let frame = runtime.accessibility_frame().unwrap();
        let button = node_id(&runtime, ID);
        assert_eq!(frame.description_target(button).is_some(), has_description);
        assert_eq!(frame.node(button).unwrap().node.is_checked(), None);
        assert!(frame.activation_target(button).is_some());
    }
}

#[test]
fn disabled_toggle_is_announced_and_rejects_pointer_keyboard_and_host_activation() {
    let mut runtime = Runtime::new(Model {
        checked: Some(true),
        disabled: true,
        ..Model::default()
    })
    .unwrap();
    paint(&mut runtime);
    let id = node_id(&runtime, ID);
    let frame = runtime.accessibility_frame().unwrap();
    let node = frame.node(id).unwrap().node;
    assert!(node.is_disabled());
    assert_eq!(node.is_checked(), Some(true));
    assert!(frame.activation_target(id).is_none());
    assert!(frame.focus_target(id).is_none());
    let hit = bounds(&runtime, ID);
    let (x, y) = (hit.x + 1., hit.y + 1.);
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let list = paint(&mut runtime);
    let face = bounds(&runtime, FACE);
    let colors = runtime.app().theme().colors;
    for opacity in [20, 26] {
        assert_ne!(
            quad(&list, face, alpha(colors.on_primary, opacity)),
            Some(12.)
        );
    }
    assert_eq!(fills(&list).count(), 1);
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("after"));
    press(&mut runtime, Key::Enter, false);
    assert_eq!(runtime.app().requests, 0);
}

#[test]
fn accepted_pointer_and_touch_use_slack_and_ripple_only_on_the_face() {
    let mut runtime = Runtime::new(Model {
        size: IconButtonSize::ExtraSmall,
        width: IconButtonWidth::Narrow,
        ..Model::default()
    })
    .unwrap();
    runtime.set_time(0.);
    paint(&mut runtime);
    let hit = bounds(&runtime, ID);
    let face = bounds(&runtime, FACE);
    let (x, y) = (hit.x + 1., hit.y + 1.);
    assert!(x < face.x && y < face.y);
    let ink = runtime.app().theme().colors.on_primary;
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    let hovered = paint(&mut runtime);
    assert_eq!(quad(&hovered, face, alpha(ink, 20)), Some(14.));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut runtime);
    assert_eq!(runtime.app().requests, 0);
    runtime.set_time(80.);
    let held = paint(&mut runtime);
    let held_radius = quad(&held, face, runtime.app().theme().colors.primary).unwrap();
    assert!(held_radius < 14.);
    let wave = fills(&held)
        .filter(|path| path.color() == alpha(ink, 26))
        .collect::<Vec<_>>();
    assert_eq!(wave.len(), 1);
    let corner_distance = |[x, y]: [f32; 2], radius: f32| {
        let dx = (face.x + radius - x)
            .max(x - (face.x + face.width - radius))
            .max(0.);
        let dy = (face.y + radius - y)
            .max(y - (face.y + face.height - radius))
            .max(0.);
        dx.hypot(dy)
    };
    for &[x, y] in wave[0].points() {
        assert!(x >= face.x && x <= face.x + face.width);
        assert!(y >= face.y && y <= face.y + face.height);
        assert!(corner_distance([x, y], held_radius) <= held_radius + 0.1);
    }
    assert!(
        wave[0]
            .points()
            .iter()
            .any(|&point| corner_distance(point, 14.) > 14.1),
        "the press wave should reach into the animated corner"
    );
    runtime.cancel_pointer().unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, 0);

    paint(&mut runtime);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime
        .dispatch(Event::PointerMove { x: 0., y: 0. })
        .unwrap();
    runtime.dispatch(Event::PointerUp { x: 0., y: 0. }).unwrap();
    assert_eq!(runtime.app().requests, 0);
    runtime.set_reduced_motion(true);
    paint(&mut runtime);
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    let reduced = paint(&mut runtime);
    assert_eq!(quad(&reduced, face, alpha(ink, 26)), Some(8.));
    assert!(fills(&reduced).all(|path| path.color() != alpha(ink, 26)));
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, 1);
    paint(&mut runtime);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, 2);
}

#[test]
fn pressed_face_and_focus_follow_the_same_spring_through_a_toggle_and_cancel() {
    let mut runtime = Runtime::new(Model {
        checked: Some(false),
        ..Model::default()
    })
    .unwrap();
    runtime.set_time(0.);
    paint(&mut runtime);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    let hit = bounds(&runtime, ID);
    let face = bounds(&runtime, FACE);
    let focus = bounds(&runtime, FOCUS);
    let colors = runtime.app().theme().colors;
    let radius = |list: &ExtendedDisplayList, fill| {
        let face_radius = quad(list, face, fill).expect("button face");
        let (focus_radius, focus_width) =
            stroke(list, focus, colors.secondary).expect("keyboard focus ring");
        assert_eq!(focus_width, 3.);
        assert!((focus_radius - face_radius - 5.).abs() < 0.001);
        face_radius
    };
    let (x, y) = (hit.x + 1., hit.y + 1.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let start = paint(&mut runtime);
    assert_eq!(radius(&start, colors.surface_container), 20.);
    assert!(runtime.needs_frame());

    runtime.set_time(50.);
    let mid = paint(&mut runtime);
    let held = radius(&mid, colors.surface_container);
    let angular_rate = 800_f32.sqrt();
    let decay = (-angular_rate * 0.6 * 0.05).exp();
    let (sin, cos) = (angular_rate * 0.8 * 0.05).sin_cos();
    let expected = 8. + 12. * decay * (cos + 0.75 * sin);
    assert!((held - expected).abs() < 0.01);
    assert_eq!(runtime.app().requests, 0);

    runtime.apply(Message::Set(true)).unwrap();
    assert_eq!(radius(&paint(&mut runtime), colors.primary), held);
    let semantic = runtime.dom().get(node_id(&runtime, ID)).unwrap();
    assert_eq!(semantic.is_checked(), Some(true));
    assert_eq!(bounds(&runtime, ID), hit);

    runtime.cancel_pointer().unwrap();
    let retarget = paint(&mut runtime);
    assert_eq!(radius(&retarget, colors.primary), held);
    runtime.set_time(60.);
    let continuing = radius(&paint(&mut runtime), colors.primary);
    assert!(continuing < held, "retarget retains incoming velocity");

    runtime.set_reduced_motion(true);
    assert_eq!(radius(&paint(&mut runtime), colors.primary), 12.);
    assert!(!runtime.needs_frame());
    runtime.set_reduced_motion(false);
    runtime.set_time(480.);
    let settled = paint(&mut runtime);
    assert_eq!(radius(&settled, colors.primary), 12.);
    assert!(!runtime.needs_frame());
    assert_eq!(runtime.focused(), Some(ID));
    assert_eq!(runtime.app().requests, 0);
}

#[test]
fn empty_action_names_and_ids_are_rejected() {
    let model = Model::default();
    let theme = model.theme();
    for (id, name) in [("  ", NAME), (ID, "  ")] {
        assert!(theme
            .icon_button_with_options(
                IconButtonOptions {
                    id,
                    name,
                    ..model.options()
                },
                model.artwork(),
                (),
            )
            .is_none());
    }
}
