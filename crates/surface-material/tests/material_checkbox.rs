use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, Node, PaintOp, Rect, Role, Style, TextAlignment,
};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, Application, Event, FrameInput, FramePress,
    InputAnimationUpdate, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, CheckValue, CheckboxMotion, CheckboxOptions, Theme,
};
use surface_wgpu::TextSystem;

const ID: &str = "updates";
const BOX: &str = "updates-box";
const MARK: &str = "updates-mark";
const LABEL: &str = "updates-label";
const LAYER: &str = "updates-state-layer";
const RING: &str = "updates-focus-ring";
const ERROR: &str = "updates-error";
const NAME: &str = "接收專案通知";
const LONG_LABEL: &str = "接收這個專案的訊息、留言與更新通知";

#[derive(Clone, Copy)]
enum Message {
    Change(CheckValue),
    Author(CheckValue),
}

struct Model {
    options: CheckboxOptions<'static>,
    theme: Theme,
    accept: bool,
    changes: Vec<CheckValue>,
    motion: CheckboxMotion,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            options: CheckboxOptions {
                id: ID,
                label: "通知",
                name: Some(NAME),
                value: CheckValue::Unchecked,
                disabled: false,
                error: false,
                error_description: Some(ERROR),
                direction: LayoutDirection::LeftToRight,
                text_scale: 1.,
            },
            theme: Theme::light(),
            accept: true,
            changes: Vec::new(),
            motion: CheckboxMotion::new(),
        }
    }
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Change(value) => {
                self.changes.push(value);
                if self.accept {
                    self.options.value = value;
                }
            }
            Message::Author(value) => self.options.value = value,
        }
    }

    fn text_scale(&self) -> f32 {
        self.options.text_scale
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
                .advance(self.options, &frame, &input, press, hovered),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        View::new(Node::new(Role::Window, "Settings").styled(Style::column())).children([
            self.theme
                .checkbox_with_options(self.options, &self.motion, Message::Change),
            View::new(Node::new(Role::Text, "").id(ERROR).text("請確認通知選項")),
            self.theme.button(
                "author-mixed",
                "切換混合",
                Message::Author(CheckValue::Mixed),
            ),
            self.theme.button(
                "author-checked",
                "切換全選",
                Message::Author(CheckValue::Checked),
            ),
            self.theme.button(
                "author-unchecked",
                "切換未選",
                Message::Author(CheckValue::Unchecked),
            ),
        ])
    }
}

fn paint(runtime: &mut Runtime<Model>, text: &mut TextSystem, time: f64) -> ExtendedDisplayList {
    runtime.set_time(time);
    runtime.render_extended(320., 400., text).unwrap()
}

fn bounds(runtime: &Runtime<Model>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn center(rect: Rect) -> (f32, f32) {
    (rect.x + rect.width / 2., rect.y + rect.height / 2.)
}

fn quad(list: &ExtendedDisplayList, rect: Rect, ink: Color, radius: f32) -> bool {
    list.ops().iter().any(|op| {
        matches!(op, ExtendedPaintOp::Legacy(PaintOp::Quad { rect: r, color, radius: corner, .. })
            if *r == rect && *color == ink && *corner == radius)
    })
}

fn stroke(list: &ExtendedDisplayList, rect: Rect, ink: Color, width: f32) -> bool {
    list.ops().iter().any(|op| {
        matches!(op, ExtendedPaintOp::Legacy(PaintOp::Stroke { rect: r, color, width: w, .. })
            if *r == rect && *color == ink && *w == width)
    })
}

fn with_alpha(c: Color, alpha: u8) -> Color {
    Color(c.0, c.1, c.2, alpha)
}

#[test]
fn three_states_and_error_or_disabled_have_exact_material_ink_and_sizes() {
    for dark in [false, true] {
        for error in [false, true] {
            for disabled in [false, true] {
                for value in [
                    CheckValue::Unchecked,
                    CheckValue::Checked,
                    CheckValue::Mixed,
                ] {
                    let mut model = Model {
                        theme: if dark { Theme::dark() } else { Theme::light() },
                        ..Model::default()
                    };
                    model.options.value = value;
                    model.options.error = error;
                    model.options.disabled = disabled;
                    let mut runtime = Runtime::new(model).unwrap();
                    let list = paint(&mut runtime, &mut TextSystem::default(), 0.);
                    let root = bounds(&runtime, ID);
                    let art = bounds(&runtime, BOX);
                    let state_layer = bounds(&runtime, LAYER);
                    let ring = bounds(&runtime, RING);
                    let label = bounds(&runtime, LABEL);
                    assert_eq!((art.width, art.height), (18., 18.));
                    assert_eq!((state_layer.width, state_layer.height), (40., 40.));
                    assert_eq!((ring.width, ring.height), (50., 50.));
                    assert_eq!(bounds(&runtime, MARK), art);
                    assert_eq!(center(art), center(state_layer));
                    assert_eq!(center(art), center(ring));
                    assert_eq!(ring.intersect(root), ring);
                    assert_eq!(label.intersect(root), label);
                    assert!(root.width >= 48. && root.height >= 48.);
                    let colors = &runtime.app().theme.colors;
                    let paths: Vec<_> = list
                        .ops()
                        .iter()
                        .filter_map(|op| match op {
                            ExtendedPaintOp::FillPath(path) => Some(path),
                            _ => None,
                        })
                        .collect();
                    if value == CheckValue::Unchecked {
                        let outline = if disabled {
                            with_alpha(colors.on_surface, 97)
                        } else if error {
                            colors.error
                        } else {
                            colors.on_surface_variant
                        };
                        assert!(stroke(&list, art, outline, 2.));
                        assert!(paths.is_empty());
                    } else {
                        let (fill, ink) = if disabled {
                            (with_alpha(colors.on_surface, 97), colors.surface)
                        } else if error {
                            (colors.error, colors.on_error)
                        } else {
                            (colors.primary, colors.on_primary)
                        };
                        assert!(quad(&list, art, fill, 2.));
                        let [path] = paths.as_slice() else {
                            panic!("expected exactly one centered checkbox mark");
                        };
                        assert_eq!(path.color(), ink);
                        assert_eq!(path.rect().intersect(art), path.rect());
                        if value == CheckValue::Mixed {
                            assert_eq!(path.rect().height, 2.);
                        } else {
                            assert!(path.rect().height > 6.);
                        }
                    }
                    let frame = runtime.accessibility_frame().unwrap();
                    let id = runtime.dom().by_id(ID).unwrap();
                    let projected = frame.node(id).unwrap().node;
                    assert_eq!(projected.role(), Role::Checkbox);
                    assert_eq!(projected.name(), NAME);
                    assert_eq!(projected.is_checked(), Some(value != CheckValue::Unchecked));
                    assert_eq!(projected.is_mixed(), value == CheckValue::Mixed);
                    assert_eq!(projected.is_invalid(), error);
                    assert_eq!(projected.is_disabled(), disabled);
                    assert_eq!(frame.activation_target(id).is_some(), !disabled);
                    let help = frame.description_target(id);
                    assert_eq!(
                        help,
                        (error && !disabled).then(|| runtime.dom().by_id(ERROR).unwrap())
                    );
                    assert!(frame.children(id).next().is_none());
                }
            }
        }
    }
}

#[test]
fn controlled_mixed_requests_checked_once_without_key_repeat_and_keeps_focus() {
    let mut model = Model {
        accept: false,
        ..Model::default()
    };
    model.options.value = CheckValue::Mixed;
    let mut runtime = Runtime::new(model).unwrap();
    let mut text = TextSystem::default();
    paint(&mut runtime, &mut text, 0.);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    let frame = paint(&mut runtime, &mut text, 1.);
    assert_eq!(runtime.focused(), Some(ID));
    let nominal = bounds(&runtime, RING);
    let colors = &runtime.app().theme.colors;
    assert!(stroke(&frame, nominal, colors.secondary, 3.));
    assert!(quad(
        &frame,
        bounds(&runtime, LAYER),
        with_alpha(colors.primary, 26),
        20.
    ));
    for key in [Key::Space, Key::Enter] {
        for repeat in [false, true] {
            let frame = runtime.accessibility_frame().unwrap();
            let focus = frame
                .focus_target(runtime.dom().by_id(ID).unwrap())
                .unwrap();
            let press = KeyPress::new(key.clone(), Modifiers::default()).with_repeat(repeat);
            assert!(runtime
                .accessibility_key(&focus, &press, Event::Activate)
                .unwrap());
            paint(&mut runtime, &mut text, 2.);
        }
    }
    let frame = runtime.accessibility_frame().unwrap();
    let target = frame
        .activation_target(runtime.dom().by_id(ID).unwrap())
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().changes, [CheckValue::Checked; 3]);
    assert_eq!(runtime.app().options.value, CheckValue::Mixed);
    assert_eq!(runtime.focused(), Some(ID));
}

#[test]
fn label_mouse_and_art_touch_share_one_target_and_cancellation_preserves_value() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let mut text = TextSystem::default();
    paint(&mut runtime, &mut text, 0.);
    let (x, y) = center(bounds(&runtime, LABEL));
    let original = bounds(&runtime, ID);
    let primary = runtime.app().theme.colors.primary;
    let on_surface = runtime.app().theme.colors.on_surface;

    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    let hover = paint(&mut runtime, &mut text, 10.);
    assert!(quad(
        &hover,
        bounds(&runtime, LAYER),
        with_alpha(on_surface, 20),
        20.
    ));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let pressed = paint(&mut runtime, &mut text, 20.);
    assert_eq!(runtime.app().options.value, CheckValue::Unchecked);
    assert!(runtime.app().changes.is_empty());
    assert_eq!(bounds(&runtime, ID), original);
    assert!(quad(
        &pressed,
        bounds(&runtime, LAYER),
        with_alpha(primary, 26),
        20.
    ));
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().changes, [CheckValue::Checked]);
    paint(&mut runtime, &mut text, 400.);

    let (x, y) = center(bounds(&runtime, BOX));
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    let pressed = paint(&mut runtime, &mut text, 410.);
    assert!(quad(
        &pressed,
        bounds(&runtime, LAYER),
        with_alpha(on_surface, 26),
        20.
    ));
    runtime.cancel_pointer().unwrap();
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    paint(&mut runtime, &mut text, 420.);
    assert_eq!(runtime.app().changes, [CheckValue::Checked]);
    assert_eq!(runtime.app().options.value, CheckValue::Checked);

    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime
        .dispatch(Event::PointerUp { x: 319., y: 399. })
        .unwrap();
    assert_eq!(runtime.app().changes, [CheckValue::Checked]);
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(
        runtime.app().changes,
        [CheckValue::Checked, CheckValue::Unchecked]
    );
}

#[test]
fn mark_grows_and_reverses_while_accessible_state_updates_and_reduced_motion_snaps() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let mut text = TextSystem::default();
    let pose = |runtime: &Runtime<Model>| runtime.app().motion.pose(runtime.app().options);
    paint(&mut runtime, &mut text, 0.);
    runtime.apply(Message::Author(CheckValue::Checked)).unwrap();
    paint(&mut runtime, &mut text, 10.);
    assert_eq!(pose(&runtime).mark_progress, 0.);
    assert_eq!(runtime.app().options.value, CheckValue::Checked);
    let mid = paint(&mut runtime, &mut text, 55.);
    let during = pose(&runtime).mark_progress;
    assert!(during > 0. && during < 1.);
    assert!(mid
        .ops()
        .iter()
        .any(|op| matches!(op, ExtendedPaintOp::FillPath(_))));
    runtime
        .apply(Message::Author(CheckValue::Unchecked))
        .unwrap();
    paint(&mut runtime, &mut text, 55.);
    assert!((pose(&runtime).mark_progress - during).abs() < 0.001);
    paint(&mut runtime, &mut text, 700.);
    assert_eq!(pose(&runtime).mark_progress, 0.);

    runtime.apply(Message::Author(CheckValue::Mixed)).unwrap();
    paint(&mut runtime, &mut text, 710.);
    assert_eq!(pose(&runtime).mixed_fraction, 1.);
    paint(&mut runtime, &mut text, 1400.);
    runtime.apply(Message::Author(CheckValue::Checked)).unwrap();
    paint(&mut runtime, &mut text, 1410.);
    paint(&mut runtime, &mut text, 1460.);
    let morph = pose(&runtime).mixed_fraction;
    assert!(morph > 0. && morph < 1.);
    runtime.set_reduced_motion(true);
    paint(&mut runtime, &mut text, 1470.);
    assert_eq!(pose(&runtime).mixed_fraction, 0.);
    assert_eq!(pose(&runtime).mark_progress, 1.);
}

#[test]
fn popup_free_compact_rtl_wrap_does_not_scale_the_art_or_add_a_focus_stop() {
    let mut model = Model {
        theme: Theme::dark(),
        ..Model::default()
    };
    model.options.label = LONG_LABEL;
    model.options.direction = LayoutDirection::RightToLeft;
    model.options.text_scale = 2.;
    let mut runtime = Runtime::new(model).unwrap();
    let mut text = TextSystem::default();
    let list = paint(&mut runtime, &mut text, 0.);
    let art = bounds(&runtime, BOX);
    let label = bounds(&runtime, LABEL);
    let ring = bounds(&runtime, RING);
    let row = bounds(&runtime, ID);
    assert_eq!((art.width, art.height), (18., 18.));
    assert!(label.height >= 96.);
    assert!(label.x + label.width < ring.x);
    assert_eq!(row.intersect(label), label);
    assert_eq!(row.intersect(ring), ring);
    assert!(list.ops().iter().any(|op| matches!(op,
        ExtendedPaintOp::AlignedText(aligned)
            if aligned.alignment() == TextAlignment::Right
                && matches!(aligned.run(), PaintOp::StyledText { text, style, clip, .. }
                    if text == LONG_LABEL && style.font_size == 32.
                        && style.line_height == Some(48.) && clip.intersect(label) == label)
    )));
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ID));
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("author-mixed"));
}
