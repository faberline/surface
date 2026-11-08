use surface::{Color, MicroDom, Role, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{shape, FabColor, FabSize, Theme};

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

struct Counter {
    count: usize,
    disabled: bool,
}
impl Application for Counter {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.count += 1;
    }
    fn view(&self) -> View<()> {
        let theme = Theme::light();
        theme.page([theme.fab(
            "fab",
            "Add",
            "+",
            FabSize::Regular,
            FabColor::Primary,
            self.disabled,
            (),
        )])
    }
}

fn render(dark: bool, size: FabSize, color: FabColor, disabled: bool) -> (Theme, Runtime<App>) {
    let build: Build = Box::new(move |t| t.fab("fab", "Add", "+", size, color, disabled, ()));
    let mut r = Runtime::new(App(dark, Box::new(move |t| t.page([build(t)])))).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn outer_and_child(r: &Runtime<App>) -> (surface::NodeId, surface::NodeId) {
    let dom: &MicroDom = r.dom();
    let id = dom.by_id("fab").expect("fab node");
    let child = dom.children(id).next().expect("visual child");
    (id, child)
}

#[test]
fn sizes_match_the_m3_containers() {
    let cases = [
        (FabSize::Small, 48., 40.),
        (FabSize::Regular, 56., 56.),
        (FabSize::Large, 96., 96.),
    ];
    for (size, outer, inner) in cases {
        let (_, r) = render(false, size, FabColor::Primary, false);
        let (id, child) = outer_and_child(&r);
        let computed = r.computed().unwrap();
        let o = computed.nodes[id.index()].bounds;
        let c = computed.nodes[child.index()].bounds;
        assert_eq!((o.width, o.height), (outer, outer), "outer {size:?}");
        assert_eq!((c.width, c.height), (inner, inner), "container {size:?}");
    }
}

#[test]
fn corners_follow_the_size() {
    let cases = [
        (FabSize::Small, shape::MEDIUM),
        (FabSize::Regular, shape::LARGE),
        (FabSize::Large, shape::EXTRA_LARGE),
    ];
    for (size, corners) in cases {
        let (_, r) = render(false, size, FabColor::Primary, false);
        let (_, child) = outer_and_child(&r);
        let dom = r.dom();
        assert_eq!(
            dom.get(child).unwrap().style().corners,
            Some(corners),
            "{size:?}"
        );
    }
}

#[test]
fn colors_use_the_container_tokens_in_both_themes() {
    for dark in [false, true] {
        // (background, foreground) token pairs from the spec.
        type Pair = fn(&Theme) -> (Color, Color);
        let cases: [(FabColor, Pair); 4] = [
            (FabColor::Surface, |t| {
                (t.colors.surface_container_high, t.colors.primary)
            }),
            (FabColor::Primary, |t| {
                (t.colors.primary_container, t.colors.on_primary_container)
            }),
            (FabColor::Secondary, |t| {
                (
                    t.colors.secondary_container,
                    t.colors.on_secondary_container,
                )
            }),
            (FabColor::Tertiary, |t| {
                (t.colors.tertiary_container, t.colors.on_tertiary_container)
            }),
        ];
        for (color, expected) in cases {
            let (theme, r) = render(dark, FabSize::Regular, color, false);
            let (_, child) = outer_and_child(&r);
            let style = r.dom().get(child).unwrap().style().clone();
            let (bg, _fg) = expected(&theme);
            assert_eq!(style.background, bg, "{color:?} dark={dark}");
            if color == FabColor::Primary {
                // Negative: the container must not be the bare primary color.
                assert_ne!(style.background, theme.primary, "dark={dark}");
            }
        }
    }
}

#[test]
fn disabled_uses_the_disabled_color() {
    let (theme, mut r) = render(false, FabSize::Regular, FabColor::Primary, true);
    let (_, child) = outer_and_child(&r);
    assert_eq!(
        r.dom().get(child).unwrap().style().background,
        theme.disabled
    );
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_ne!(r.focused(), Some("fab"));
}

#[test]
fn keyboard_activation_sends_the_message() {
    let mut r = Runtime::new(Counter {
        count: 0,
        disabled: false,
    })
    .unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("fab"));
    r.dispatch(Event::Activate).unwrap();
    assert_eq!(r.app().count, 1);
}

#[test]
fn disabled_keyboard_activation_sends_nothing() {
    let mut r = Runtime::new(Counter {
        count: 0,
        disabled: true,
    })
    .unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), None);
    // No node is focused, so Activate may return Err; the message count is the proof.
    let _ = r.dispatch(Event::Activate);
    assert_eq!(r.app().count, 0);
}

#[test]
fn exposes_a_button_role_and_the_label_as_its_name() {
    let (_, r) = render(false, FabSize::Regular, FabColor::Primary, false);
    let (id, _) = outer_and_child(&r);
    let node = r.dom().get(id).unwrap();
    assert_eq!(node.role(), Role::Button);
    assert_eq!(node.name(), "Add");
}
