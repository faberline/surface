use surface::{Corners, ExtendedDisplayList, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, Application, Event, FrameInput,
    InputAnimationUpdate, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, Elevation, ExtendedFabAppearance, ExtendedFabContent,
    ExtendedFabElevation, ExtendedFabMotion, ExtendedFabOptions, ExtendedFabSize, Theme,
    VectorIcon,
};

const LABEL: &str = "建立完整報表";

#[derive(Clone, Copy)]
enum Message {
    Activate,
    Outside,
    Expand(bool),
    Visible(bool),
    Disable,
}

struct Demo {
    size: ExtendedFabSize,
    appearance: ExtendedFabAppearance,
    elevation: ExtendedFabElevation,
    direction: LayoutDirection,
    label: &'static str,
    name: Option<&'static str>,
    width: f32,
    scale: f32,
    dark: bool,
    icon: bool,
    expanded: bool,
    visible: bool,
    disabled: bool,
    hits: usize,
    motion: ExtendedFabMotion,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            size: ExtendedFabSize::Medium,
            appearance: ExtendedFabAppearance::TonalPrimary,
            elevation: ExtendedFabElevation::Standard,
            direction: LayoutDirection::LeftToRight,
            label: LABEL,
            name: Some("建立完整報表並儲存"),
            width: 550.,
            scale: 1.,
            dark: false,
            icon: true,
            expanded: true,
            visible: true,
            disabled: false,
            hits: 0,
            motion: ExtendedFabMotion::new(true, true),
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Activate => self.hits += 1,
            Message::Outside => {}
            Message::Expand(expanded) => self.expanded = expanded,
            Message::Visible(visible) => self.visible = visible,
            Message::Disable => self.disabled = true,
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_press(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            animation: self.motion.advance(
                "fab",
                &frame,
                &input,
                pressed,
                self.expanded || !self.icon,
                self.visible,
            ),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = theme(self.dark);
        let icon = self
            .icon
            .then(|| VectorIcon::new([[2., 2.], [22., 2.], [12., 22.]]).unwrap());
        View::new(Node::new(Role::Window, "Demo").styled(Style::column().padding(16.).gap(16.)))
            .children([
                theme.button("before", "Back", Message::Outside),
                theme.extended_fab(
                    ExtendedFabOptions {
                        id: "fab",
                        label: self.label,
                        name: self.name,
                        size: self.size,
                        appearance: self.appearance,
                        elevation: self.elevation,
                        direction: self.direction,
                        available_width: self.width,
                        visible: self.visible,
                        disabled: self.disabled,
                        restore_focus_to: Some("before"),
                    },
                    ExtendedFabContent {
                        icon,
                        on_press: Message::Activate,
                    },
                    &self.motion,
                ),
                theme.button("after", "Continue", Message::Outside),
            ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let count = text.chars().count().max(1);
        let per_line = limit.map_or(count, |limit| ((limit / size).floor() as usize).max(1));
        (
            count.min(per_line) as f32 * size,
            count.div_ceil(per_line) as f32 * size * 1.5,
        )
    }
}

fn theme(dark: bool) -> Theme {
    if dark {
        Theme::dark()
    } else {
        Theme::light()
    }
}

fn paint(runtime: &mut Runtime<Demo>, time: f64) -> ExtendedDisplayList {
    runtime.set_time(time);
    runtime
        .render_extended(runtime.app().width + 32., 720., &mut Metrics)
        .unwrap()
}

fn settle(runtime: &mut Runtime<Demo>) {
    for time in [0., 16., 32., 48., 64.] {
        paint(runtime, time);
    }
}

fn node<'a>(runtime: &'a Runtime<Demo>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn rect(runtime: &Runtime<Demo>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.35, "{actual} != {expected}");
}

fn assert_inside(face: Rect, radius: f32, content: Rect) {
    let radius = radius.min(face.width.min(face.height) / 2.);
    for x in [content.x, content.x + content.width] {
        for y in [content.y, content.y + content.height] {
            let dx = (x - face.x).min(face.x + face.width - x);
            let dy = (y - face.y).min(face.y + face.height - y);
            assert!(dx >= -0.03 && dy >= -0.03);
            assert!(
                (radius - dx).max(0.).hypot((radius - dy).max(0.)) <= radius + 0.03,
                "{content:?} crosses {face:?} at ({x}, {y})",
            );
        }
    }
}

#[test]
fn exact_size_and_palette_tokens_in_light_and_dark() {
    use ExtendedFabAppearance::*;
    use ExtendedFabSize::*;
    for dark in [false, true] {
        let c = theme(dark).colors;
        for (appearance, background, ink) in [
            (TonalPrimary, c.primary_container, c.on_primary_container),
            (
                TonalSecondary,
                c.secondary_container,
                c.on_secondary_container,
            ),
            (TonalTertiary, c.tertiary_container, c.on_tertiary_container),
            (Primary, c.primary, c.on_primary),
            (Secondary, c.secondary, c.on_secondary),
            (Tertiary, c.tertiary, c.on_tertiary),
        ] {
            for (size, height, radius, icon_size, inset, gap, font) in [
                (Small, 56., 16., 24., 16., 8., 16.),
                (Medium, 80., 20., 28., 26., 12., 22.),
                (Large, 96., 28., 36., 28., 16., 24.),
            ] {
                let mut a = Runtime::new(Demo {
                    size,
                    appearance,
                    dark,
                    ..Demo::default()
                })
                .unwrap();
                settle(&mut a);
                let face = rect(&a, "fab-face");
                let icon = rect(&a, "fab-icon");
                let label = rect(&a, "fab-label");
                near(face.height, height);
                near(icon.width, icon_size);
                near(icon.x - face.x, inset);
                near(label.x - icon.x - icon.width, gap);
                near(face.x + face.width - label.x - label.width, inset);
                near(node(&a, "fab-label").style().font_size, font);
                assert_eq!(node(&a, "fab-label").style().foreground, ink);
                let face_node = node(&a, "fab-face");
                assert_eq!(face_node.style().corners, Some(Corners::all(radius)));
                assert_eq!(face_node.style().background, background);
                assert_eq!(
                    face_node.box_shadow_layers(),
                    theme(dark).elevation(Elevation::Level3)
                );
                assert_eq!(node(&a, "fab").name(), "建立完整報表並儲存");
            }
        }
    }
}

#[test]
fn compact_large_fab_contains_the_wrapped_chinese_label_and_icon() {
    for icon in [false, true] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut a = Runtime::new(Demo {
                size: ExtendedFabSize::Large,
                label: "建立完整的每月中文報表",
                direction,
                width: 288.,
                scale: 2.,
                icon,
                ..Demo::default()
            })
            .unwrap();
            settle(&mut a);
            let face = rect(&a, "fab-face");
            let label = rect(&a, "fab-label");
            near(face.width, 288.);
            assert!(label.height >= 128.);
            let id = a.dom().by_id("fab-label").unwrap();
            assert_eq!(
                label,
                label.intersect(a.computed().unwrap().nodes[id.index()].clip)
            );
            assert_inside(face, 28., label);
            if icon {
                let icon = rect(&a, "fab-icon");
                assert_inside(face, 28., icon);
                assert_eq!(icon.x < label.x, direction == LayoutDirection::LeftToRight);
            }
            assert!(rect(&a, "fab").height >= 48.);
            assert_eq!(node(&a, "fab").name(), "建立完整的每月中文報表");
        }
    }
}

#[test]
fn pointer_keyboard_and_accessibility_use_one_action_with_exact_feedback() {
    let mut a = Runtime::new(Demo::default()).unwrap();
    settle(&mut a);
    let body = rect(&a, "fab");
    let (x, y) = (body.x + body.width / 2., body.y + body.height / 2.);
    a.dispatch(Event::PointerMove { x, y }).unwrap();
    paint(&mut a, 80.);
    assert_eq!(node(&a, "fab-state-layer").style().background.3, 20);
    assert_eq!(
        node(&a, "fab-face").box_shadow_layers(),
        Theme::light().elevation(Elevation::Level4)
    );
    a.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut a, 96.);
    assert_eq!(a.focused(), Some("fab"));
    assert_eq!(node(&a, "fab-state-layer").style().background.3, 26);
    assert_eq!(
        node(&a, "fab-face").box_shadow_layers(),
        Theme::light().elevation(Elevation::Level3)
    );
    let focus = rect(&a, "fab-focus-ring");
    near(node(&a, "fab-focus-ring").style().border_width, 3.);
    near(body.x - focus.x, 5.);
    near(focus.width - body.width, 10.);
    a.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(a.app().hits, 1);
    a.dispatch(Event::PointerLeave).unwrap();
    paint(&mut a, 112.);
    assert_eq!(node(&a, "fab-state-layer").style().background.3, 26);
    a.dispatch(Event::Activate).unwrap();
    assert_eq!(a.app().hits, 2);
    paint(&mut a, 128.);
    let id = a.dom().by_id("fab").unwrap();
    let frame = a.accessibility_frame().unwrap();
    assert_eq!(frame.node(id).unwrap().node.role(), Role::Button);
    assert!(frame.children(id).next().is_none());
    let target = frame.activation_target(id).unwrap();
    assert_eq!(
        a.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(a.app().hits, 3);
    a.apply(Message::Disable).unwrap();
    assert_eq!(a.focused(), Some("before"));
    paint(&mut a, 144.);
    assert!(a
        .accessibility_frame()
        .unwrap()
        .activation_target(a.dom().by_id("fab").unwrap())
        .is_none());
    a.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(a.focused(), Some("after"));
}

#[test]
fn lowered_shadows_follow_interaction_and_disabled_stays_flat() {
    let mut a = Runtime::new(Demo {
        elevation: ExtendedFabElevation::Lowered,
        appearance: ExtendedFabAppearance::Tertiary,
        dark: true,
        ..Demo::default()
    })
    .unwrap();
    settle(&mut a);
    let theme = Theme::dark();
    let shadow = |a: &Runtime<Demo>| node(a, "fab-face").box_shadow_layers();
    assert_eq!(
        node(&a, "fab-face").style().background,
        theme.colors.tertiary
    );
    assert_eq!(shadow(&a), theme.elevation(Elevation::Level1));
    for _ in 0..2 {
        a.dispatch(Event::FocusNext { reverse: false }).unwrap();
    }
    paint(&mut a, 80.);
    assert_eq!(a.focused(), Some("fab"));
    assert_eq!(shadow(&a), theme.elevation(Elevation::Level1));
    let body = rect(&a, "fab");
    let (x, y) = (body.x + body.width / 2., body.y + body.height / 2.);
    a.dispatch(Event::PointerMove { x, y }).unwrap();
    paint(&mut a, 96.);
    assert_eq!(shadow(&a), theme.elevation(Elevation::Level2));
    a.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut a, 112.);
    assert_eq!(shadow(&a), theme.elevation(Elevation::Level1));
    a.dispatch(Event::PointerUp { x, y }).unwrap();
    a.apply(Message::Disable).unwrap();
    paint(&mut a, 128.);
    assert_eq!(shadow(&a), theme.elevation(Elevation::Level0));
}

#[test]
fn collapse_keeps_identity_and_hidden_exit_cannot_take_input() {
    for icon in [true, false] {
        let mut a = Runtime::new(Demo {
            icon,
            ..Demo::default()
        })
        .unwrap();
        settle(&mut a);
        a.activate("fab").unwrap();
        a.apply(Message::Expand(false)).unwrap();
        paint(&mut a, 80.);
        paint(&mut a, 500.);
        assert_eq!(a.focused(), Some("fab"));
        assert_eq!(node(&a, "fab").name(), "建立完整報表並儲存");
        if icon {
            near(rect(&a, "fab-face").width, 80.);
            assert!(a.dom().by_id("fab-label").is_none());
        } else {
            assert!(a.dom().by_id("fab-label").is_some());
        }
        a.apply(Message::Visible(false)).unwrap();
        assert_eq!(a.focused(), Some("before"));
        assert!(a.activate("fab").is_err());
        paint(&mut a, 516.);
        paint(&mut a, 566.);
        assert!(node(&a, "fab-face").style().background.3 > 0);
        assert!(a
            .accessibility_frame()
            .unwrap()
            .node(a.dom().by_id("fab").unwrap())
            .is_none());
        assert_eq!(a.app().hits, 1);
        a.set_reduced_motion(true);
        paint(&mut a, 582.);
        assert_eq!(node(&a, "fab-face").style().background.3, 0);
    }
}

#[test]
fn fading_label_is_clipped_to_the_face_during_a_width_change() {
    let mut a = Runtime::new(Demo {
        size: ExtendedFabSize::Large,
        label: "建立完整的每月中文報表",
        direction: LayoutDirection::RightToLeft,
        width: 288.,
        scale: 2.,
        ..Demo::default()
    })
    .unwrap();
    settle(&mut a);
    a.apply(Message::Expand(false)).unwrap();
    paint(&mut a, 80.);
    let mut visible_while_narrower = false;
    for time in [106., 136., 176., 216.] {
        paint(&mut a, time);
        let face = rect(&a, "fab-face");
        if let Some(id) = a.dom().by_id("fab-label") {
            let label = &a.computed().unwrap().nodes[id.index()];
            let visible = label.bounds.intersect(label.clip);
            assert!(visible.width > 0. && visible.height > 0.);
            assert_inside(face, 28., visible);
            visible_while_narrower |= face.width < 288.;
        }
    }
    assert!(visible_while_narrower);
    paint(&mut a, 500.);
    near(rect(&a, "fab-face").width, 96.);
    assert!(a.dom().by_id("fab-label").is_none());
    assert_eq!(node(&a, "fab").role(), Role::Button);
}
