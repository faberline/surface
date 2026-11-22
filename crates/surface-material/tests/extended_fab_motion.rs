use surface::{ComputedDom, ComputedNode, Length, MicroDom, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, Event, FrameInput, Runtime, VerticalScroll, View,
};
use surface_material::{ExtendedFabMotion, ExtendedFabScroll};

const NO_INPUT: FrameInput<'static> = FrameInput {
    pointer: None,
    focused: None,
    touch: None,
    touch_cancelled: false,
};

fn tick(
    motion: &mut ExtendedFabMotion,
    time: f64,
    expanded: bool,
    visible: bool,
    reduced_motion: bool,
) -> AnimationUpdate {
    motion.advance(
        "fab",
        &AnimationFrame {
            now_ms: Some(time),
            reduced_motion,
            dom: &MicroDom::new(Node::new(Role::Window, "Test")),
            layout: None,
        },
        &NO_INPUT,
        None,
        expanded,
        visible,
    )
}

fn between(value: f32, low: f32, high: f32) {
    assert!(
        value > low && value < high,
        "{value} not between {low} and {high}"
    );
}

#[test]
fn size_and_label_use_separate_extend_and_shrink_timings() {
    let mut motion = ExtendedFabMotion::new(false, true);
    assert!(tick(&mut motion, 0., true, true, false).request_next);
    tick(&mut motion, 100., true, true, false);
    between(motion.expansion(), 0., 1.);
    assert_eq!(motion.label_opacity(), 0.);
    tick(&mut motion, 200., true, true, false);
    between(motion.label_opacity(), 0., 1.);
    tick(&mut motion, 300., true, true, false);
    between(motion.expansion(), 0., 1.);
    assert_eq!(motion.label_opacity(), 1.);
    tick(&mut motion, 400., true, true, false);
    assert_eq!(motion.expansion(), 1.);
    assert!(!tick(&mut motion, 401., true, true, false).request_next);

    tick(&mut motion, 500., false, true, false);
    tick(&mut motion, 550., false, true, false);
    between(motion.expansion(), 0., 1.);
    assert_eq!(motion.label_opacity(), 1.);
    tick(&mut motion, 600., false, true, false);
    between(motion.label_opacity(), 0., 1.);
    tick(&mut motion, 650., false, true, false);
    between(motion.expansion(), 0., 1.);
    assert_eq!(motion.label_opacity(), 0.);
    tick(&mut motion, 900., false, true, false);
    assert_eq!(motion.expansion(), 0.);
}

#[test]
fn reversing_reuses_the_displayed_width_and_label_and_ignores_a_regressing_clock() {
    let mut motion = ExtendedFabMotion::default();
    tick(&mut motion, 0., false, true, false);
    tick(&mut motion, 100., false, true, false);
    let width = motion.expansion();
    let label = motion.label_opacity();
    between(width, 0., 1.);
    between(label, 0., 1.);

    tick(&mut motion, 100., true, true, false);
    assert_eq!((motion.expansion(), motion.label_opacity()), (width, label));
    tick(&mut motion, 150., true, true, false);
    between(motion.expansion(), width, 1.);
    assert_eq!(motion.label_opacity(), label);
    let new_width = motion.expansion();
    tick(&mut motion, 120., true, true, false);
    assert_eq!(motion.expansion(), new_width);
    tick(&mut motion, 225., true, true, false);
    between(motion.label_opacity(), label, 1.);
    tick(&mut motion, 500., true, true, false);
    assert_eq!((motion.expansion(), motion.label_opacity()), (1., 1.));
}

#[test]
fn visibility_scale_and_opacity_reverse_without_a_jump() {
    let mut motion = ExtendedFabMotion::new(false, false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (0.4, 0.)
    );
    tick(&mut motion, 0., false, true, false);
    tick(&mut motion, 100., false, true, false);
    let scale = motion.visibility_scale();
    let opacity = motion.visibility_opacity();
    between(scale, 0.4, 1.);
    between(opacity, 0., 1.);
    tick(&mut motion, 100., false, false, false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (scale, opacity)
    );

    assert!(tick(&mut motion, 250., false, false, false).request_next);
    assert_eq!(motion.visibility_opacity(), 0.);
    between(motion.visibility_scale(), 0.4, scale);
    tick(&mut motion, 350., false, false, false);
    assert_eq!(motion.visibility_scale(), 0.4);
    tick(&mut motion, 400., false, true, false);
    tick(&mut motion, 899., false, true, false);
    between(motion.visibility_opacity(), 0., 1.);
    tick(&mut motion, 900., false, true, false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (1., 1.)
    );
}

#[test]
fn reduced_motion_and_invalid_time_choose_the_requested_endpoints() {
    let mut motion = ExtendedFabMotion::default();
    tick(&mut motion, 0., false, false, false);
    tick(&mut motion, 100., false, false, false);
    between(motion.expansion(), 0., 1.);
    tick(&mut motion, 100., false, false, true);
    assert_eq!((motion.expansion(), motion.label_opacity()), (0., 0.));
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (0.4, 0.)
    );
    tick(&mut motion, f64::NAN, true, true, false);
    assert_eq!((motion.expansion(), motion.label_opacity()), (1., 1.));
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (1., 1.)
    );
}

#[test]
fn measured_label_changes_schedule_new_geometry_while_the_action_is_hidden() {
    let mut motion = ExtendedFabMotion::new(false, false);
    let mut dom = MicroDom::new(Node::new(Role::Window, "Test"));
    let root = dom.root();
    let natural = dom
        .append(root, Node::new(Role::Text, "Label").id("fab-measure"))
        .unwrap();
    let wrapped = dom
        .append(
            root,
            Node::new(Role::Text, "Label").id("fab-measure-wrapped"),
        )
        .unwrap();
    let viewport = Rect::new(0., 0., 320., 240.);
    let mut layout = ComputedDom {
        viewport,
        nodes: [
            (root, viewport),
            (natural, Rect::new(-1024., 0., 177., 24.)),
            (wrapped, Rect::new(-1024., 0., 80., 48.)),
        ]
        .into_iter()
        .map(|(id, bounds)| ComputedNode {
            id,
            bounds,
            clip: viewport,
        })
        .collect(),
        paint_order: vec![root, natural, wrapped],
    };
    let step = |motion: &mut ExtendedFabMotion, layout: Option<&ComputedDom>| {
        motion.advance(
            "fab",
            &AnimationFrame {
                now_ms: Some(0.),
                reduced_motion: false,
                dom: &dom,
                layout,
            },
            &NO_INPUT,
            None,
            false,
            false,
        )
    };
    assert_eq!(
        step(&mut motion, None),
        AnimationUpdate {
            rebuild: false,
            request_next: true
        }
    );
    assert!(step(&mut motion, Some(&layout)).rebuild);
    assert_eq!(step(&mut motion, Some(&layout)), AnimationUpdate::default());
    layout.nodes[wrapped.index()].bounds.height = 72.;
    assert!(step(&mut motion, Some(&layout)).rebuild);
}

#[derive(Default)]
struct ScrollApp(ExtendedFabScroll);

impl Application for ScrollApp {
    type Message = ExtendedFabScroll;

    fn update(&mut self, state: ExtendedFabScroll) {
        self.0 = state;
    }

    fn view(&self) -> View<Self::Message> {
        let scroll = self.0;
        let clip = Style::column().height(Length::Px(80.)).clip(true);
        let content = Style::column().height(Length::Px(240.));
        View::new(Node::new(Role::Window, "Test")).children([View::new(
            Node::new(Role::Group, "Body").id("body").styled(clip),
        )
        .children([View::new(Node::new(Role::Group, "Content").styled(content))])
        .on_vertical_scroll(move |gesture| scroll.apply(gesture, false, |state| state))])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

#[test]
fn scroll_observer_keeps_the_real_body_delta_and_uses_the_applied_distance() {
    let mut runtime = Runtime::new(ScrollApp::default()).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    for (delta_y, offset, expanded) in [
        (-300., 0., true),
        (39., 39., true),
        (1., 40., false),
        (300., 160., false),
        (10., 160., false),
        (-1., 159., true),
        (300., 160., true),
        (100., 160., true),
    ] {
        assert!(runtime
            .dispatch(Event::Scroll {
                x: 12.,
                y: 12.,
                delta_x: 0.,
                delta_y
            })
            .unwrap());
        runtime.render(320., 240., &mut Metrics).unwrap();
        assert_eq!(runtime.scroll_metrics("body").unwrap().offset.1, offset);
        assert_eq!(runtime.app().0.expanded(), expanded);
    }
}

#[test]
fn text_only_or_focused_policy_can_keep_the_action_expanded_at_a_scroll_bound() {
    let gesture = VerticalScroll {
        delta: 20.,
        offset: 100.,
        max_offset: 100.,
    };
    let collapsed = ExtendedFabScroll::default().with_expanded(false);
    assert!(collapsed.apply(gesture, false, |state| state).is_none());
    let response = collapsed.apply(gesture, true, |state| state).unwrap();
    assert_eq!(response.consumed, 0.);
    assert!(response.message.expanded());
}
