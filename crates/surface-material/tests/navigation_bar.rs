use surface::{Color, Role, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{ChoiceState, Theme};

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
}
impl Application for Counter {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.count += 1;
    }
    fn view(&self) -> View<()> {
        let theme = Theme::light();
        theme.page([theme.navigation_bar("nav", "Destinations", items(&theme, 0))])
    }
}

/// Three destinations: `a` Home, `b` Search, `c` More (disabled).
fn items(t: &Theme, sel: usize) -> Vec<View<()>> {
    [("a", "Home", "H"), ("b", "Search", "S"), ("c", "More", "M")]
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, icon))| {
            t.navigation_bar_item(
                id,
                name,
                icon,
                ChoiceState {
                    selected: sel == i,
                    disabled: i == 2,
                },
                (),
            )
        })
        .collect()
}

fn render(dark: bool, sel: usize) -> (Theme, Runtime<App>) {
    let build: Build =
        Box::new(move |t| t.page([t.navigation_bar("nav", "Destinations", items(t, sel))]));
    let mut r = Runtime::new(App(dark, build)).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn bounds<A: Application>(r: &Runtime<A>, id: &str) -> surface::Rect {
    let computed = r.computed().unwrap();
    computed.nodes[r.dom().by_id(id).unwrap().index()].bounds
}

fn first_child(r: &Runtime<App>, id: &str) -> surface::NodeId {
    let node = r.dom().by_id(id).unwrap();
    r.dom().children(node).next().expect("visual child")
}

#[test]
fn bar_and_items_are_80_high() {
    let (_, r) = render(false, 0);
    assert_eq!(bounds(&r, "nav").height, 80.);
    for id in ["a", "b", "c"] {
        assert_eq!(bounds(&r, id).height, 80., "item {id}");
    }
}

#[test]
fn roles_and_state() {
    let (_, r) = render(false, 0);
    let nav = r.dom().get(r.dom().by_id("nav").unwrap()).unwrap();
    assert_eq!(nav.role(), Role::Navigation);
    for (id, name) in [("a", "Home"), ("b", "Search"), ("c", "More")] {
        let node = r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
        assert_eq!(node.role(), Role::Button, "{id}");
        assert_eq!(node.name(), name, "{id}");
    }
    let a = r.dom().get(r.dom().by_id("a").unwrap()).unwrap();
    let b = r.dom().get(r.dom().by_id("b").unwrap()).unwrap();
    let c = r.dom().get(r.dom().by_id("c").unwrap()).unwrap();
    assert_eq!(a.is_selected(), Some(true));
    assert_ne!(b.is_selected(), Some(true));
    assert!(c.is_disabled());
    assert!(!a.is_disabled());
}

#[test]
fn indicator_is_64_by_32_and_only_when_selected() {
    let (theme, r) = render(false, 0);
    let indicator_a = first_child(&r, "a");
    let rect = r.computed().unwrap().nodes[indicator_a.index()].bounds;
    assert_eq!((rect.width, rect.height), (64., 32.));
    assert_eq!(
        r.dom().get(indicator_a).unwrap().style().background,
        theme.colors.secondary_container
    );
    let indicator_b = first_child(&r, "b");
    assert_eq!(
        r.dom().get(indicator_b).unwrap().style().background,
        Color::TRANSPARENT
    );
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        let bar = r.dom().get(r.dom().by_id("nav").unwrap()).unwrap();
        assert_eq!(
            bar.style().background,
            theme.colors.surface_container,
            "bar dark={dark}"
        );
        let icon_of = |id: &str| {
            let indicator = first_child(&r, id);
            let icon = r.dom().children(indicator).next().expect("icon");
            r.dom().get(icon).unwrap().style().foreground
        };
        assert_eq!(
            icon_of("a"),
            theme.colors.on_secondary_container,
            "selected dark={dark}"
        );
        assert_eq!(
            icon_of("b"),
            theme.colors.on_surface_variant,
            "unselected dark={dark}"
        );
        assert_eq!(icon_of("c"), theme.outline, "disabled dark={dark}");
    }
}

#[test]
fn press_sends_the_message() {
    let mut r = Runtime::new(Counter { count: 0 }).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();

    // Read the bounds before any event: dispatch clears the computed layout.
    let b = bounds(&r, "b");
    let c = bounds(&r, "c");

    let (x, y) = (b.x + b.width / 2., b.y + b.height / 2.);
    r.dispatch(Event::PointerDown { x, y }).unwrap();
    r.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(r.app().count, 1);

    let (x, y) = (c.x + c.width / 2., c.y + c.height / 2.);
    r.dispatch(Event::PointerDown { x, y }).unwrap();
    r.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(r.app().count, 1, "disabled item must not send");

    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("a"));
    r.dispatch(Event::Activate).unwrap();
    assert_eq!(r.app().count, 2);
}

#[test]
fn selection_changes_the_output() {
    let (theme, r) = render(false, 0);
    assert_eq!(
        r.dom()
            .get(first_child(&r, "a"))
            .unwrap()
            .style()
            .background,
        theme.colors.secondary_container
    );
    assert_eq!(
        r.dom()
            .get(first_child(&r, "b"))
            .unwrap()
            .style()
            .background,
        Color::TRANSPARENT
    );

    let (theme, r) = render(false, 1);
    assert_eq!(
        r.dom()
            .get(first_child(&r, "b"))
            .unwrap()
            .style()
            .background,
        theme.colors.secondary_container
    );
    assert_eq!(
        r.dom()
            .get(first_child(&r, "a"))
            .unwrap()
            .style()
            .background,
        Color::TRANSPARENT
    );
}

#[test]
fn arrow_keys_move_focus() {
    let (_, mut r) = render(false, 0);
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("a"));

    r.dispatch(Event::Edit {
        key: EditKey::Right,
        extend: false,
        by_word: false,
    })
    .unwrap();
    assert_eq!(r.focused(), Some("b"));

    // Right again wraps past the disabled item `c` back to `a`.
    r.dispatch(Event::Edit {
        key: EditKey::Right,
        extend: false,
        by_word: false,
    })
    .unwrap();
    assert_eq!(r.focused(), Some("a"));

    r.dispatch(Event::Edit {
        key: EditKey::Left,
        extend: false,
        by_word: false,
    })
    .unwrap();
    assert_eq!(r.focused(), Some("b"));
}
