use surface::{
    Color, Corners, ExtendedDisplayList, ExtendedPaintOp, MicroDom, NodeId, PathFill, Rect, Role,
    TextMeasurer,
};
use surface_app::{AnimationFrame, AnimationUpdate, Application, Runtime, View};
use surface_material::{
    shape, LoadingIndicatorMotion, LoadingIndicatorOptions, LoadingKind, Theme,
};

type Build = Box<dyn Fn(&Theme) -> View<()>>;
struct App(bool, Build);
impl Application for App {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn view(&self) -> View<()> {
        let theme = if self.0 {
            Theme::dark()
        } else {
            Theme::light()
        };
        self.1(&theme)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn render(dark: bool, kind: LoadingKind, phase: f32) -> (Theme, Runtime<App>) {
    let build: Build = Box::new(move |t| t.loading_indicator("li", "Loading", kind, phase));
    let mut r = Runtime::new(App(dark, Box::new(move |t| t.page([build(t)])))).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn outer_and_child(r: &Runtime<App>) -> (NodeId, NodeId) {
    let dom: &MicroDom = r.dom();
    let outer = dom.by_id("li").expect("loading node");
    let child = dom.children(outer).next().expect("indicator child");
    (outer, child)
}

#[test]
fn sizes_match_the_m3_values() {
    for kind in [LoadingKind::Default, LoadingKind::Contained] {
        let (_, r) = render(false, kind, 0.);
        let (outer, child) = outer_and_child(&r);
        let computed = r.computed().unwrap();
        let o = computed.nodes[outer.index()].bounds;
        let c = computed.nodes[child.index()].bounds;
        assert_eq!((o.width, o.height), (48., 48.), "{kind:?} outer");
        assert_eq!((c.width, c.height), (38., 38.), "{kind:?} child");
    }
}

#[test]
fn role_is_progress_and_has_no_range() {
    let (_, r) = render(false, LoadingKind::Default, 0.);
    let (outer, _) = outer_and_child(&r);
    let node = r.dom().get(outer).unwrap();
    assert_eq!(node.role(), Role::Progress);
    assert!(node.range_value().is_none(), "indeterminate: no range");
    assert_eq!(node.name(), "Loading");
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, LoadingKind::Default, 0.);
        let (outer, child) = outer_and_child(&r);
        let dom = r.dom();
        assert_eq!(
            dom.get(child).unwrap().style().background,
            theme.colors.primary,
            "default indicator dark={dark}"
        );
        assert_eq!(
            dom.get(outer).unwrap().style().background,
            Color::TRANSPARENT,
            "default has no container dark={dark}"
        );

        let (theme, r) = render(dark, LoadingKind::Contained, 0.);
        let (outer, child) = outer_and_child(&r);
        let dom = r.dom();
        assert_eq!(
            dom.get(outer).unwrap().style().background,
            theme.colors.primary_container,
            "contained container dark={dark}"
        );
        assert_eq!(
            dom.get(child).unwrap().style().background,
            theme.colors.on_primary_container,
            "contained indicator dark={dark}"
        );
        assert_eq!(
            dom.get(outer).unwrap().style().corners,
            Some(shape::FULL),
            "contained container is round dark={dark}"
        );
    }
}

#[test]
fn phase_selects_the_shape_step() {
    let cases = [
        (0., 19.),
        (0.15, 12.),
        (0.3, 19.),
        (0.45, 8.),
        (0.99, 19.),
        (1.0, 19.),
        (-1., 19.),
        (f32::NAN, 19.),
        (2., 19.),
    ];
    for (phase, radius) in cases {
        let (_, r) = render(false, LoadingKind::Default, phase);
        let (_, child) = outer_and_child(&r);
        assert_eq!(
            r.dom().get(child).unwrap().style().corners,
            Some(Corners::all(radius)),
            "phase={phase}"
        );
    }
}

#[test]
fn phase_changes_the_output() {
    // Negative test: this fails if the application's phase is ignored.
    let (_, r0) = render(false, LoadingKind::Default, 0.);
    let (_, r1) = render(false, LoadingKind::Default, 0.15);
    let (_, c0) = outer_and_child(&r0);
    let (_, c1) = outer_and_child(&r1);
    assert_ne!(
        r0.dom().get(c0).unwrap().style().corners,
        r1.dom().get(c1).unwrap().style().corners
    );
}

struct AnimatedIndicator {
    motion: LoadingIndicatorMotion,
    active: bool,
    dark: bool,
    size: f32,
    kind: LoadingKind,
}

impl Application for AnimatedIndicator {
    type Message = bool;

    fn update(&mut self, active: bool) {
        self.active = active;
    }

    fn view(&self) -> View<bool> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        theme.page([theme.loading_indicator_with_motion(
            LoadingIndicatorOptions {
                id: "loading",
                name: "正在同步專案",
                kind: self.kind,
                size: self.size,
            },
            &self.motion,
        )])
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.motion.advance(&frame, self.active)
    }
}

fn animated(kind: LoadingKind, dark: bool, size: f32) -> Runtime<AnimatedIndicator> {
    Runtime::new(AnimatedIndicator {
        motion: LoadingIndicatorMotion::default(),
        active: true,
        dark,
        size,
        kind,
    })
    .unwrap()
}

fn tick(runtime: &mut Runtime<AnimatedIndicator>, clock: f64) -> ExtendedDisplayList {
    runtime.set_time(clock);
    render_animated(runtime)
}

fn render_animated(runtime: &mut Runtime<AnimatedIndicator>) -> ExtendedDisplayList {
    runtime.render_extended(320., 320., &mut Metrics).unwrap()
}

fn active_outline(frame: &ExtendedDisplayList) -> &PathFill {
    let mut outlines = frame.ops().iter().filter_map(|op| match op {
        ExtendedPaintOp::FillPath(path) => Some(path),
        _ => None,
    });
    let path = outlines.next().expect("real active contour");
    assert!(outlines.next().is_none(), "only one active contour");
    path
}

fn active_bounds(runtime: &Runtime<AnimatedIndicator>) -> Rect {
    let shape = runtime.dom().by_id("loading-shape").unwrap();
    runtime.computed().unwrap().nodes[shape.index()].bounds
}

fn close_enough(value: f32, reference: f32, tolerance: f32) {
    assert!(
        (value - reference).abs() < tolerance,
        "{value} differs from {reference} by more than {tolerance}"
    );
}

#[test]
fn expressive_timeline_keeps_the_shape_steps_and_spin_continuous() {
    let mut runtime = animated(LoadingKind::Default, false, 48.);
    let cases = [
        (0., 0, 0., 90.),
        (80., 0, 0.38755, 131.052),
        (200., 0, 1.01109, 196.429),
        (277., 0, 1.09478, 209.901),
        (297., 0, 1.09162, 211.160),
        (298., 0, 1.09129, 211.208),
        (350., 0, 1.04568, 211.115),
        (450., 0, 1., 214.719),
        (649., 0, 1., 230.073),
        (650., 1, 0., 230.150),
        (4549., 6, 1., 350.973),
        (4550., 0, 0., 351.050),
        (4665., 0, 0.62713, 56.364),
        (4666., 0, 0.63353, 57.018),
    ];
    let mut previous = None;
    for (time, pair, morph, rotation) in cases {
        let frame = tick(&mut runtime, 1000. + time);
        let pose = runtime.app().motion.pose();
        assert_eq!(pose.shape_index, pair, "{time} ms");
        close_enough(pose.morph, morph, 0.00001);
        close_enough(pose.rotation_degrees, rotation, 0.001);
        if let Some(previous) = previous.as_ref() {
            assert_ne!(
                active_outline(previous).points(),
                active_outline(&frame).points()
            );
        }
        previous = Some(frame);
        assert!(runtime.needs_frame());
        assert_eq!(runtime.app().motion.elapsed_ms(), time);
    }
}

#[test]
fn expressive_indicator_paints_the_palette_and_keeps_one_semantic_status() {
    for (size, clamped) in [(0., 24.), (48., 48.), (300., 240.), (f32::NAN, 48.)] {
        for kind in [LoadingKind::Default, LoadingKind::Contained] {
            for (dark, theme) in [(false, Theme::light()), (true, Theme::dark())] {
                let mut runtime = animated(kind, dark, size);
                let frame = render_animated(&mut runtime);
                let root = runtime.dom().by_id("loading").unwrap();
                let shape = runtime.dom().by_id("loading-shape").unwrap();
                let outer = runtime.computed().unwrap().nodes[root.index()].bounds;
                let inner = active_bounds(&runtime);
                let semantics = runtime.accessibility_frame().unwrap();
                let progress = semantics.node(root).unwrap().node;
                assert_eq!(progress.name(), "正在同步專案");
                assert_eq!(progress.role(), Role::Progress);
                assert!(progress.range_value().is_none());
                assert!(!semantics.accepts_focus(root));
                assert!(semantics.activation_target(root).is_none());
                assert!(semantics.children(root).next().is_none());
                assert!(semantics.node(shape).is_none());
                assert_eq!((outer.width, outer.height), (clamped, clamped));
                close_enough(inner.width, clamped * 38. / 48., 0.001);
                assert_eq!(inner.width, inner.height);
                close_enough(
                    inner.x + inner.width / 2.,
                    outer.x + outer.width / 2.,
                    0.001,
                );
                close_enough(
                    inner.y + inner.height / 2.,
                    outer.y + outer.height / 2.,
                    0.001,
                );
                assert_eq!(active_outline(&frame).clip(), outer);
                let (ink, container) = match kind {
                    LoadingKind::Default => (theme.colors.primary, Color::TRANSPARENT),
                    LoadingKind::Contained => (
                        theme.colors.on_primary_container,
                        theme.colors.primary_container,
                    ),
                };
                assert_eq!(active_outline(&frame).color(), ink);
                assert_eq!(progress.style().background, container);
                if kind == LoadingKind::Contained {
                    assert_eq!(progress.style().corners, Some(shape::FULL));
                }
                assert!(runtime.focused().is_none());
            }
        }
    }
}

#[test]
fn expressive_contour_stays_inside_a_fixed_box_through_each_pair() {
    let mut runtime = animated(LoadingKind::Contained, true, 240.);
    tick(&mut runtime, 0.);
    let bounds = active_bounds(&runtime);
    let root = runtime.dom().by_id("loading").unwrap();
    let outer = runtime.computed().unwrap().nodes[root.index()].bounds;
    let center_x = bounds.x + bounds.width / 2.;
    let center_y = bounds.y + bounds.height / 2.;
    let limit = bounds.width / 2. + 0.001;
    for pair in 0..7 {
        for time in [0., 40., 80., 140., 200., 277., 350., 450., 649.] {
            let frame = tick(&mut runtime, f64::from(pair) * 650. + time);
            assert_eq!(runtime.app().motion.pose().shape_index, pair as usize);
            assert_eq!(active_bounds(&runtime), bounds);
            let path = active_outline(&frame);
            assert_eq!(path.clip(), outer);
            assert!(path
                .points()
                .iter()
                .all(|[x, y]| { (x - center_x).hypot(y - center_y) <= limit }));
        }
    }
}

#[test]
fn expressive_paint_preserves_accessibility_until_a_view_rebuild() {
    let mut runtime = animated(LoadingKind::Default, false, 48.);
    let first = tick(&mut runtime, 1000.);
    let generation = runtime.accessibility_frame().unwrap().generation();
    let moving = tick(&mut runtime, 1080.);
    assert_ne!(
        active_outline(&first).points(),
        active_outline(&moving).points()
    );
    assert_eq!(
        runtime.accessibility_frame().unwrap().generation(),
        generation
    );

    runtime.set_reduced_motion(true);
    let reduced = tick(&mut runtime, 1080.);
    assert_eq!(active_outline(&first), active_outline(&reduced));
    assert_eq!(
        runtime.accessibility_frame().unwrap().generation(),
        generation
    );
    assert!(!runtime.needs_frame());

    runtime.set_reduced_motion(false);
    runtime.apply(false).unwrap();
    tick(&mut runtime, 1090.);
    let stopped = runtime.accessibility_frame().unwrap().generation();
    assert_ne!(
        stopped, generation,
        "an application message rebuilt the view"
    );

    runtime.apply(true).unwrap();
    let restarted = tick(&mut runtime, 2000.);
    let resumed = runtime.accessibility_frame().unwrap().generation();
    assert_ne!(resumed, stopped);
    assert_eq!(active_outline(&first), active_outline(&restarted));
    let moving_again = tick(&mut runtime, 2080.);
    assert_eq!(active_outline(&moving), active_outline(&moving_again));
    assert_eq!(runtime.accessibility_frame().unwrap().generation(), resumed);
}

#[test]
fn expressive_reduced_or_idle_indicator_stops_and_restarts_at_the_first_pose() {
    let mut runtime = animated(LoadingKind::Contained, false, 48.);
    let still = render_animated(&mut runtime);
    let initial_pose = runtime.app().motion.pose();
    assert!(!runtime.needs_frame(), "without a clock");
    assert_eq!(tick(&mut runtime, 1000.).ops(), still.ops());
    assert_ne!(tick(&mut runtime, 1080.).ops(), still.ops());
    assert!(runtime.needs_frame());

    runtime.set_reduced_motion(true);
    assert_eq!(tick(&mut runtime, 1080.).ops(), still.ops());
    assert_eq!(runtime.app().motion.elapsed_ms(), 0.);
    assert!(!runtime.needs_frame());
    assert_eq!(tick(&mut runtime, 5000.).ops(), still.ops());

    runtime.set_reduced_motion(false);
    assert_eq!(tick(&mut runtime, 5000.).ops(), still.ops());
    assert!(runtime.needs_frame());
    assert_ne!(tick(&mut runtime, 5080.).ops(), still.ops());
    runtime.apply(false).unwrap();
    assert_eq!(tick(&mut runtime, 5080.).ops(), still.ops());
    assert_eq!(runtime.app().motion.pose(), initial_pose);
    assert!(!runtime.needs_frame());
    runtime.apply(true).unwrap();
    assert_eq!(tick(&mut runtime, 9000.).ops(), still.ops());
    assert!(runtime.needs_frame());
}
