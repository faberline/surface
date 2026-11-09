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

/// Three items: `a` Cut (leading `X`, trailing `Ctrl+X`), `b` Copy,
/// `c` Paste (disabled). `selected` is the index of the selected item.
fn items<M: Clone>(t: &Theme, selected: usize, message: impl Fn(usize) -> M) -> Vec<View<M>> {
    [
        ("a", "Cut", Some("X"), Some("Ctrl+X")),
        ("b", "Copy", None, None),
        ("c", "Paste", None, None),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, text, leading, trailing))| {
        t.menu_item_with(
            id,
            text,
            leading,
            trailing,
            ChoiceState {
                selected: selected == i,
                disabled: i == 2,
            },
            message(i),
        )
    })
    .collect()
}

fn render(dark: bool, selected: usize) -> (Theme, Runtime<App>) {
    let build: Build = Box::new(move |t| {
        t.page([t.menu_at("menu", "Menu", (20., 20.), items(t, selected, |_| ()), ())])
    });
    let mut r = Runtime::new(App(dark, build)).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn bounds<A: Application>(r: &Runtime<A>, id: &str) -> surface::Rect {
    let computed = r.computed().unwrap();
    computed.nodes[r.dom().by_id(id).unwrap().index()].bounds
}

fn background(r: &Runtime<App>, id: &str) -> Color {
    let node = r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
    node.style().background
}

fn child_names(r: &Runtime<App>, id: &str) -> Vec<String> {
    let item = r.dom().by_id(id).unwrap();
    r.dom()
        .children(item)
        .map(|c| r.dom().get(c).unwrap().name().to_string())
        .collect()
}

/// Foreground of the child whose name is `text` (a label inside the item).
fn text_foreground(r: &Runtime<App>, id: &str, text: &str) -> Color {
    let item = r.dom().by_id(id).unwrap();
    let child = r
        .dom()
        .children(item)
        .find(|c| r.dom().get(*c).unwrap().name() == text)
        .unwrap_or_else(|| panic!("no child {text:?} in {id}"));
    r.dom().get(child).unwrap().style().foreground
}

#[test]
fn item_is_48_high_and_fills_the_menu_width() {
    let (_, r) = render(false, 0);
    for id in ["a", "b", "c"] {
        assert_eq!(bounds(&r, id).height, 48., "item {id}");
    }
    // The menu has 8 dp padding on each side.
    assert_eq!(bounds(&r, "a").width, bounds(&r, "menu").width - 16.);
}

#[test]
fn roles_names_and_state() {
    let (_, r) = render(false, 0);
    let node = |id: &str| r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
    for (id, name) in [("a", "Cut"), ("b", "Copy"), ("c", "Paste")] {
        assert_eq!(node(id).role(), Role::MenuItem, "{id}");
        assert_eq!(node(id).name(), name, "{id}");
    }
    assert_eq!(node("a").is_selected(), Some(true));
    assert_eq!(node("b").is_selected(), Some(false));
    assert!(node("c").is_disabled());
    assert!(!node("a").is_disabled());
}

#[test]
fn selected_row_uses_the_secondary_container_token() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        assert_eq!(
            background(&r, "a"),
            theme.colors.secondary_container,
            "selected dark={dark}"
        );
        assert_eq!(background(&r, "b"), Color::TRANSPARENT, "dark={dark}");
        // Negative: the selected row is not the menu surface colour.
        assert_ne!(
            background(&r, "a"),
            theme.colors.surface_container_high,
            "dark={dark}"
        );
    }
}

#[test]
fn text_colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        assert_eq!(
            text_foreground(&r, "a", "Cut"),
            theme.colors.on_secondary_container,
            "selected label dark={dark}"
        );
        assert_eq!(
            text_foreground(&r, "b", "Copy"),
            theme.on_container,
            "unselected label dark={dark}"
        );
        assert_eq!(
            text_foreground(&r, "c", "Paste"),
            theme.outline,
            "disabled label dark={dark}"
        );
        assert_eq!(
            text_foreground(&r, "a", "X"),
            theme.colors.on_secondary_container,
            "selected glyph dark={dark}"
        );
    }
    let (theme, r) = render(false, 1);
    assert_eq!(
        text_foreground(&r, "a", "X"),
        theme.colors.on_surface_variant,
        "unselected glyph"
    );
    assert_eq!(
        text_foreground(&r, "a", "Ctrl+X"),
        theme.colors.on_surface_variant,
        "unselected shortcut"
    );
}

#[test]
fn leading_and_trailing_are_optional_and_ordered() {
    let (_, r) = render(false, 0);
    let a = child_names(&r, "a");
    assert_eq!(a.len(), 4, "a children: {a:?}");
    assert_eq!(a[0], "X");
    assert_eq!(a[1], "Cut");
    assert_eq!(a[3], "Ctrl+X");
    assert_eq!(child_names(&r, "b"), vec!["Copy".to_string()]);
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
        theme.page([theme.menu_at("menu", "Menu", (20., 20.), items(&theme, 0, |_| ()), ())])
    }
}

#[test]
fn press_sends_the_message_and_disabled_does_not() {
    let mut r = Runtime::new(Counter { count: 0 }).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    assert!(r.activate("b").unwrap());
    assert_eq!(r.app().count, 1);
    // A disabled item sends nothing; the call may fail or report false.
    let _ = r.activate("c");
    assert_eq!(r.app().count, 1);
}

#[derive(Clone)]
enum Msg {
    Open,
    Close,
    Pick(usize),
}

#[derive(Default)]
struct Modal {
    open: bool,
    opens: usize,
    picked: Vec<usize>,
}
impl Application for Modal {
    type Message = Msg;
    fn update(&mut self, m: Msg) {
        match m {
            Msg::Open => {
                self.open = true;
                self.opens += 1;
            }
            Msg::Close => self.open = false,
            Msg::Pick(i) => self.picked.push(i),
        }
    }
    fn view(&self) -> View<Msg> {
        let t = Theme::light();
        let mut children = vec![t.button_with(
            "open",
            "Open",
            surface_material::ButtonKind::Filled,
            false,
            Msg::Open,
        )];
        if self.open {
            children.push(t.menu_at(
                "menu",
                "Menu",
                (20., 20.),
                items(&t, 0, Msg::Pick),
                Msg::Close,
            ));
        }
        t.page(children)
    }
}

#[test]
fn arrow_keys_skip_disabled_items() {
    let mut r = Runtime::new(Modal::default()).unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r.activate("open").unwrap();
    assert_eq!(r.focused(), Some("a"), "focus moves into the menu");

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

#[test]
fn escape_dismisses_the_menu() {
    let mut r = Runtime::new(Modal::default()).unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r.activate("open").unwrap();
    assert!(r.app().open);
    assert_eq!(r.focused(), Some("a"));

    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().open);
    assert_eq!(r.focused(), Some("open"));
    assert!(r.app().picked.is_empty());
}
