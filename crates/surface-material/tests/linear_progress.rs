use surface::{MicroDom, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{shape, Theme};

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

fn render(dark: bool, value: Option<f32>, phase: f32) -> (Theme, Runtime<App>) {
    let build: Build = Box::new(move |t| t.linear_progress("lp", "Loading", value, phase));
    let mut r = Runtime::new(App(dark, Box::new(move |t| t.page([build(t)])))).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

/// Returns (outer node, track, active bar) ids.
fn parts(r: &Runtime<App>) -> (surface::NodeId, surface::NodeId, surface::NodeId) {
    let dom: &MicroDom = r.dom();
    let id = dom.by_id("lp").expect("progress node");
    let track = dom.children(id).next().expect("track");
    let bar = dom.children(track).nth(1).expect("active bar");
    (id, track, bar)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn size_is_240_by_4() {
    for value in [Some(0.4), None] {
        let (_, r) = render(false, value, 0.);
        let (id, _, _) = parts(&r);
        let o = r.computed().unwrap().nodes[id.index()].bounds;
        assert_eq!((o.width, o.height), (240., 4.), "value {value:?}");
    }
}

#[test]
fn determinate_has_range_and_indeterminate_has_none() {
    let (_, r) = render(false, Some(0.4), 0.);
    let (id, _, _) = parts(&r);
    let node = r.dom().get(id).unwrap();
    assert_eq!(node.role(), Role::Progress);
    assert_eq!(node.name(), "Loading");
    let range = node.range_value().expect("determinate range");
    assert_eq!(range.min, 0.);
    assert_eq!(range.max, 1.);
    assert!((range.value - 0.4).abs() < 1e-6);
    assert!((range.step - 0.01).abs() < 1e-6);

    let (_, r) = render(false, None, 0.);
    let (id, _, _) = parts(&r);
    let node = r.dom().get(id).unwrap();
    assert_eq!(node.role(), Role::Progress);
    assert_eq!(node.name(), "Loading");
    assert!(node.range_value().is_none());
}

#[test]
fn determinate_bar_width_follows_value() {
    let cases = [
        (Some(0.25), 60.),
        (Some(0.75), 180.),
        (Some(2.0), 240.),
        (Some(-1.0), 0.),
        (Some(f32::NAN), 0.),
    ];
    for (value, width) in cases {
        let (_, r) = render(false, value, 0.);
        let (_, _, bar) = parts(&r);
        let b = r.computed().unwrap().nodes[bar.index()].bounds;
        assert!(near(b.width, width), "value {value:?}: width {}", b.width);
    }
}

#[test]
fn indeterminate_bar_moves_with_phase() {
    let cases = [(0., 0.), (0.5, 72.), (1., 144.), (2., 144.)];
    for (phase, left) in cases {
        let (_, r) = render(false, None, phase);
        let (_, track, bar) = parts(&r);
        let computed = r.computed().unwrap();
        let t = computed.nodes[track.index()].bounds;
        let b = computed.nodes[bar.index()].bounds;
        assert!(near(b.width, 96.), "phase {phase}: width {}", b.width);
        assert!(near(b.x - t.x, left), "phase {phase}: left {}", b.x - t.x);
    }
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, Some(0.5), 0.);
        let (id, track, bar) = parts(&r);
        assert_eq!(
            r.dom().get(id).unwrap().style().background,
            Style::default().background,
            "outer has no background dark={dark}"
        );
        let bg = r.dom().children(track).next().expect("track background");
        assert_eq!(
            r.dom().get(bg).unwrap().style().background,
            theme.colors.secondary_container,
            "track dark={dark}"
        );
        assert_eq!(
            r.dom().get(bar).unwrap().style().background,
            theme.primary,
            "bar dark={dark}"
        );
        assert_eq!(
            r.dom().get(bar).unwrap().style().corners,
            Some(shape::FULL),
            "bar corners dark={dark}"
        );
    }
}

#[test]
fn value_is_not_ignored() {
    let (_, r) = render(false, Some(0.25), 0.);
    let (_, _, bar) = parts(&r);
    let a = r.computed().unwrap().nodes[bar.index()].bounds.width;
    let (_, r) = render(false, Some(0.75), 0.);
    let (_, _, bar) = parts(&r);
    let b = r.computed().unwrap().nodes[bar.index()].bounds.width;
    assert!(!near(a, b), "0.25 and 0.75 gave the same width {a}");
}
