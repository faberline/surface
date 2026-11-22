use surface::{
    Color, Corners, ExtendedDisplayList, ExtendedPaintOp, Node, PathFill, Rect, Role, TextMeasurer,
};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, Application, Event, FrameInput, FramePress,
    InputAnimationUpdate, Runtime,
};
use surface_material::{
    Elevation, ExpressiveFabAppearance, ExpressiveFabContent, ExpressiveFabElevation,
    ExpressiveFabMotion, ExpressiveFabOptions, ExpressiveFabSize, Theme, VectorIcon,
};

#[derive(Clone, Copy)]
enum Message {
    Activate,
    Other,
    Visible(bool),
    Disabled(bool),
}

#[derive(Clone, Copy)]
enum TriggerKind {
    Menu,
    Disclosure,
}

struct Demo {
    size: ExpressiveFabSize,
    appearance: ExpressiveFabAppearance,
    elevation: ExpressiveFabElevation,
    dark: bool,
    disabled: bool,
    visible: bool,
    trigger: Option<TriggerKind>,
    expanded: bool,
    hits: usize,
    motion: ExpressiveFabMotion,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            size: ExpressiveFabSize::Medium,
            appearance: ExpressiveFabAppearance::default(),
            elevation: ExpressiveFabElevation::default(),
            dark: false,
            disabled: false,
            visible: true,
            trigger: None,
            expanded: false,
            hits: 0,
            motion: ExpressiveFabMotion::new(),
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Activate => {
                self.hits += 1;
                self.expanded = !self.expanded;
            }
            Message::Other => {}
            Message::Visible(visible) => self.visible = visible,
            Message::Disabled(disabled) => self.disabled = disabled,
        }
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_press_detail(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        _: Option<&str>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            animation: self
                .motion
                .advance("fab", &frame, &input, press, hovered, self.visible),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> surface_app::View<Message> {
        let theme = theme(self.dark);
        let button = theme.expressive_fab(
            ExpressiveFabOptions {
                id: "fab",
                name: "建立報表",
                size: self.size,
                appearance: self.appearance,
                elevation: self.elevation,
                visible: self.visible,
                disabled: self.disabled,
                restore_focus_to: Some("before"),
            },
            ExpressiveFabContent {
                icon: VectorIcon::new([[2., 2.], [22., 2.], [12., 22.]]).unwrap(),
                on_press: Message::Activate,
            },
            &self.motion,
        );
        let button = match self.trigger {
            Some(TriggerKind::Menu) => button.menu_expanded(self.expanded),
            Some(TriggerKind::Disclosure) => button.disclosure_expanded(self.expanded),
            None => button,
        };
        theme.page([
            theme.button("before", "Back", Message::Other),
            button,
            theme.button("after", "Continue", Message::Other),
        ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size * 1.5)
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
    runtime.render_extended(320., 420., &mut Metrics).unwrap()
}

fn node<'a>(runtime: &'a Runtime<Demo>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(runtime: &Runtime<Demo>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn fills(display: &ExtendedDisplayList) -> impl Iterator<Item = &PathFill> {
    display
        .ops()
        .iter()
        .filter_map(|operation| match operation {
            ExtendedPaintOp::FillPath(fill) => Some(fill),
            _ => None,
        })
}

fn ripple(display: &ExtendedDisplayList) -> &PathFill {
    fills(display).find(|fill| fill.color().3 < 255).unwrap()
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.25, "{actual} != {expected}");
}

#[test]
fn all_sizes_and_palettes_match_tokens_in_light_and_dark() {
    use ExpressiveFabAppearance::*;
    use ExpressiveFabSize::*;
    for dark in [false, true] {
        let theme = theme(dark);
        let c = theme.colors;
        let palettes = [
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
        ];
        for (size, side, radius, icon_side) in [
            (Regular, 56., 16., 24.),
            (Medium, 80., 20., 28.),
            (Large, 96., 28., 36.),
        ] {
            for (appearance, fill, ink) in palettes {
                let mut a = Runtime::new(Demo {
                    size,
                    appearance,
                    dark,
                    ..Demo::default()
                })
                .unwrap();
                let display = paint(&mut a, 0.);
                let face = bounds(&a, "fab-face");
                let icon = bounds(&a, "fab-icon");
                near(face.width, side);
                near(face.height, side);
                assert_eq!(bounds(&a, "fab"), face);
                near(icon.width, icon_side);
                near(icon.height, icon_side);
                near(icon.x - face.x, (side - icon_side) / 2.);
                near(icon.y - face.y, (side - icon_side) / 2.);
                assert_eq!(
                    node(&a, "fab-face").style().corners,
                    Some(Corners::all(radius))
                );
                assert_eq!(node(&a, "fab-face").style().background, fill);
                assert_eq!(
                    node(&a, "fab-face").box_shadow_layers(),
                    theme.elevation(Elevation::Level3)
                );
                assert_eq!(
                    fills(&display).map(PathFill::color).collect::<Vec<_>>(),
                    [ink]
                );
            }
        }
    }
}

#[test]
fn named_button_handles_keyboard_pointer_touch_and_accessibility_once() {
    let mut a = Runtime::new(Demo {
        trigger: Some(TriggerKind::Menu),
        ..Demo::default()
    })
    .unwrap();
    paint(&mut a, 0.);
    for _ in 0..2 {
        a.dispatch(Event::FocusNext { reverse: false }).unwrap();
    }
    paint(&mut a, 16.);
    assert_eq!(a.focused(), Some("fab"));
    let body = bounds(&a, "fab");
    let ring = bounds(&a, "fab-focus-ring");
    near(body.x - ring.x, 5.);
    near(body.y - ring.y, 5.);
    near(ring.width - body.width, 10.);
    assert_eq!(node(&a, "fab-focus-ring").style().border_width, 3.);
    assert_eq!(
        node(&a, "fab-focus-ring").style().border_color,
        Theme::light().colors.secondary
    );

    let id = a.dom().by_id("fab").unwrap();
    let ax = a.accessibility_frame().unwrap();
    let button = ax.node(id).unwrap().node;
    assert_eq!(button.role(), Role::Button);
    assert_eq!(button.name(), "建立報表");
    assert_eq!(button.expanded_state(), Some(false));
    assert!(ax.children(id).next().is_none());
    let target = ax.activation_target(id).unwrap();
    assert_eq!(
        a.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(a.app().hits, 1);
    a.dispatch(Event::Activate).unwrap();
    assert_eq!(a.app().hits, 2);

    paint(&mut a, 24.);
    let (x, y) = (body.x + body.width / 2., body.y + body.height / 2.);
    a.dispatch(Event::PointerDown { x, y }).unwrap();
    a.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(a.app().hits, 3);
    paint(&mut a, 32.);
    a.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    a.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(a.app().hits, 4);

    a.apply(Message::Disabled(true)).unwrap();
    assert_eq!(a.focused(), Some("before"));
    let display = paint(&mut a, 48.);
    let disabled = node(&a, "fab-face");
    let Color(r, g, b, _) = Theme::light().colors.on_surface;
    assert_eq!(disabled.style().background, Color(r, g, b, 31));
    assert_eq!(
        disabled.box_shadow_layers(),
        Theme::light().elevation(Elevation::Level0)
    );
    assert_eq!(
        fills(&display).map(PathFill::color).collect::<Vec<_>>(),
        [Color(r, g, b, 97)]
    );
    let id = a.dom().by_id("fab").unwrap();
    assert!(node(&a, "fab").is_disabled());
    assert!(a
        .accessibility_frame()
        .unwrap()
        .activation_target(id)
        .is_none());
    assert!(a.activate("fab").is_err());
    a.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(a.focused(), Some("after"));
}

#[test]
fn menu_and_disclosure_metadata_keep_one_focused_action_with_stable_geometry() {
    for (trigger, has_menu_popup) in [(TriggerKind::Menu, true), (TriggerKind::Disclosure, false)] {
        let mut a = Runtime::new(Demo {
            trigger: Some(trigger),
            ..Demo::default()
        })
        .unwrap();
        paint(&mut a, 0.);
        let hit = bounds(&a, "fab");
        assert_eq!((hit.width, hit.height), (80., 80.));

        for _ in 0..2 {
            a.dispatch(Event::FocusNext { reverse: false }).unwrap();
        }
        for (hits, expanded) in [false, true, false].into_iter().enumerate() {
            paint(&mut a, 16. * (hits + 1) as f64);
            let id = a.dom().by_id("fab").unwrap();
            assert_eq!(a.dom().by_role_name(Role::Button, "建立報表"), &[id]);
            assert_eq!(a.focused(), Some("fab"));
            assert_eq!(a.app().hits, hits);

            assert_eq!(bounds(&a, "fab"), hit);
            assert_eq!(bounds(&a, "fab-face"), hit);
            let computed = a.computed().unwrap();
            assert_eq!(
                computed.hit_test(a.dom(), hit.x + 40., hit.y + 1.),
                Some(id)
            );
            assert_eq!(computed.hit_test(a.dom(), hit.x + 40., hit.y - 1.), None);

            let ax = a.accessibility_frame().unwrap();
            let button = ax.node(id).unwrap().node;
            assert_eq!(button.role(), Role::Button);
            assert_eq!(button.name(), "建立報表");
            assert_eq!(button.expanded_state(), Some(expanded));
            assert_eq!(button.has_menu_popup(), has_menu_popup);
            assert_eq!(ax.focused(), Some(id));
            assert!(ax.children(id).next().is_none());
            assert!(ax.activation_target(id).is_some());

            if hits < 2 {
                assert!(a.dispatch(Event::Activate).unwrap());
            }
        }
    }
}

#[test]
fn visible_press_ripple_expands_inside_the_actual_rounded_face() {
    let mut a = Runtime::new(Demo::default()).unwrap();
    paint(&mut a, 0.);
    let face = bounds(&a, "fab-face");
    let (x, y) = (face.x + 8., face.y + 9.);
    a.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut a, 16.);
    let early = paint(&mut a, 40.);
    let later = paint(&mut a, 120.);
    let early = ripple(&early).rect();
    assert!(early.width < ripple(&later).rect().width);
    assert!(early.x + early.width / 2. < face.x + face.width / 2.);
    assert!(early.y + early.height / 2. < face.y + face.height / 2.);
    assert_eq!(a.focused(), Some("fab"));
    for [x, y] in ripple(&later).points() {
        let dx = (x - face.x).min(face.x + face.width - x);
        let dy = (y - face.y).min(face.y + face.height - y);
        assert!(dx >= -0.03 && dy >= -0.03);
        assert!((20. - dx).max(0.).hypot((20. - dy).max(0.)) <= 20.03);
    }
    a.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(a.app().hits, 1);
    paint(&mut a, 136.);
    paint(&mut a, 520.);
    assert!(a.dom().by_id("fab-ripple").is_none());
}

#[test]
fn standard_and_lowered_shadows_and_reduced_press_follow_actual_input() {
    for (elevation, rest, hover) in [
        (
            ExpressiveFabElevation::Standard,
            Elevation::Level3,
            Elevation::Level4,
        ),
        (
            ExpressiveFabElevation::Lowered,
            Elevation::Level1,
            Elevation::Level2,
        ),
    ] {
        let mut a = Runtime::new(Demo {
            elevation,
            dark: true,
            ..Demo::default()
        })
        .unwrap();
        a.set_reduced_motion(true);
        paint(&mut a, 0.);
        let theme = Theme::dark();
        let shadow = |runtime: &Runtime<Demo>| node(runtime, "fab-face").box_shadow_layers();
        assert_eq!(shadow(&a), theme.elevation(rest));
        let face = bounds(&a, "fab");
        let (x, y) = (face.x + 20., face.y + 20.);
        a.dispatch(Event::PointerMove { x, y }).unwrap();
        paint(&mut a, 16.);
        assert_eq!(shadow(&a), theme.elevation(hover));
        assert_eq!(node(&a, "fab-state-layer").style().background.3, 20);
        a.dispatch(Event::PointerDown { x, y }).unwrap();
        paint(&mut a, 32.);
        assert_eq!(shadow(&a), theme.elevation(rest));
        assert_eq!(node(&a, "fab-state-layer").style().background.3, 26);
        assert!(a.dom().by_id("fab-ripple").is_none());
    }
}

#[test]
fn exit_keeps_allocation_and_drops_focus_and_input_before_the_paint_ends() {
    let mut a = Runtime::new(Demo::default()).unwrap();
    paint(&mut a, 0.);
    let allocation = bounds(&a, "fab");
    a.activate("fab").unwrap();
    a.apply(Message::Visible(false)).unwrap();
    assert_eq!(a.focused(), Some("before"));
    assert!(a.activate("fab").is_err());
    paint(&mut a, 16.);
    paint(&mut a, 48.);
    assert_eq!(bounds(&a, "fab"), allocation);
    let face = bounds(&a, "fab-face");
    assert!(face.width < allocation.width);
    assert!(node(&a, "fab-face").style().background.3 > 0);
    assert!((face.x + face.width / 2. - (allocation.x + allocation.width / 2.)).abs() <= 0.5);
    let id = a.dom().by_id("fab").unwrap();
    assert!(a.accessibility_frame().unwrap().node(id).is_none());
    assert_eq!(a.app().hits, 1);

    a.set_reduced_motion(true);
    paint(&mut a, 64.);
    assert_eq!(node(&a, "fab-face").style().background.3, 0);
    a.apply(Message::Visible(true)).unwrap();
    paint(&mut a, 80.);
    assert_eq!(bounds(&a, "fab-face"), allocation);
    assert_eq!(node(&a, "fab-face").style().background.3, 255);
    assert_eq!(a.focused(), Some("before"));
    a.activate("fab").unwrap();
    assert_eq!(a.app().hits, 2);
}
