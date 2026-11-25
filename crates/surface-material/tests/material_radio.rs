use surface::{Color, ExtendedDisplayList, ExtendedPaintOp, Node, PaintOp, Rect, Role, Style};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, Application, EditKey, Event, FrameInput,
    FramePress, InputAnimationUpdate, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, ChoiceState, RadioGroupLayout, RadioGroupOptions, RadioMotion,
    RadioOptions, Theme,
};
use surface_wgpu::TextSystem;

const IDS: [&str; 4] = ["home", "inactive", "office", "other"];
const LONG_LABEL: &str = "將這個專案的所有重要更新寄到辦公室";

#[derive(Clone, Copy)]
enum Message {
    Select(usize),
    Author(Option<usize>),
    Noop,
}

struct Model {
    selected: Option<usize>,
    requests: Vec<usize>,
    accept: bool,
    direction: LayoutDirection,
    orientation: RadioGroupLayout,
    scale: f32,
    theme: Theme,
    motions: [RadioMotion; 4],
}

impl Default for Model {
    fn default() -> Self {
        Self {
            selected: None,
            requests: Vec::new(),
            accept: true,
            direction: LayoutDirection::LeftToRight,
            orientation: RadioGroupLayout::Vertical,
            scale: 1.,
            theme: Theme::light(),
            motions: std::array::from_fn(|_| RadioMotion::new()),
        }
    }
}

fn options(
    index: usize,
    selected: Option<usize>,
    direction: LayoutDirection,
    text_scale: f32,
) -> RadioOptions<'static> {
    RadioOptions {
        id: IDS[index],
        label: ["住家", "停用", LONG_LABEL, "其他"][index],
        name: (index == 0).then_some("住家地址"),
        state: ChoiceState {
            selected: selected == Some(index),
            disabled: index == 1,
        },
        direction,
        text_scale,
    }
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Select(index) => {
                self.requests.push(index);
                if self.accept {
                    self.selected = Some(index);
                }
            }
            Message::Author(selected) => self.selected = selected,
            Message::Noop => {}
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
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
        let mut update = InputAnimationUpdate::default();
        for (index, motion) in self.motions.iter_mut().enumerate() {
            let change = motion.advance(
                options(index, self.selected, self.direction, self.scale),
                &frame,
                &input,
                press,
                hovered,
            );
            update.animation.rebuild |= change.rebuild;
            update.animation.request_next |= change.request_next;
        }
        update
    }

    fn view(&self) -> View<Message> {
        let theme = &self.theme;
        View::new(Node::new(Role::Window, "Settings").styled(Style::column())).children([
            theme.button("before", "Before", Message::Noop),
            theme.radio_group_with_options(
                RadioGroupOptions {
                    id: "delivery",
                    name: "寄送地址",
                    layout: self.orientation,
                    direction: self.direction,
                },
                self.motions.iter().enumerate().map(|(index, motion)| {
                    theme.radio_with_options(
                        options(index, self.selected, self.direction, self.scale),
                        motion,
                        Message::Select(index),
                    )
                }),
            ),
            theme.button("after", "After", Message::Noop),
        ])
    }
}

fn render(
    runtime: &mut Runtime<Model>,
    text: &mut TextSystem,
    width: f32,
    time: f64,
) -> ExtendedDisplayList {
    runtime.set_time(time);
    runtime.render_extended(width, 500., text).unwrap()
}

fn bounds(runtime: &Runtime<Model>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn center(rect: Rect) -> (f32, f32) {
    (rect.x + rect.width / 2., rect.y + rect.height / 2.)
}

fn quad(list: &ExtendedDisplayList, rect: Rect, color: Color, radius: f32) -> bool {
    list.ops().iter().any(|op| {
        matches!(op, ExtendedPaintOp::Legacy(PaintOp::Quad { rect: r, color: c, radius: s, .. })
            if *r == rect && *c == color && *s == radius)
    })
}

fn stroke(list: &ExtendedDisplayList, rect: Rect, color: Color, width: f32) -> bool {
    list.ops().iter().any(|op| {
        matches!(op, ExtendedPaintOp::Legacy(PaintOp::Stroke { rect: r, color: c, width: w, .. })
            if *r == rect && *c == color && *w == width)
    })
}

fn alpha(c: Color, a: u8) -> Color {
    Color(c.0, c.1, c.2, a)
}

fn key(runtime: &mut Runtime<Model>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

fn tab(runtime: &mut Runtime<Model>, reverse: bool) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
}

#[test]
fn published_geometry_ink_and_semantics_in_both_palettes() {
    for dark in [false, true] {
        for checked in [0, 1] {
            let seed = Model {
                selected: Some(checked),
                theme: if dark { Theme::dark() } else { Theme::light() },
                ..Model::default()
            };
            let mut runtime = Runtime::new(seed).unwrap();
            let paint = render(&mut runtime, &mut TextSystem::default(), 600., 0.);
            let frame = runtime.accessibility_frame().unwrap();
            let dom = runtime.dom();
            let group_id = dom.by_id("delivery").unwrap();
            let group = frame.node(group_id).unwrap().node;
            assert_eq!(group.role(), Role::RadioGroup);
            assert_eq!(group.name(), "寄送地址");
            assert_eq!(frame.children(group_id).count(), IDS.len());
            let colors = &runtime.app().theme.colors;
            for (i, id) in IDS.iter().enumerate() {
                let id_in_dom = dom.by_id(id).unwrap();
                let radio = frame.node(id_in_dom).unwrap().node;
                let b = |suffix| bounds(&runtime, &format!("{id}-{suffix}"));
                let (ring, dot, layer, focus) =
                    (b("ring"), b("dot"), b("state-layer"), b("focus-ring"));
                let root = bounds(&runtime, id);
                assert_eq!(radio.role(), Role::Radio);
                assert_eq!(radio.is_checked(), Some(i == checked));
                assert_eq!(radio.is_disabled(), i == 1);
                assert_eq!(frame.children(id_in_dom).count(), 0);
                assert!(root.width >= 48. && root.height >= 48.);
                assert_eq!((ring.width, ring.height), (20., 20.));
                assert_eq!(
                    (dot.width, dot.height),
                    if i == checked { (10., 10.) } else { (0., 0.) }
                );
                assert_eq!((layer.width, layer.height), (40., 40.));
                assert_eq!((focus.width, focus.height), (30., 30.));
                assert_eq!(center(ring), center(dot));
                assert_eq!(center(ring), center(layer));
                assert_eq!(center(ring), center(focus));
                assert_eq!(focus.intersect(root), focus);
                let ink = if i == 1 {
                    alpha(colors.on_surface, 97)
                } else if i == checked {
                    colors.primary
                } else {
                    colors.on_surface_variant
                };
                assert!(stroke(&paint, ring, ink, 2.));
                if i == checked {
                    assert!(quad(&paint, dot, ink, 5.));
                }
                assert_eq!(
                    frame.activation_target(id_in_dom).is_some(),
                    i != checked && i != 1
                );
            }
            assert_eq!(
                dom.get(dom.by_id("home").unwrap()).unwrap().name(),
                "住家地址"
            );
            assert_eq!(
                dom.get(dom.by_id("office").unwrap()).unwrap().name(),
                LONG_LABEL
            );
        }
    }
}

#[test]
fn pointer_and_touch_share_the_target_and_cancel_or_checked_repeated_press_is_inert() {
    let model = Model {
        selected: Some(0),
        ..Model::default()
    };
    let mut runtime = Runtime::new(model).unwrap();
    let mut text = TextSystem::default();
    let colors = runtime.app().theme.colors;
    render(&mut runtime, &mut text, 600., 0.);
    let (x, y) = center(bounds(&runtime, "office-label"));
    let down = Event::PointerDown { x, y };
    let up = Event::PointerUp { x, y };
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    let hover = render(&mut runtime, &mut text, 600., 20.);
    assert!(quad(
        &hover,
        bounds(&runtime, "office-state-layer"),
        alpha(colors.on_surface, 20),
        20.
    ));
    runtime.dispatch(down.clone()).unwrap();
    let held = render(&mut runtime, &mut text, 600., 30.);
    assert!(quad(
        &held,
        bounds(&runtime, "office-state-layer"),
        alpha(colors.primary, 26),
        20.
    ));
    assert!(stroke(
        &held,
        bounds(&runtime, "office-focus-ring"),
        colors.secondary,
        3.
    ));
    assert_eq!(runtime.focused(), Some("office"));
    assert!(runtime.app().requests.is_empty());
    runtime
        .dispatch(Event::PointerUp { x: 1., y: 499. })
        .unwrap();
    assert!(runtime.app().requests.is_empty());

    runtime.dispatch_touch(down.clone()).unwrap();
    runtime.cancel_pointer().unwrap();
    runtime.dispatch_touch(up.clone()).unwrap();
    assert!(runtime.app().requests.is_empty());
    runtime.dispatch_touch(down.clone()).unwrap();
    runtime.dispatch_touch(up.clone()).unwrap();
    assert_eq!(runtime.app().requests, [2]);
    assert_eq!(runtime.app().selected, Some(2));

    render(&mut runtime, &mut text, 600., 700.);
    runtime.dispatch(down).unwrap();
    let checked_held = render(&mut runtime, &mut text, 600., 710.);
    assert!(quad(
        &checked_held,
        bounds(&runtime, "office-state-layer"),
        alpha(colors.on_surface, 26),
        20.
    ));
    runtime.dispatch(up).unwrap();
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().requests, [2]);
    assert_eq!(runtime.focused(), Some("office"));

    let (x, y) = center(bounds(&runtime, "inactive-ring"));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, [2]);
}

#[test]
fn one_tab_stop_enters_at_the_checked_or_directional_enabled_edge() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, &mut text, 600., 0.);
    for (outside, reverse, entry, opposite) in [
        ("before", false, "home", "after"),
        ("after", true, "other", "before"),
    ] {
        runtime.activate(outside).unwrap();
        tab(&mut runtime, reverse);
        assert_eq!(runtime.focused(), Some(entry));
        tab(&mut runtime, reverse);
        assert_eq!(runtime.focused(), Some(opposite));
    }
    assert!(runtime.app().requests.is_empty());
    runtime.apply(Message::Author(Some(2))).unwrap();
    for (outside, reverse) in [("before", false), ("after", true)] {
        runtime.activate(outside).unwrap();
        tab(&mut runtime, reverse);
        assert_eq!(runtime.focused(), Some("office"));
    }
    assert!(runtime.app().requests.is_empty());
    key(&mut runtime, EditKey::Up);
    assert_eq!(runtime.focused(), Some("home"));
    assert_eq!(runtime.app().selected, Some(0));
    key(&mut runtime, EditKey::Left);
    assert_eq!(runtime.focused(), Some("other"));
    assert_eq!(runtime.app().selected, Some(3));
    key(&mut runtime, EditKey::Right);
    assert_eq!(runtime.focused(), Some("home"));
    key(&mut runtime, EditKey::Down);
    assert_eq!(runtime.focused(), Some("office"));
    assert_eq!(runtime.app().requests, [0, 3, 0, 2]);
}

#[test]
fn controlled_activation_honors_projected_space_and_suppresses_repeat() {
    let mut runtime = Runtime::new(Model {
        accept: false,
        ..Model::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, &mut text, 600., 0.);
    tab(&mut runtime, false);
    tab(&mut runtime, false);
    assert_eq!(runtime.focused(), Some("home"));
    render(&mut runtime, &mut text, 600., 10.);
    for repeat in [false, true, true] {
        let frame = runtime.accessibility_frame().unwrap();
        let home = runtime.dom().by_id("home").unwrap();
        let target = frame.focus_target(home).unwrap();
        let press = KeyPress::new(Key::Space, Modifiers::default()).with_repeat(repeat);
        assert!(runtime
            .accessibility_key(&target, &press, Event::Activate)
            .unwrap());
        render(&mut runtime, &mut text, 600., 20.);
    }
    assert_eq!(runtime.app().requests, [0]);
    assert_eq!(runtime.app().selected, None);
    let frame = runtime.accessibility_frame().unwrap();
    let home = runtime.dom().by_id("home").unwrap();
    let target = frame.activation_target(home).unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().requests, [0, 0]);
    assert_eq!(runtime.app().selected, None);
}

#[test]
fn measured_dot_grows_and_retargets_without_altering_semantics_or_center() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let mut text = TextSystem::default();
    let primary = runtime.app().theme.colors.primary;
    render(&mut runtime, &mut text, 600., 0.);
    runtime.apply(Message::Author(Some(0))).unwrap();
    render(&mut runtime, &mut text, 600., 10.);
    assert_eq!(bounds(&runtime, "home-dot").width, 0.);
    let middle = render(&mut runtime, &mut text, 600., 50.);
    let mid = bounds(&runtime, "home-dot");
    let origin = center(bounds(&runtime, "home-ring"));
    assert!(mid.width > 0. && mid.width < 10.);
    let midpoint = center(mid);
    assert!((midpoint.0 - origin.0).abs() < 0.000_1);
    assert!((midpoint.1 - origin.1).abs() < 0.000_1);
    assert!(quad(&middle, mid, primary, mid.width / 2.));
    let home = runtime.dom().by_id("home").unwrap();
    assert_eq!(runtime.dom().get(home).unwrap().is_checked(), Some(true));
    runtime.apply(Message::Author(None)).unwrap();
    render(&mut runtime, &mut text, 600., 50.);
    assert_eq!(bounds(&runtime, "home-dot"), mid);
    render(&mut runtime, &mut text, 600., 800.);
    assert_eq!(bounds(&runtime, "home-dot").width, 0.);
    runtime.apply(Message::Author(Some(0))).unwrap();
    runtime.set_reduced_motion(true);
    render(&mut runtime, &mut text, 600., 810.);
    assert_eq!(bounds(&runtime, "home-dot").width, 10.);
}

#[test]
fn narrow_rtl_label_wraps_and_horizontal_group_mirrors_in_logical_arrow_order() {
    let mut runtime = Runtime::new(Model {
        theme: Theme::dark(),
        scale: 2.,
        direction: LayoutDirection::RightToLeft,
        orientation: RadioGroupLayout::Horizontal,
        selected: Some(0),
        ..Model::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, &mut text, 320., 0.);
    let (head, inactive) = (bounds(&runtime, "home"), bounds(&runtime, "inactive"));
    assert_eq!(head.y, inactive.y);
    assert!(head.x > inactive.x);
    let label = bounds(&runtime, "office-label");
    let art = bounds(&runtime, "office-ring");
    let row = bounds(&runtime, "office");
    assert!(label.height >= 96., "wrapped label: {label:?}");
    assert_eq!(label.intersect(row), label);
    assert!(art.x > label.x + label.width);
    assert_eq!((art.width, art.height), (20., 20.));
    assert!(row.x >= 0. && row.x + row.width <= 320.);
    runtime.activate("before").unwrap();
    tab(&mut runtime, false);
    assert_eq!(runtime.focused(), Some("home"));
    key(&mut runtime, EditKey::Left);
    assert_eq!(runtime.focused(), Some("office"));
    key(&mut runtime, EditKey::Right);
    assert_eq!(runtime.focused(), Some("home"));
    key(&mut runtime, EditKey::Down);
    assert_eq!(runtime.focused(), Some("office"));
    assert_eq!(runtime.app().requests, [2, 0, 2]);
}
