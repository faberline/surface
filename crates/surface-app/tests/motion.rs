use std::cell::Cell;
use std::rc::Rc;

use surface::{
    Color, DisplayList, Easing, Insets, Length, Node, PaintOp, Role, Style, TextMeasurer,
    Transition,
};
use surface_app::{Application, DomExport, Runtime, View};

const FROM: Color = Color::rgb(0, 0, 0);
const TO: Color = Color::rgb(200, 100, 60);

#[derive(Clone)]
enum Message {
    Toggle,
    Open,
    Close,
}

struct App {
    dark: bool,
    modal: bool,
    transition: Option<Transition>,
    views: Rc<Cell<usize>>,
}

fn app(transition: Option<Transition>) -> App {
    App {
        dark: false,
        modal: false,
        transition,
        views: Rc::new(Cell::new(0)),
    }
}

fn linear() -> Option<Transition> {
    Some(Transition::new(100., Easing::Linear))
}

fn button(id: &str, text: &str, message: Message) -> View<Message> {
    View::new(
        Node::new(Role::Button, text)
            .id(id)
            .text(text)
            .styled(Style::default().height(Length::Px(40.))),
    )
    .on_press(message)
}

impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.dark = !self.dark,
            Message::Open => self.modal = true,
            Message::Close => self.modal = false,
        }
    }
    fn view(&self) -> View<Message> {
        self.views.set(self.views.get() + 1);
        let mut box_style = Style::default()
            .width(Length::Px(100.))
            .height(Length::Px(40.))
            .background(if self.dark { TO } else { FROM });
        box_style.transition = self.transition;
        let mut children = vec![
            View::new(Node::new(Role::Group, "box").id("box").styled(box_style)),
            button("toggle", "Toggle", Message::Toggle),
            button("open", "Open", Message::Open),
        ];
        if self.modal {
            let mut dialog = Style::column()
                .height(Length::Px(80.))
                .fixed(Insets::all(0.))
                .background(TO);
            dialog.transition = self.transition;
            children.push(
                View::new(
                    Node::new(Role::Dialog, "Dialog")
                        .id("dialog")
                        .styled(dialog),
                )
                .modal(Message::Close),
            );
        }
        View::new(
            Node::new(Role::Window, "Motion test")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children(children)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size * 1.5)
    }
}

fn frame(runtime: &mut Runtime<App>) -> DisplayList {
    runtime.render(320., 240., &mut Metrics).unwrap()
}

/// The background quad painted at the node's laid-out bounds.
fn color(runtime: &Runtime<App>, list: &DisplayList, id: &str) -> Color {
    let node = runtime.dom().by_id(id).expect("node exists");
    let bounds = &runtime.computed().expect("frame rendered").nodes[node.index()].bounds;
    list.ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Quad { rect, color, .. } if rect == bounds => Some(*color),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no background quad for {id}"))
}

#[test]
fn without_clock_changes_apply_immediately_and_need_no_frame() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    assert!(!runtime.needs_frame());
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);
    assert!(!runtime.needs_frame());
}

#[test]
fn linear_transition_interpolates_with_the_host_clock() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(1000.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    let list = frame(&mut runtime);
    assert_eq!(
        color(&runtime, &list, "box"),
        FROM,
        "a transition starts at the presented value"
    );
    assert!(runtime.needs_frame());
    runtime.set_time(1050.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(100, 50, 30, 255));
    assert!(runtime.needs_frame());
    runtime.set_time(1100.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);
    assert!(!runtime.needs_frame());
}

#[test]
fn interrupted_transition_retargets_from_the_presented_value() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(100, 50, 30, 255));
    runtime.activate("toggle").unwrap();
    let list = frame(&mut runtime);
    assert_eq!(
        color(&runtime, &list, "box"),
        Color(100, 50, 30, 255),
        "retargeting must not jump"
    );
    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(50, 25, 15, 255));
    runtime.set_time(150.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), FROM);
    assert!(!runtime.needs_frame());
}

#[test]
fn time_never_moves_backwards() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(100, 50, 30, 255));
    for ignored in [25., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        runtime.set_time(ignored);
        let list = frame(&mut runtime);
        assert_eq!(
            color(&runtime, &list, "box"),
            Color(100, 50, 30, 255),
            "{ignored} is ignored"
        );
        assert!(runtime.needs_frame(), "{ignored} is ignored");
    }
    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);
    assert!(!runtime.needs_frame());
}

#[test]
fn reduced_motion_applies_targets_immediately() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_reduced_motion(true);
    assert!(runtime.reduced_motion());
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    assert!(!runtime.needs_frame());
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);
    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);

    // Reduced motion switched on in the middle of a transition.
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(100, 50, 30, 255));
    assert!(runtime.needs_frame());
    runtime.set_reduced_motion(true);
    assert!(!runtime.needs_frame(), "reduced motion asks for no frames");
    let list = frame(&mut runtime);
    assert_eq!(
        color(&runtime, &list, "box"),
        TO,
        "a running transition jumps to its target"
    );
    assert!(!runtime.needs_frame());
    runtime.set_reduced_motion(false);
    assert!(!runtime.reduced_motion());
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert_eq!(
        color(&runtime, &list, "box"),
        Color(100, 50, 30, 255),
        "motion resumes when reduced motion is turned off"
    );
}

#[test]
fn zero_duration_and_missing_transition_apply_immediately() {
    for transition in [None, Some(Transition::new(0., Easing::Linear))] {
        let mut runtime = Runtime::new(app(transition)).unwrap();
        runtime.set_time(0.);
        frame(&mut runtime);
        runtime.activate("toggle").unwrap();
        assert!(!runtime.needs_frame(), "{transition:?}");
        let list = frame(&mut runtime);
        assert_eq!(color(&runtime, &list, "box"), TO, "{transition:?}");
    }
}

#[test]
fn first_appearance_does_not_animate() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("open").unwrap();
    assert!(!runtime.needs_frame());
    let list = frame(&mut runtime);
    assert_eq!(
        color(&runtime, &list, "dialog"),
        TO,
        "a new element starts at its target"
    );
    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "dialog"), TO);
    assert!(!runtime.needs_frame());
}

#[test]
fn export_dom_reports_target_styles_mid_transition() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(100, 50, 30, 255));
    let mid = runtime.export_dom().unwrap();
    let export: DomExport = serde_json::from_str(&mid).unwrap();
    let node = export
        .dom
        .nodes
        .iter()
        .find(|n| n.stable_id() == Some("box"))
        .unwrap();
    assert_eq!(
        node.style().background,
        TO,
        "export reports the target, not the presented value"
    );
    assert!(runtime.needs_frame());
    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);
    assert_eq!(
        runtime.export_dom().unwrap(),
        mid,
        "the export does not change while a transition runs"
    );
}

#[test]
fn modal_opening_mid_transition_keeps_animating() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), Color(100, 50, 30, 255));
    runtime.activate("open").unwrap();
    runtime.set_time(75.);
    let list = frame(&mut runtime);
    assert_eq!(
        color(&runtime, &list, "box"),
        Color(150, 75, 45, 255),
        "a fixed layer must not reset the running transition"
    );
    assert!(runtime.needs_frame());
    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert_eq!(color(&runtime, &list, "box"), TO);
    assert!(!runtime.needs_frame());
}

#[test]
fn view_is_not_called_by_time_or_render() {
    let mut runtime = Runtime::new(app(linear())).unwrap();
    let after_new = runtime.app().views.get();
    runtime.set_time(0.);
    frame(&mut runtime);
    runtime.set_time(100.);
    frame(&mut runtime);
    runtime.set_time(200.);
    frame(&mut runtime);
    assert_eq!(
        runtime.app().views.get(),
        after_new,
        "time and render must not rebuild the view"
    );
    runtime.activate("toggle").unwrap();
    assert_eq!(
        runtime.app().views.get(),
        after_new + 1,
        "a message rebuilds the view once"
    );
}

// Corner motion. The tests below drive Style::corners and read what is painted.

use surface::Corners;

#[derive(Clone, Copy)]
struct Spec {
    radius: f32,
    corners: Option<Corners>,
    width: f32,
    height: f32,
}

impl Spec {
    fn corners(corners: Corners, width: f32, height: f32) -> Self {
        Self {
            radius: 0.,
            corners: Some(corners),
            width,
            height,
        }
    }
}

struct CornerApp {
    on: bool,
    a: Spec,
    b: Spec,
    transition: Option<Transition>,
    // Background colors for the two states.
    colors: (Color, Color),
}

impl Application for CornerApp {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.on = !self.on;
    }
    fn view(&self) -> View<()> {
        let spec = if self.on { self.b } else { self.a };
        let mut style = Style::default()
            .width(Length::Px(spec.width))
            .height(Length::Px(spec.height))
            .background(if self.on {
                self.colors.1
            } else {
                self.colors.0
            });
        style.radius = spec.radius;
        style.corners = spec.corners;
        style.transition = self.transition;
        View::new(
            Node::new(Role::Window, "Corner test")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children([
            View::new(Node::new(Role::Group, "box").id("box").styled(style)),
            View::new(
                Node::new(Role::Button, "Toggle")
                    .id("toggle")
                    .text("Toggle")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(()),
        ])
    }
}

fn corner_app(a: Spec, b: Spec, transition: Option<Transition>) -> Runtime<CornerApp> {
    Runtime::new(CornerApp {
        on: false,
        a,
        b,
        transition,
        // Equal colors, so only corners can start a run.
        colors: (FROM, FROM),
    })
    .unwrap()
}

fn lin200() -> Option<Transition> {
    Some(Transition::new(200., Easing::Linear))
}

fn corner_frame(runtime: &mut Runtime<CornerApp>) -> DisplayList {
    runtime.render(320., 240., &mut Metrics).unwrap()
}

/// The four corners painted for the box. A uniform shape may be a Quad.
fn painted(runtime: &Runtime<CornerApp>, list: &DisplayList) -> Corners {
    let node = runtime.dom().by_id("box").expect("box exists");
    let bounds = &runtime.computed().expect("frame rendered").nodes[node.index()].bounds;
    list.ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Shape { rect, corners, .. } if rect == bounds => Some(*corners),
            PaintOp::Quad { rect, radius, .. } if rect == bounds => Some(Corners::all(*radius)),
            _ => None,
        })
        .expect("no background shape for the box")
}

fn all_of(c: Corners) -> [f32; 4] {
    [c.top_left, c.top_right, c.bottom_right, c.bottom_left]
}

/// Start a run at t=0: render the first state, toggle, render again.
fn start(runtime: &mut Runtime<CornerApp>) {
    runtime.set_time(0.);
    corner_frame(runtime);
    runtime.activate("toggle").unwrap();
    corner_frame(runtime);
}

fn at(runtime: &mut Runtime<CornerApp>, t: f64) -> Corners {
    runtime.set_time(t);
    let list = corner_frame(runtime);
    painted(runtime, &list)
}

#[test]
fn c1_uniform_corners_interpolate_linearly() {
    let spec = |r| Spec::corners(Corners::all(r), 100., 100.);
    let mut runtime = corner_app(spec(4.), spec(16.), lin200());
    start(&mut runtime);
    assert_eq!(at(&mut runtime, 0.), Corners::all(4.), "starts shown");
    assert_eq!(at(&mut runtime, 100.), Corners::all(10.));
    assert!(runtime.needs_frame(), "a corner run asks for frames");
    assert_eq!(at(&mut runtime, 200.), Corners::all(16.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c2_full_radius_interpolates_toward_the_resolved_value() {
    let a = Spec::corners(Corners::all(4.), 120., 40.);
    let b = Spec::corners(Corners::full(), 120., 40.);
    let mut runtime = corner_app(a, b, lin200());
    start(&mut runtime);
    assert_eq!(at(&mut runtime, 100.), Corners::all(12.));
    for t in (0..=200).step_by(10) {
        let corners = at(&mut runtime, f64::from(t));
        for r in all_of(corners) {
            assert!(r.is_finite() && (0. ..=20.).contains(&r), "t={t} r={r}");
        }
    }
    assert_eq!(at(&mut runtime, 200.), Corners::all(20.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c3_overshooting_easing_stays_finite_and_in_range() {
    let a = Spec::corners(Corners::all(4.), 120., 40.);
    let b = Spec::corners(Corners::full(), 120., 40.);
    let easing = Easing::CubicBezier(0.3, 1.6, 0.7, 1.6);
    let mut runtime = Runtime::new(CornerApp {
        on: false,
        a,
        b,
        transition: Some(Transition::new(200., easing)),
        colors: (Color::rgb(0, 0, 0), Color::rgb(255, 255, 255)),
    })
    .unwrap();
    start(&mut runtime);
    for t in (0..=200).step_by(5) {
        let corners = at(&mut runtime, f64::from(t));
        for r in all_of(corners) {
            assert!(r.is_finite() && (0. ..=20.).contains(&r), "t={t} r={r}");
        }
        let list = corner_frame(&mut runtime);
        let node = runtime.dom().by_id("box").unwrap();
        let bounds = &runtime.computed().unwrap().nodes[node.index()].bounds;
        let color = list
            .ops
            .iter()
            .find_map(|op| match op {
                PaintOp::Shape { rect, color, .. } | PaintOp::Quad { rect, color, .. }
                    if rect == bounds =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .unwrap();
        assert!(color.0 == color.1 && color.1 == color.2, "t={t} {color:?}");
    }
    let corners = at(&mut runtime, 200.);
    assert_eq!(corners, Corners::all(20.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c4_constant_full_radius_follows_a_resize_without_a_run() {
    let a = Spec::corners(Corners::full(), 100., 40.);
    let b = Spec::corners(Corners::full(), 200., 40.);
    let mut runtime = corner_app(a, b, lin200());
    start(&mut runtime);
    assert!(!runtime.needs_frame(), "a resize is not a corner change");
    assert_eq!(at(&mut runtime, 0.), Corners::all(20.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c5_radius_to_equal_explicit_corners_starts_no_run() {
    let a = Spec {
        radius: 8.,
        corners: None,
        width: 100.,
        height: 100.,
    };
    let b = Spec {
        radius: 8.,
        ..Spec::corners(Corners::all(8.), 100., 100.)
    };
    let mut runtime = corner_app(a, b, lin200());
    start(&mut runtime);
    assert!(!runtime.needs_frame());
    assert_eq!(at(&mut runtime, 0.), Corners::all(8.));
    assert_eq!(at(&mut runtime, 100.), Corners::all(8.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c6_top_to_bottom_passes_through_a_uniform_shape() {
    let a = Spec::corners(Corners::top(16.), 100., 100.);
    let b = Spec::corners(Corners::bottom(16.), 100., 100.);
    let mut runtime = corner_app(a, b, lin200());
    start(&mut runtime);
    assert_eq!(at(&mut runtime, 0.), Corners::top(16.));
    assert_eq!(at(&mut runtime, 100.), Corners::all(8.));
    assert_eq!(at(&mut runtime, 200.), Corners::bottom(16.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c7_export_dom_keeps_the_target_corners_mid_run() {
    let spec = |r| Spec::corners(Corners::all(r), 100., 100.);
    let mut runtime = corner_app(spec(4.), spec(16.), lin200());
    start(&mut runtime);
    at(&mut runtime, 100.);
    let export: DomExport = serde_json::from_str(&runtime.export_dom().unwrap()).unwrap();
    let node = export
        .dom
        .nodes
        .iter()
        .find(|n| n.stable_id() == Some("box"))
        .unwrap();
    assert_eq!(node.style().corners, Some(Corners::all(16.)));
}

#[test]
fn c8_reduced_motion_jumps_corners_to_the_target() {
    let spec = |r| Spec::corners(Corners::all(r), 100., 100.);
    let mut runtime = corner_app(spec(4.), spec(16.), lin200());
    runtime.set_reduced_motion(true);
    start(&mut runtime);
    assert!(!runtime.needs_frame());
    assert_eq!(at(&mut runtime, 0.), Corners::all(16.));
    assert_eq!(at(&mut runtime, 100.), Corners::all(16.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c9_retargeting_a_corner_run_starts_from_the_shown_corners() {
    let spec = |r| Spec::corners(Corners::all(r), 100., 100.);
    let mut runtime = corner_app(spec(4.), spec(16.), lin200());
    start(&mut runtime);
    assert_eq!(at(&mut runtime, 100.), Corners::all(10.));
    runtime.activate("toggle").unwrap();
    assert_eq!(
        at(&mut runtime, 100.),
        Corners::all(10.),
        "retargeting must not jump"
    );
    assert_eq!(at(&mut runtime, 200.), Corners::all(7.));
    assert_eq!(at(&mut runtime, 300.), Corners::all(4.));
    assert!(!runtime.needs_frame());
}

#[test]
fn c10_color_run_keeps_a_legacy_radius_unresolved() {
    // Radius 100 is wider than half of the 40 x 40 box. A legacy node paints
    // it as is. A color run must not turn it into resolved corners.
    let spec = Spec {
        radius: 100.,
        corners: None,
        width: 40.,
        height: 40.,
    };
    let mut runtime = Runtime::new(CornerApp {
        on: false,
        a: spec,
        b: spec,
        transition: lin200(),
        colors: (FROM, TO),
    })
    .unwrap();
    start(&mut runtime);
    assert_eq!(at(&mut runtime, 100.), Corners::all(100.));
    assert!(runtime.needs_frame(), "the color run is active");
}

#[test]
fn c11_reduced_motion_settles_the_corner_target() {
    let spec = |r| Spec::corners(Corners::all(r), 100., 100.);
    let mut runtime = corner_app(spec(4.), spec(16.), lin200());
    runtime.set_reduced_motion(true);
    start(&mut runtime);
    runtime.set_reduced_motion(false);
    assert_eq!(at(&mut runtime, 50.), Corners::all(16.));
    assert!(
        !runtime.needs_frame(),
        "turning reduced motion off must not replay the change"
    );
}

struct KeyApp {
    on: bool,
}

impl Application for KeyApp {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.on = !self.on;
    }
    fn view(&self) -> View<()> {
        let bg = if self.on { TO } else { FROM };
        let boxed = |side: f32| {
            Style::default()
                .width(Length::Px(side))
                .height(Length::Px(side))
                .background(bg)
                .transition(Transition::new(200., Easing::Linear))
        };
        View::new(
            Node::new(Role::Window, "Keys")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children([
            // The unstable child of "save" has the path key "save" + "0".
            View::new(
                Node::new(Role::Group, "save")
                    .id("save")
                    .styled(Style::column()),
            )
            .children([View::new(
                Node::new(Role::Group, "inner").styled(boxed(30.)),
            )]),
            // A stable ID that looks like that path.
            View::new(
                Node::new(Role::Group, "twin")
                    .id("save/0")
                    .styled(boxed(50.)),
            ),
            View::new(
                Node::new(Role::Button, "Toggle")
                    .id("toggle")
                    .text("Toggle")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(()),
        ])
    }
}

#[test]
fn m2_a_stable_id_that_looks_like_a_child_path_keeps_its_own_track() {
    let mut runtime = Runtime::new(KeyApp { on: false }).unwrap();
    runtime.set_time(0.);
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime.activate("toggle").unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime.set_time(100.);
    let list = runtime.render(320., 240., &mut Metrics).unwrap();
    let save = runtime.dom().by_id("save").unwrap();
    let inner = runtime.dom().children(save).next().unwrap();
    let twin = runtime.dom().by_id("save/0").unwrap();
    let nodes = &runtime.computed().unwrap().nodes;
    let painted = |id: surface::NodeId| {
        let bounds = &nodes[id.index()].bounds;
        list.ops
            .iter()
            .find_map(|op| match op {
                PaintOp::Quad { rect, color, .. } if rect == bounds => Some(*color),
                _ => None,
            })
            .expect("background quad")
    };
    let mid = Color::rgb(100, 50, 30);
    assert_eq!(painted(inner), mid, "child of save");
    assert_eq!(painted(twin), mid, "stable id save/0");
}

/// Pins the first-change limit from both sides. `other` is a node that keeps
/// a transition before the update; without it motion holds no state.
struct Late {
    on: bool,
    other: bool,
}

impl Application for Late {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.on = true;
    }
    fn view(&self) -> View<()> {
        let mut style = Style::default()
            .width(Length::Px(100.))
            .height(Length::Px(40.))
            .background(if self.on { TO } else { FROM });
        if self.on {
            style.transition = linear();
        }
        let mut other = Style::default().height(Length::Px(40.));
        if self.other {
            other.transition = linear();
        }
        View::new(
            Node::new(Role::Window, "Late")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children(vec![
            View::new(Node::new(Role::Group, "box").id("box").styled(style)),
            View::new(Node::new(Role::Group, "other").id("other").styled(other)),
            View::new(
                Node::new(Role::Button, "Go")
                    .id("go")
                    .text("Go")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(()),
        ])
    }
}

fn late_color(runtime: &Runtime<Late>, list: &DisplayList) -> Color {
    let node = runtime.dom().by_id("box").expect("node exists");
    let bounds = &runtime.computed().expect("frame rendered").nodes[node.index()].bounds;
    list.ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Quad { rect, color, .. } if rect == bounds => Some(*color),
            _ => None,
        })
        .expect("background quad")
}

fn late_run(other: bool) -> (Color, Color) {
    let mut runtime = Runtime::new(Late { on: false, other }).unwrap();
    runtime.set_time(0.);
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime.activate("go").unwrap();
    let first = runtime.render(320., 240., &mut Metrics).unwrap();
    let first = late_color(&runtime, &first);
    runtime.set_time(50.);
    let mid = runtime.render(320., 240., &mut Metrics).unwrap();
    (first, late_color(&runtime, &mid))
}

#[test]
fn a_transition_added_with_the_first_change_does_not_animate_without_other_transitions() {
    let (first, half) = late_run(false);
    assert_eq!(first, TO, "no state was kept, so the target shows at once");
    assert_eq!(half, TO);
}

#[test]
fn a_transition_added_with_the_first_change_animates_when_another_node_has_one() {
    let (first, half) = late_run(true);
    assert_eq!(first, FROM);
    assert_eq!(half, Color(100, 50, 30, 255));
}
