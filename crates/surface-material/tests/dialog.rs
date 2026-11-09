use surface::{Color, Role, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{shape, typography, ButtonKind, Theme};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

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

fn render_dialog(dark: bool, icon: Option<&'static str>, supporting: &'static str) -> Runtime<App> {
    let build = move |t: &Theme| {
        t.page([t.dialog_with(
            "d",
            icon,
            "Title",
            supporting,
            [],
            [t.button_with("ok", "OK", ButtonKind::Filled, false, ())],
            (),
        )])
    };
    let mut r = Runtime::new(App(dark, Box::new(build))).unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r
}

#[test]
fn container_and_scrim_use_the_m3_tokens() {
    for dark in [false, true] {
        let r = render_dialog(dark, None, "Body");
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let dom = r.dom();

        let panel = dom.get(dom.by_id("d").unwrap()).unwrap();
        assert_eq!(panel.role(), Role::Dialog);
        assert_eq!(panel.name(), "Title");
        assert_eq!(
            panel.style().background,
            theme.colors.surface_container_high,
            "container dark={dark}"
        );
        assert_eq!(panel.style().corners, Some(shape::EXTRA_LARGE));

        let sc = theme.colors.scrim;
        let layer = dom.get(dom.by_id("d-layer").unwrap()).unwrap();
        assert_eq!(
            layer.style().background,
            Color(sc.0, sc.1, sc.2, 82),
            "scrim dark={dark}"
        );
    }
    // Negative: the old container token must not be used by dialog_with.
    let r = render_dialog(false, None, "Body");
    let theme = Theme::light();
    let dom = r.dom();
    let panel = dom.get(dom.by_id("d").unwrap()).unwrap();
    assert_ne!(panel.style().background, theme.container);
}

#[test]
fn text_roles_follow_the_spec() {
    let r = render_dialog(false, None, "Body");
    let theme = Theme::light();
    let dom = r.dom();

    let headline = dom.get(dom.by_id("d-headline").unwrap()).unwrap();
    assert_eq!(headline.role(), Role::Heading);
    assert_eq!(headline.style().text_style(), typography::HEADLINE_SMALL);
    assert_eq!(headline.style().foreground, theme.colors.on_surface);

    let supporting = dom.get(dom.by_id("d-supporting").unwrap()).unwrap();
    assert_eq!(supporting.role(), Role::Text);
    assert_eq!(supporting.style().text_style(), typography::BODY_MEDIUM);
    assert_eq!(
        supporting.style().foreground,
        theme.colors.on_surface_variant
    );
}

#[test]
fn icon_and_supporting_text_are_optional() {
    let with_icon = render_dialog(false, Some("✎"), "Body");
    assert!(with_icon.dom().by_id("d-icon").is_some());

    let without_icon = render_dialog(false, None, "Body");
    assert!(without_icon.dom().by_id("d-icon").is_none());

    let no_supporting = render_dialog(false, None, "");
    assert!(no_supporting.dom().by_id("d-supporting").is_none());
}

#[test]
fn old_dialog_is_unchanged() {
    let build = |t: &Theme| {
        t.page([t.dialog(
            "o",
            "Old",
            [],
            [t.button_with("ok", "OK", ButtonKind::Filled, false, ())],
            (),
        )])
    };
    let mut r = Runtime::new(App(false, Box::new(build))).unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    let theme = Theme::light();
    let dom = r.dom();
    let panel = dom.get(dom.by_id("o").unwrap()).unwrap();
    assert_eq!(panel.style().background, theme.container);
}

struct Host {
    open: bool,
}
impl Application for Host {
    type Message = bool;
    fn update(&mut self, m: bool) {
        self.open = m;
    }
    fn view(&self) -> View<bool> {
        let t = Theme::light();
        let mut children = vec![t.button_with("opener", "Open", ButtonKind::Filled, false, true)];
        if self.open {
            children.push(t.dialog_with(
                "d",
                Some("✎"),
                "Title",
                "Body",
                [],
                [t.button_with("ok", "OK", ButtonKind::Filled, false, false)],
                false,
            ));
        }
        t.page(children)
    }
}

#[test]
fn dialog_traps_focus_and_dismisses() {
    let mut r = Runtime::new(Host { open: false }).unwrap();
    r.render(640., 480., &mut Metrics).unwrap();

    r.activate("opener").unwrap();
    assert!(r.app().open);
    assert_eq!(r.focused(), Some("ok"));

    // Focus stays inside the dialog.
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("ok"));

    // Escape closes it and restores focus to the opener.
    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().open);
    assert_eq!(r.focused(), Some("opener"));

    // Outside click (on the scrim) closes it.
    r.activate("opener").unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r.dispatch(Event::PointerDown { x: 1., y: 1. }).unwrap();
    r.dispatch(Event::PointerUp { x: 1., y: 1. }).unwrap();
    assert!(!r.app().open);
}
