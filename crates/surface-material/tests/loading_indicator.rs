use surface::{Color, Corners, MicroDom, NodeId, Role, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{shape, LoadingKind, Theme};

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
