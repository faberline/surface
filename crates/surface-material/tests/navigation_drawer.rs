use surface::{Color, Role, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{ButtonKind, ChoiceState, Theme};

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

/// Three destinations: `a` Inbox, `b` Sent, `c` Trash (disabled).
/// `selected` is the index of the selected destination.
fn items<M: Clone>(t: &Theme, selected: usize, message: impl Fn(usize) -> M) -> Vec<View<M>> {
    [("a", "Inbox", "I"), ("b", "Sent", "S"), ("c", "Trash", "T")]
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, icon))| {
            t.navigation_drawer_item(
                id,
                name,
                icon,
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
        t.page([t.navigation_drawer("drawer", "Destinations", items(t, selected, |_| ()))])
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

fn icon_foreground(r: &Runtime<App>, id: &str) -> Color {
    let item = r.dom().by_id(id).unwrap();
    let icon = r.dom().children(item).next().expect("icon");
    r.dom().get(icon).unwrap().style().foreground
}

#[test]
fn drawer_is_360_wide_and_items_56_high() {
    let (_, r) = render(false, 0);
    assert_eq!(bounds(&r, "drawer").width, 360.);
    for id in ["a", "b", "c"] {
        assert_eq!(bounds(&r, id).height, 56., "item {id}");
    }
}

#[test]
fn roles_and_state() {
    let (_, r) = render(false, 0);
    let drawer = r.dom().get(r.dom().by_id("drawer").unwrap()).unwrap();
    assert_eq!(drawer.role(), Role::Navigation);
    for (id, name) in [("a", "Inbox"), ("b", "Sent"), ("c", "Trash")] {
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
fn indicator_is_the_selected_row_only() {
    let (theme, r) = render(false, 0);
    assert_eq!(background(&r, "a"), theme.colors.secondary_container);
    assert_eq!(background(&r, "b"), Color::TRANSPARENT);
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        assert_eq!(
            icon_foreground(&r, "a"),
            theme.colors.on_secondary_container,
            "selected dark={dark}"
        );
        assert_eq!(
            icon_foreground(&r, "b"),
            theme.colors.on_surface_variant,
            "unselected dark={dark}"
        );
        assert_eq!(
            icon_foreground(&r, "c"),
            theme.outline,
            "disabled dark={dark}"
        );
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
        theme.page([theme.navigation_drawer("drawer", "Destinations", items(&theme, 0, |_| ()))])
    }
}

#[test]
fn press_sends_the_message() {
    let mut r = Runtime::new(Counter { count: 0 }).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    assert!(r.activate("b").unwrap());
    assert_eq!(r.app().count, 1);
    // A disabled destination sends nothing; the call may fail or report false.
    let _ = r.activate("c");
    assert_eq!(r.app().count, 1);
}

#[test]
fn selection_changes_the_output() {
    let (theme, r) = render(false, 0);
    assert_eq!(background(&r, "a"), theme.colors.secondary_container);
    assert_eq!(background(&r, "b"), Color::TRANSPARENT);

    let (theme, r) = render(false, 1);
    assert_eq!(background(&r, "b"), theme.colors.secondary_container);
    assert_eq!(background(&r, "a"), Color::TRANSPARENT);
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
        let mut children =
            vec![t.button_with("open", "Open", ButtonKind::Filled, false, Msg::Open)];
        if self.open {
            children.push(t.modal_navigation_drawer(
                "drawer",
                "Destinations",
                items(&t, 0, Msg::Pick),
                Msg::Close,
            ));
        }
        t.page(children)
    }
}

#[test]
fn modal_drawer_traps_focus_and_dismisses() {
    let mut r = Runtime::new(Modal::default()).unwrap();
    r.render(640., 480., &mut Metrics).unwrap();

    r.activate("open").unwrap();
    assert!(r.app().open);
    assert_eq!(r.focused(), Some("a"), "focus moves into the drawer");
    // The scrim blocks the control behind it.
    assert!(r.activate("open").is_err());

    // Escape closes the drawer and returns focus to its opener.
    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().open);
    assert_eq!(r.focused(), Some("open"));
    assert!(r.app().picked.is_empty());

    // A click outside the 360-wide panel dismisses it without activating
    // the control underneath.
    r.activate("open").unwrap();
    r.render(640., 480., &mut Metrics).unwrap();
    r.dispatch(Event::PointerDown { x: 500., y: 240. }).unwrap();
    r.dispatch(Event::PointerUp { x: 500., y: 240. }).unwrap();
    assert!(!r.app().open);
    assert_eq!(r.app().opens, 2);
    assert!(r.app().picked.is_empty());
    assert_eq!(r.focused(), Some("open"));
}
