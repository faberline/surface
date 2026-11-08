use surface::{Color, MicroDom, Role, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{shape, IconButtonKind, Theme};

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
        theme.page([theme.icon_button(
            "ib",
            "Star",
            "★",
            IconButtonKind::Filled,
            None,
            self.disabled,
            (),
        )])
    }
}

fn render(
    dark: bool,
    kind: IconButtonKind,
    selected: Option<bool>,
    disabled: bool,
) -> (Theme, Runtime<App>) {
    let build: Build =
        Box::new(move |t| t.icon_button("ib", "Star", "★", kind, selected, disabled, ()));
    let mut r = Runtime::new(App(dark, Box::new(move |t| t.page([build(t)])))).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn outer_and_child(r: &Runtime<App>) -> (surface::NodeId, surface::NodeId) {
    let dom: &MicroDom = r.dom();
    let id = dom.by_id("ib").expect("icon button node");
    let child = dom.children(id).next().expect("visual child");
    (id, child)
}

#[test]
fn touch_area_is_48_and_container_is_40() {
    let (_, r) = render(false, IconButtonKind::Standard, None, false);
    let (id, child) = outer_and_child(&r);
    let computed = r.computed().unwrap();
    let o = computed.nodes[id.index()].bounds;
    let c = computed.nodes[child.index()].bounds;
    assert_eq!((o.width, o.height), (48., 48.));
    assert_eq!((c.width, c.height), (40., 40.));
    assert_eq!(
        r.dom().get(child).unwrap().style().corners,
        Some(shape::FULL)
    );
}

#[test]
fn variants_use_the_m3_container_colors_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, IconButtonKind::Standard, None, false);
        let (_, child) = outer_and_child(&r);
        let s = r.dom().get(child).unwrap().style();
        assert_eq!(s.background, Color::TRANSPARENT, "standard dark={dark}");

        let (theme2, r) = render(dark, IconButtonKind::Filled, None, false);
        let (_, child) = outer_and_child(&r);
        let s = r.dom().get(child).unwrap().style();
        assert_eq!(s.background, theme2.colors.primary, "filled dark={dark}");

        let (_, r) = render(dark, IconButtonKind::Tonal, None, false);
        let (_, child) = outer_and_child(&r);
        let s = r.dom().get(child).unwrap().style();
        assert_eq!(
            s.background, theme.colors.secondary_container,
            "tonal dark={dark}"
        );
        // NEGATIVE: Tonal must not use the Filled colors.
        assert_ne!(s.background, theme.colors.primary, "tonal dark={dark}");

        let (_, r) = render(dark, IconButtonKind::Outlined, None, false);
        let (_, child) = outer_and_child(&r);
        let s = r.dom().get(child).unwrap().style();
        assert_eq!(s.background, Color::TRANSPARENT, "outlined dark={dark}");
        assert_eq!(s.border_width, 1., "outlined dark={dark}");
        assert_eq!(s.border_color, theme.outline, "outlined dark={dark}");
    }
}

#[test]
fn toggle_changes_the_colors() {
    let (theme, r) = render(false, IconButtonKind::Filled, Some(false), false);
    let (_, child) = outer_and_child(&r);
    assert_eq!(
        r.dom().get(child).unwrap().style().background,
        theme.colors.surface_container_highest
    );

    let (theme, r) = render(false, IconButtonKind::Filled, Some(true), false);
    let (_, child) = outer_and_child(&r);
    assert_eq!(
        r.dom().get(child).unwrap().style().background,
        theme.colors.primary
    );

    let (theme, r) = render(false, IconButtonKind::Outlined, Some(true), false);
    let (_, child) = outer_and_child(&r);
    let s = r.dom().get(child).unwrap().style();
    assert_eq!(s.background, theme.colors.inverse_surface);
    assert_eq!(s.border_width, 0.);

    let (theme, r) = render(false, IconButtonKind::Standard, Some(true), false);
    let (_, child) = outer_and_child(&r);
    let label = r.dom().children(child).next().expect("label");
    assert_eq!(
        r.dom().get(label).unwrap().style().foreground,
        theme.colors.primary
    );
}

#[test]
fn toggle_state_reaches_the_node() {
    for (selected, expected) in [
        (Some(true), Some(true)),
        (Some(false), Some(false)),
        (None, None),
    ] {
        let (_, r) = render(false, IconButtonKind::Standard, selected, false);
        let (id, _) = outer_and_child(&r);
        assert_eq!(
            r.dom().get(id).unwrap().is_selected(),
            expected,
            "{selected:?}"
        );
    }
}

#[test]
fn disabled_uses_the_disabled_color_and_is_not_focusable() {
    let (theme, r) = render(false, IconButtonKind::Filled, None, true);
    let (id, child) = outer_and_child(&r);
    assert_eq!(
        r.dom().get(child).unwrap().style().background,
        theme.disabled
    );
    assert!(r.dom().get(id).unwrap().is_disabled());
    let mut r = r;
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_ne!(r.focused(), Some("ib"));
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
    assert_eq!(r.focused(), Some("ib"));
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
    let (_, r) = render(false, IconButtonKind::Standard, None, false);
    let (id, _) = outer_and_child(&r);
    let node = r.dom().get(id).unwrap();
    assert_eq!(node.role(), Role::Button);
    assert_eq!(node.name(), "Star");
}
