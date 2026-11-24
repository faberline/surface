use surface::{
    Align, Color, ExtendedDisplayList, ExtendedPaintOp, Length, Node, PaintOp, Rect, Role, Style,
    TextAlignment,
};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, AppError, Application, Event, FrameInput,
    FramePress, InputAnimationUpdate, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, ChoiceState, Elevation, SwitchIcons, SwitchMotion, SwitchOptions,
    Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const NAME: &str = "接收專案更新與通知";
const FULL_LABEL: &str = "接收這個專案的訊息與更新通知";
const ID: &str = "settings";
const TRACK: &str = "settings-track";
const HANDLE: &str = "settings-handle";
const ICON: &str = "settings-icon";
const RING: &str = "settings-focus-ring";
const LAYER: &str = "settings-state-layer";
const LABEL: &str = "settings-label";

#[derive(Clone, Copy)]
enum Message {
    Change(bool),
    Authored(bool),
    Enabled(bool),
    Noop,
}

struct Model {
    choice: ChoiceState,
    config: usize,
    dark: bool,
    rtl: bool,
    enlarged: bool,
    accept: bool,
    messages: Vec<bool>,
    icons: [VectorIcon; 2],
    motion: SwitchMotion,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            choice: ChoiceState::default(),
            config: 0,
            dark: false,
            rtl: false,
            enlarged: false,
            accept: true,
            messages: Vec::new(),
            icons: [
                VectorIcon::new([[2., 2.], [22., 2.], [2., 22.]]).unwrap(),
                VectorIcon::new([[2., 2.], [22., 2.], [22., 22.]]).unwrap(),
            ],
            motion: SwitchMotion::new(),
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

fn options(
    icons: &[VectorIcon; 2],
    config: usize,
    choice: ChoiceState,
    rtl: bool,
    enlarged: bool,
) -> SwitchOptions<'_> {
    SwitchOptions {
        id: ID,
        label: if enlarged { FULL_LABEL } else { "通知" },
        name: Some(NAME),
        state: choice,
        icons: match config {
            1 => SwitchIcons::Selected(&icons[1]),
            2 => SwitchIcons::Both {
                unselected: &icons[0],
                selected: &icons[1],
            },
            _ => SwitchIcons::None,
        },
        direction: if rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        },
        text_scale: if enlarged { 2. } else { 1. },
    }
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Change(checked) => {
                self.messages.push(checked);
                if self.accept {
                    self.choice.selected = checked;
                }
            }
            Message::Authored(checked) => self.choice.selected = checked,
            Message::Enabled(enabled) => self.choice.disabled = !enabled,
            Message::Noop => (),
        }
    }

    fn text_scale(&self) -> f32 {
        if self.enlarged {
            2.
        } else {
            1.
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
        let options = options(
            &self.icons,
            self.config,
            self.choice,
            self.rtl,
            self.enlarged,
        );
        InputAnimationUpdate {
            animation: self.motion.advance(options, &frame, &input, press, hovered),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let options = options(
            &self.icons,
            self.config,
            self.choice,
            self.rtl,
            self.enlarged,
        );
        View::new(Node::new(Role::Window, "Settings")).children([
            theme.switch_with_options(options, &self.motion, Message::Change),
            theme.button("after", "返回", Message::Noop),
        ])
    }
}

fn paint(runtime: &mut Runtime<Model>, text: &mut TextSystem, t: f64) -> ExtendedDisplayList {
    runtime.set_time(t);
    runtime.render_extended(320., 300., text).unwrap()
}

fn node<'a>(runtime: &'a Runtime<Model>, key: &str) -> &'a Node {
    runtime
        .dom()
        .get(runtime.dom().by_id(key).unwrap())
        .unwrap()
}

fn bounds(runtime: &Runtime<Model>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(key).unwrap().index()].bounds
}

fn center(bounds: Rect) -> (f32, f32) {
    (bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
}

fn track_center(runtime: &Runtime<Model>) -> f32 {
    center(bounds(runtime, HANDLE)).0 - bounds(runtime, TRACK).x
}

fn quad(list: &ExtendedDisplayList, bounds: Rect, ink: Color, radius: f32) -> bool {
    list.ops().iter().any(|op| {
        matches!(
            op, ExtendedPaintOp::Legacy(PaintOp::Quad { rect, color, radius: r, .. })
                if *rect == bounds && *color == ink && *r == radius
        )
    })
}

#[test]
fn exact_off_on_and_three_icon_configurations_keep_the_same_track() {
    for dark in [false, true] {
        for rtl in [false, true] {
            for config in 0..3 {
                for selected in [false, true] {
                    let mut runtime = Runtime::new(Model {
                        dark,
                        rtl,
                        config,
                        choice: ChoiceState {
                            selected,
                            disabled: false,
                        },
                        ..Model::default()
                    })
                    .unwrap();
                    let list = paint(&mut runtime, &mut TextSystem::default(), 0.);
                    let theme = runtime.app().theme();
                    let track = bounds(&runtime, TRACK);
                    let handle = bounds(&runtime, HANDLE);
                    let label = bounds(&runtime, LABEL);
                    let focus = bounds(&runtime, RING);
                    let diameter = if selected || config == 2 { 24. } else { 16. };
                    let track_ink = if selected {
                        theme.colors.primary
                    } else {
                        theme.colors.surface_container_highest
                    };
                    let handle_ink = if selected {
                        theme.colors.on_primary
                    } else {
                        theme.colors.outline
                    };
                    assert_eq!((track.width, track.height), (52., 32.));
                    assert_eq!((handle.width, handle.height), (diameter, diameter));
                    assert_eq!(node(&runtime, TRACK).style().border_width, 2.);
                    assert_eq!(
                        node(&runtime, TRACK).style().border_color,
                        if selected {
                            Color::TRANSPARENT
                        } else {
                            theme.colors.outline
                        }
                    );
                    assert_eq!(
                        node(&runtime, HANDLE).box_shadow_layers(),
                        theme.elevation(Elevation::Level1)
                    );
                    assert_eq!(
                        track_center(&runtime),
                        if selected == rtl { 16. } else { 36. }
                    );
                    assert_eq!(center(track).1, center(handle).1);
                    assert_eq!(
                        bounds(&runtime, LAYER),
                        Rect::new(center(handle).0 - 20., center(track).1 - 20., 40., 40.)
                    );
                    assert_eq!(focus, Rect::new(track.x - 5., track.y - 5., 62., 42.));
                    assert!(quad(&list, track, track_ink, 16.));
                    assert!(quad(&list, handle, handle_ink, diameter / 2.));
                    assert_eq!(
                        node(&runtime, LABEL).style().foreground,
                        theme.colors.on_surface
                    );
                    assert!(if rtl {
                        label.x > track.x + track.width
                    } else {
                        label.x + label.width < track.x
                    });
                    if config == 2 || (config == 1 && selected) {
                        let icon = bounds(&runtime, ICON);
                        let ink = if selected {
                            theme.colors.primary
                        } else {
                            theme.colors.surface_container_highest
                        };
                        assert_eq!((icon.width, icon.height), (16., 16.));
                        assert_eq!(center(icon), center(handle));
                        assert!(list
                            .ops()
                            .iter()
                            .any(|op| matches!(op, ExtendedPaintOp::FillPath(path)
                            if path.color() == ink && path.rect().intersect(icon) == path.rect())));
                    } else {
                        assert!(runtime.dom().by_id(ICON).is_none());
                    }
                }
            }
        }
    }
}

#[test]
fn one_stable_switch_exposes_author_owned_value_and_one_action_per_key_or_ax_press() {
    let mut runtime = Runtime::new(Model {
        accept: false,
        ..Model::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    paint(&mut runtime, &mut text, 0.);
    let id = runtime.dom().by_id(ID).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let target = frame.activation_target(id).unwrap();
    assert_eq!(frame.node(id).unwrap().node.role(), Role::Switch);
    assert_eq!(frame.node(id).unwrap().node.name(), NAME);
    assert_eq!(frame.node(id).unwrap().node.is_checked(), Some(false));
    assert!(frame.children(id).next().is_none());
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ID));
    assert!(runtime.app().messages.is_empty());
    let list = paint(&mut runtime, &mut text, 16.);
    let ring = bounds(&runtime, RING);
    assert!(list.ops().iter().any(|op| matches!(
        op, ExtendedPaintOp::Legacy(PaintOp::Stroke { rect, color, width: 3., .. })
            if *rect == ring && *color == Theme::light().colors.secondary
    )));

    for key in [Key::Enter, Key::Space] {
        let frame = runtime.accessibility_frame().unwrap();
        let focused = frame
            .focus_target(runtime.dom().by_id(ID).unwrap())
            .unwrap();
        let press = KeyPress::new(key, Modifiers::default());
        assert!(runtime
            .accessibility_key(&focused, &press, Event::Activate)
            .unwrap());
        paint(&mut runtime, &mut text, 17.);
        let focused = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(runtime.dom().by_id(ID).unwrap())
            .unwrap();
        assert!(runtime
            .accessibility_key(&focused, &press.with_repeat(true), Event::Activate)
            .unwrap());
        assert_eq!(node(&runtime, ID).is_checked(), Some(false));
    }
    assert_eq!(runtime.app().messages, [true, true]);
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Rejected
    );
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(runtime.dom().by_id(ID).unwrap())
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().messages, [true, true, true]);
    assert_eq!(node(&runtime, ID).name(), NAME);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("after"));
}

#[test]
fn pointer_and_touch_press_grow_to_28_without_changing_the_target() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let mut text = TextSystem::default();
    paint(&mut runtime, &mut text, 0.);
    let row = bounds(&runtime, ID);
    let (x, y) = center(bounds(&runtime, LABEL));
    let Color(r, g, b, _) = Theme::light().colors.on_surface;
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    let hover = paint(&mut runtime, &mut text, 4.);
    assert!(quad(
        &hover,
        bounds(&runtime, LAYER),
        Color(r, g, b, 20),
        20.
    ));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let press = paint(&mut runtime, &mut text, 8.);
    assert_eq!(track_center(&runtime), 16.);
    assert_eq!(bounds(&runtime, HANDLE).width, 28.);
    assert_eq!(bounds(&runtime, ID), row);
    assert!(quad(
        &press,
        bounds(&runtime, LAYER),
        Color(r, g, b, 26),
        20.
    ));
    assert_eq!(node(&runtime, ID).is_checked(), Some(false));
    assert!(runtime.app().messages.is_empty());
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().messages, [true]);
    assert_eq!(node(&runtime, ID).is_checked(), Some(true));
    paint(&mut runtime, &mut text, 300.);
    paint(&mut runtime, &mut text, 700.);
    assert_eq!(track_center(&runtime), 36.);

    let (x, y) = center(bounds(&runtime, TRACK));
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    paint(&mut runtime, &mut text, 710.);
    assert_eq!(bounds(&runtime, HANDLE).width, 28.);
    runtime.cancel_pointer().unwrap();
    paint(&mut runtime, &mut text, 720.);
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    paint(&mut runtime, &mut text, 1100.);
    assert_eq!(bounds(&runtime, HANDLE).width, 24.);
    assert_eq!(runtime.app().messages, [true]);
    assert_eq!(node(&runtime, ID).is_checked(), Some(true));

    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().messages, [true, false]);
    assert_eq!(node(&runtime, ID).is_checked(), Some(false));
    paint(&mut runtime, &mut text, 1600.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y: 290. }).unwrap();
    paint(&mut runtime, &mut text, 1700.);
    assert_eq!(runtime.app().messages, [true, false]);
    assert_eq!(node(&runtime, ID).is_checked(), Some(false));
}

#[test]
fn disabled_has_off_and_on_paint_but_no_focus_or_action() {
    let mut text = TextSystem::default();
    for selected in [false, true] {
        let model = Model {
            config: 2,
            choice: ChoiceState {
                selected,
                disabled: true,
            },
            ..Model::default()
        };
        let theme = model.theme();
        let mut runtime = Runtime::new(model).unwrap();
        paint(&mut runtime, &mut text, 0.);
        let id = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        assert!(frame.node(id).unwrap().node.is_disabled());
        assert_eq!(frame.node(id).unwrap().node.is_checked(), Some(selected));
        assert!(frame.focus_target(id).is_none());
        assert!(frame.activation_target(id).is_none());
        let Color(r, g, b, _) = if selected {
            theme.colors.on_surface
        } else {
            theme.colors.surface_container_highest
        };
        assert_eq!(node(&runtime, TRACK).style().background, Color(r, g, b, 31));
        assert_eq!(
            node(&runtime, HANDLE).box_shadow_layers(),
            theme.elevation(Elevation::Level0)
        );
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("after"));
        let (x, y) = center(bounds(&runtime, TRACK));
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
        assert!(runtime.app().messages.is_empty());
        assert!(matches!(runtime.activate(ID), Err(AppError::MissingControl(id)) if id == ID));
    }
}

#[test]
fn authored_changes_animate_and_reverse_but_reduce_motion_snaps_to_exact_value() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let mut text = TextSystem::default();
    paint(&mut runtime, &mut text, 0.);
    runtime.apply(Message::Authored(true)).unwrap();
    assert_eq!(node(&runtime, ID).is_checked(), Some(true));
    paint(&mut runtime, &mut text, 10.);
    assert_eq!(track_center(&runtime), 16.);
    paint(&mut runtime, &mut text, 45.);
    let mid = track_center(&runtime);
    assert!(mid > 16. && mid < 36.);
    runtime.apply(Message::Authored(false)).unwrap();
    paint(&mut runtime, &mut text, 45.);
    assert_eq!(track_center(&runtime), mid);
    paint(&mut runtime, &mut text, 600.);
    assert_eq!(track_center(&runtime), 16.);
    runtime.apply(Message::Authored(true)).unwrap();
    runtime.set_reduced_motion(true);
    paint(&mut runtime, &mut text, 616.);
    assert_eq!(track_center(&runtime), 36.);
    assert_eq!(bounds(&runtime, HANDLE).width, 24.);

    runtime.apply(Message::Enabled(false)).unwrap();
    paint(&mut runtime, &mut text, 616.);
    let id = runtime.dom().by_id(ID).unwrap();
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id)
        .is_none());
}

#[test]
fn compact_rtl_200_percent_wraps_a_stable_full_label_after_the_fixed_track() {
    let mut runtime = Runtime::new(Model {
        dark: true,
        rtl: true,
        enlarged: true,
        ..Model::default()
    })
    .unwrap();
    let list = paint(&mut runtime, &mut TextSystem::default(), f64::NAN);
    let row = bounds(&runtime, ID);
    let label = bounds(&runtime, LABEL);
    let track = bounds(&runtime, TRACK);
    assert_eq!((track.width, track.height), (52., 32.));
    assert_eq!(track_center(&runtime), 36.);
    assert!(row.width <= 320.);
    assert!(row.height >= 96., "row: {row:?}, label: {label:?}");
    assert!(label.x >= track.x + track.width);
    assert_eq!(row.intersect(label), label);
    assert_eq!(node(&runtime, ID).name(), NAME);
    assert!(list
        .ops()
        .iter()
        .any(|op| matches!(op, ExtendedPaintOp::AlignedText(aligned)
        if aligned.alignment() == TextAlignment::Right &&
        matches!(aligned.run(), PaintOp::StyledText { text, style, clip, .. }
            if text == FULL_LABEL && style.font_size == 32. && style.line_height == Some(48.)
                && clip.intersect(label) == label))));
}

#[test]
fn stretched_rows_share_the_logical_trailing_edge_with_short_long_and_empty_labels() {
    struct Rows {
        rtl: bool,
        width: f32,
        labels: &'static [(&'static str, &'static str)],
        changes: Vec<&'static str>,
    }

    impl Application for Rows {
        type Message = (&'static str, bool);

        fn update(&mut self, (id, next): Self::Message) {
            assert!(next);
            self.changes.push(id);
        }

        fn view(&self) -> View<Self::Message> {
            let motion = SwitchMotion::new();
            let theme = Theme::light();
            View::new(
                Node::new(Role::Window, "Settings").styled(
                    Style::column()
                        .align(Align::Stretch)
                        .width(Length::Px(self.width)),
                ),
            )
            .children(self.labels.iter().map(|&(id, label)| {
                theme.switch_with_options(
                    SwitchOptions {
                        id,
                        label,
                        name: Some("接收通知"),
                        state: ChoiceState::default(),
                        icons: SwitchIcons::None,
                        direction: if self.rtl {
                            LayoutDirection::RightToLeft
                        } else {
                            LayoutDirection::LeftToRight
                        },
                        text_scale: 2.,
                    },
                    &motion,
                    move |next| (id, next),
                )
            }))
        }
    }

    let mut text = TextSystem::default();
    for rtl in [false, true] {
        for (width, labels) in [
            (
                272.,
                [("short", "通知"), ("long", FULL_LABEL), ("blank", "  ")].as_slice(),
            ),
            (62., [("blank", "")].as_slice()),
        ] {
            let mut runtime = Runtime::new(Rows {
                rtl,
                width,
                labels,
                changes: Vec::new(),
            })
            .unwrap();
            runtime.render_extended(320., 300., &mut text).unwrap();
            let rect = |key: &str| {
                let id = runtime.dom().by_id(key).unwrap();
                runtime.computed().unwrap().nodes[id.index()].bounds
            };
            for &(id, label) in labels {
                let row = rect(id);
                let track = rect(&format!("{id}-track"));
                let outline = rect(&format!("{id}-focus-ring"));
                let caption = rect(&format!("{id}-label"));
                assert_eq!(row.width, width);
                assert!(row.height >= 48.);
                assert_eq!((track.width, track.height), (52., 32.));
                assert_eq!(center(track).1, center(row).1);
                let expected_x = if rtl {
                    row.x + 5.
                } else {
                    row.x + row.width - 57.
                };
                assert_eq!(track.x, expected_x, "{id}, rtl={rtl}, width={width}");
                assert_eq!(row.intersect(outline), outline);
                assert_eq!(row.intersect(caption), caption);
                assert_eq!(
                    caption.width,
                    width - 62. - if label.trim().is_empty() { 0. } else { 12. }
                );
            }
            let id = labels[0].0;
            let (x, y) = center(rect(id));
            runtime.dispatch(Event::PointerDown { x, y }).unwrap();
            runtime.dispatch(Event::PointerUp { x, y }).unwrap();
            assert_eq!(runtime.app().changes, [id]);
            assert_eq!(runtime.focused(), Some(id));
        }
    }
}
