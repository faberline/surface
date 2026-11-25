use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, Node, NodeId, PaintOp, Rect, Role, Style,
    TextMeasurer, TextStyle,
};
use surface_app::{
    AccessibilityActivationResult, AccessibilityScrollResult, Application, EditKey, Event, Key,
    KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, BaselineSegmentedButtonOptions, BaselineSegmentedItem,
    BaselineSegmentedSelection, Theme, VectorIcon,
};

use surface_wgpu::TextSystem;

const IDS: [&str; 5] = ["walk", "tram", "bike", "boat", "rail"];

#[derive(Clone, Copy)]
enum Message {
    Choose(usize),
    Disable(usize),
    Modal(bool),
    Noop,
}

struct Model {
    selection: [bool; 5],
    disabled: [bool; 5],
    count: usize,
    rtl: bool,
    dark: bool,
    available_width: f32,
    text_scale: f32,
    multiple: bool,
    accept: bool,
    modal: bool,
    requests: Vec<usize>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            selection: [false, false, true, false, false],
            disabled: [false, true, false, false, false],
            count: 3,
            rtl: false,
            dark: false,
            available_width: 400.,
            text_scale: 1.,
            multiple: false,
            accept: true,
            modal: false,
            requests: Vec::new(),
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

    fn group(&self) -> Option<View<Message>> {
        let selected = (0..self.count)
            .filter(|&i| self.selection[i])
            .map(|i| IDS[i])
            .collect::<Vec<_>>();
        let options = BaselineSegmentedButtonOptions {
            id: "travel",
            name: "交通方式",
            available_width: self.available_width,
            text_scale: self.text_scale,
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            selection: if self.multiple {
                BaselineSegmentedSelection::Multiple(&selected)
            } else {
                BaselineSegmentedSelection::Single(selected.first().copied().unwrap_or("unknown"))
            },
        };
        let art = VectorIcon::new([[0., 0.], [24., 0.], [0., 24.]]).unwrap();
        let labels = if self.text_scale == 1. {
            [Some("步行"), Some("電車"), None, Some("渡輪"), Some("火車")]
        } else {
            [
                Some("沿河散步"),
                Some("社區電車"),
                None,
                Some("市區渡輪"),
                Some("城際火車"),
            ]
        };
        self.theme().baseline_segmented_button(
            options,
            (0..self.count).map(|i| BaselineSegmentedItem {
                id: IDS[i],
                label: labels[i],
                name: (i == 2).then_some("騎自行車"),
                icon: (i == 0 || i == 2).then_some(&art),
                disabled: self.disabled[i],
                on_activate: Message::Choose(i),
            }),
        )
    }
}

impl Application for Model {
    type Message = Message;

    fn text_scale(&self) -> f32 {
        self.text_scale
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Choose(i) => {
                self.requests.push(i);
                if self.accept && self.multiple {
                    self.selection[i] = !self.selection[i];
                } else if self.accept {
                    self.selection = std::array::from_fn(|selected| i == selected);
                }
            }
            Message::Disable(i) => self.disabled[i] = true,
            Message::Modal(open) => self.modal = open,
            Message::Noop => {}
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let modal = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Confirm").id("dialog"))
                .children([theme.button("dismiss", "Close", Message::Modal(false))])
                .modal(Message::Modal(false))
        });
        View::new(Node::new(Role::Window, "Travel").styled(Style::column().padding(12.).gap(12.)))
            .children(
                [
                    self.group().unwrap(),
                    theme.button("after", "After", Message::Noop),
                ]
                .into_iter()
                .chain(modal),
            )
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        let width = width.unwrap_or(natural).min(natural);
        (width, (natural / width.max(1.)).ceil().max(1.) * size)
    }

    fn measure_text(&mut self, text: &str, style: &TextStyle, width: Option<f32>) -> (f32, f32) {
        let (width, height) = self.measure(text, style.font_size, width);
        let lines = height / style.font_size;
        (width, lines * style.line_height.unwrap_or(style.font_size))
    }
}

fn draw(runtime: &mut Runtime<Model>) -> ExtendedDisplayList {
    let width = (runtime.app().available_width + 24.).max(320.);
    runtime.render_extended(width, 820., &mut Metrics).unwrap()
}

fn id(runtime: &Runtime<Model>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn rect(runtime: &Runtime<Model>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn tab(runtime: &mut Runtime<Model>, reverse: bool, will_focus: &str) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
    assert_eq!(runtime.focused(), Some(will_focus));
}

fn arrow(runtime: &mut Runtime<Model>, key: EditKey, will_focus: &str) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
    assert_eq!(runtime.focused(), Some(will_focus));
}

fn projected_key(runtime: &mut Runtime<Model>, key: Key, repeated: bool) {
    draw(runtime);
    let frame = runtime.accessibility_frame().unwrap();
    let node = id(runtime, runtime.focused().unwrap());
    let target = frame.focus_target(node).unwrap();
    let press = KeyPress::new(key, Modifiers::default()).with_repeat(repeated);
    assert!(runtime
        .accessibility_key(&target, &press, Event::Activate)
        .unwrap());
}

fn alpha(Color(r, g, b, _): Color, alpha: u8) -> Color {
    Color(r, g, b, alpha)
}

fn ink(op: &ExtendedPaintOp) -> Option<Color> {
    match op {
        ExtendedPaintOp::Legacy(
            PaintOp::Quad { color, .. }
            | PaintOp::Shape { color, .. }
            | PaintOp::Stroke { color, .. }
            | PaintOp::ShapeStroke { color, .. }
            | PaintOp::StyledText { color, .. },
        ) => Some(*color),
        ExtendedPaintOp::FillPath(path) => Some(path.color()),
        ExtendedPaintOp::AlignedText(text) => match text.run() {
            PaintOp::StyledText { color, .. } => Some(*color),
            _ => None,
        },
        _ => None,
    }
}

fn fills(list: &ExtendedDisplayList, color: Color) -> impl Iterator<Item = Rect> + '_ {
    list.ops().iter().filter_map(move |op| {
        let rect = match op {
            ExtendedPaintOp::Legacy(PaintOp::Quad { rect, .. } | PaintOp::Shape { rect, .. }) => {
                *rect
            }
            _ => return None,
        };
        (ink(op) == Some(color)).then_some(rect)
    })
}

#[test]
fn defaults_use_one_outline_one_seam_and_equal_40_dp_faces_with_48_dp_targets() {
    for dark in [false, true] {
        let mut runtime = Runtime::new(Model {
            dark,
            ..Model::default()
        })
        .unwrap();
        let list = draw(&mut runtime);
        assert!(rect(&runtime, "travel-track").width < rect(&runtime, "travel").width);
        let width = rect(&runtime, "walk").width;
        assert!(width >= 48.);
        for key in &IDS[..3] {
            let hit = rect(&runtime, key);
            let face = rect(&runtime, &format!("{key}-face"));
            assert_eq!(hit.height, 48.);
            assert_eq!(face.height, 40.);
            assert_eq!(hit.width, width);
            assert_eq!(face.width, width);
            assert_eq!(face.y, hit.y + 4.);
        }
        let c = runtime.app().theme().colors;
        let outline = rect(&runtime, "travel-outline");
        let face = rect(&runtime, "bike-face");
        assert!(fills(&list, c.secondary_container).any(|rect| rect == face));
        let segment_rect = rect(&runtime, "travel-seam-0");
        assert!(fills(&list, c.outline).any(|rect| rect == segment_rect));
        assert_eq!(
            fills(&list, c.outline)
                .filter(|rect| rect.width == 1. && rect.height == 38.)
                .count(),
            2
        );
        for seam_index in 0..2 {
            let seam = rect(&runtime, &format!("travel-seam-{seam_index}"));
            assert_eq!(seam.y, outline.y + 1.);
            assert_eq!(seam.height, 38.);
            assert_eq!(seam.width, 1.);
        }
        let disabled_ink = alpha(c.on_surface, 31);
        assert_eq!(
            fills(&list, disabled_ink)
                .filter(|r| r.height == 1.)
                .count(),
            2
        );
        assert!(list
            .ops()
            .iter()
            .any(|op| ink(op) == Some(alpha(c.on_surface, 97))));
        assert!(list.ops().iter().any(|op| matches!(op,
            ExtendedPaintOp::Legacy(PaintOp::Stroke { rect, clip, color, width, .. })
            if *width == 1. && *color == c.outline && rect.height == 40. && clip.width == 20.
        )));
    }
}

#[test]
fn manual_radio_entry_projection_and_keyboard_match_material() {
    let mut runtime = Runtime::new(Model {
        accept: false,
        ..Model::default()
    })
    .unwrap();
    draw(&mut runtime);
    let frame = runtime.accessibility_frame().unwrap();
    let root = id(&runtime, "travel");
    let a = id(&runtime, "walk");
    let b = id(&runtime, "tram");
    let c = id(&runtime, "bike");
    assert_eq!(frame.node(root).unwrap().node.role(), Role::RadioGroup);
    for (node, checked) in [(a, false), (b, false), (c, true)] {
        let node = frame.node(node).unwrap().node;
        assert_eq!(node.role(), Role::Radio);
        assert_eq!(node.is_checked(), Some(checked));
    }
    assert_eq!(frame.node(c).unwrap().node.name(), "騎自行車");
    assert_eq!(frame.children(a).count(), 0);
    assert!(frame.focus_target(b).is_none() && frame.activation_target(b).is_none());
    assert!(frame.focus_target(c).is_some() && frame.activation_target(c).is_none());
    let projected = frame.activation_target(a).unwrap();
    tab(&mut runtime, false, "walk");
    assert_eq!(
        runtime.accessibility_activate(&projected).unwrap(),
        AccessibilityActivationResult::Activated
    );
    draw(&mut runtime);
    arrow(&mut runtime, EditKey::Right, "bike");
    assert_eq!(runtime.app().requests, [0]);
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().requests, [0]);
    tab(&mut runtime, false, "after");
    tab(&mut runtime, true, "walk");
    projected_key(&mut runtime, Key::Enter, false);
    projected_key(&mut runtime, Key::Enter, true);
    projected_key(&mut runtime, Key::Space, false);
    assert_eq!(runtime.app().requests, [0, 0, 0]);
    assert!(runtime.app().selection[2]);
    draw(&mut runtime);
    let stale = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(a)
        .unwrap();
    runtime.apply(Message::Disable(0)).unwrap();
    draw(&mut runtime);
    assert_ne!(
        runtime.accessibility_activate(&stale).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(a)
        .is_none());
}

#[test]
fn multi_checkboxes_allow_clear_and_real_projected_activation() {
    let mut runtime = Runtime::new(Model {
        multiple: true,
        selection: [false; 5],
        ..Model::default()
    })
    .unwrap();
    draw(&mut runtime);
    let frame = runtime.accessibility_frame().unwrap();
    let root = id(&runtime, "travel");
    let a = id(&runtime, "walk");
    let c = id(&runtime, "bike");
    assert_eq!(frame.node(root).unwrap().node.role(), Role::Group);
    let target = frame.activation_target(a).unwrap();
    assert_eq!(frame.node(a).unwrap().node.role(), Role::Checkbox);
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert!(runtime.app().selection[0]);
    draw(&mut runtime);
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(c)
        .unwrap();
    runtime.accessibility_activate(&target).unwrap();
    assert_eq!(runtime.app().selection, [true, false, true, false, false]);
    draw(&mut runtime);
    for member in [a, c] {
        let target = runtime
            .accessibility_frame()
            .unwrap()
            .activation_target(member)
            .unwrap();
        runtime.accessibility_activate(&target).unwrap();
        draw(&mut runtime);
    }
    assert_eq!(runtime.app().selection, [false; 5]);
    assert_eq!(runtime.app().requests, [0, 2, 0, 2]);
}

#[test]
fn compact_rtl_grows_one_row_and_paints_the_whole_focus_outline() {
    let mut runtime = Runtime::new(Model {
        count: 5,
        rtl: true,
        dark: true,
        available_width: 296.,
        text_scale: 2.,
        ..Model::default()
    })
    .unwrap();
    draw(&mut runtime);
    tab(&mut runtime, false, "walk");
    arrow(&mut runtime, EditKey::Left, "bike");
    let list = draw(&mut runtime);
    let focus = rect(&runtime, "bike-focus");
    let face = rect(&runtime, "bike-face");
    assert_eq!(
        focus,
        Rect::new(
            face.x - 5.,
            face.y - 5.,
            face.width + 10.,
            face.height + 10.
        )
    );
    let secondary = runtime.app().theme().colors.secondary;
    assert!(
        matches!(list.ops().last(), Some(ExtendedPaintOp::Legacy(PaintOp::Stroke { rect, clip, width, color, .. })) if *rect == focus && rect.intersect(*clip) == *rect && *width == 3. && *color == secondary)
    );
    let outline = rect(&runtime, "travel-outline");
    let mut prev = f32::INFINITY;
    for key in IDS {
        let hit = rect(&runtime, key);
        let face = rect(&runtime, &format!("{key}-face"));
        assert!(hit.x < prev && hit.height >= 48. && hit.width >= 48. && face.height > 40.);
        assert_eq!(face.y, outline.y);
        assert_eq!(face.width, outline.width / 5.);
        prev = hit.x;
        let label_key = format!("{key}-label");
        if let Some(label) = runtime.dom().by_id(&label_key) {
            let rect = runtime.computed().unwrap().nodes[label.index()].bounds;
            let visible = rect.intersect(face);
            assert!(
                (visible.width - rect.width).abs() < 0.01
                    && (visible.height - rect.height).abs() < 0.01,
                "{key} label must stay within face"
            );
        }
    }
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn narrow_rtl_scroll_keeps_whole_targets_and_reveals_keyboard_and_ax_choices() {
    let mut runtime = Runtime::new(Model {
        count: 5,
        available_width: 174.,
        rtl: true,
        ..Model::default()
    })
    .unwrap();
    draw(&mut runtime);
    let view = rect(&runtime, "travel");
    let first = rect(&runtime, "walk");
    let last = rect(&runtime, "rail");
    assert_eq!(first.intersect(view), first);
    assert!(last.intersect(view).width <= 0.);
    assert_eq!(last.width, 48.);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.scroll_target(id(&runtime, "rail")).is_some());
    tab(&mut runtime, false, "walk");
    arrow(&mut runtime, EditKey::End, "rail");
    draw(&mut runtime);
    let last = rect(&runtime, "rail");
    assert_eq!(last.intersect(view), last);
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn touch_accepts_target_slack_and_cancellation_clears_the_wave() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    runtime.set_time(0.);
    draw(&mut runtime);
    let hit = rect(&runtime, "walk");
    let face = rect(&runtime, "walk-face");
    let (x, y) = (hit.x + hit.width / 2., hit.y + 1.);
    assert!(hit.contains(x, y) && !face.contains(x, y));
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    draw(&mut runtime);
    runtime.set_time(50.);
    let held = draw(&mut runtime);
    let wave = held
        .ops()
        .iter()
        .find_map(|op| match op {
            ExtendedPaintOp::FillPath(path) if path.color().3 < 255 && path.color().3 > 0 => {
                Some(path)
            }
            _ => None,
        })
        .expect("accepted ripple");
    assert!(wave.points().iter().all(|p| face.contains(p[0], p[1])));
    assert!(runtime.app().requests.is_empty());
    assert!(runtime.cancel_pointer().unwrap());
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert!(runtime.app().requests.is_empty());
    runtime.set_reduced_motion(true);
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    let reduced = draw(&mut runtime);
    assert!(fills(&reduced, alpha(runtime.app().theme().colors.on_surface, 26)).any(|r| r == face));
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.app().requests, [0]);
    assert!(runtime.app().selection[0]);
}

#[test]
fn rejects_invalid_range_count_name_and_selection() {
    assert!(Model {
        count: 2,
        multiple: true,
        ..Model::default()
    }
    .group()
    .is_some());
    let invalid = [
        Model {
            count: 1,
            ..Model::default()
        },
        Model {
            available_width: 50.,
            ..Model::default()
        },
        Model {
            text_scale: f32::NAN,
            ..Model::default()
        },
        Model {
            selection: [false; 5],
            ..Model::default()
        },
    ];
    for model in invalid {
        assert!(model.group().is_none());
    }
    let options = BaselineSegmentedButtonOptions {
        id: "bad",
        name: "Options",
        available_width: 200.,
        direction: LayoutDirection::LeftToRight,
        text_scale: 1.,
        selection: BaselineSegmentedSelection::Multiple(&[]),
    };
    let item = |id| BaselineSegmentedItem {
        id,
        label: Some("Option"),
        name: None,
        icon: None,
        disabled: false,
        on_activate: (),
    };
    assert!(Theme::light()
        .baseline_segmented_button(options, [item("duplicate"), item("duplicate")])
        .is_none());
    let art = VectorIcon::new([[0., 0.], [24., 0.], [0., 24.]]).unwrap();
    let unnamed = BaselineSegmentedItem {
        label: None,
        icon: Some(&art),
        ..item("unnamed")
    };
    assert!(Theme::light()
        .baseline_segmented_button(options, [item("named"), unnamed])
        .is_none());
}

#[test]
fn action_projection_cannot_cross_a_later_modal_scope() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    draw(&mut runtime);
    let walk = id(&runtime, "walk");
    let action = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(walk)
        .unwrap();
    runtime.apply(Message::Modal(true)).unwrap();
    draw(&mut runtime);
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(walk)
        .is_none());
    assert_ne!(
        runtime.accessibility_activate(&action).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert!(runtime.app().requests.is_empty());
    runtime.apply(Message::Modal(false)).unwrap();
    draw(&mut runtime);
    let current = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(walk)
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&current).unwrap(),
        AccessibilityActivationResult::Activated
    );
}

#[test]
fn measured_cjk_and_actual_accessibility_reveal_at_two_times_scale() {
    let mut runtime = Runtime::new(Model {
        count: 5,
        rtl: true,
        dark: true,
        text_scale: 2.,
        available_width: 174.,
        ..Model::default()
    })
    .unwrap();
    let mut fonts = TextSystem::default();
    let mut draw =
        |runtime: &mut Runtime<Model>| runtime.render_extended(320., 820., &mut fonts).unwrap();
    let list = draw(&mut runtime);
    let view = rect(&runtime, "travel");
    let first = rect(&runtime, "walk");
    let last = rect(&runtime, "rail");
    assert!(first.width >= 52. && first.intersect(view) == first);
    assert!(last.intersect(view).width <= 0.);
    assert!(list.ops().iter().any(|op| matches!(op, ExtendedPaintOp::AlignedText(text)
        if matches!(text.run(), PaintOp::StyledText { text, rect, .. } if text == "沿河散步" && rect.height >= 40.)
    )));
    let rail = id(&runtime, "rail");
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.activation_target(rail).is_none());
    let target = frame.scroll_target(rail).unwrap();
    let result = runtime.accessibility_scroll_into_view(&target).unwrap();
    assert_eq!(result, AccessibilityScrollResult::Scheduled);
    draw(&mut runtime);
    let last = rect(&runtime, "rail");
    assert_eq!(last.intersect(view), last);
    let after = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(rail)
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&after).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().requests, [4]);
}

#[test]
fn icon_stacks_when_more_chinese_text_fits_on_each_line() {
    let mut runtime = Runtime::new(Model {
        available_width: 288.,
        rtl: true,
        text_scale: 2.,
        ..Model::default()
    })
    .unwrap();
    draw(&mut runtime);
    let icon = rect(&runtime, "walk-icon");
    let label = rect(&runtime, "walk-label");
    assert!(icon.y + icon.height < label.y);
    assert!(label.width >= 56.);
    assert!(label.height <= 80.);
}
