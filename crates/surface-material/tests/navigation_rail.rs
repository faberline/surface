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
        theme.page([theme.navigation_rail("rail", "Destinations", items(&theme, 0))])
    }
}

/// Three destinations: `a` Home, `b` Search, `c` More (disabled).
fn items(t: &Theme, sel: usize) -> Vec<View<()>> {
    [("a", "Home", "H"), ("b", "Search", "S"), ("c", "More", "M")]
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, icon))| {
            t.navigation_rail_item(
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
        Box::new(move |t| t.page([t.navigation_rail("rail", "Destinations", items(t, sel))]));
    let mut r = Runtime::new(App(dark, build)).unwrap();
    r.render(400., 600., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn bounds<A: Application>(r: &Runtime<A>, id: &str) -> surface::Rect {
    let computed = r.computed().unwrap();
    computed.nodes[r.dom().by_id(id).unwrap().index()].bounds
}

fn first_child(r: &Runtime<App>, id: &str) -> surface::NodeId {
    let node = r.dom().by_id(id).unwrap();
    r.dom().children(node).next().expect("indicator")
}

#[test]
fn rail_is_80_wide_and_items_64_high() {
    let (_, r) = render(false, 0);
    assert_eq!(bounds(&r, "rail").width, 80.);
    for id in ["a", "b", "c"] {
        assert_eq!(bounds(&r, id).height, 64., "item {id}");
    }
}

#[test]
fn roles_and_state() {
    let (_, r) = render(false, 0);
    let rail = r.dom().get(r.dom().by_id("rail").unwrap()).unwrap();
    assert_eq!(rail.role(), Role::Navigation);
    for (id, name) in [("a", "Home"), ("b", "Search"), ("c", "More")] {
        let node = r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
        assert_eq!(node.role(), Role::Button, "{id}");
        assert_eq!(node.name(), name, "{id}");
    }
    let node = |id: &str| r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
    assert_eq!(node("a").is_selected(), Some(true));
    assert_eq!(node("b").is_selected(), Some(false));
    assert!(node("c").is_disabled());
    assert!(!node("a").is_disabled());
}

#[test]
fn indicator_is_56_by_32_and_only_when_selected() {
    let (theme, r) = render(false, 0);
    let indicator_a = first_child(&r, "a");
    let rect = r.computed().unwrap().nodes[indicator_a.index()].bounds;
    assert_eq!((rect.width, rect.height), (56., 32.));
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
    r.render(400., 600., &mut Metrics).unwrap();
    assert!(r.activate("b").unwrap());
    assert_eq!(r.app().count, 1);
    // A disabled destination sends nothing; the call may fail or report false.
    let _ = r.activate("c");
    assert_eq!(r.app().count, 1);
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

    let down = Event::Edit {
        key: EditKey::Down,
        extend: false,
        by_word: false,
    };
    r.dispatch(down.clone()).unwrap();
    assert_eq!(r.focused(), Some("b"));

    // Down again skips the disabled `c` and wraps to `a`.
    r.dispatch(down).unwrap();
    assert_eq!(r.focused(), Some("a"));
}
